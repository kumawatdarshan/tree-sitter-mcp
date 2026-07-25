use crate::config::extension::ExtensionMap;

pub(crate) mod extension;

/// Currently Implemented only
/// ```toml
/// [extensions]
/// string = string[]
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Config {
    /// Extension → grammar name mapping.
    #[serde(default)]
    pub extensions: ExtensionMap,
}
