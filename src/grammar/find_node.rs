use rmcp::schemars;
use serde::Serialize;
use std::fmt;

use crate::grammar::error::GrammarError;

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct FindNodeResult {
    pub(crate) ancestors: Vec<super::parser::NodeInfo>,
}

impl fmt::Display for FindNodeResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Ancestor chain ({} nodes):", self.ancestors.len())?;
        for (i, node) in self.ancestors.iter().rev().enumerate() {
            let indent = "  ".repeat(i);
            writeln!(f, "{indent}{node}")?;
        }
        Ok(())
    }
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

        let mut node = root.descendant_for_byte_range(byte, byte).unwrap_or(root);

        let mut ancestors = Vec::new();
        loop {
            ancestors.push((node, source.as_str()).into());
            match node.parent() {
                Some(parent) => node = parent,
                None => break,
            }
        }

        Ok(FindNodeResult { ancestors })
    }
}
