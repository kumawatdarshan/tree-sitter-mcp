use std::path::Path;

use config::extension::ExtensionEntry;

#[derive(Debug, Clone)]
pub struct LoadedLanguage {
    pub id: String,
    pub extensions: Vec<ExtensionEntry>,
    pub language: tree_sitter::Language,
}

/// Declared-in-config language spec. Availability without I/O.
#[derive(Debug, Clone)]
pub(crate) struct LanguageSpec {
    pub id: String,
    pub extensions: Vec<ExtensionEntry>,
}

impl LanguageSpec {
    pub(crate) fn new(id: &str, extensions: Vec<ExtensionEntry>) -> Self {
        Self {
            id: id.to_owned(),
            extensions,
        }
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

impl From<&LoadedLanguage> for LanguageSpec {
    fn from(lang: &LoadedLanguage) -> Self {
        Self {
            id: lang.id.clone(),
            extensions: lang.extensions.clone(),
        }
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

impl From<&LanguageSpec> for LanguageSummary {
    fn from(spec: &LanguageSpec) -> Self {
        Self {
            id: spec.id.clone(),
            extensions: spec.extensions.iter().map(|e| e.to_string()).collect(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use config::extension::{ext, glob};
    use rstest::rstest;

    fn spec(id: &str, exts: &[ExtensionEntry]) -> LanguageSpec {
        LanguageSpec::new(id, exts.to_vec())
    }

    fn specs(entries: &[(&str, &[ExtensionEntry])]) -> Vec<LanguageSpec> {
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
        assert_eq!(matched.map(|s| s.id.as_str()), Some(expected));
    }
}
