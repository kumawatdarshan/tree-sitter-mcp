use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GrammarError {
    #[error(transparent)]
    Config(#[from] config::ConfigError),

    #[error("unknown language {0}")]
    UnknownLanguage(String),

    #[error("failed to load grammar library `{path}`")]
    LoadGrammar {
        path: PathBuf,
        #[source]
        source: LoadGrammarError,
    },

    #[error("failed to set tree-sitter language")]
    SetLanguage(#[source] tree_sitter::LanguageError),

    #[error("query compile failed")]
    Query(#[from] tree_sitter::QueryError),

    #[error("byte offset {byte} is past end of file ({len} bytes)")]
    ByteOutOfBounds { byte: usize, len: usize },
}

#[derive(Debug, Error)]
pub enum LoadGrammarError {
    #[error("The requested id isn't a language declared in config")]
    NotConfigured { id: String },

    #[error("grammar library not found: {path}")]
    LibraryNotFound { id: String, path: PathBuf },

    #[error("could not open shared library")]
    Open(#[from] dlopen2::Error),

    #[error("constructor symbol `{0}` not found")]
    MissingSymbol(String, #[source] dlopen2::Error),

    #[error(
        "incompatible grammar ABI version {abi} (supported {}..={})",
        tree_sitter::MIN_COMPATIBLE_LANGUAGE_VERSION,
        tree_sitter::LANGUAGE_VERSION
    )]
    IncompatibleAbi { abi: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_out_of_bounds_error_displays_offsets() {
        let err = GrammarError::ByteOutOfBounds { byte: 42, len: 7 };

        assert_eq!(
            err.to_string(),
            "byte offset 42 is past end of file (7 bytes)"
        );
    }
}
