pub(crate) mod error;
pub(crate) mod find_node;
pub(crate) mod loader;
pub(crate) mod parser;
pub(crate) mod query;
pub(crate) mod registry;

pub use error::GrammarError;
pub use find_node::FindNodeResult;
pub use parser::NodeInfo;
pub use query::{Capture, QueryMatch};
pub use registry::LanguageSummary;

use self::loader::load_grammar;
use registry::LanguageEntry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tree_sitter::Node;

#[derive(Debug)]
pub struct GrammarEngine {
    pub(crate) entries: HashMap<String, LanguageEntry>,
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

            let entry = LanguageEntry::new(&lang, language, extensions);
            entries.insert(lang, entry);
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

        if let Some(ext) = path_buf.extension().and_then(|e| e.to_str())
            && let Some(entry) = self.entries.values().find(|e| e.matches_extension(ext))
        {
            return Ok(entry);
        }

        for entry in self.entries.values() {
            if entry.matches_path(path_buf) {
                return Ok(entry);
            }
        }

        Err(GrammarError::LanguageInference(PathBuf::from(path)))
    }

    pub fn loaded_language_ids(&self) -> impl Iterator<Item = &str> {
        self.entries
            .iter()
            .filter(|(_, e)| e.is_loaded())
            .map(|(x, _)| x.as_str())
    }
    pub fn language_summaries(&self) -> Vec<LanguageSummary> {
        self.entries.values().map(LanguageSummary::from).collect()
    }
}

impl GrammarEngine {
    pub fn dump_ast<R>(
        &self,
        path: &str,
        language: Option<&str>,
        range: Option<R>,
    ) -> Result<String, GrammarError>
    where
        R: RangeBounds<usize>,
    {
        let (_source, tree) = self.load_tree(path, language)?;
        let root = apply_range(tree.root_node(), range);
        Ok(root.to_sexp())
    }
}

use std::ops::{Bound, RangeBounds};

pub(crate) fn apply_range<'a, R>(root: Node<'a>, range: Option<R>) -> Node<'a>
where
    R: RangeBounds<usize>,
{
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::extension::{ExtensionEntry, ext, glob},
        grammar::registry::entry,
    };

    fn engine(entries: &[(&str, &[ExtensionEntry])]) -> GrammarEngine {
        entries
            .iter()
            .map(|&(id, exts)| (id, entry(id, exts)))
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
