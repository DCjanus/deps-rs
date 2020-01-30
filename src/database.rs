use std::{collections::HashSet, path::Path, sync::RwLock};

use git2::{FetchOptions, FetchPrune, ObjectType, Oid, ProxyOptions, Repository, TreeWalkMode};
use rustsec::{Collection, Database};
use semver::{Version, VersionReq};
use sled::Tree;

use crate::utils::AnyResult;

lazy_static! {
    static ref CRATE_DB: CrateDB = CrateDB::new(&crate::command::crate_db_path()).unwrap();
    static ref AUDIT_DB: RwLock<Database> = RwLock::new(Database::fetch().unwrap());
}

pub fn init() -> AnyResult {
    fn tick() -> AnyResult {
        debug!("fetching index ....");
        let begin = std::time::Instant::now();
        CRATE_DB.fetch()?;
        debug!("fetch index used {:?}", begin.elapsed());

        let begin = std::time::Instant::now();
        CRATE_DB.fresh()?;
        debug!("fresh index used {:?}", begin.elapsed());

        let begin = std::time::Instant::now();
        let new_db = rustsec::Database::fetch()?;
        *AUDIT_DB.write().unwrap() = new_db;
        debug!("fresh audit database used: {:?}", begin.elapsed());

        Ok(())
    }

    debug!("fetching audit database");
    // TODO: fetching audit database via proxy
    lazy_static::initialize(&AUDIT_DB);
    debug!("creating crate database");
    lazy_static::initialize(&CRATE_DB);
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
    CRATE_DB.get_crate_metas(crate_name)
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

impl CrateDB {
    fn new(path: &Path) -> AnyResult<Self> {
        let db = sled::open(path)?;
        let index = db.open_tree("index")?;
        let extra = db.open_tree("extra")?;

        Ok(Self { index, extra })
    }

    fn fetch(&self) -> AnyResult {
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

    fn set_last_tree(&self, oid: Oid) -> AnyResult {
        self.extra.insert("last_loaded_tree_id", oid.as_bytes())?;
        Ok(())
    }

    fn get_last_tree(&self) -> AnyResult<Option<Oid>> {
        Ok(self
            .extra
            .get("last_loaded_tree_id")?
            .map(|x| Oid::from_bytes(&*x).expect("broken 'last_loaded_tree_id'")))
    }

    fn set_crate_metas(&self, metas: Vec<CrateMeta>) -> AnyResult {
        if metas.is_empty() {
            bail!("given metas is empty");
        }

        let key = metas.first().unwrap().name.as_bytes();
        let value = bincode::serialize(&metas)?;
        self.index.insert(key, value)?;
        Ok(())
    }

    fn get_crate_metas(&self, crate_name: &str) -> AnyResult<Option<Vec<CrateMeta>>> {
        let key = crate_name.as_bytes();
        let content = match self.index.get(key)? {
            None => {
                return Ok(None);
            }
            Some(x) => x,
        };
        let result = bincode::deserialize(&*content)?;
        Ok(Some(result))
    }

    fn fresh(&self) -> AnyResult {
        let index_dir = crate::command::cache_dir();
        let repo = Repository::open_bare(&index_dir)?;
        let new_tree = repo
            .find_reference("refs/remotes/upstream/master")?
            .peel_to_tree()?;

        let old_tree_id = self.get_last_tree()?;
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
            if let Err(e) = self.set_crate_metas(content) {
                error!("failed to set crate metas {:?}", e);
            }

            0
        })?;

        self.set_last_tree(new_tree.id())?;

        Ok(())
    }
}
