use crate::config::extension::ExtensionMap;

pub(crate) mod extension;

/// Top-level configuration matching the TOML file layout.
///
/// ```toml
/// grammar_dir = "~/.local/share/tree-sitter-mcp/runtime"
///
/// [extensions]
/// rs = ["rust"]
/// py = ["python"]
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Config {
    /// Extension → grammar name mapping.
    #[serde(default)]
    pub extensions: ExtensionMap,
}
