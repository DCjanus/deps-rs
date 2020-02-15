use std::path::Path;

use bytes::Bytes;
use once_cell::sync::Lazy;
use reqwest::{Client, Proxy};

use crate::{
    model::{RepoIdentity, Site},
    utils::AnyResult,
};

static GLOBAL_CLIENT: Lazy<Client> = Lazy::new(|| match init_client() {
    Ok(x) => x,
    Err(e) => {
        error!("init client failed: {:?}", e);
        std::process::exit(1);
    }
});

pub fn init() -> AnyResult {
    Lazy::force(&GLOBAL_CLIENT);
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

pub async fn fetch(ident: &RepoIdentity, rel_path: &Path) -> AnyResult<Bytes> {
    // TODO: cache on disk?

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
    trace!("fetching {}", url);
    Ok(GLOBAL_CLIENT.get(&url).send().await?.bytes().await?)
}
