use std::path::Path;

use bytes::Buf;
use once_cell::sync::Lazy;
use reqwest::{Client, Proxy, Response};
use sled::IVec;

use crate::{
    model::{RepoIdentity, Site},
    utils::AnyResult,
};

mod cache;

static GLOBAL_CLIENT: Lazy<Client> = Lazy::new(|| match init_client() {
    Ok(x) => x,
    Err(e) => {
        error!("init client failed: {:?}", e);
        std::process::exit(1);
    }
});

pub fn init() -> AnyResult {
    Lazy::force(&GLOBAL_CLIENT);
    self::cache::init()?;
    Ok(())
}

fn init_client() -> AnyResult<Client> {
    let mut builder = Client::builder();
    if let Some(proxy_url) = &crate::command::proxy() {
        let proxy = Proxy::all(*proxy_url)?;
        debug!("using proxy {}", proxy_url);
        builder = builder.proxy(proxy)
    }
    Ok(builder.build()?)
}

pub async fn fetch(ident: &RepoIdentity, rel_path: &Path) -> AnyResult<IVec> {
    let url = match ident.site {
        Site::GitHub => format!(
            "https://raw.githubusercontent.com/{owner}/{repo}/HEAD/{rel_path}",
            owner = ident.owner,
            repo = ident.repo,
            rel_path = rel_path.display()
        ),
        Site::GitLab => format!(
            "https://gitlab.com/{owner}/{repo}/raw/HEAD/{rel_path}",
            owner = ident.owner,
            repo = ident.repo,
            rel_path = rel_path.display()
        ),
        Site::BitBucket => format!(
            "https://bitbucket.org/{owner}/{repo}/raw/HEAD/{rel_path}",
            owner = ident.owner,
            repo = ident.repo,
            rel_path = rel_path.display()
        ),
    };

    let cache = self::cache::get(&url)?;

    let mut request = GLOBAL_CLIENT.get(&url);
    if let Some((etag, _)) = &cache {
        request = request.header(reqwest::header::IF_NONE_MATCH, etag);
    }

    trace!("fetching {}", url);
    let response: Response = request.send().await?.error_for_status()?;
    let new_etag = match response.headers().get(reqwest::header::ETAG) {
        None => (None),
        Some(x) => Some(x.to_str()?.to_string()),
    };

    if response.status().is_success() {
        let data: IVec = response.bytes().await?.bytes().into();
        if let Some(new_etag) = new_etag {
            trace!("fresh cache: {}", url);
            self::cache::set(url, data.clone(), new_etag)?;
        }
        return Ok(data);
    }

    if response.status().as_u16() == 304 {
        trace!("resource not modified: {}", url);
        let (_, data) =
            cache.ok_or_else(|| anyhow::Error::msg(format!("304 without conditional: {}", url)))?;
        return Ok(data);
    }

    Err(anyhow::Error::msg(format!(
        "unexpected response status: {:?} {:?}",
        url,
        response.status()
    )))
}
