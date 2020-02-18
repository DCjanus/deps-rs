use std::{
    collections::VecDeque,
    path::PathBuf,
    time::{Duration, Instant},
};

use futures::lock::Mutex;
use indexmap::map::IndexMap;
use lru::LruCache;
use once_cell::sync::Lazy;
use semver::{Version, VersionReq};

use crate::{
    database::{CrateMeta, DependencyKind},
    model::{RepoIdentity, Status},
    parser::{Dependency, Manifest},
    utils::AnyResult,
};

#[derive(Debug, Deserialize, Clone)]
pub struct AnalyzedCrate {
    pub name: String,
    pub dependencies: Vec<AnalyzedDependency>,
    pub dev_dependencies: Vec<AnalyzedDependency>,
    pub build_dependencies: Vec<AnalyzedDependency>,
}

impl AnalyzedCrate {
    pub fn status(&self) -> Status {
        if self.dependencies.iter().any(|x| x.is_insecure())
            || self.dev_dependencies.iter().any(|x| x.is_insecure())
            || self.build_dependencies.iter().any(|x| x.is_insecure())
        {
            return Status::Insecure;
        }

        let total =
            self.dependencies.len() + self.dev_dependencies.len() + self.build_dependencies.len();
        let total = total as u32;
        let outdated = self.dependencies.iter().filter(|x| x.is_outdated()).count()
            + self
                .dev_dependencies
                .iter()
                .filter(|x| x.is_outdated())
                .count()
            + self
                .build_dependencies
                .iter()
                .filter(|x| x.is_outdated())
                .count();
        let outdated = outdated as u32;
        Status::Normal { total, outdated }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AnalyzedDependency {
    pub name: String,
    pub required: VersionReq,
    pub latest_that_matches: Option<Version>,
    pub latest: Option<Version>,
}

impl AnalyzedDependency {
    pub fn is_outdated(&self) -> bool {
        self.latest > self.latest_that_matches
    }

    pub fn is_insecure(&self) -> bool {
        if self.latest_that_matches.is_none() {
            return false;
        }

        let version = self.latest_that_matches.as_ref().unwrap().clone();
        match crate::database::is_insecure(&self.name, version) {
            Ok(x) => x,
            Err(error) => {
                error!("failed to query audit database: {:?}", error);
                false
            }
        }
    }
}

fn analyze_dependencies(
    input: IndexMap<String, crate::parser::Dependency>,
) -> Vec<AnalyzedDependency> {
    let mut result = vec![];

    for (name, dep) in input.into_iter() {
        let required = match dep {
            Dependency::Direct(version) => version,
            Dependency::Table { version } => version,
            _ => continue,
        };
        let crates = match crate::database::get_crate_metas(&name) {
            Ok(Some(x)) => x,
            Err(error) => {
                error!("failed to get crate metadata: {:?}", error);
                continue;
            }
            Ok(None) => {
                debug!("no such crate found: {}", name);
                continue;
            }
        };

        let latest = crates
            .iter()
            .filter(|x| !x.yanked && !x.vers.is_prerelease())
            .map(|x| x.vers.clone())
            .max();

        let latest_that_matches = crates
            .iter()
            .map(|x| x.vers.clone())
            .filter(|x| required.matches(x))
            .max();

        result.push(AnalyzedDependency {
            name,
            required,
            latest_that_matches,
            latest,
        });
    }

    result
}

pub fn analyze_crate(crate_name: &str, version: Version) -> Option<AnalyzedCrate> {
    let meta: CrateMeta = crate::database::get_crate_metas(crate_name)
        .ok()??
        .into_iter()
        .find(|x| x.vers == version)?;
    let dependencies = meta
        .deps
        .iter()
        .filter(|x| x.kind == Some(DependencyKind::Normal) || x.kind == None)
        .cloned()
        .map(|x| (x.name, crate::parser::Dependency::Direct(x.req)))
        .collect();
    let dev_dependencies = meta
        .deps
        .iter()
        .filter(|x| x.kind == Some(DependencyKind::Dev))
        .cloned()
        .map(|x| (x.name, crate::parser::Dependency::Direct(x.req)))
        .collect();
    let build_dependencies = meta
        .deps
        .iter()
        .filter(|x| x.kind == Some(DependencyKind::Build))
        .cloned()
        .map(|x| (x.name, crate::parser::Dependency::Direct(x.req)))
        .collect();
    let result = AnalyzedCrate {
        name: crate_name.to_string(),
        dependencies: analyze_dependencies(dependencies),
        dev_dependencies: analyze_dependencies(dev_dependencies),
        build_dependencies: analyze_dependencies(build_dependencies),
    };
    Some(result)
}

pub async fn analyze_repo(identity: &RepoIdentity) -> AnyResult<Vec<AnalyzedCrate>> {
    static CACHE: Lazy<Mutex<LruCache<RepoIdentity, (Instant, Vec<AnalyzedCrate>)>>> =
        Lazy::new(|| Mutex::new(LruCache::new(1024)));

    {
        if let Some((created_at, cache)) = CACHE.lock().await.get(identity).cloned() {
            let created_at: Instant = created_at;
            if created_at.elapsed() < Duration::from_secs(60) {
                trace!("cache hint: {:?}", identity);
                return Ok(cache);
            }
        }
    }

    let mut result = vec![];
    let mut rel_paths = VecDeque::new();

    rel_paths.push_back(PathBuf::from(""));
    while let Some(rel_path) = rel_paths.pop_front() {
        let content = crate::fetch::fetch(identity, &rel_path.join("Cargo.toml")).await?;
        let manifest: Manifest = toml::from_slice(content.as_ref())?;

        if let Some(package) = manifest.package {
            result.push(AnalyzedCrate {
                name: package.name,
                dependencies: analyze_dependencies(manifest.dependencies),
                dev_dependencies: analyze_dependencies(manifest.dev_dependencies),
                build_dependencies: analyze_dependencies(manifest.build_dependencies),
            });
        }
        for i in manifest.workspace.members {
            let rel_path = rel_path.join(i);
            rel_paths.push_back(rel_path);
        }
    }

    {
        CACHE
            .lock()
            .await
            .put(identity.clone(), (Instant::now(), result.clone()));
    }

    Ok(result)
}
