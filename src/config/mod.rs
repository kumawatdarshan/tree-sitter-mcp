pub(crate) mod extension;

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to choose config directory")]
    ConfigDir(#[from] etcetera::HomeDirError),

    #[error("failed to read config file {0}")]
    ConfigRead(PathBuf, #[source] std::io::Error),

    #[error("TOML syntax error: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("missing [extensions] table")]
    MissingExtensionsTable,

    #[error("duplicate language key: {0}")]
    DuplicateKey(String),

    #[error("empty extension array for language: {0}")]
    EmptyExtensions(String),

    #[error(r#"bare basename not allowed (use {{ glob = "..." }}): {0} in language {1}"#)]
    BareBasename(String, String),

    #[error("invalid glob pattern '{glob}' for language {language}: {error}")]
    InvalidGlob {
        glob: String,
        language: String,
        error: anyhow::Error,
    },

    #[error("{0}")]
    Unknown(String),
}

use crate::config::extension::ExtensionMap;

pub fn load() -> Result<ExtensionMap, ConfigError> {
    use etcetera::AppStrategy;

    let strategy = etcetera::choose_app_strategy(etcetera::AppStrategyArgs {
        app_name: "tree-sitter-mcp".to_string(),
        top_level_domain: "org".to_string(),
        author: "tree-sitter-mcp".to_string(),
    })?;

    let config_path = strategy.config_dir().join("languages.toml");
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| ConfigError::ConfigRead(config_path, e))?;

    ExtensionMap::from_toml_str(&content)
}
