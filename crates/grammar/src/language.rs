use std::collections::HashMap;
use std::path::Path;

use config::extension::ExtensionEntry;

use crate::error::GrammarError;

#[derive(Debug, Clone)]
pub struct LoadedLanguage {
    pub id: String,
    pub extensions: Vec<ExtensionEntry>,
    pub language: tree_sitter::Language,
}

impl LoadedLanguage {
    pub(crate) fn matches_extension(&self, ext: &str) -> bool {
        self.extensions
            .iter()
            .any(|e| matches!(e, ExtensionEntry::Ext(s) if s == ext))
    }

    pub(crate) fn matches_path(&self, path: &Path) -> bool {
        self.extensions.iter().any(|entry| {
            matches!(entry, ExtensionEntry::Glob { glob }
                if glob.compile_matcher().is_match(path))
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageSummary {
    id: String,
    extensions: Vec<String>,
}

impl std::fmt::Display for LanguageSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.id, self.extensions.join(", "))
    }
}

impl From<&LoadedLanguage> for LanguageSummary {
    fn from(lang: &LoadedLanguage) -> Self {
        Self {
            id: lang.id.clone(),
            extensions: lang.extensions.iter().map(|e| e.to_string()).collect(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct GrammarRegistry {
    by_id: HashMap<String, LoadedLanguage>,
}

impl GrammarRegistry {
    pub(crate) fn new(languages: Vec<LoadedLanguage>) -> Self {
        Self {
            by_id: languages.into_iter().map(|l| (l.id.clone(), l)).collect(),
        }
    }

    pub(crate) fn get(&self, id: &str) -> Option<&LoadedLanguage> {
        self.by_id.get(id)
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &LoadedLanguage> {
        self.by_id.values()
    }
}

/// Resolve a file path (+ optional explicit id) to a loaded language.
pub(crate) fn resolve<'r>(
    registry: &'r GrammarRegistry,
    path: &str,
    requested: Option<&str>,
) -> Result<&'r LoadedLanguage, GrammarError> {
    if let Some(id) = requested {
        return registry
            .get(id)
            .ok_or_else(|| GrammarError::UnknownLanguage(id.to_string()));
    }

    let p = Path::new(path);

    if let Some(ext) = p.extension().and_then(|e| e.to_str())
        && let Some(lang) = registry.values().find(|l| l.matches_extension(ext))
    {
        return Ok(lang);
    }

    registry
        .values()
        .find(|l| l.matches_path(p))
        .ok_or_else(|| GrammarError::UnknownLanguage(path.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::extension::{ext, glob};

    fn lang(id: &str, exts: &[ExtensionEntry]) -> LoadedLanguage {
        LoadedLanguage {
            id: id.to_string(),
            extensions: exts.to_vec(),
            language: tree_sitter_rust::LANGUAGE.into(),
        }
    }

    fn registry(entries: &[(&str, &[ExtensionEntry])]) -> GrammarRegistry {
        GrammarRegistry::new(entries.iter().map(|&(id, exts)| lang(id, exts)).collect())
    }

    #[test]
    fn explicit_request_resolves_by_id() {
        let reg = registry(&[
            ("rust", &[ext("rs")]),
            ("toml", &[ext("toml"), glob("Cargo.lock")]),
        ]);
        let result = resolve(&reg, "anything.rs", Some("toml")).unwrap();
        assert_eq!(result.id, "toml");
    }

    #[test]
    fn unknown_explicit_request_returns_error() {
        let reg = registry(&[("rust", &[ext("rs")])]);
        let err = resolve(&reg, "main.rs", Some("brainfuck")).unwrap_err();
        assert!(matches!(err, GrammarError::UnknownLanguage(ref id) if id == "brainfuck"));
    }

    #[test]
    fn infers_language_from_extension() {
        let reg = registry(&[("rust", &[ext("rs")]), ("python", &[ext("py")])]);
        let result = resolve(&reg, "main.rs", None).unwrap();
        assert_eq!(result.id, "rust");
    }

    #[test]
    fn infers_language_from_exact_filename_glob() {
        let reg = registry(&[
            ("rust", &[ext("rs")]),
            ("toml", &[ext("toml"), glob("Cargo.lock")]),
        ]);
        let result = resolve(&reg, "Cargo.lock", None).unwrap();
        assert_eq!(result.id, "toml");
    }

    #[test]
    fn infers_language_from_wildcard_glob() {
        let reg = registry(&[
            ("python", &[ext("py")]),
            ("dockerfile", &[glob("Dockerfile.*")]),
        ]);
        let result = resolve(&reg, "Dockerfile.prod", None).unwrap();
        assert_eq!(result.id, "dockerfile");
    }

    #[test]
    fn infers_language_from_subdirectory_glob() {
        let reg = registry(&[("bash", &[glob("bash-completion/completions/*")])]);
        let result = resolve(&reg, "bash-completion/completions/docker", None).unwrap();
        assert_eq!(result.id, "bash");
    }

    #[test]
    fn extension_match_beats_glob_because_checked_first() {
        let reg = registry(&[
            ("glob_only", &[glob("*.txt")]),
            ("ext_match", &[ext("txt")]),
        ]);
        let result = resolve(&reg, "file.txt", None).unwrap();
        assert_eq!(result.id, "ext_match");
    }

    #[test]
    fn extension_matching_only_checks_ext_entries_not_globs() {
        let reg = registry(&[("dockerfile", &[glob("Dockerfile.*")])]);
        let result = resolve(&reg, "Dockerfile.py", None).unwrap();
        assert_eq!(result.id, "dockerfile");
    }

    #[test]
    fn no_match_returns_language_inference_error() {
        let reg = registry(&[("rust", &[ext("rs")])]);
        let err = resolve(&reg, "unknown.xyz", None).unwrap_err();
        assert!(matches!(err, GrammarError::UnknownLanguage(_)));
    }

    #[test]
    fn resolve_matches_multiple_extensions_for_same_language() {
        let reg = registry(&[("cpp", &[ext("cpp"), ext("h"), ext("cc")])]);
        assert_eq!(resolve(&reg, "main.cpp", None).unwrap().id, "cpp");
        assert_eq!(resolve(&reg, "main.cc", None).unwrap().id, "cpp");
        assert_eq!(resolve(&reg, "main.h", None).unwrap().id, "cpp");
    }

    #[test]
    fn resolve_matches_first_matching_entry_by_extension() {
        let reg = registry(&[
            ("ruby", &[ext("rb")]),
            ("python", &[ext("py")]),
            ("perl", &[ext("pl"), ext("pm")]),
        ]);
        assert_eq!(resolve(&reg, "script.pl", None).unwrap().id, "perl");
        assert_eq!(resolve(&reg, "script.py", None).unwrap().id, "python");
    }
}
