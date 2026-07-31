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

    fn default_source() -> String {
        "fn main() {}".to_owned()
    }

    fn parse_session() -> ParseSession {
        let src = default_source();
        ParseSession::new(rust_language(), src.to_string()).expect("parse should succeed")
    }

    fn parse_session_with_source(src: &str) -> ParseSession {
        ParseSession::new(rust_language(), src.to_string()).expect("parse should succeed")
    }

    #[test]
    fn byte_at_exact_len_is_currently_accepted() {
        let session = parse_session();
        let len = default_source().len();

        let result = session.find_node(len);
        assert!(
            result.is_ok(),
            "byte == len is currently treated as in-bounds, got {:?}",
            result
        );
    }

    #[test]
    fn byte_one_past_len_is_rejected() {
        let session = parse_session();
        let len = default_source().len();

        let err = session.find_node(len + 1).unwrap_err();
        match err {
            GrammarError::ByteOutOfBounds {
                byte,
                len: reported_len,
            } => {
                assert_eq!(byte, len + 1);
                assert_eq!(reported_len, len);
            }
            other => panic!("expected ByteOutOfBounds, got {other:?}"),
        }
    }

    #[test]
    fn byte_far_out_of_bounds_is_rejected() {
        let session = parse_session();
        let err = session.find_node(usize::MAX).unwrap_err();
        assert!(matches!(err, GrammarError::ByteOutOfBounds { .. }));
    }

    #[test]
    fn empty_source_only_accepts_byte_zero() {
        let session = parse_session_with_source("");

        assert!(session.find_node(0).is_ok(), "byte 0 on empty source");
        assert!(matches!(
            session.find_node(1).unwrap_err(),
            GrammarError::ByteOutOfBounds { byte: 1, len: 0 }
        ));
    }

    #[test]
    fn ancestors_end_at_root_and_root_has_no_parent_left_dangling() {
        let source = "fn main() { let x = 1; }";
        let session = parse_session_with_source(source);

        let byte = source.find('1').unwrap();
        let result = session.find_node(byte).unwrap();

        assert!(!result.ancestors.is_empty());
        let last = result.ancestors.last().unwrap();
        assert_eq!(
            last.kind, "source_file",
            "chain should terminate at the root node"
        );

        for pair in result.ancestors.windows(2) {
            assert_ne!(
                (
                    pair[0].kind.clone(),
                    pair[0].range.start_byte,
                    pair[0].range.end_byte
                ),
                (
                    pair[1].kind.clone(),
                    pair[1].range.start_byte,
                    pair[1].range.end_byte
                ),
                "consecutive ancestors should differ"
            );
        }
    }

    #[test]
    fn deepest_node_is_first_in_chain() {
        let source = "fn main() { let x = 1; }";
        let session = parse_session_with_source(source);

        let byte = source.find('1').unwrap();
        let result = session.find_node(byte).unwrap();

        let first = &result.ancestors[0];
        assert_eq!(first.kind, "integer_literal");
        assert_eq!(first.text, "1");
    }

    #[test]
    fn byte_between_siblings_picks_a_deterministic_node() {
        let source = "fn main() { let a = [1, 2]; }";
        let session = parse_session_with_source(source);

        let comma_byte = source.find(',').unwrap();
        let result_before = session.find_node(comma_byte).unwrap();
        let result_after = session.find_node(comma_byte + 1).unwrap();

        assert!(!result_before.ancestors.is_empty());
        assert!(!result_after.ancestors.is_empty());
    }

    #[test]
    fn whitespace_only_source() {
        let session = parse_session_with_source(&" ".repeat(3));

        for b in 0..=3 {
            assert!(session.find_node(b).is_ok(), "byte {b} should be in range");
        }
        assert!(matches!(
            session.find_node(4).unwrap_err(),
            GrammarError::ByteOutOfBounds { byte: 4, len: 3 }
        ));
    }

    #[test]
    fn malformed_source_does_not_panic() {
        let session = parse_session_with_source("fn main() { invalid !!! }");
        let len = session.tree.root_node().end_byte();

        for b in [0, len / 2, len] {
            let result = session.find_node(b);
            assert!(
                result.is_ok(),
                "byte {b} on malformed source should not panic/error"
            );
        }
    }
}
