use std::collections::HashMap;
use std::path::{Path, PathBuf};

use config::extension::{ExtensionEntry, ExtensionMap};

use crate::error::LoadGrammarError;
use crate::language::LoadedLanguage;

/// Intermediate type — declared in config, grammar not yet resolved.
#[derive(Debug, Clone)]
pub(crate) struct LanguageSpec {
    pub id: String,
    pub extensions: Vec<ExtensionEntry>,
}

impl LanguageSpec {
    fn new(id: &str, extensions: impl IntoIterator<Item = ExtensionEntry>) -> Self {
        Self {
            id: id.to_owned(),
            extensions: extensions.into_iter().collect(),
        }
    }
}

pub(crate) fn specs_from_config(map: ExtensionMap) -> Vec<LanguageSpec> {
    map.into_iter()
        .map(|(id, extensions)| LanguageSpec { id, extensions })
        .collect()
}

fn dylib_extension() -> &'static str {
    if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    }
}

fn grammar_key(id: &str) -> String {
    id.replace('-', "_")
}

fn symbol_for_stem(stem: &str) -> String {
    format!("tree_sitter_{}", grammar_key(stem))
}

fn load_language(path: &Path, symbol: &str) -> Result<tree_sitter::Language, LoadGrammarError> {
    let library = dlopen2::symbor::Library::open(path).map_err(LoadGrammarError::Open)?;

    type LibConstructor = unsafe extern "C" fn() -> tree_sitter::Language;
    // SAFETY: `symbol` was derived from the library filename, and every
    // tree-sitter grammar exports `tree_sitter_<name>` with this exact
    // signature. `Library::open` only binds a raw pointer to the .so; no
    // code runs until the call below.
    let constructor = unsafe { library.symbol::<LibConstructor>(symbol) }
        .map_err(|err| LoadGrammarError::MissingSymbol(symbol.to_string(), err))?;

    // SAFETY: calling the grammar constructor is safe as long as the symbol
    // is well-typed, which we guarantee by deriving it from the filename.
    let language = unsafe { constructor() };

    check_abi(&language)?;

    // SAFETY: Keep the library mapped for the process lifetime. The Language borrows
    // its tables and strings from the .so; `Library` must outlive it.
    std::mem::forget(library);

    Ok(language)
}

fn check_abi(language: &tree_sitter::Language) -> Result<(), LoadGrammarError> {
    let abi = language.abi_version();
    if (tree_sitter::MIN_COMPATIBLE_LANGUAGE_VERSION..=tree_sitter::LANGUAGE_VERSION).contains(&abi)
    {
        Ok(())
    } else {
        Err(LoadGrammarError::IncompatibleAbi { abi })
    }
}

pub fn discover_grammars(
    grammar_dir: &Path,
) -> impl Iterator<Item = Result<(String, tree_sitter::Language), LoadGrammarError>> {
    let paths: Box<dyn Iterator<Item = PathBuf>> = match std::fs::read_dir(grammar_dir) {
        Ok(entries) => Box::new(entries.flatten().map(|entry| entry.path())),
        Err(err) => {
            tracing::warn!(
                path = %grammar_dir.display(),
                error = %err,
                "grammar directory unavailable — no languages loaded"
            );
            Box::new(std::iter::empty())
        }
    };

    paths
        .filter(|path| path.is_file())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some(dylib_extension()))
        .filter_map(|path| {
            let stem = path.file_stem().and_then(|s| s.to_str())?.to_string();
            Some((path, stem))
        })
        .map(|(path, stem)| {
            let symbol = symbol_for_stem(&stem);
            load_language(&path, &symbol).map(|language| (grammar_key(&stem), language))
        })
}

pub fn discover_selected_grammars<'a, I>(
    grammar_dir: &'a Path,
    langs: I,
) -> impl Iterator<Item = Result<(String, tree_sitter::Language), LoadGrammarError>> + 'a
where
    I: IntoIterator<Item = &'a str> + 'a,
{
    langs.into_iter().map(move |lang| {
        let path = grammar_dir.join(format!("{lang}.{}", dylib_extension()));
        let lang_owned = lang.to_string();

        if !path.is_file() {
            return Err(LoadGrammarError::LibraryNotFound {
                id: lang_owned,
                path,
            });
        }

        let symbol = symbol_for_stem(lang);
        load_language(&path, &symbol).map(|language| (lang_owned, language))
    })
}

pub(crate) fn join(
    specs: impl IntoIterator<Item = LanguageSpec>,
    results: impl IntoIterator<Item = Result<(String, tree_sitter::Language), LoadGrammarError>>,
) -> (
    Vec<LoadedLanguage>,
    Vec<LoadGrammarError>,
    Vec<LanguageSpec>,
) {
    let mut specs_by_key: HashMap<String, LanguageSpec> = specs
        .into_iter()
        .map(|spec| (grammar_key(&spec.id), spec))
        .collect();

    let mut loaded = Vec::new();
    let mut errors = Vec::new();

    for result in results {
        match result {
            Ok((id, language)) => match specs_by_key.remove(&grammar_key(&id)) {
                Some(spec) => loaded.push(LoadedLanguage {
                    id: spec.id,
                    extensions: spec.extensions,
                    language,
                }),
                None => errors.push(LoadGrammarError::NotConfigured { id }),
            },
            Err(err) => errors.push(err),
        }
    }

    let missing = specs_by_key.into_values().collect();
    (loaded, errors, missing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn non_existent_dir() -> PathBuf {
        std::env::temp_dir().join("definitely-not-a-real-grammar-dir-xyz")
    }

    #[rstest]
    #[case("rust", "tree_sitter_rust")]
    #[case("c-sharp", "tree_sitter_c_sharp")]
    #[case("markdown_inline", "tree_sitter_markdown_inline")]
    #[case("ssh_client_config", "tree_sitter_ssh_client_config")]
    fn symbol_for_stem_handles_dashes_and_underscores(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(symbol_for_stem(input), expected);
    }

    #[rstest]
    fn discover_skips_subdirectories_and_foreign_extensions() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sources")).unwrap();
        std::fs::write(dir.path().join("notes.txt"), "not a grammar").unwrap();

        assert!(discover_grammars(dir.path()).next().is_none());
    }

    #[rstest]
    #[case(Path::new("/nonexistent/definitely/missing"))]
    fn discover_empty_or_missing_dir_yields_empty_iterator(#[case] path: &Path) {
        assert!(discover_grammars(path).next().is_none());
    }

    #[test]
    fn join_flags_loaded_grammar_without_spec_as_not_configured() {
        let specs = [LanguageSpec::new("rust", [])];
        let results = [Ok((
            "python".to_string(),
            tree_sitter_rust::LANGUAGE.into(),
        ))];

        let (loaded, errors, missing) = join(specs, results);

        assert!(loaded.is_empty());
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].id, "rust");
        assert!(matches!(
            &errors.get(0),
            Some(LoadGrammarError::NotConfigured { id }) if id == "python"
        ));
    }

    #[rstest]
    fn selective_load_reports_missing_library_and_preserves_order(non_existent_dir: PathBuf) {
        let ids = ["a", "z", "b"];
        let results: Vec<_> = discover_selected_grammars(&non_existent_dir, ids).collect();

        assert_eq!(results.len(), ids.len());
        for (result, expected_id) in results.iter().zip(ids) {
            assert!(matches!(
                result,
                Err(LoadGrammarError::LibraryNotFound { id, .. }) if id == expected_id
            ));
        }
    }
}
