use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

use crate::config::Config;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("TOML syntax error: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("missing [extensions] table")]
    MissingExtensionsTable,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ExtensionEntry {
    Ext(String),
    Glob { glob: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExtensionMap(HashMap<String, Vec<ExtensionEntry>>);

impl Deref for ExtensionMap {
    type Target = HashMap<String, Vec<ExtensionEntry>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for ExtensionMap {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let root: Config = toml::from_str(s)?;

        if root.extensions.is_empty() {
            return Err(ConfigError::MissingExtensionsTable);
        }

        Ok(root.extensions)
    }
}

impl fmt::Display for ExtensionMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let root = Config {
            extensions: self.clone(),
        };

        match toml::to_string(&root) {
            Ok(s) => f.write_str(&s),
            Err(_) => Err(fmt::Error),
        }
    }
}
