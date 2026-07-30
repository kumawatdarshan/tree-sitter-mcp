use thiserror::Error;

#[derive(Debug, Error)]
pub enum GrammarError {
    #[error(transparent)]
    Config(#[from] config::ConfigError),

    #[error("unknown language {0}")]
    UnknownLanguage(String),

    #[error(transparent)]
    Loader(#[from] tree_sitter_loader::LoaderError),

    #[error("failed to set tree-sitter language")]
    SetLanguage(#[source] tree_sitter::LanguageError),

    #[error("query compile failed")]
    Query(#[from] tree_sitter::QueryError),

    #[error("byte offset {byte} is past end of file ({len} bytes)")]
    ByteOutOfBounds { byte: usize, len: usize },

    #[error("{0}")]
    UnknownError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Query;

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
