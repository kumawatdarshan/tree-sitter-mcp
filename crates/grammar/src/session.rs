use std::ops::{Bound, RangeBounds};

use schemars;
use serde::Serialize;
use tree_sitter::{Node, Parser};

use crate::error::GrammarError;

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

/// Shared node view used by query and find operations.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct NodeInfo {
    pub(crate) kind: String,
    #[serde(with = "RangeDef")]
    pub(crate) range: tree_sitter::Range,
    pub(crate) text: String,
}

impl<'a> From<(Node<'a>, &'a str)> for NodeInfo {
    fn from((node, source): (Node<'a>, &'a str)) -> Self {
        Self {
            kind: node.kind().to_string(),
            range: node.range(),
            text: node
                .utf8_text(source.as_bytes())
                .unwrap_or("<invalid utf8>")
                .to_string(),
        }
    }
}

pub(crate) fn truncated_text(text: &str) -> String {
    text.chars().take(50).collect()
}

pub(crate) fn apply_range<'a, R: RangeBounds<usize>>(root: Node<'a>, range: Option<R>) -> Node<'a> {
    match range {
        Some(r) => {
            let start = match r.start_bound() {
                Bound::Included(&s) => s,
                Bound::Excluded(&s) => s + 1,
                Bound::Unbounded => 0,
            };
            let end = match r.end_bound() {
                Bound::Included(&e) => e + 1,
                Bound::Excluded(&e) => e,
                Bound::Unbounded => usize::MAX,
            };
            root.descendant_for_byte_range(start, end).unwrap_or(root)
        }
        None => root,
    }
}

/// A parsed source file, ready for tree-sitter operations.
///
/// Created via [`ParseSession::new`] from a grammar ABI handle obtained
/// from [`crate::GrammarEngine`]; owns the source text + parse tree.
pub struct ParseSession {
    pub(crate) grammar: tree_sitter::Language,
    pub(crate) source: String,
    pub(crate) tree: tree_sitter::Tree,
}

impl ParseSession {
    pub fn new(grammar: tree_sitter::Language, source: String) -> Result<Self, GrammarError> {
        let mut parser = Parser::new();
        parser
            .set_language(&grammar)
            .map_err(GrammarError::SetLanguage)?;

        // TODO: i think it cannot return a None state due to existing checks.
        let tree = parser.parse(&source, None).unwrap();

        Ok(Self {
            grammar,
            source,
            tree,
        })
    }

    /// Dump the S-expression AST, optionally restricted to a byte range.
    pub fn dump_ast<R: RangeBounds<usize>>(&self, range: Option<R>) -> String {
        apply_range(self.tree.root_node(), range).to_sexp()
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
