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

    #[error("empty extension array for language: {0}")]
    EmptyExtensions(String),

    #[error("invalid glob pattern '{glob}' for language {language}: {error}")]
    InvalidGlob {
        glob: String,
        language: String,
        error: anyhow::Error,
    },
}

use crate::config::extension::ExtensionMap;
use etcetera::AppStrategy;

fn strategy() -> Result<impl etcetera::AppStrategy, ConfigError> {
    Ok(etcetera::choose_app_strategy(etcetera::AppStrategyArgs {
        app_name: "tree-sitter-mcp".to_string(),
        top_level_domain: "org".to_string(),
        author: "tree-sitter-mcp".to_string(),
    })?)
}

pub fn grammar_dir() -> Result<PathBuf, ConfigError> {
    Ok(strategy()?.data_dir().join("grammars"))
}

pub fn load() -> Result<ExtensionMap, ConfigError> {
    let config_path = strategy()?.config_dir().join("languages.toml");
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| ConfigError::ConfigRead(config_path, e))?;

    ExtensionMap::from_toml_str(&content)
}
