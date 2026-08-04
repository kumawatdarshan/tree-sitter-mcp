pub mod error;
pub mod find;
pub mod language;
pub mod query;
pub mod session;

pub(crate) mod loader;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use config::extension::ExtensionMap;
pub use error::{GrammarError, LoadGrammarError};
pub use find::FindNodeResult;
pub use language::{LanguageSummary, LoadedLanguage};
pub use loader::discover_selected_grammars;
pub use query::{Capture, QueryMatch};
pub use session::{NodeInfo, ParseSession};

use language::LanguageSpec;

#[derive(Debug)]
pub struct GrammarEngine {
    grammar_dir: std::path::PathBuf,
    /// Declared-in-config specs (id + extensions). Availability, no I/O.
    specs: HashMap<String, LanguageSpec>,
    /// dlopen'd on demand, cached for the process lifetime.
    loaded: Mutex<HashMap<String, LoadedLanguage>>,
}

impl GrammarEngine {
    /// Load the config's extension map. Nothing is dlopen'd here —
    /// grammars load lazily on first `load_language`/`resolve_language`.
    pub fn load(ext_map: ExtensionMap, grammar_dir: &Path) -> Self {
        let specs = loader::specs_from_config(ext_map)
            .into_iter()
            .map(|spec| (loader::grammar_key(&spec.id), spec))
            .collect();
        Self {
            grammar_dir: grammar_dir.to_path_buf(),
            specs,
            loaded: Mutex::new(HashMap::new()),
        }
    }

    /// Construct from pre-built languages. Useful for tests and embedding.
    pub fn from_languages(languages: Vec<LoadedLanguage>) -> Self {
        let specs = languages
            .iter()
            .map(|l| (loader::grammar_key(&l.id), LanguageSpec::from(l)))
            .collect();
        let loaded = languages
            .into_iter()
            .map(|l| (loader::grammar_key(&l.id), l))
            .collect();
        Self {
            grammar_dir: std::path::PathBuf::new(),
            specs,
            loaded: Mutex::new(loaded),
        }
    }

    /// Lazy-load a grammar by its configured id, caching it for the
    /// process lifetime. Ids declared in config but without a compiled
    /// grammar library yield an error.
    pub fn load_language(&self, id: &str) -> Result<LoadedLanguage, GrammarError> {
        let key = loader::grammar_key(id);
        if let Some(lang) = self.loaded.lock().unwrap().get(&key) {
            return Ok(lang.clone());
        }

        let spec = self
            .specs
            .get(&key)
            .ok_or_else(|| GrammarError::UnknownLanguage(id.to_owned()))?;

        let path = self.grammar_dir.join(loader::grammar_filename(&spec.id));
        let language = loader::load_language_from(&path, &spec.id)
            .map_err(|source| GrammarError::LoadGrammar { path, source })?;

        let lang = LoadedLanguage {
            id: spec.id.clone(),
            extensions: spec.extensions.clone(),
            language,
        };
        self.loaded.lock().unwrap().insert(key, lang.clone());
        Ok(lang)
    }

    /// Resolve a language for `path`. Pure config lookup (plus lazy
    /// load on miss) — pass an explicit `language` to override inference.
    pub fn resolve_language(
        &self,
        path: &str,
        language: Option<&str>,
    ) -> Result<LoadedLanguage, GrammarError> {
        if let Some(id) = language {
            return self.load_language(id);
        }

        let p = Path::new(path);
        let id = if let Some(ext) = p.extension().and_then(|e| e.to_str())
            && let Some(spec) = self.specs.values().find(|s| s.matches_extension(ext))
        {
            spec.id.clone()
        } else {
            self.specs
                .values()
                .find(|s| s.matches_path(p))
                .ok_or_else(|| GrammarError::UnknownLanguage(path.to_owned()))?
                .id
                .clone()
        };
        self.load_language(&id)
    }

    /// All configured language ids, sorted. The empty-list discovery
    /// surface for capabilities.
    pub fn available_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.specs.keys().map(ToString::to_string).collect();
        ids.sort();
        ids
    }

    pub fn language_summaries(&self) -> impl Iterator<Item = LanguageSummary> + '_ {
        self.specs
            .values()
            .map(LanguageSummary::from)
            .collect::<Vec<_>>()
            .into_iter()
    }
}
