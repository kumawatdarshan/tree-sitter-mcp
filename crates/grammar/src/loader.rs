use std::collections::HashMap;
use std::path::Path;

use config::extension::{ExtensionEntry, ExtensionMap};

use crate::error::GrammarError;
use crate::language::LoadedLanguage;

/// Intermediate type — declared in config, grammar not yet resolved.
#[derive(Debug, Clone)]
pub(crate) struct LanguageSpec {
    pub id: String,
    pub extensions: Vec<ExtensionEntry>,
}

/// Step 1: config → specs. No I/O, no grammar loading.
pub(crate) fn specs_from_config(map: ExtensionMap) -> Vec<LanguageSpec> {
    map.into_iter()
        .map(|(id, extensions)| LanguageSpec { id, extensions })
        .collect()
}

/// Step 2: scan `grammar_dir` for compiled `.so` files.
///
/// Returns a map from the **normalized** symbol name to the Language.
/// "tree_sitter_rust" → ("rust", Language)
pub(crate) fn discover_grammars(
    grammar_dir: &Path,
) -> Result<HashMap<String, tree_sitter::Language>, GrammarError> {
    let mut loader = tree_sitter_loader::Loader::new()?;
    let raw = loader.languages_at_path(grammar_dir)?;

    Ok(raw
        .into_iter()
        .filter_map(|(lang, ident)| {
            let name = ident.strip_prefix("tree_sitter_")?.to_string();
            Some((name, lang))
        })
        .collect())
}

fn grammar_key(id: &str) -> String {
    id.replace('-', "_")
}

/// Step 3: join specs to discovered grammars.
///
/// Returns `(loaded, missing)` so the caller decides policy on gaps.
pub(crate) fn join(
    specs: Vec<LanguageSpec>,
    mut grammars: HashMap<String, tree_sitter::Language>,
) -> (Vec<LoadedLanguage>, Vec<LanguageSpec>) {
    let mut loaded = Vec::new();
    let mut missing = Vec::new();

    for spec in specs {
        match grammars.remove(&grammar_key(&spec.id)) {
            Some(language) => loaded.push(LoadedLanguage {
                id: spec.id,
                extensions: spec.extensions,
                language,
            }),
            None => missing.push(spec),
        }
    }

    (loaded, missing)
}
