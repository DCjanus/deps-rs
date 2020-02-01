use crate::analyze::AnalyzedDependency;

#[derive(Debug)]
pub struct DepData {
    pub name: String,
    pub required: String,
    pub latest: String,
    pub outdated: bool,
    pub insecure: bool,
}

impl From<AnalyzedDependency> for DepData {
    fn from(source: AnalyzedDependency) -> Self {
        Self {
            required: source.required.to_string(),
            latest: source
                .latest
                .as_ref()
                .map(|x| x.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            outdated: source.is_outdated(),
            insecure: source.is_insecure(),
            name: source.name,
        }
    }
}
