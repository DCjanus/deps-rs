use indexmap::map::IndexMap;

// TODO: support glob syntax in "members"
// TODO: support platform specific dependencies

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct Manifest {
    pub package: Option<Package>,
    #[serde(default)]
    pub workspace: Workspace,
    #[serde(default)]
    pub dependencies: IndexMap<String, Dependency>,
    #[serde(rename = "build-dependencies", default)]
    pub build_dependencies: IndexMap<String, Dependency>,
    #[serde(rename = "dev-dependencies", default)]
    pub dev_dependencies: IndexMap<String, Dependency>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum Dependency {
    Path { path: String },
    Git { git: String },
    CustomRegistry { registry: String },
    Direct(semver::VersionReq),
    Table { version: semver::VersionReq },
}

#[derive(Debug, Deserialize, Eq, PartialEq, Default)]
pub struct Workspace {
    members: Vec<String>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct Package {
    name: String,
}

#[test]
fn test_simple_manifest_parse() {
    let input = include_str!("../fixture/manifest.toml");
    let actual: Manifest = toml::from_str(input).unwrap();

    let mut expect_dependencies = IndexMap::new();
    expect_dependencies.insert("all".to_string(), Dependency::Direct("*".parse().unwrap()));
    expect_dependencies.insert(
        "direct1".to_string(),
        Dependency::Direct("0.1.0".parse().unwrap()),
    );
    expect_dependencies.insert(
        "direct2".to_string(),
        Dependency::Direct("=0.1.0".parse().unwrap()),
    );
    expect_dependencies.insert(
        "table1".to_string(),
        Dependency::Table {
            version: "0.1.0".parse().unwrap(),
        },
    );
    expect_dependencies.insert(
        "table2".to_string(),
        Dependency::Table {
            version: "0.1.0".parse().unwrap(),
        },
    );
    expect_dependencies.insert(
        "table3".to_string(),
        Dependency::Table {
            version: "0.1.0".parse().unwrap(),
        },
    );
    expect_dependencies.insert(
        "git".to_string(),
        Dependency::Git {
            git: "https://github.com/xxx/xxx".to_string(),
        },
    );
    expect_dependencies.insert(
        "custom-registry".to_string(),
        Dependency::CustomRegistry {
            registry: "xxx".to_string(),
        },
    );
    expect_dependencies.insert(
        "path".to_string(),
        Dependency::Path {
            path: "xxx".to_string(),
        },
    );

    let mut expect_build_dependencies = IndexMap::new();
    expect_build_dependencies.insert(
        "build-dependency".to_string(),
        Dependency::Direct("0.1.0".parse().unwrap()),
    );

    let expect = Manifest {
        package: Some(Package {
            name: "simple".to_string(),
        }),
        workspace: Workspace {
            members: vec!["a", "b", "c"]
                .into_iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>(),
        },
        dependencies: expect_dependencies,
        build_dependencies: expect_build_dependencies,
        dev_dependencies: IndexMap::new(),
    };
    assert_eq!(expect, actual);
}
