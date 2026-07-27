use rmcp::schemars;
use serde::{Deserialize, Serialize};
use tree_sitter::Parser;

use crate::grammar::{LanguageEntry, error::GrammarError};

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
pub struct ByteRange {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct NodeInfo {
    pub(crate) kind: String,
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
    pub(crate) start_point: (usize, usize),
    pub(crate) end_point: (usize, usize),
    pub(crate) text: String,
}

impl super::GrammarEngine {
    pub(crate) fn load_tree(
        &self,
        path: &str,
        language: Option<&str>,
    ) -> Result<(String, tree_sitter::Tree), GrammarError> {
        let entry = self.resolve(path, language)?;
        self.load_tree_for_entry(entry, path)
    }

    pub(crate) fn load_tree_for_entry(
        &self,
        entry: &LanguageEntry,
        path: &str,
    ) -> Result<(String, tree_sitter::Tree), GrammarError> {
        let lang = entry
            .language
            .as_ref()
            .ok_or_else(|| GrammarError::GrammarNotLoaded(entry.id.clone()))?;

        let source = std::fs::read_to_string(path)
            .map_err(|e| GrammarError::SourceRead(std::path::PathBuf::from(path), e))?;

        let mut parser = Parser::new();
        parser
            .set_language(lang)
            .map_err(GrammarError::SetLanguage)?;

        let tree = parser
            .parse(&source, None)
            .ok_or(GrammarError::ParseReturnedNoTree)?;

        Ok((source, tree))
    }
}
