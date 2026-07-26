pub mod error;
pub(crate) mod find_node;
pub(crate) mod parser;
pub(crate) mod query;
pub(crate) mod registry;

pub use error::GrammarError;
pub use find_node::FindNodeResult;
pub use parser::{ByteRange, NodeInfo};
pub use query::{Capture, QueryMatch};
pub use registry::LanguageSummary;

use registry::LanguageRegistry;

pub struct GrammarEngine {
    pub(crate) registry: LanguageRegistry,
}

impl GrammarEngine {
    pub fn load_default() -> Result<Self, GrammarError> {
        Ok(Self {
            registry: LanguageRegistry::build()?,
        })
    }

    pub fn dump_ast(
        &self,
        path: &str,
        language: Option<&str>,
        range: Option<&ByteRange>,
    ) -> Result<String, GrammarError> {
        let (_source, tree) = self.load_tree(path, language)?;
        let root = Self::apply_range(tree.root_node(), range);
        Ok(root.to_sexp())
    }

    pub fn loaded_language_ids(&self) -> Vec<&str> {
        self.registry.loaded_ids()
    }

    pub fn language_summaries(&self) -> Vec<LanguageSummary> {
        self.registry.summaries()
    }
}
