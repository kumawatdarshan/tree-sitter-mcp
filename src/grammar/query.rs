use rmcp::schemars;
use serde::Serialize;
use tree_sitter::{Query, QueryCursor, StreamingIterator};

use crate::grammar::{apply_range, error::GrammarError, node_text};

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct Capture {
    pub(crate) name: String,
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
    pub(crate) start_point: (usize, usize),
    pub(crate) end_point: (usize, usize),
    pub(crate) text: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct QueryMatch {
    pub(crate) pattern_index: usize,
    pub(crate) captures: Vec<Capture>,
}

impl super::GrammarEngine {
    pub fn run_query(
        &self,
        path: &str,
        language: Option<&str>,
        query_str: &str,
        range: Option<&super::parser::ByteRange>,
    ) -> Result<Vec<QueryMatch>, GrammarError> {
        let entry = self.resolve(path, language)?;
        let (source, tree) = self.load_tree_for_entry(entry, path)?;
        let root = apply_range(tree.root_node(), range);

        let lang = entry
            .language
            .as_ref()
            .ok_or_else(|| GrammarError::GrammarNotLoaded(entry.id.clone()))?;

        let query = Query::new(lang, query_str)?;

        let mut cursor = QueryCursor::new();
        let mut matches_iter = cursor.matches(&query, root, source.as_bytes());
        let mut matches = Vec::new();

        while let Some(m) = matches_iter.next() {
            let captures = m
                .captures
                .iter()
                .map(|c| Capture {
                    name: query.capture_names()[c.index as usize].to_string(),
                    start_byte: c.node.start_byte(),
                    end_byte: c.node.end_byte(),
                    start_point: (c.node.start_position().row, c.node.start_position().column),
                    end_point: (c.node.end_position().row, c.node.end_position().column),
                    text: node_text(c.node, &source),
                })
                .collect();
            matches.push(QueryMatch {
                pattern_index: m.pattern_index,
                captures,
            });
        }

        Ok(matches)
    }
}
