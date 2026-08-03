use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::language::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Members,
    References,
    Implementors,
    Modules,
    TypeAlias,
    TestFixtures,
    Complexity,
    EditPlan,
}

/// What a client needs to know about one language: its display name, the
/// file extensions used to infer it, and the capabilities its query
/// directory actually supports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LanguageInfo {
    pub name: String,
    pub extensions: Vec<String>,
    pub capabilities: Vec<Capability>,
}

impl From<&Language> for LanguageInfo {
    fn from(language: &Language) -> Self {
        Self {
            name: language.id().to_string(),
            extensions: language
                .extensions()
                .iter()
                .map(ToString::to_string)
                .collect(),
            capabilities: vec![],
        }
    }
}

/// Per-language outcome of a capabilities request. Distinguishes why a
/// requested language is unavailable so the client can self-correct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LanguageStatus {
    Loaded { info: LanguageInfo },
    NotConfigured { suggestions: Vec<String> },
    LoadFailed { language: String, reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::extension::ext;

    #[test]
    fn language_info_projects_from_language() {
        let language = Language::loaded(
            crate::LanguageId::new("rust").unwrap(),
            vec![ext("rs")],
            tree_sitter_rust::LANGUAGE.into(),
        );

        let info = LanguageInfo::from(&language);

        assert_eq!(info.name, "rust");
        assert_eq!(info.extensions, vec![".rs"]);
        assert!(info.capabilities.is_empty());
    }
}
