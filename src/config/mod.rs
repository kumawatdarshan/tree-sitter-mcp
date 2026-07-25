pub(crate) mod extension;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("TOML syntax error: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("missing [extensions] table")]
    MissingExtensionsTable,

    #[error("duplicate language key: {0}")]
    DuplicateKey(String),

    #[error("empty extension array for language: {0}")]
    EmptyExtensions(String),

    #[error(r#"bare basename not allowed (use {{ glob = "..." }}): {0} in language {1}"#)]
    BareBasename(String, String),

    #[error("invalid glob pattern '{glob}' for language {language}: {error}")]
    InvalidGlob {
        glob: String,
        language: String,
        error: anyhow::Error,
    },

    #[error("{0}")]
    Unknown(String),
}
