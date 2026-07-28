use std::fmt;
use std::path::Path;

use crate::config::extension::ExtensionEntry;
use crate::grammar::error::GrammarError;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LanguageEntry {
    pub(crate) id: String,
    pub(crate) language: Option<tree_sitter::Language>,
    pub(crate) extensions: Vec<ExtensionEntry>,
}

impl LanguageEntry {
    pub(crate) fn new<I, E>(
        id: &str,
        language: Option<tree_sitter::Language>,
        extensions: I,
    ) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<ExtensionEntry>,
    {
        Self {
            id: id.into(),
            language,
            extensions: extensions.into_iter().map(Into::into).collect(),
        }
    }

    pub(super) fn is_loaded(&self) -> bool {
        self.language.is_some()
    }

    pub(super) fn language(&self) -> Result<&tree_sitter::Language, GrammarError> {
        self.language
            .as_ref()
            .ok_or_else(|| GrammarError::GrammarNotLoaded(self.id.clone()))
    }

    pub(super) fn matches_extension(&self, ext: &str) -> bool {
        self.extensions
            .iter()
            .any(|e| matches!(e, ExtensionEntry::Ext(s) if s == ext))
    }

    pub(super) fn matches_path(&self, path: &Path) -> bool {
        for ext_entry in &self.extensions {
            if let ExtensionEntry::Glob { glob } = ext_entry
                && glob.compile_matcher().is_match(path)
            {
                return true;
            }
        }
        false
    }
}

impl fmt::Display for LanguageEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let exts: Vec<String> = self.extensions.iter().map(|e| e.to_string()).collect();
        write!(f, "{}: {}", self.id, exts.join(", "))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageSummary {
    pub(crate) id: String,
    pub(crate) loaded: bool,
    pub(crate) extensions: Vec<String>,
}

impl LanguageSummary {
    pub fn new<I>(id: &str, loaded: bool, extensions: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        Self {
            id: id.into(),
            loaded,
            extensions: extensions.into_iter().map(Into::into).collect(),
        }
    }
}

impl fmt::Display for LanguageSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let exts = self.extensions.join(", ");
        if self.loaded {
            write!(f, "{}: {exts}", self.id)
        } else {
            write!(f, "{}: {exts} (grammar not loaded)", self.id)
        }
    }
}

impl From<&LanguageEntry> for LanguageSummary {
    fn from(entry: &LanguageEntry) -> Self {
        Self {
            id: entry.id.clone(),
            loaded: entry.is_loaded(),
            extensions: entry.extensions.iter().map(|e| e.to_string()).collect(),
        }
    }
}

#[cfg(test)]
pub(crate) fn entry(id: &str, extensions: &[ExtensionEntry]) -> LanguageEntry {
    LanguageEntry::new(id, None, extensions.iter().cloned())
}

#[cfg(test)]
mod tests {
    use crate::config::extension::{ext, glob};

    use super::*;
    use std::path::Path;

    #[test]
    fn matches_path_for_exact_filename_glob() {
        let lang = entry("toml", &[glob("Cargo.lock")]);
        assert!(lang.matches_path(Path::new("Cargo.lock")));
    }

    #[test]
    fn exact_filename_glob_does_not_match_unrelated_path() {
        let lang = entry("toml", &[glob("Cargo.lock")]);
        assert!(!lang.matches_path(Path::new("main.rs")));
    }

    #[test]
    fn wildcard_glob_matches_variant_filenames() {
        let lang = entry("dockerfile", &[glob("Dockerfile.*")]);
        assert!(lang.matches_path(Path::new("Dockerfile.prod")));
        assert!(lang.matches_path(Path::new("Dockerfile.dev")));
        assert!(!lang.matches_path(Path::new("Dockerfile")));
    }

    #[test]
    fn subdirectory_glob_literal_separator() {
        let lang = entry("systemd", &[glob("systemd/**/*.conf")]);
        assert!(lang.matches_path(Path::new("systemd/system/my.service.conf")));
        assert!(!lang.matches_path(Path::new("etc/systemd/my.conf")));
    }

    #[test]
    fn brace_glob_matches_alternatives() {
        let lang = entry("jsonc", &[glob("{t,j}sconfig.json")]);
        assert!(lang.matches_path(Path::new("tsconfig.json")));
        assert!(lang.matches_path(Path::new("jsconfig.json")));
        assert!(!lang.matches_path(Path::new("tsconfig")));
    }

    #[test]
    fn dotfile_glob_matches_hidden_file() {
        let lang = entry("env", &[glob(".env"), glob(".env.*")]);
        assert!(lang.matches_path(Path::new(".env")));
        assert!(lang.matches_path(Path::new(".env.production")));
        assert!(!lang.matches_path(Path::new("env")));
    }

    #[test]
    fn ext_only_entry_does_not_match_path_via_glob() {
        let lang = entry("rust", &[ext("rs"), ext("rsx")]);
        assert!(!lang.matches_path(Path::new("main.rs")));
    }

    #[test]
    fn matches_extension_picks_ext_entries_only() {
        let lang = entry("ruby", &[ext("rb"), glob("Rakefile")]);
        assert!(lang.matches_extension("rb"));
        assert!(!lang.matches_extension("Rakefile"));
    }

    #[test]
    fn glob_does_not_match_path_for_different_subdirectory() {
        let lang = entry("bash", &[glob("bash-completion/completions/*")]);
        assert!(!lang.matches_path(Path::new("other/completions/docker")));
        assert!(lang.matches_path(Path::new("bash-completion/completions/docker")));
    }

    #[test]
    fn display_formats_ext_and_glob() {
        let lang = entry("toml", &[ext("toml"), glob("Cargo.lock"), glob("pdm.lock")]);
        let formatted: Vec<String> = lang.extensions.iter().map(|e| e.to_string()).collect();
        assert_eq!(formatted, &[".toml", "{ Cargo.lock }", "{ pdm.lock }"]);
    }

    #[test]
    fn display_formats_globs_only() {
        let lang = entry("gomod", &[glob("go.mod")]);
        let formatted: Vec<String> = lang.extensions.iter().map(|e| e.to_string()).collect();
        assert_eq!(formatted, &["{ go.mod }"]);
    }

    #[test]
    fn display_line_shows_loaded_language() {
        let summary = LanguageSummary::new("rust", true, [".rs", ".rsx"]);
        assert_eq!(summary.to_string(), "rust: .rs, .rsx");
    }

    #[test]
    fn display_line_shows_unloaded_language() {
        let summary = LanguageSummary::new("brainfuck", false, [".b", ".bf"]);
        assert_eq!(
            summary.to_string(),
            "brainfuck: .b, .bf (grammar not loaded)"
        );
    }
}
