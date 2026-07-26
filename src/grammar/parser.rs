use rmcp::schemars;
use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser};

use crate::grammar::error::GrammarError;

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct NodeInfo {
    pub kind: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_point: (usize, usize),
    pub end_point: (usize, usize),
    pub text: String,
}

impl super::GrammarEngine {
    pub(crate) fn load_tree(
        &self,
        path: &str,
        language: Option<&str>,
    ) -> Result<(String, tree_sitter::Tree), GrammarError> {
        let entry = self.registry.resolve(path, language)?;

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

    pub(crate) fn apply_range<'a>(
        root: Node<'a>,
        range: Option<&ByteRange>,
    ) -> Node<'a> {
        match range {
            Some(r) => root
                .descendant_for_byte_range(r.start, r.end)
                .unwrap_or(root),
            None => root,
        }
    }

    pub(crate) fn node_info(node: Node<'_>, source: &str) -> NodeInfo {
        NodeInfo {
            kind: node.kind().to_string(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            start_point: (node.start_position().row, node.start_position().column),
            end_point: (node.end_position().row, node.end_position().column),
            text: node
                .utf8_text(source.as_bytes())
                .unwrap_or("<invalid utf8>")
                .to_string(),
        }
    }
}
