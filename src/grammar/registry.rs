use std::path::Path;

use crate::config::extension::ExtensionEntry;
use crate::grammar::error::GrammarError;

#[derive(Debug)]
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

    pub(super) fn matches_extension(&self, ext: &str) -> bool {
        self.extensions
            .iter()
            .any(|e| matches!(e, ExtensionEntry::Ext(s) if s == ext))
    }

    pub(super) fn matches_path(&self, path: &Path) -> Result<bool, GrammarError> {
        for ext_entry in &self.extensions {
            if let ExtensionEntry::Glob { glob } = ext_entry
                && glob.compile_matcher().is_match(path)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(super) fn extensions_display(&self) -> Vec<String> {
        self.extensions
            .iter()
            .map(|e| match e {
                ExtensionEntry::Ext(s) => format!(".{s}"),
                ExtensionEntry::Glob { glob } => format!("{{ {} }}", glob.glob()),
            })
            .collect()
    }
}

pub struct LanguageSummary {
    pub(crate) id: String,
    pub(crate) loaded: bool,
    pub(crate) extensions: Vec<String>,
}

impl LanguageSummary {
    pub fn new<I>(id: &str, loaded: bool, exts: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        Self {
            id: id.into(),
            loaded,
            extensions: exts.into_iter().map(Into::into).collect(),
        }
    }

    pub fn display_line(&self) -> String {
        if self.loaded {
            format!("{}: {}", self.id, self.extensions.join(", "))
        } else {
            format!(
                "{}: {} (grammar not loaded)",
                self.id,
                self.extensions.join(", ")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::extension::ExtensionEntry;
    use globset::GlobBuilder;
    use std::path::Path;

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

    fn entry(id: &str, extensions: &[ExtensionEntry]) -> LanguageEntry {
        LanguageEntry {
            id: id.to_string(),
            language: None,
            extensions: extensions.to_vec(),
        }
    }

    #[test]
    fn matches_path_for_exact_filename_glob() {
        let lang = entry("toml", &[glob("Cargo.lock")]);
        assert!(lang.matches_path(Path::new("Cargo.lock")).unwrap());
    }

    #[test]
    fn exact_filename_glob_does_not_match_unrelated_path() {
        let lang = entry("toml", &[glob("Cargo.lock")]);
        assert!(!lang.matches_path(Path::new("main.rs")).unwrap());
    }

    #[test]
    fn wildcard_glob_matches_variant_filenames() {
        let lang = entry("dockerfile", &[glob("Dockerfile.*")]);
        assert!(lang.matches_path(Path::new("Dockerfile.prod")).unwrap());
        assert!(lang.matches_path(Path::new("Dockerfile.dev")).unwrap());
        assert!(!lang.matches_path(Path::new("Dockerfile")).unwrap());
    }

    #[test]
    fn subdirectory_glob_literal_separator() {
        let lang = entry("systemd", &[glob("systemd/**/*.conf")]);
        assert!(
            lang.matches_path(Path::new("systemd/system/my.service.conf"))
                .unwrap()
        );
        assert!(!lang.matches_path(Path::new("etc/systemd/my.conf")).unwrap());
    }

    #[test]
    fn brace_glob_matches_alternatives() {
        let lang = entry("jsonc", &[glob("{t,j}sconfig.json")]);
        assert!(lang.matches_path(Path::new("tsconfig.json")).unwrap());
        assert!(lang.matches_path(Path::new("jsconfig.json")).unwrap());
        assert!(!lang.matches_path(Path::new("tsconfig")).unwrap());
    }

    #[test]
    fn dotfile_glob_matches_hidden_file() {
        let lang = entry("env", &[glob(".env"), glob(".env.*")]);
        assert!(lang.matches_path(Path::new(".env")).unwrap());
        assert!(lang.matches_path(Path::new(".env.production")).unwrap());
        assert!(!lang.matches_path(Path::new("env")).unwrap());
    }

    #[test]
    fn ext_only_entry_does_not_match_path_via_glob() {
        let lang = entry("rust", &[ext("rs"), ext("rsx")]);
        assert!(!lang.matches_path(Path::new("main.rs")).unwrap());
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
        assert!(
            !lang
                .matches_path(Path::new("other/completions/docker"))
                .unwrap()
        );
        assert!(
            lang.matches_path(Path::new("bash-completion/completions/docker"))
                .unwrap()
        );
    }

    #[test]
    fn extensions_display_formats_ext_and_glob() {
        let lang = entry("toml", &[ext("toml"), glob("Cargo.lock"), glob("pdm.lock")]);
        assert_eq!(
            lang.extensions_display(),
            &[".toml", "{ Cargo.lock }", "{ pdm.lock }"]
        );
    }

    #[test]
    fn extensions_display_formats_globs_only() {
        let lang = entry("gomod", &[glob("go.mod")]);
        assert_eq!(lang.extensions_display(), &["{ go.mod }"]);
    }

    #[test]
    fn display_line_shows_loaded_language() {
        let summary = LanguageSummary::new("rust", true, [".rs", ".rsx"]);
        assert_eq!(summary.display_line(), "rust: .rs, .rsx");
    }

    #[test]
    fn display_line_shows_unloaded_language() {
        let summary = LanguageSummary::new("brainfuck", false, [".b", ".bf"]);
        assert_eq!(
            summary.display_line(),
            "brainfuck: .b, .bf (grammar not loaded)"
        );
    }
}
