use rmcp::schemars;
use serde::{Deserialize, Serialize};
use std::fmt;
use tree_sitter::Parser;

use crate::grammar::{LanguageEntry, error::GrammarError};

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
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

impl fmt::Display for NodeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let truncated: String = self.text.chars().take(50).collect();
        write!(
            f,
            "{}@{}..{}: {}",
            self.kind, self.start_byte, self.end_byte, truncated
        )
    }
}

impl fmt::Display for ByteRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
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
