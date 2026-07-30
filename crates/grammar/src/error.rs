use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GrammarError {
    #[error(transparent)]
    Config(#[from] config::ConfigError),

    #[error("failed to read source file {0}")]
    SourceRead(PathBuf, #[source] std::io::Error),

    #[error("unknown language {0}")]
    UnknownLanguage(String),

    #[error("could not infer language for {0}")]
    LanguageInference(PathBuf),

    #[error("grammar for language {0} is not loaded")]
    GrammarNotLoaded(String),

    #[error(transparent)]
    Loader(#[from] tree_sitter_loader::LoaderError),

    #[error("grammar {id} has incompatible ABI version {version} (expected {expected})")]
    IncompatibleAbi {
        id: String,
        version: usize,
        expected: String,
    },

    #[error("failed to set tree-sitter language")]
    SetLanguage(#[source] tree_sitter::LanguageError),

    #[error("tree-sitter returned no parse tree")]
    ParseReturnedNoTree,

    #[error("query compile failed")]
    Query(#[from] tree_sitter::QueryError),

    #[error("byte offset {byte} is past end of file ({len} bytes)")]
    ByteOutOfBounds { byte: usize, len: usize },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use tree_sitter::Query;

    #[test]
    fn source_read_error_displays_path() {
        let err = GrammarError::SourceRead(
            PathBuf::from("missing.rs"),
            io::Error::new(io::ErrorKind::NotFound, "not found"),
        );

        assert_eq!(err.to_string(), "failed to read source file missing.rs");
    }

    #[test]
    fn simple_error_variants_display_context() {
        assert_eq!(
            GrammarError::UnknownLanguage("brainfuck".into()).to_string(),
            "unknown language brainfuck"
        );
        assert_eq!(
            GrammarError::LanguageInference(PathBuf::from("unknown.xyz")).to_string(),
            "could not infer language for unknown.xyz"
        );
        assert_eq!(
            GrammarError::GrammarNotLoaded("rust".into()).to_string(),
            "grammar for language rust is not loaded"
        );
    }

    #[test]
    fn incompatible_abi_error_displays_expected_range() {
        let err = GrammarError::IncompatibleAbi {
            id: "rust".into(),
            version: 1,
            expected: "13..=15".into(),
        };

        assert_eq!(
            err.to_string(),
            "grammar rust has incompatible ABI version 1 (expected 13..=15)"
        );
    }

    #[test]
    fn byte_out_of_bounds_error_displays_offsets() {
        let err = GrammarError::ByteOutOfBounds { byte: 42, len: 7 };

        assert_eq!(
            err.to_string(),
            "byte offset 42 is past end of file (7 bytes)"
        );
    }

    #[test]
    fn query_error_converts_to_grammar_error() {
        let query_err = Query::new(&tree_sitter_rust::LANGUAGE.into(), "(unclosed-pattern")
            .expect_err("query should be invalid");
        let err = GrammarError::from(query_err);

        assert!(matches!(err, GrammarError::Query(_)));
        assert_eq!(err.to_string(), "query compile failed");
    }
}
