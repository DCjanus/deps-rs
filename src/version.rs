use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, MutexGuard, RwLock},
};

use git2::{FetchOptions, FetchPrune, ObjectType, Oid, ProxyOptions, Repository, TreeWalkMode};
use semver::Version;

use crate::utils::AnyResult;

lazy_static! {
    static ref VERSION_DB: RwLock<HashMap<String, Version>> = Default::default();
}

pub fn init() -> AnyResult {
    fn tick() -> AnyResult {
        let begin = std::time::Instant::now();
        sync_index()?;
        debug!("sync index took {:?}", begin.elapsed());

        let begin = std::time::Instant::now();
        let new_db = load_index()?;
        debug!("load index took {:?}", begin.elapsed());

        let begin = std::time::Instant::now();
        let mut db = VERSION_DB.write().unwrap();
        db.extend(new_db);
        debug!("fresh version db took {:?}", begin.elapsed());

        Ok(())
    }
    lazy_static::initialize(&VERSION_DB);
    tick()?;

    std::thread::spawn(|| loop {
        let sleep_duration = crate::command::COMMAND.interval;
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

pub fn latest(crate_name: &str) -> Option<Version> {
    VERSION_DB.read().unwrap().get(crate_name).cloned()
}

fn load_index() -> AnyResult<HashMap<String, Version>> {
    lazy_static! {
        static ref LAST_TREE: Mutex<Oid> = Mutex::new(Oid::zero());
    }

    #[derive(Debug, Deserialize)]
    struct VersionMeta {
        name: String,
        vers: Version,
        yanked: bool,
    }

    let index_dir = crate::command::COMMAND.cache.join("crates.io-index");
    let repo = Repository::open_bare(&index_dir)?;
    let new_tree = repo
        .find_reference("refs/remotes/upstream/master")?
        .peel_to_tree()?;

    let mut old_tree_id: MutexGuard<Oid> = LAST_TREE.lock().unwrap();
    if new_tree.id() == *old_tree_id {
        trace!("skip whole crates index");
        return Ok(Default::default());
    }

    let mut old_ids = HashSet::new();
    if let Ok(old_tree) = repo.find_tree(*old_tree_id) {
        old_tree.walk(TreeWalkMode::PostOrder, |_, entry| {
            old_ids.insert(entry.id());
            0
        })?;
    }

    // XXX: only scan changed files
    let mut versions = HashMap::new();
    new_tree.walk(TreeWalkMode::PreOrder, |pwd, entry| {
        if old_ids.contains(&entry.id()) {
            debug!("skip {}/{}", pwd, entry.name().unwrap());
            return 1;
        }
        if entry.kind() != Some(ObjectType::Blob) {
            return 0;
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

        if let Some(latest) = blob
            .content()
            .split(|x| *x == b'\n')
            .rev()
            .map(|line| serde_json::from_slice::<VersionMeta>(line))
            .filter_map(|x| x.ok())
            .filter(|x| !x.yanked && !x.vers.is_prerelease())
            .max_by(|x, y| x.vers.cmp(&y.vers))
        {
            versions.insert(latest.name, latest.vers);
        }

        0
    })?;

    *old_tree_id = new_tree.id();
    Ok(versions)
}

fn sync_index() -> AnyResult {
    let index_dir = crate::command::COMMAND.cache.join("crates.io-index");
    if !index_dir.exists() {
        std::fs::create_dir_all(&index_dir)?;
        debug!("created index directory {}", index_dir.display());
    }

    let repo = Repository::init_bare(&index_dir)?;
    if repo.find_remote("upstream").is_err() {
        repo.remote("upstream", crate::command::COMMAND.index.as_str())?;
        debug!("created remote: {}", crate::command::COMMAND.index);
    }

    let mut proxy_option = ProxyOptions::new();
    if let Some(proxy_url) = &crate::command::COMMAND.proxy {
        proxy_option.url(proxy_url.as_str());
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