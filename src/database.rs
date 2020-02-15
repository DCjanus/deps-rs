use std::{collections::HashSet, sync::RwLock};

use git2::{FetchOptions, FetchPrune, ObjectType, Oid, ProxyOptions, Repository, TreeWalkMode};
use once_cell::sync::Lazy;
use rustsec::{Collection, Database};
use semver::{Version, VersionReq};
use sled::Tree;

use crate::utils::AnyResult;

static INDEX_DB: Lazy<Tree> = Lazy::new(|| crate::command::database().open_tree("index").unwrap());
static EXTRA_DB: Lazy<Tree> = Lazy::new(|| crate::command::database().open_tree("extra").unwrap());
static AUDIT_DB: Lazy<RwLock<Database>> = Lazy::new(|| RwLock::new(Database::fetch().unwrap()));

pub fn init() -> AnyResult {
    fn tick() -> AnyResult {
        debug!("fetching index ....");
        let begin = std::time::Instant::now();
        fetch_index()?;
        debug!("fetch index used {:?}", begin.elapsed());

        let begin = std::time::Instant::now();
        fresh_index_db()?;
        debug!("fresh index used {:?}", begin.elapsed());

        let begin = std::time::Instant::now();
        let new_db = rustsec::Database::fetch()?;
        *AUDIT_DB.write().unwrap() = new_db;
        debug!("fresh audit database used: {:?}", begin.elapsed());

        Ok(())
    }

    debug!("fetching audit database");
    // TODO: fetching audit database via proxy
    Lazy::force(&AUDIT_DB);
    debug!("creating crate database");
    tick()?;

    std::thread::spawn(|| loop {
        let sleep_duration = crate::command::tick_interval();
        debug!(
            "fresh version db after {}",
            humantime::format_duration(sleep_duration)
        );
        std::thread::sleep(sleep_duration);

        if let Err(error) = tick() {
            error!("failed to fresh version db: {}", error);
        }
    });

    Ok(())
}

pub fn get_crate_metas(crate_name: &str) -> AnyResult<Option<Vec<CrateMeta>>> {
    let key = crate_name.as_bytes();
    let content = match INDEX_DB.get(key)? {
        None => {
            return Ok(None);
        }
        Some(x) => x,
    };
    let result = bincode::deserialize(&*content)?;
    Ok(Some(result))
}

pub fn is_insecure(name: &str, version: Version) -> AnyResult<bool> {
    let package: rustsec::package::Name = name.parse()?;
    let query = rustsec::database::Query::new()
        .collection(Collection::Crates)
        .package_version(package, version);

    let safe = AUDIT_DB.read().unwrap().query(&query).is_empty();
    Ok(!safe)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CrateMeta {
    pub name: String,
    pub vers: Version,
    pub yanked: bool,
    pub deps: Vec<DependencyMeta>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DependencyMeta {
    pub name: String,
    pub req: VersionReq,
    pub kind: Option<DependencyKind>,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub registry: Option<String>,
    pub package: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    Normal,
    Build,
    Dev,
}

struct CrateDB {
    index: Tree,
    extra: Tree,
}

fn fetch_index() -> AnyResult {
    let index_dir = crate::command::cache_dir();
    if !index_dir.exists() {
        std::fs::create_dir_all(&index_dir)?;
        debug!("created index directory {}", index_dir.display());
    }

    let repo = Repository::init_bare(&index_dir)?;
    if repo.find_remote("upstream").is_err() {
        repo.remote("upstream", crate::command::index_url())?;
        debug!("created remote: {}", crate::command::index_url());
    }
    repo.remote_set_url("upstream", crate::command::index_url())?;

    let mut proxy_option = ProxyOptions::new();
    if let Some(proxy_url) = &crate::command::proxy() {
        proxy_option.url(proxy_url);
    } else {
        proxy_option.auto();
    }

    let mut fetch_option = FetchOptions::new();
    fetch_option.prune(FetchPrune::On);
    fetch_option.proxy_options(proxy_option);

    repo.find_remote("upstream")?.fetch(
        &["+refs/heads/master:refs/remotes/upstream/master"],
        Some(&mut fetch_option),
        None,
    )?;

    Ok(())
}

fn fresh_index_db() -> AnyResult {
    fn set_crate_metas(metas: Vec<CrateMeta>) -> AnyResult {
        if metas.is_empty() {
            bail!("given metas is empty");
        }

        let key = metas.first().unwrap().name.as_bytes();
        let value = bincode::serialize(&metas)?;
        INDEX_DB.insert(key, value)?;
        Ok(())
    }

    const LAST_LOADED_TREE_ID: &str = "last_loaded_tree_id";

    let index_dir = crate::command::cache_dir();
    let repo = Repository::open_bare(&index_dir)?;
    let new_tree = repo
        .find_reference("refs/remotes/upstream/master")?
        .peel_to_tree()?;

    let old_tree_id = EXTRA_DB
        .get(LAST_LOADED_TREE_ID)?
        .map(|x| Oid::from_bytes(&*x).expect("broken 'last_loaded_tree_id'"));
    if Some(new_tree.id()) == old_tree_id {
        trace!("skip whole crates index");
        return Ok(());
    }

    let mut old_ids = HashSet::new();
    if let Some(old_tree_id) = old_tree_id {
        repo.find_tree(old_tree_id)?
            .walk(TreeWalkMode::PostOrder, |_, entry| {
                old_ids.insert(entry.id());
                0
            })?;
    }

    new_tree.walk(TreeWalkMode::PostOrder, |pwd, entry| {
        if pwd.is_empty() || entry.kind() != Some(ObjectType::Blob) {
            return 0;
        }
        if old_ids.contains(&entry.id()) {
            trace!("skip {}/{}", pwd, entry.name().unwrap());
            return 1;
        }
        let blob = match repo.find_blob(entry.id()) {
            Ok(x) => x,
            Err(e) => {
                error!(
                    "failed to find blob: {}, pwd: {} entry: {:?} id: {}",
                    e,
                    pwd,
                    entry.name(),
                    entry.id()
                );
                return 0;
            }
        };

        let content = blob
            .content()
            .split(|x| *x == b'\n')
            .map(|line| serde_json::from_slice::<CrateMeta>(line))
            .filter_map(|x| x.ok())
            .collect::<Vec<_>>();
        if content.is_empty() {
            error!("no valid crate meta found in {}/{:?}", pwd, entry.name());
            return 0;
        }

        if let Err(e) = set_crate_metas(content) {
            error!("failed to set crate metas {:?}", e);
        }

        0
    })?;

    EXTRA_DB.insert(LAST_LOADED_TREE_ID, new_tree.id().as_bytes())?;

    Ok(())
}
