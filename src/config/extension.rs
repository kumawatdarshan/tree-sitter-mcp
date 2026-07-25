use crate::config::ConfigError;
use std::collections::HashMap;

use globset::GlobBuilder;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ExtensionEntry {
    #[serde(deserialize_with = "deserialize_glob")]
    Glob { glob: String },

    #[serde(deserialize_with = "deserialize_ext")]
    Ext(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "ExtensionMapWrapper")]
pub struct ExtensionMap(pub HashMap<String, Vec<ExtensionEntry>>);

#[derive(Deserialize)]
struct ExtensionMapWrapper {
    extensions: HashMap<String, Vec<ExtensionEntry>>,
}

impl TryFrom<ExtensionMapWrapper> for ExtensionMap {
    type Error = ConfigError;

    fn try_from(wire: ExtensionMapWrapper) -> Result<Self, Self::Error> {
        for (lang_key, entries) in &wire.extensions {
            if entries.is_empty() {
                return Err(ConfigError::EmptyExtensions(lang_key.clone()));
            }
        }
        Ok(ExtensionMap(wire.extensions))
    }
}

impl ExtensionMap {
    pub fn from_toml_str(s: &str) -> Result<Self, ConfigError> {
        let wire: ExtensionMapWrapper = toml::from_str(s).map_err(|e| match e.message() {
            m if m.contains("missing field `extensions`") => ConfigError::MissingExtensionsTable,
            _ => e.into(),
        })?;

        Self::try_from(wire)
    }
}

impl std::ops::Deref for ExtensionMap {
    type Target = HashMap<String, Vec<ExtensionEntry>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn deserialize_ext<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    if s.contains(['/', '*', '?', '[']) || s.is_empty() {
        return Err(serde::de::Error::custom("Invalid plain extension string"));
    }
    Ok(s)
}

fn deserialize_glob<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct GlobHelper {
        glob: String,
    }

    let helper = GlobHelper::deserialize(deserializer)?;

    GlobBuilder::new(&helper.glob)
        .build()
        .map_err(|e| serde::de::Error::custom(e.to_string()))?;

    Ok(helper.glob)
}
