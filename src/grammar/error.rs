use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GrammarError {
    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),

    #[error("failed to read source file {0}")]
    SourceRead(PathBuf, #[source] std::io::Error),

    #[error("invalid glob pattern")]
    InvalidGlob(#[from] globset::Error),

    #[error("unknown language {0}")]
    UnknownLanguage(String),

    #[error("could not infer language for {0}")]
    LanguageInference(PathBuf),

    #[error("grammar for language {0} is not loaded")]
    GrammarNotLoaded(String),

    #[error("failed to set tree-sitter language")]
    SetLanguage(#[source] tree_sitter::LanguageError),

    #[error("tree-sitter returned no parse tree")]
    ParseReturnedNoTree,

    #[error("query compile failed")]
    Query(#[from] tree_sitter::QueryError),

    #[error("byte offset {byte} is past end of file ({len} bytes)")]
    ByteOutOfBounds { byte: usize, len: usize },
}
