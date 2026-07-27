pub(crate) mod error;
pub(crate) mod find_node;
pub(crate) mod loader;
pub(crate) mod parser;
pub(crate) mod query;
pub(crate) mod registry;

pub use error::GrammarError;
pub use find_node::FindNodeResult;
pub use parser::{ByteRange, NodeInfo};
pub use query::{Capture, QueryMatch};
pub use registry::LanguageSummary;

use self::loader::load_grammar;
use registry::LanguageEntry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tree_sitter::Node;

pub struct GrammarEngine {
    entries: HashMap<String, LanguageEntry>,
}

impl<K, V> FromIterator<(K, V)> for GrammarEngine
where
    K: Into<String>,
    V: Into<LanguageEntry>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self {
            entries: iter
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }
}

impl GrammarEngine {
    pub fn load_default() -> Result<Self, GrammarError> {
        let ext_map = crate::config::load()?;
        let grammar_dir = crate::config::grammar_dir()?;

        let mut entries = HashMap::new();

        for (lang, extensions) in ext_map {
            let so_path = grammar_dir.join(format!("{lang}.so"));
            let language = if so_path.exists() {
                match load_grammar(&so_path, &lang) {
                    Ok(language) => Some(language),
                    Err(e) => {
                        tracing::warn!("skipping grammar {lang}: {e}");
                        None
                    }
                }
            } else {
                None
            };

            entries.insert(
                lang.clone(),
                LanguageEntry::new(&lang, language, extensions),
            );
        }

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
        let ids = self
            .entries
            .iter()
            .filter(|(_, e)| e.is_loaded())
            .map(|(x, _)| x.as_str())
            .collect();
        ids
    }

    pub fn language_summaries(&self) -> Vec<LanguageSummary> {
        let list = self
            .entries
            .iter()
            .map(|(id, entry)| LanguageSummary {
                id: id.clone(),
                loaded: entry.is_loaded(),
                extensions: entry.extensions_display(),
            })
            .collect();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::extension::ExtensionEntry;
    use globset::GlobBuilder;

    fn ext(s: &str) -> ExtensionEntry {
        ExtensionEntry::Ext(s.to_string())
    }

    fn glob(pattern: &str) -> ExtensionEntry {
        ExtensionEntry::Glob {
            glob: GlobBuilder::new(pattern)
                .literal_separator(true)
                .build()
                .unwrap(),
        }
    }

    fn language(id: &str, extensions: &[ExtensionEntry]) -> LanguageEntry {
        LanguageEntry::new(id, None, extensions.iter().cloned())
    }

    fn engine(entries: &[(&str, &[ExtensionEntry])]) -> GrammarEngine {
        entries
            .iter()
            .map(|&(id, exts)| (id, language(id, exts)))
            .collect()
    }

    #[test]
    fn explicit_request_resolves_by_id() {
        let eng = engine(&[
            ("rust", &[ext("rs")]),
            ("toml", &[ext("toml"), glob("Cargo.lock")]),
        ]);
        let result = eng.resolve("anything.rs", Some("toml")).unwrap();
        assert_eq!(result.id, "toml");
    }

    #[test]
    fn unknown_explicit_request_returns_error() {
        let eng = engine(&[("rust", &[ext("rs")])]);
        let err = eng.resolve("main.rs", Some("brainfuck")).unwrap_err();
        assert!(matches!(err, GrammarError::UnknownLanguage(ref id) if id == "brainfuck"));
    }

    #[test]
    fn infers_language_from_extension() {
        let eng = engine(&[("rust", &[ext("rs")]), ("python", &[ext("py")])]);
        let result = eng.resolve("main.rs", None).unwrap();
        assert_eq!(result.id, "rust");
    }

    #[test]
    fn infers_language_from_exact_filename_glob() {
        let eng = engine(&[
            ("rust", &[ext("rs")]),
            ("toml", &[ext("toml"), glob("Cargo.lock")]),
        ]);
        let result = eng.resolve("Cargo.lock", None).unwrap();
        assert_eq!(result.id, "toml");
    }

    #[test]
    fn infers_language_from_wildcard_glob() {
        let eng = engine(&[
            ("python", &[ext("py")]),
            ("dockerfile", &[glob("Dockerfile.*")]),
        ]);
        let result = eng.resolve("Dockerfile.prod", None).unwrap();
        assert_eq!(result.id, "dockerfile");
    }

    #[test]
    fn infers_language_from_subdirectory_glob() {
        let eng = engine(&[("bash", &[glob("bash-completion/completions/*")])]);
        let result = eng
            .resolve("bash-completion/completions/docker", None)
            .unwrap();
        assert_eq!(result.id, "bash");
    }

    #[test]
    fn extension_match_beats_glob_because_checked_first() {
        let eng = engine(&[
            ("glob_only", &[glob("*.txt")]),
            ("ext_match", &[ext("txt")]),
        ]);
        let result = eng.resolve("file.txt", None).unwrap();
        assert_eq!(result.id, "ext_match");
    }

    #[test]
    fn extension_matching_only_checks_ext_entries_not_globs() {
        let eng = engine(&[("dockerfile", &[glob("Dockerfile.*")])]);
        let result = eng.resolve("Dockerfile.py", None).unwrap();
        assert_eq!(result.id, "dockerfile");
    }

    #[test]
    fn no_match_returns_language_inference_error() {
        let eng = engine(&[("rust", &[ext("rs")])]);
        let err = eng.resolve("unknown.xyz", None).unwrap_err();
        assert!(matches!(err, GrammarError::LanguageInference(_)));
    }

    #[test]
    fn resolve_matches_multiple_extensions_for_same_language() {
        let eng = engine(&[("cpp", &[ext("cpp"), ext("h"), ext("cc")])]);
        assert_eq!(eng.resolve("main.cpp", None).unwrap().id, "cpp");
        assert_eq!(eng.resolve("main.cc", None).unwrap().id, "cpp");
        assert_eq!(eng.resolve("main.h", None).unwrap().id, "cpp");
    }

    #[test]
    fn resolve_matches_first_matching_entry_by_extension() {
        let eng = engine(&[
            ("ruby", &[ext("rb")]),
            ("python", &[ext("py")]),
            ("perl", &[ext("pl"), ext("pm")]),
        ]);
        assert_eq!(eng.resolve("script.pl", None).unwrap().id, "perl");
        assert_eq!(eng.resolve("script.py", None).unwrap().id, "python");
    }
}
