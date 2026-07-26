use std::collections::HashMap;
use std::path::{Path, PathBuf};

use globset::GlobBuilder;

use crate::config::extension::ExtensionEntry;
use crate::grammar::error::GrammarError;

pub(crate) struct LanguageEntry {
    pub(crate) id: String,
    pub(crate) language: Option<tree_sitter::Language>,
    pub(crate) extensions: Vec<ExtensionEntry>,
}

pub(crate) struct LanguageRegistry {
    entries: HashMap<String, LanguageEntry>,
}

impl LanguageRegistry {
    pub(crate) fn build() -> Result<Self, GrammarError> {
        use etcetera::AppStrategy;

        let strategy = etcetera::choose_app_strategy(etcetera::AppStrategyArgs {
            app_name: "tree-sitter-mcp".to_string(),
            top_level_domain: "org".to_string(),
            author: "tree-sitter-mcp".to_string(),
        })?;

        let config_path = strategy.config_dir().join("languages.toml");
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| GrammarError::ConfigRead(config_path.clone(), e))?;

        let ext_map = crate::config::extension::ExtensionMap::from_toml_str(&content)?;

        let mut entries = HashMap::new();
        for (lang, extensions) in ext_map.0.into_iter() {
            entries.insert(
                lang.clone(),
                LanguageEntry {
                    id: lang,
                    language: None,
                    extensions,
                },
            );
        }
        Ok(Self { entries })
    }

    pub(crate) fn resolve(&self, path: &str, requested: Option<&str>) -> Result<&LanguageEntry, GrammarError> {
        if let Some(id) = requested {
            return self
                .entries
                .get(id)
                .ok_or_else(|| GrammarError::UnknownLanguage(id.to_string()));
        }

        let path_buf = Path::new(path);

        if let Some(ext) = path_buf.extension().and_then(|e| e.to_str()) {
            for entry in self.entries.values() {
                if entry
                    .extensions
                    .iter()
                    .any(|e| matches!(e, ExtensionEntry::Ext(s) if s == ext))
                {
                    return Ok(entry);
                }
            }
        }

        for entry in self.entries.values() {
            for ext_entry in &entry.extensions {
                if let ExtensionEntry::Glob { glob } = ext_entry {
                    let glob = GlobBuilder::new(glob)
                        .literal_separator(true)
                        .build()?;
                    let matcher = glob.compile_matcher();
                    if matcher.is_match(path_buf) {
                        return Ok(entry);
                    }
                }
            }
        }

        Err(GrammarError::LanguageInference(PathBuf::from(path)))
    }

    pub(crate) fn loaded_ids(&self) -> Vec<&str> {
        let mut ids: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, e)| e.language.is_some())
            .map(|(k, _)| k.as_str())
            .collect();
        ids.sort_unstable();
        ids
    }

    pub(crate) fn summaries(&self) -> Vec<LanguageSummary> {
        let mut list: Vec<_> = self
            .entries
            .iter()
            .map(|(id, entry)| {
                let exts: Vec<String> = entry
                    .extensions
                    .iter()
                    .map(|e| match e {
                        ExtensionEntry::Ext(s) => format!(".{s}"),
                        ExtensionEntry::Glob { glob } => format!("{{ {glob} }}"),
                    })
                    .collect();
                LanguageSummary {
                    id: id.clone(),
                    loaded: entry.language.is_some(),
                    extensions: exts,
                }
            })
            .collect();
        list.sort_by_key(|s| s.id.clone());
        list
    }
}

pub struct LanguageSummary {
    pub id: String,
    pub loaded: bool,
    pub extensions: Vec<String>,
}
