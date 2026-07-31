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
