use schemars;
use serde::Serialize;
use tree_sitter::{Node, Parser};

use crate::{LanguageEntry, error::GrammarError};

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(remote = "tree_sitter::Point")]
pub struct PointDef {
    pub row: usize,
    pub column: usize,
}

#[derive(Serialize, schemars::JsonSchema)]
#[serde(remote = "tree_sitter::Range")]
pub struct RangeDef {
    pub start_byte: usize,
    pub end_byte: usize,
    #[serde(with = "PointDef")]
    pub start_point: tree_sitter::Point,
    #[serde(with = "PointDef")]
    pub end_point: tree_sitter::Point,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct NodeInfo {
    pub(crate) kind: String,
    #[serde(with = "RangeDef")]
    pub(crate) range: tree_sitter::Range,
    pub(crate) text: String,
}

pub(crate) fn node_text(node: Node<'_>, source: &str) -> String {
    node.utf8_text(source.as_bytes())
        .unwrap_or("<invalid utf8>")
        .to_string()
}

pub(crate) fn truncated_text(text: &str) -> String {
    text.chars().take(50).collect()
}

impl<'a> From<(Node<'a>, &'a str)> for NodeInfo {
    fn from((node, source): (Node<'a>, &'a str)) -> Self {
        Self {
            kind: node.kind().to_string(),
            range: node.range(),
            text: node_text(node, source),
        }
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
        let lang = entry.language()?;

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
