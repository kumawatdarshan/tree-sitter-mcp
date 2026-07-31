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
    use rstest::rstest;

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

    #[rstest]
    #[case::extension(
        &[("rust", &[ext("rs")][..]), ("python", &[ext("py")][..])],
        "main.rs",
        None,
        Ok("rust")
    )]
    #[case::exact_filename_glob(
        &[("rust", &[ext("rs")][..]), ("toml", &[ext("toml"), glob("Cargo.lock")][..])],
        "Cargo.lock",
        None,
        Ok("toml")
    )]
    #[case::wildcard_glob(
        &[("python", &[ext("py")][..]), ("dockerfile", &[glob("Dockerfile.*")][..])],
        "Dockerfile.prod",
        None,
        Ok("dockerfile")
    )]
    #[case::glob_matches_dotted_name(
        &[("dockerfile", &[glob("Dockerfile.*")][..])],
        "Dockerfile.py",
        None,
        Ok("dockerfile")
    )]
    #[case::subdirectory_glob(
        &[("bash", &[glob("bash-completion/completions/*")][..])],
        "bash-completion/completions/docker",
        None,
        Ok("bash")
    )]
    #[case::multiple_extensions(
        &[("cpp", &[ext("cpp"), ext("h"), ext("cc")][..])],
        "main.h",
        None,
        Ok("cpp")
    )]
    #[case::distinct_extensions(
        &[("ruby", &[ext("rb")][..]), ("python", &[ext("py")][..]), ("perl", &[ext("pl"), ext("pm")][..])],
        "script.pl",
        None,
        Ok("perl")
    )]
    #[case::extension_beats_glob(
        &[("glob_only", &[glob("*.txt")][..]), ("ext_match", &[ext("txt")][..])],
        "file.txt",
        None,
        Ok("ext_match")
    )]
    #[case::explicit_request(
        &[("rust", &[ext("rs")][..]), ("toml", &[ext("toml"), glob("Cargo.lock")][..])],
        "anything.rs",
        Some("toml"),
        Ok("toml")
    )]
    #[case::unknown_explicit_request(
        &[("rust", &[ext("rs")][..])],
        "main.rs",
        Some("brainfuck"),
        Err("brainfuck")
    )]
    #[case::no_match(
        &[("rust", &[ext("rs")][..])],
        "unknown.xyz",
        None,
        Err("unknown.xyz")
    )]
    fn resolves_language(
        #[case] entries: &[(&str, &[ExtensionEntry])],
        #[case] path: &str,
        #[case] requested: Option<&str>,
        #[case] expected: Result<&str, &str>,
    ) {
        let reg = registry(entries);
        let result = resolve(&reg, path, requested);

        match expected {
            Ok(id) => assert_eq!(result.unwrap().id, id),
            Err(lang) => assert!(matches!(
                result,
                Err(GrammarError::UnknownLanguage(ref id)) if id == lang
            )),
        }
    }
}
