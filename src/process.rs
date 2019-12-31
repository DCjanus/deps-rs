use std::{collections::VecDeque, path::PathBuf};

use indexmap::map::IndexMap;
use semver::{Version, VersionReq};

use crate::{
    model::Identity,
    parser::{Dependency, Manifest},
    utils::AnyResult,
};

#[derive(Debug, Deserialize)]
pub struct CrateMeta {
    name: String,
    dependencies: Vec<DependencyMeta>,
    dev_dependencies: Vec<DependencyMeta>,
    build_dependencies: Vec<DependencyMeta>,
}

#[derive(Debug, Deserialize)]
pub struct DependencyMeta {
    name: String,
    version_require: VersionReq,
    latest: Version,
}

fn process_dependencies(input: IndexMap<String, crate::parser::Dependency>) -> Vec<DependencyMeta> {
    let mut result = vec![];

    for (name, dep) in input.into_iter() {
        let version_require = match dep {
            Dependency::Direct(version) => version,
            Dependency::Table { version } => version,
            _ => continue,
        };
        let latest = match crate::version::latest(&name) {
            None => continue,
            Some(x) => x,
        };

        result.push(DependencyMeta {
            name,
            version_require,
            latest,
        });
    }

    result
}

#[allow(dead_code)]
pub async fn process(
    identity: &Identity,
    rel_path: impl Into<PathBuf>,
) -> AnyResult<Vec<CrateMeta>> {
    let mut result = vec![];
    let mut rel_paths = VecDeque::new();

    rel_paths.push_back(rel_path.into());
    while let Some(rel_path) = rel_paths.pop_front() {
        let content = crate::fetch::fetch(identity, &rel_path.join("Cargo.toml")).await?;
        let manifest: Manifest = toml::from_slice(content.as_ref())?;

        if let Some(package) = manifest.package {
            result.push(CrateMeta {
                name: package.name,
                dependencies: process_dependencies(manifest.dependencies),
                dev_dependencies: process_dependencies(manifest.dev_dependencies),
                build_dependencies: process_dependencies(manifest.build_dependencies),
            });
        }
        for i in manifest.workspace.members {
            let rel_path = rel_path.join(i);
            rel_paths.push_back(rel_path);
        }
    }

    Ok(result)
}
