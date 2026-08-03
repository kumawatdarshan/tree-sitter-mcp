use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

/// Per-language outcome of a capabilities request. Distinguishes why a
/// requested language is unavailable so the client can self-correct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LanguageStatus {
    Loaded { info: LanguageInfo },
    NotConfigured { suggestions: Vec<String> },
    LoadFailed { language: String, reason: String },
}
