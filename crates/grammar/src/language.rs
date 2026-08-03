use std::fmt;
use std::path::Path;

use config::extension::ExtensionEntry;

use crate::error::GrammarError;

/// The canonical identity of a configured language.
///
/// The id is exactly as declared in config (e.g. `c-sharp`); it is the
/// single identity for a language. Wire and filename/symbol forms are
/// projections of this id, never alternate identities.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, schemars::JsonSchema,
)]
pub struct LanguageId(String);

impl LanguageId {
    pub fn new(id: impl Into<String>) -> Result<Self, GrammarError> {
        let s = id.into();
        if s.is_empty() {
            return Err(GrammarError::UnknownLanguage(s));
        }
        Ok(Self(s))
    }

    #[cfg(test)]
    pub(crate) fn new_unchecked(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The dlopen constructor symbol, `tree_sitter_<id>` with `-` mapped
    /// to `_` (e.g. `c-sharp` -> `tree_sitter_c_sharp`).
    pub fn constructor_symbol(&self) -> String {
        format!("tree_sitter_{}", self.0.replace('-', "_"))
    }

    /// The grammar library filename in the grammar directory.
    pub fn library_filename(&self) -> String {
        format!("{}.{}", self.0, dylib_extension())
    }
}

impl fmt::Display for LanguageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for LanguageId {
    type Error = GrammarError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl std::str::FromStr for LanguageId {
    type Err = GrammarError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

pub(crate) fn dylib_extension() -> &'static str {
    if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    }
}

/// A configured language. Owns the id + extensions (the declarative
/// truth) and an optional lazily-loaded grammar ABI handle.
#[derive(Debug, Clone)]
pub struct Language {
    id: LanguageId,
    extensions: Vec<ExtensionEntry>,
    grammar: Option<tree_sitter::Language>,
}

impl Language {
    pub(crate) fn new(id: LanguageId, extensions: Vec<ExtensionEntry>) -> Self {
        Self {
            id,
            extensions,
            grammar: None,
        }
    }

    pub fn loaded(
        id: LanguageId,
        extensions: Vec<ExtensionEntry>,
        grammar: tree_sitter::Language,
    ) -> Self {
        Self {
            id,
            extensions,
            grammar: Some(grammar),
        }
    }

    pub fn id(&self) -> &LanguageId {
        &self.id
    }

    pub fn extensions(&self) -> &[ExtensionEntry] {
        &self.extensions
    }

    pub(crate) fn grammar(&self) -> Option<&tree_sitter::Language> {
        self.grammar.as_ref()
    }

    pub(crate) fn set_grammar(&mut self, grammar: tree_sitter::Language) {
        self.grammar = Some(grammar);
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use config::extension::{ext, glob};
    use rstest::rstest;

    fn spec(id: &str, exts: &[ExtensionEntry]) -> Language {
        Language::new(LanguageId::new_unchecked(id), exts.to_vec())
    }

    fn specs(entries: &[(&str, &[ExtensionEntry])]) -> Vec<Language> {
        entries.iter().map(|&(id, exts)| spec(id, exts)).collect()
    }

    #[rstest]
    #[case::extension(
        &[("rust", &[ext("rs")][..]), ("python", &[ext("py")][..])],
        "main.rs",
        "rust"
    )]
    #[case::exact_filename_glob(
        &[("rust", &[ext("rs")][..]), ("toml", &[ext("toml"), glob("Cargo.lock")][..])],
        "Cargo.lock",
        "toml"
    )]
    #[case::wildcard_glob(
        &[("python", &[ext("py")][..]), ("dockerfile", &[glob("Dockerfile.*")][..])],
        "Dockerfile.prod",
        "dockerfile"
    )]
    #[case::glob_matches_dotted_name(
        &[("dockerfile", &[glob("Dockerfile.*")][..])],
        "Dockerfile.py",
        "dockerfile"
    )]
    #[case::subdirectory_glob(
        &[("bash", &[glob("bash-completion/completions/*")][..])],
        "bash-completion/completions/docker",
        "bash"
    )]
    #[case::multiple_extensions(
        &[("cpp", &[ext("cpp"), ext("h"), ext("cc")][..])],
        "main.h",
        "cpp"
    )]
    #[case::distinct_extensions(
        &[("ruby", &[ext("rb")][..]), ("python", &[ext("py")][..]), ("perl", &[ext("pl"), ext("pm")][..])],
        "script.pl",
        "perl"
    )]
    #[case::extension_beats_glob(
        &[("glob_only", &[glob("*.txt")][..]), ("ext_match", &[ext("txt")][..])],
        "file.txt",
        "ext_match"
    )]
    fn finds_matching_spec(
        #[case] entries: &[(&str, &[ExtensionEntry])],
        #[case] path: &str,
        #[case] expected: &str,
    ) {
        let specs = specs(entries);
        let p = Path::new(path);
        let matched = specs
            .iter()
            .find(|s| {
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    s.matches_extension(ext)
                } else {
                    false
                }
            })
            .or_else(|| specs.iter().find(|s| s.matches_path(p)));
        assert_eq!(
            matched.map(|s| s.id().to_string()),
            Some(expected.to_string())
        );
    }

    #[rstest]
    #[case("rust", "tree_sitter_rust")]
    #[case("c-sharp", "tree_sitter_c_sharp")]
    #[case("markdown_inline", "tree_sitter_markdown_inline")]
    fn constructor_symbol_maps_dashes_and_underscores(#[case] input: &str, #[case] expected: &str) {
        let id = LanguageId::new_unchecked(input);
        assert_eq!(id.constructor_symbol(), expected);
    }

    #[rstest]
    #[case("rust", "rust")]
    #[case("c-sharp", "c-sharp")]
    fn library_filename_bears_id(#[case] input: &str, #[case] expected_id: &str) {
        let id = LanguageId::new_unchecked(input);
        let name = id.library_filename();
        assert!(name.starts_with(expected_id));
        assert!(name.ends_with(id.library_filename().trim_start_matches(expected_id)));
    }
}
