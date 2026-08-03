use std::fmt;
use std::ops::RangeBounds;

use schemars;
use serde::Serialize;
use tree_sitter::{Query, QueryCursor, StreamingIterator};

use crate::error::GrammarError;
use crate::session::{NodeInfo, ParseSession, apply_range, truncated_text};

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct Capture {
    pub(crate) name: String,
    pub(crate) node: NodeInfo,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct QueryMatch {
    pub(crate) pattern_index: usize,
    pub(crate) captures: Vec<Capture>,
}

impl fmt::Display for Capture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}@{}..{}: {}",
            self.name,
            self.node.range.start_byte,
            self.node.range.end_byte,
            truncated_text(&self.node.text)
        )
    }
}

impl fmt::Display for QueryMatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Pattern {}:", self.pattern_index)?;
        for capture in &self.captures {
            writeln!(f, "  {capture}")?;
        }
        Ok(())
    }
}

impl ParseSession {
    pub fn run_query<R: RangeBounds<usize>>(
        &self,
        query_str: &str,
        range: Option<R>,
    ) -> Result<Vec<QueryMatch>, GrammarError> {
        let root = apply_range(self.tree.root_node(), range);
        let query = Query::new(&self.grammar, query_str)?;

        let mut cursor = QueryCursor::new();
        let mut matches_iter = cursor.matches(&query, root, self.source.as_bytes());
        let mut matches = Vec::new();

        while let Some(m) = matches_iter.next() {
            let captures = m
                .captures
                .iter()
                .map(|c| Capture {
                    name: query.capture_names()[c.index as usize].to_string(),
                    node: NodeInfo::from((c.node, self.source.as_str())),
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
