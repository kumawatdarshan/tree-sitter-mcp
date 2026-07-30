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

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn rust_root(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .expect("rust language should load");
        parser.parse(source, None).expect("source should parse")
    }

    #[test]
    fn node_info_captures_kind_range_and_text() {
        let source = "fn main() {}";
        let tree = rust_root(source);
        let root_node = tree.root_node();
        let node = root_node
            .descendant_for_byte_range(3, 7)
            .expect("identifier should be found");

        let info = NodeInfo::from((node, source));

        assert_eq!(info.kind, "identifier");
        assert_eq!(info.text, "main");
        assert_eq!(info.range.start_byte, 3);
        assert_eq!(info.range.end_byte, 7);
    }
}
