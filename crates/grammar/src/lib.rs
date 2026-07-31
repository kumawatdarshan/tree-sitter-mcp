pub mod error;
pub mod find;
pub mod language;
pub mod query;
pub mod session;

pub(crate) mod loader;

use std::path::Path;

use config::extension::ExtensionMap;
pub use error::GrammarError;
pub use find::FindNodeResult;
pub use language::{LanguageSummary, LoadedLanguage};
pub use query::{Capture, QueryMatch};
pub use session::{NodeInfo, ParseSession};

use language::GrammarRegistry;

#[derive(Debug)]
pub struct GrammarEngine {
    registry: GrammarRegistry,
}

impl GrammarEngine {
    pub fn load(ext_map: ExtensionMap, grammar_dir: &Path) -> Result<Self, GrammarError> {
        let specs = loader::specs_from_config(ext_map);
        let grammars = loader::discover_grammars(grammar_dir)?;
        let (loaded, missing) = loader::join(specs, grammars);

        for spec in missing {
            tracing::warn!(lang = %spec.id, "no compiled grammar — language unavailable");
        }

        Ok(Self {
            registry: GrammarRegistry::new(loaded),
        })
    }

    /// Construct from pre-built languages. Useful for tests and embedding.
    pub fn from_languages(languages: Vec<LoadedLanguage>) -> Self {
        Self {
            registry: GrammarRegistry::new(languages),
        }
    }

    /// Resolve a language for `path`. Pure — no I/O.
    ///
    /// Returns the [`LoadedLanguage`] that matches the file extension
    /// or glob pattern. Pass an explicit `language` to override inference.
    pub fn resolve_language(
        &self,
        path: &str,
        language: Option<&str>,
    ) -> Result<&LoadedLanguage, GrammarError> {
        language::resolve(&self.registry, path, language)
    }

    pub fn language_summaries(&self) -> impl Iterator<Item = LanguageSummary> + '_ {
        self.registry.values().map(LanguageSummary::from)
    }

    pub fn loaded_language_ids(&self) -> impl Iterator<Item = &str> {
        self.registry.values().map(|l| l.id.as_str())
    }
}
