use std::path::Path;

use tree_sitter::{LANGUAGE_VERSION, Language, MIN_COMPATIBLE_LANGUAGE_VERSION};
use tree_sitter_loader::Loader;

use super::error::GrammarError;

pub(crate) fn load_grammar(path: &Path, name: &str) -> Result<Language, GrammarError> {
    let function_name = format!("tree_sitter_{}", name.replace('-', "_"));
    let language = Loader::load_language(path, &function_name)?;

    let version = language.abi_version();
    if !(MIN_COMPATIBLE_LANGUAGE_VERSION..=LANGUAGE_VERSION).contains(&version) {
        return Err(GrammarError::IncompatibleAbi {
            id: name.to_string(),
            version,
            expected: format!("{MIN_COMPATIBLE_LANGUAGE_VERSION}..={LANGUAGE_VERSION}"),
        });
    }

    Ok(language)
}
