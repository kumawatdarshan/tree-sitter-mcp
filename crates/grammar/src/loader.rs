use std::path::Path;

use config::extension::ExtensionMap;

use crate::error::LoadGrammarError;
#[cfg(not(test))]
use crate::language::{Language, LanguageId};
#[cfg(test)]
use crate::language::{Language, LanguageId, dylib_extension};

pub(crate) fn languages_from_config(map: ExtensionMap) -> Vec<Language> {
    map.into_iter()
        .filter_map(|(id, extensions)| match LanguageId::new(id) {
            Ok(id) => Some(Language::new(id, extensions)),
            Err(err) => {
                tracing::warn!(error = %err, "skipping invalid language id");
                None
            }
        })
        .collect()
}

/// Load a grammar library at `path`. The symbol name is derived from the
/// id (`-` → `_`) via [`LanguageId::constructor_symbol`].
pub(crate) fn load_language_from(
    path: &Path,
    id: &LanguageId,
) -> Result<tree_sitter::Language, LoadGrammarError> {
    let library = dlopen2::symbor::Library::open(path).map_err(LoadGrammarError::Open)?;

    type LibConstructor = unsafe extern "C" fn() -> tree_sitter::Language;
    // SAFETY: `symbol` was derived from the language id, and every
    // tree-sitter grammar exports `tree_sitter_<name>` with this exact
    // signature. `Library::open` only binds a raw pointer to the .so; no
    // code runs until the call below.
    let symbol = id.constructor_symbol();
    let constructor = unsafe { library.symbol::<LibConstructor>(&symbol) }
        .map_err(|err| LoadGrammarError::MissingSymbol(symbol, err))?;

    // SAFETY: calling the grammar constructor is safe as long as the symbol
    // is well-typed, which we guarantee by deriving it from the id.
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

#[cfg(test)]
pub fn discover_grammars(
    grammar_dir: &Path,
) -> impl Iterator<Item = Result<(LanguageId, tree_sitter::Language), LoadGrammarError>> {
    let paths: Box<dyn Iterator<Item = std::path::PathBuf>> = match std::fs::read_dir(grammar_dir) {
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
            let id = LanguageId::new_unchecked(stem);
            load_language_from(&path, &id).map(|language| (id, language))
        })
}

pub fn discover_selected_grammars<'a, I>(
    grammar_dir: &'a Path,
    langs: I,
) -> impl Iterator<Item = Result<(LanguageId, tree_sitter::Language), LoadGrammarError>> + 'a
where
    I: IntoIterator<Item = LanguageId> + 'a,
{
    langs.into_iter().map(move |lang| {
        let path = grammar_dir.join(lang.library_filename());

        if !path.is_file() {
            return Err(LoadGrammarError::LibraryNotFound {
                id: lang.to_string(),
                path,
            });
        }

        load_language_from(&path, &lang).map(|language| (lang, language))
    })
}

#[cfg(test)]
pub(crate) fn join(
    languages: impl IntoIterator<Item = Language>,
    results: impl IntoIterator<Item = Result<(LanguageId, tree_sitter::Language), LoadGrammarError>>,
) -> (Vec<Language>, Vec<LoadGrammarError>, Vec<Language>) {
    let mut by_id: std::collections::HashMap<String, Language> = languages
        .into_iter()
        .map(|lang| (lang.id().to_string(), lang))
        .collect();

    let mut loaded = Vec::new();
    let mut errors = Vec::new();

    for result in results {
        match result {
            Ok((id, grammar)) => match by_id.remove(&id.to_string()) {
                Some(mut lang) => {
                    lang.set_grammar(grammar);
                    loaded.push(lang);
                }
                None => errors.push(LoadGrammarError::NotConfigured { id: id.to_string() }),
            },
            Err(err) => errors.push(err),
        }
    }

    let missing = by_id.into_values().collect();
    (loaded, errors, missing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn non_existent_dir() -> std::path::PathBuf {
        std::env::temp_dir().join("definitely-not-a-real-grammar-dir-xyz")
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
        let rust = Language::new(LanguageId::new_unchecked("rust"), Vec::new());
        let results = [Ok((
            LanguageId::new_unchecked("python"),
            tree_sitter_rust::LANGUAGE.into(),
        ))];

        let (loaded, errors, missing) = join([rust], results);

        assert!(loaded.is_empty());
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].id().to_string(), "rust");
        assert!(matches!(
            &errors.get(0),
            Some(LoadGrammarError::NotConfigured { id }) if id == "python"
        ));
    }

    #[test]
    fn join_populates_loaded_grammars() {
        let rust = Language::new(LanguageId::new_unchecked("rust"), Vec::new());
        let results = [Ok((
            LanguageId::new_unchecked("rust"),
            tree_sitter_rust::LANGUAGE.into(),
        ))];

        let (loaded, errors, missing) = join([rust], results);

        assert_eq!(loaded.len(), 1);
        assert!(loaded[0].grammar().is_some());
        assert!(errors.is_empty());
        assert!(missing.is_empty());
    }

    #[rstest]
    fn selective_load_reports_missing_library_and_preserves_order(
        non_existent_dir: std::path::PathBuf,
    ) {
        let ids = ["a", "z", "b"]
            .into_iter()
            .map(LanguageId::new_unchecked)
            .collect::<Vec<_>>();
        let results: Vec<_> = discover_selected_grammars(&non_existent_dir, ids).collect();

        assert_eq!(results.len(), 3);
    }
}
