use std::path::Path;

use globset::GlobBuilder;

use crate::config::extension::ExtensionEntry;
use crate::grammar::error::GrammarError;

pub(crate) struct LanguageEntry {
    pub(crate) id: String,
    pub(crate) language: Option<tree_sitter::Language>,
    pub(crate) extensions: Vec<ExtensionEntry>,
}

impl LanguageEntry {
    pub(super) fn is_loaded(&self) -> bool {
        self.language.is_some()
    }

    pub(super) fn matches_extension(&self, ext: &str) -> bool {
        self.extensions
            .iter()
            .any(|e| matches!(e, ExtensionEntry::Ext(s) if s == ext))
    }

    pub(super) fn matches_path(&self, path: &Path) -> Result<bool, GrammarError> {
        for ext_entry in &self.extensions {
            if let ExtensionEntry::Glob { glob } = ext_entry {
                let compiled = GlobBuilder::new(glob).literal_separator(true).build()?;
                if compiled.compile_matcher().is_match(path) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub(super) fn extensions_display(&self) -> Vec<String> {
        self.extensions
            .iter()
            .map(|e| match e {
                ExtensionEntry::Ext(s) => format!(".{s}"),
                ExtensionEntry::Glob { glob } => format!("{{ {glob} }}"),
            })
            .collect()
    }
}

pub struct LanguageSummary {
    pub(crate) id: String,
    pub(crate) loaded: bool,
    pub(crate) extensions: Vec<String>,
}

impl LanguageSummary {
    pub fn display_line(&self) -> String {
        if self.loaded {
            format!("{}: {}", self.id, self.extensions.join(", "))
        } else {
            format!(
                "{}: {} (grammar not loaded)",
                self.id,
                self.extensions.join(", ")
            )
        }
    }
}
