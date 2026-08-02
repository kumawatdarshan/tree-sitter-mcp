pub mod extension;

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to choose config directory")]
    ConfigDir(#[from] etcetera::HomeDirError),

    #[error("failed to read config file {0}")]
    ConfigRead(PathBuf, #[source] std::io::Error),

    #[error("TOML syntax error: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("missing [extensions] table")]
    MissingExtensionsTable,

    #[error("empty extension array for language: {0}")]
    EmptyExtensions(String),
}

use crate::extension::ExtensionMap;
use etcetera::AppStrategy;

pub const XDG_APP_NAME: &str = "tree-sitter-mcp";

pub fn strategy() -> Result<impl etcetera::AppStrategy, ConfigError> {
    Ok(etcetera::choose_app_strategy(etcetera::AppStrategyArgs {
        app_name: XDG_APP_NAME.to_string(),
        top_level_domain: "org".to_string(),
        author: XDG_APP_NAME.to_string(),
    })?)
}

pub fn grammar_dir(strategy: &impl AppStrategy) -> Result<PathBuf, ConfigError> {
    if let Ok(dir) = std::env::var("TREE_SITTER_MCP_GRAMMAR_DIR") {
        return Ok(PathBuf::from(dir));
    }
    Ok(strategy.data_dir().join("grammars"))
}

pub fn load(strategy: &impl AppStrategy) -> Result<ExtensionMap, ConfigError> {
    let config_path = strategy.config_dir().join("languages.toml");
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| ConfigError::ConfigRead(config_path, e))?;

    ExtensionMap::from_toml_str(&content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::path::{Path, PathBuf};
    use tempfile::{TempDir, tempdir};

    struct FakeStrategy(PathBuf);

    impl FakeStrategy {
        fn new(path: &Path) -> Self {
            Self(path.to_path_buf())
        }
    }

    impl AppStrategy for FakeStrategy {
        fn home_dir(&self) -> &Path {
            &self.0
        }
        fn config_dir(&self) -> PathBuf {
            self.0.join("config")
        }
        fn data_dir(&self) -> PathBuf {
            self.0.join("data")
        }
        fn cache_dir(&self) -> PathBuf {
            self.0.join("cache")
        }
        fn state_dir(&self) -> Option<PathBuf> {
            Some(self.0.join("state"))
        }
        fn runtime_dir(&self) -> Option<PathBuf> {
            Some(self.0.join("runtime"))
        }
    }

    #[fixture]
    fn temp_workspace() -> TempDir {
        tempdir().expect("Failed to initialize temporary directory context")
    }

    #[fixture]
    fn strategy(temp_workspace: TempDir) -> FakeStrategy {
        let strategy = FakeStrategy::new(temp_workspace.path());
        strategy
    }

    #[rstest]
    #[case::absolute_path("/custom/grammar/path")]
    #[case::relative_path("relative/grammars")]
    fn grammar_dir_env_var_takes_precedence(strategy: FakeStrategy, #[case] env_value: &str) {
        temp_env::with_var("TREE_SITTER_MCP_GRAMMAR_DIR", Some(env_value), || {
            let res = grammar_dir(&strategy);
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), PathBuf::from(env_value));
        });
    }

    #[rstest]
    fn grammar_dir_falls_back_to_strategy_data_dir(strategy: FakeStrategy) {
        let expected = strategy.data_dir().join("grammars");
        temp_env::with_var("TREE_SITTER_MCP_GRAMMAR_DIR", None::<&str>, || {
            let res = grammar_dir(&strategy);
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), expected);
        });
    }

    #[rstest]
    #[case::valid_single_extension(r#"rust = ["rs"]"#, "rust", &["rs"][..])]
    #[case::valid_multiple_extensions(r#"web = ["html", "css", "js"]"#, "web", &["html", "css", "js"][..])]
    fn test_load_valid_configs(
        strategy: FakeStrategy,
        #[case] config_content: &str,
        #[case] target_key: &str,
        #[case] expected_extensions: &[&str],
    ) {
        use crate::extension::ExtensionEntry;

        std::fs::create_dir_all(strategy.config_dir()).unwrap();
        std::fs::write(strategy.config_dir().join("languages.toml"), config_content).unwrap();

        let config = load(&strategy).unwrap();
        let parsed = config.get(target_key).expect("Target key not found");

        let expected: Vec<ExtensionEntry> = expected_extensions
            .iter()
            .map(|&s| ExtensionEntry::from(s))
            .collect();

        assert_eq!(parsed, &expected);
    }

    #[rstest]
    #[case::empty_extension_list(r#"empty = []"#, "empty")]
    fn test_load_rejects_empty_extension_list(
        strategy: FakeStrategy,
        #[case] config_content: &str,
        #[case] lang: &str,
    ) {
        std::fs::create_dir_all(strategy.config_dir()).unwrap();
        std::fs::write(strategy.config_dir().join("languages.toml"), config_content).unwrap();

        let result = load(&strategy);
        assert!(matches!(
            result,
            Err(ConfigError::EmptyExtensions(ref l)) if l == lang
        ));
    }

    #[rstest]
    #[case::missing_file(None, "missing")]
    #[case::malformed_toml(Some(r#"rust = ["rs"#), "malformed")]
    #[case::invalid_type(Some(r#"rust = "123""#), "invalid_type")]
    fn test_load_error_paths(
        strategy: FakeStrategy,
        #[case] file_content: Option<&str>,
        #[case] error_type: &str,
    ) {
        if let Some(content) = file_content {
            std::fs::create_dir_all(strategy.config_dir()).unwrap();
            std::fs::write(strategy.config_dir().join("languages.toml"), content).unwrap();
        }

        let result = load(&strategy);
        assert!(result.is_err());

        match error_type {
            "missing" => {
                assert!(matches!(result, Err(ConfigError::ConfigRead(_, _))));
            }
            "malformed" | "invalid_type" => {
                assert!(matches!(result, Err(ConfigError::Parse(_))));
            }
            _ => unreachable!(),
        }
    }
}
