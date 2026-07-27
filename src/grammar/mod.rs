pub(crate) mod error;
pub(crate) mod find_node;
pub(crate) mod parser;
pub(crate) mod query;
pub(crate) mod registry;

pub use error::GrammarError;
pub use find_node::FindNodeResult;
pub use parser::{ByteRange, NodeInfo};
pub use query::{Capture, QueryMatch};
pub use registry::LanguageSummary;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tree_sitter::Node;

use registry::LanguageEntry;

pub struct GrammarEngine {
    entries: HashMap<String, LanguageEntry>,
}

impl GrammarEngine {
    pub fn load_default() -> Result<Self, GrammarError> {
        let ext_map = crate::config::load()?;

        let entries = ext_map
            .into_iter()
            .map(|(lang, extensions)| {
                (
                    lang.clone(),
                    LanguageEntry {
                        id: lang,
                        language: None,
                        extensions,
                    },
                )
            })
            .collect();

        Ok(Self { entries })
    }

    pub(crate) fn resolve(
        &self,
        path: &str,
        requested: Option<&str>,
    ) -> Result<&LanguageEntry, GrammarError> {
        if let Some(id) = requested {
            return self
                .entries
                .get(id)
                .ok_or_else(|| GrammarError::UnknownLanguage(id.to_string()));
        }

        let path_buf = Path::new(path);

        if let Some(ext) = path_buf.extension().and_then(|e| e.to_str()) {
            if let Some(entry) = self.entries.values().find(|e| e.matches_extension(ext)) {
                return Ok(entry);
            }
        }

        for entry in self.entries.values() {
            if entry.matches_path(path_buf)? {
                return Ok(entry);
            }
        }

        Err(GrammarError::LanguageInference(PathBuf::from(path)))
    }

    pub fn loaded_language_ids(&self) -> Vec<&str> {
        let mut ids: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, e)| e.is_loaded())
            .map(|(k, _)| k.as_str())
            .collect();
        ids.sort_unstable();
        ids
    }

    pub fn language_summaries(&self) -> Vec<LanguageSummary> {
        let mut list: Vec<_> = self
            .entries
            .iter()
            .map(|(id, entry)| LanguageSummary {
                id: id.clone(),
                loaded: entry.is_loaded(),
                extensions: entry.extensions_display(),
            })
            .collect();
        list.sort_by_key(|s| s.id.clone());
        list
    }

    pub fn dump_ast(
        &self,
        path: &str,
        language: Option<&str>,
        range: Option<&ByteRange>,
    ) -> Result<String, GrammarError> {
        let (_source, tree) = self.load_tree(path, language)?;
        let root = apply_range(tree.root_node(), range);
        Ok(root.to_sexp())
    }
}

pub(crate) fn node_text(node: Node<'_>, source: &str) -> String {
    node.utf8_text(source.as_bytes())
        .unwrap_or("<invalid utf8>")
        .to_string()
}

pub(crate) fn apply_range<'a>(root: Node<'a>, range: Option<&ByteRange>) -> Node<'a> {
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
        text: node_text(node, source),
    }
}
