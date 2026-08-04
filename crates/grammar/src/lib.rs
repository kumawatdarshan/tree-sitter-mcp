pub mod capabilities;
pub mod error;
pub mod find;
pub mod language;
pub mod query;
pub mod session;

pub(crate) mod loader;

use std::path::Path;
use std::sync::Mutex;

pub use capabilities::{Capability, LanguageInfo, LanguageStatus};
use config::extension::ExtensionMap;
pub use error::{GrammarError, LoadGrammarError};
pub use find::FindNodeResult;
pub use language::{Language, LanguageId};
pub use loader::discover_selected_grammars;
pub use query::{Capture, QueryMatch};
pub use session::{NodeInfo, ParseSession};

#[derive(Debug)]
pub struct GrammarEngine {
    grammar_dir: std::path::PathBuf,
    /// Declared languages (id + extensions). The single owner of
    /// language identity; the grammar ABI handle is filled lazily.
    languages: Mutex<Vec<Language>>,
}

impl GrammarEngine {
    /// Load the config's extension map. Nothing is dlopen'd here —
    /// grammars load lazily on first `load_language`/`resolve_language`.
    pub fn load(ext_map: ExtensionMap, grammar_dir: &Path) -> Self {
        Self {
            grammar_dir: grammar_dir.to_path_buf(),
            languages: Mutex::new(loader::languages_from_config(ext_map)),
        }
    }

    /// Construct from pre-built languages. Useful for tests and embedding.
    pub fn from_languages(languages: Vec<Language>) -> Self {
        Self {
            grammar_dir: std::path::PathBuf::new(),
            languages: Mutex::new(languages),
        }
    }

    /// Look up a declared language's metadata (id + extensions) without
    /// loading its grammar. Returns an owned clone of the record.
    pub fn language(&self, id: &LanguageId) -> Option<Language> {
        self.languages
            .lock()
            .unwrap()
            .iter()
            .find(|lang| lang.id() == id)
            .cloned()
    }

    /// Lazy-load a grammar by its configured id, caching it for the
    /// process lifetime. Ids declared in config but without a compiled
    /// grammar library yield an error.
    pub fn load_language(&self, id: &LanguageId) -> Result<tree_sitter::Language, GrammarError> {
        let mut languages = self.languages.lock().unwrap();
        let lang = languages
            .iter_mut()
            .find(|lang| lang.id() == id)
            .ok_or_else(|| GrammarError::UnknownLanguage(id.to_string()))?;

        if let Some(grammar) = lang.grammar() {
            return Ok(grammar.clone());
        }

        let path = self.grammar_dir.join(id.library_filename());
        let grammar = loader::load_language_from(&path, id)
            .map_err(|source| GrammarError::LoadGrammar { path, source })?;
        lang.set_grammar(grammar.clone());
        Ok(grammar)
    }

    /// Resolve a language for `path`. Pure config lookup (plus lazy
    /// load on miss) — pass an explicit `language` to override inference.
    pub fn resolve_language(
        &self,
        path: &str,
        language: Option<&LanguageId>,
    ) -> Result<tree_sitter::Language, GrammarError> {
        if let Some(id) = language {
            return self.load_language(id);
        }

        let p = Path::new(path);
        let languages = self.languages.lock().unwrap();
        let id = if let Some(ext) = p.extension().and_then(|e| e.to_str())
            && let Some(lang) = languages.iter().find(|l| l.matches_extension(ext))
        {
            lang.id().clone()
        } else {
            languages
                .iter()
                .find(|l| l.matches_path(p))
                .ok_or_else(|| GrammarError::UnknownLanguage(path.to_owned()))?
                .id()
                .clone()
        };
        drop(languages);
        self.load_language(&id)
    }

    /// All configured language ids, sorted. The empty-list discovery
    /// surface for capabilities.
    pub fn available_ids(&self) -> Vec<LanguageId> {
        let mut ids: Vec<LanguageId> = self
            .languages
            .lock()
            .unwrap()
            .iter()
            .map(|l| l.id().clone())
            .collect();
        ids.sort();
        ids
    }
}
