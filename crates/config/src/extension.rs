use super::ConfigError;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExtensionEntry {
    Glob {
        #[serde(
            serialize_with = "serialize_glob",
            deserialize_with = "deserialize_glob"
        )]
        glob: globset::Glob,
    },
    #[serde(deserialize_with = "deserialize_plain_ext")]
    Ext(String),
}

impl<S> From<S> for ExtensionEntry
where
    S: Into<String>,
{
    #[inline]
    fn from(ext: S) -> Self {
        Self::Ext(ext.into())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionMap(HashMap<String, Vec<ExtensionEntry>>);

impl ExtensionMap {
    pub fn from_toml_str(s: &str) -> Result<Self, ConfigError> {
        let map: Self = toml::from_str(s)?;
        for (lang_key, entries) in &map.0 {
            if entries.is_empty() {
                return Err(ConfigError::EmptyExtensions(lang_key.clone()));
            }
        }
        Ok(map)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn get(&self, lang_key: &str) -> Option<&Vec<ExtensionEntry>> {
        self.0.get(lang_key)
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

fn serialize_glob<S>(glob: &globset::Glob, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(glob.glob())
}

fn deserialize_glob<'de, D>(deserializer: D) -> Result<globset::Glob, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    globset::GlobBuilder::new(&s)
        .literal_separator(true)
        .build()
        .map_err(de::Error::custom)
}

fn deserialize_plain_ext<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.contains(['/', '*', '?', '[']) || s.is_empty() {
        return Err(de::Error::custom("Invalid plain extension string"));
    }
    Ok(s)
}

impl fmt::Display for ExtensionEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtensionEntry::Ext(s) => write!(f, ".{s}"),
            ExtensionEntry::Glob { glob } => write!(f, "{{ {} }}", glob.glob()),
        }
    }
}

pub fn ext(s: &str) -> ExtensionEntry {
    ExtensionEntry::from(s)
}

pub fn glob(pattern: &str) -> ExtensionEntry {
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
    use rstest::rstest;

    #[test]
    fn rejects_empty_extension_array() {
        let result = ExtensionMap::from_toml_str("ruby = []");
        assert!(matches!(
            result,
            Err(ConfigError::EmptyExtensions(ref lang)) if lang == "ruby"
        ));
    }

    #[test]
    fn rejects_invalid_value_type() {
        let result = ExtensionMap::from_toml_str("foo = \"bar\"");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_glob_pattern() {
        let result = ExtensionMap::from_toml_str("x = [{ glob = \"[\" }]");
        assert!(result.is_err());
    }

    #[rstest]
    #[case::path_separator("a/b")]
    #[case::star_glob("*.rs")]
    #[case::question_mark("?")]
    #[case::bracket("[")]
    #[case::empty("")]
    fn rejects_plain_ext(#[case] ext: &str) {
        let result = ExtensionMap::from_toml_str(&format!("x = [\"{ext}\"]"));
        assert!(result.is_err());
    }

    #[test]
    fn parses_iterates_and_measures() {
        let toml_str = r#"
rust = ["rs", "rsx"]
python = [{ glob = "*.py" }]
"#;
        let map = ExtensionMap::from_toml_str(toml_str).unwrap();
        assert_eq!(map.len(), 2);
        assert!(!map.is_empty());

        let mut pairs: Vec<_> = map.into_iter().collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));

        assert_eq!(pairs[0].0, "python");
        assert_eq!(pairs[0].1.len(), 1);
        assert!(matches!(
            pairs[0].1[0],
            ExtensionEntry::Glob { ref glob } if glob.glob() == "*.py"
        ));

        assert_eq!(pairs[1].0, "rust");
        assert_eq!(pairs[1].1.len(), 2);
        assert!(matches!(pairs[1].1[0], ExtensionEntry::Ext(ref s) if s == "rs"));
        assert!(matches!(pairs[1].1[1], ExtensionEntry::Ext(ref s) if s == "rsx"));
    }
}
