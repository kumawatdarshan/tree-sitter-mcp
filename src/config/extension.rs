use crate::config::ConfigError;
use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser::SerializeMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionEntry {
    Glob { glob: globset::Glob },
    Ext(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "ExtensionMapWrapper")]
pub struct ExtensionMap(HashMap<String, Vec<ExtensionEntry>>);

#[derive(Deserialize)]
struct ExtensionMapWrapper {
    extensions: Option<HashMap<String, Vec<ExtensionEntry>>>,
}

impl TryFrom<ExtensionMapWrapper> for ExtensionMap {
    type Error = ConfigError;

    fn try_from(wire: ExtensionMapWrapper) -> Result<Self, Self::Error> {
        let extensions = wire.extensions.ok_or(ConfigError::MissingExtensionsTable)?;
        for (lang_key, entries) in &extensions {
            if entries.is_empty() {
                return Err(ConfigError::EmptyExtensions(lang_key.clone()));
            }
        }
        Ok(ExtensionMap(extensions))
    }
}

impl ExtensionMap {
    pub(crate) fn from_toml_str(s: &str) -> Result<Self, ConfigError> {
        let wire: ExtensionMapWrapper = toml::from_str(s)?;
        Self::try_from(wire)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl IntoIterator for ExtensionMap {
    type Item = (String, Vec<ExtensionEntry>);
    type IntoIter = std::collections::hash_map::IntoIter<String, Vec<ExtensionEntry>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Serialize for ExtensionEntry {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            ExtensionEntry::Glob { glob } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("glob", glob.glob())?;
                map.end()
            }
            ExtensionEntry::Ext(s) => s.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ExtensionEntry {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            GlobObj { glob: String },
            Plain(String),
        }

        match Raw::deserialize(deserializer)? {
            Raw::GlobObj { glob } => {
                let compiled = globset::GlobBuilder::new(&glob)
                    .literal_separator(true)
                    .build()
                    .map_err(de::Error::custom)?;
                Ok(ExtensionEntry::Glob { glob: compiled })
            }
            Raw::Plain(s) => {
                if s.contains(['/', '*', '?', '[']) || s.is_empty() {
                    return Err(de::Error::custom("Invalid plain extension string"));
                }
                Ok(ExtensionEntry::Ext(s))
            }
        }
    }
}

impl fmt::Display for ExtensionEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtensionEntry::Ext(s) => write!(f, ".{s}"),
            ExtensionEntry::Glob { glob } => write!(f, "{{ {} }}", glob.glob()),
        }
    }
}

#[cfg(test)]
pub(crate) fn ext(s: &str) -> ExtensionEntry {
    ExtensionEntry::Ext(s.to_string())
}

#[cfg(test)]
pub(crate) fn glob(pattern: &str) -> ExtensionEntry {
    use globset::GlobBuilder;
    ExtensionEntry::Glob {
        glob: GlobBuilder::new(pattern)
            .literal_separator(true)
            .build()
            .unwrap(),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn rejects_empty_extension_array() {
        let result = ExtensionMap::from_toml_str(
            r#"
[extensions]
ruby = []
"#,
        );
        assert!(matches!(
            result,
            Err(ConfigError::EmptyExtensions(ref lang)) if lang == "ruby"
        ));
    }

    #[test]
    fn rejects_missing_extensions_table() {
        let result = ExtensionMap::from_toml_str(
            r#"
foo = "bar"
"#,
        );
        assert!(matches!(result, Err(ConfigError::MissingExtensionsTable)));
    }

    #[test]
    fn rejects_invalid_glob_pattern() {
        let result = ExtensionMap::from_toml_str(
            r#"
[extensions]
x = [{ glob = "[" }]
"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_path_separator_in_ext() {
        let result = ExtensionMap::from_toml_str(
            r#"
[extensions]
x = ["a/b"]
"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_glob_chars_in_ext() {
        let result = ExtensionMap::from_toml_str(
            r#"
[extensions]
x = ["*.rs"]
"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_empty_ext_string() {
        let result = ExtensionMap::from_toml_str(
            r#"
[extensions]
x = [""]
"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn parses_valid_ext_and_glob_entries() {
        let toml_str = r#"
[extensions]
rust = ["rs", "rsx"]
python = [{ glob = "*.py" }]
"#;
        let map = ExtensionMap::from_toml_str(toml_str).unwrap();
        assert_eq!(map.len(), 2);
        assert!(!map.is_empty());
    }

    #[test]
    fn yields_correct_items_via_into_iter() {
        let toml_str = r#"
[extensions]
rust = ["rs"]
python = [{ glob = "*.py" }]
"#;
        let map = ExtensionMap::from_toml_str(toml_str).unwrap();
        let mut pairs: Vec<_> = map.into_iter().collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));

        assert_eq!(pairs.len(), 2);

        assert_eq!(pairs[0].0, "python");
        assert_eq!(pairs[0].1.len(), 1);
        assert!(
            matches!(pairs[0].1[0], ExtensionEntry::Glob { ref glob } if glob.glob() == "*.py")
        );

        assert_eq!(pairs[1].0, "rust");
        assert_eq!(pairs[1].1.len(), 1);
        assert!(matches!(pairs[1].1[0], ExtensionEntry::Ext(ref s) if s == "rs"));
    }

    #[test]
    fn len_and_is_empty() {
        let toml_str = r#"
[extensions]
rust = ["rs"]
"#;
        let map = ExtensionMap::from_toml_str(toml_str).unwrap();
        assert_eq!(map.len(), 1);
        assert!(!map.is_empty());

        let empty = ExtensionMap::default();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }
}
