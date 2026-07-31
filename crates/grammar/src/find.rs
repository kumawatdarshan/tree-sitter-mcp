use serde::Serialize;

use crate::error::GrammarError;
use crate::session::{NodeInfo, ParseSession};

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct FindNodeResult {
    pub(crate) ancestors: Vec<NodeInfo>,
}

impl ParseSession {
    pub fn find_node(&self, byte: usize) -> Result<FindNodeResult, GrammarError> {
        let root = self.tree.root_node();
        let len = root.end_byte();
        if byte > len {
            return Err(GrammarError::ByteOutOfBounds { byte, len });
        }

        // TODO: i think it cannot return a None state due to existing checks.
        let node = root.descendant_for_byte_range(byte, byte).unwrap();

        let ancestors = collect_ancestors(node, &self.source);

        Ok(FindNodeResult { ancestors })
    }
}

/// Walk from `node` up through its ancestors (inclusive), collecting
/// `NodeInfo` for each, ending at (and including) the root.
fn collect_ancestors(node: tree_sitter::Node, source: &str) -> Vec<NodeInfo> {
    let mut ancestors = Vec::new();
    let mut current = Some(node);
    while let Some(n) = current {
        ancestors.push(NodeInfo::from((n, source)));
        current = n.parent();
    }
    ancestors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LoadedLanguage;

    fn rust_language() -> LoadedLanguage {
        LoadedLanguage {
            id: "rust".to_string(),
            extensions: vec![],
            language: tree_sitter_rust::LANGUAGE.into(),
        }
    }

    fn parse_session(source: &str) -> ParseSession {
        ParseSession::new(rust_language(), source.to_string()).expect("parse should succeed")
    }

    #[test]
    fn chain_starts_deepest_and_ends_at_root() {
        let source = "fn main() { let x = 1; }";
        let session = parse_session(source);

        let byte = source.find('1').unwrap();
        let result = session.find_node(byte).unwrap();

        assert!(!result.ancestors.is_empty());
        let first = &result.ancestors[0];
        assert_eq!(first.kind, "integer_literal");
        assert_eq!(first.text, "1");

        let last = result.ancestors.last().unwrap();
        assert_eq!(
            last.kind, "source_file",
            "chain should terminate at the root node"
        );
    }
}
