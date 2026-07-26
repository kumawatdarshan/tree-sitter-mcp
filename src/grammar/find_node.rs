use rmcp::schemars;
use serde::Serialize;

use crate::grammar::error::GrammarError;

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct FindNodeResult {
    pub ancestors: Vec<super::parser::NodeInfo>,
}

impl super::GrammarEngine {
    pub fn find_node(
        &self,
        path: &str,
        language: Option<&str>,
        byte: usize,
    ) -> Result<FindNodeResult, GrammarError> {
        let (source, tree) = self.load_tree(path, language)?;
        let root = tree.root_node();

        if byte > root.end_byte() {
            return Err(GrammarError::ByteOutOfBounds {
                byte,
                len: root.end_byte(),
            });
        }

        let mut node = root
            .descendant_for_byte_range(byte, byte)
            .unwrap_or(root);

        let mut ancestors = Vec::new();
        loop {
            ancestors.push(Self::node_info(node, &source));
            match node.parent() {
                Some(parent) => node = parent,
                None => break,
            }
        }

        Ok(FindNodeResult { ancestors })
    }
}
