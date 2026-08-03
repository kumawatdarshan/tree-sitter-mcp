use grammar::{GrammarError, LanguageInfo, LanguageStatus};
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool, tool_router,
};
use serde::Deserialize;

use crate::{McpError, tools::json_result};

#[tool_router(router = capabilities_router, vis = "pub(crate)")]
impl crate::TreeSitterServer {
    #[tool(
        description = "Load the requested grammars and report each language's capabilities. \
                        Pass the languages you intend to work with; an empty list reports every \
                        configured language.",
        annotations(
            title = "Get Capabilities",
            read_only_hint = true,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn tree_sitter_get_capabilities(
        &self,
        Parameters(params): Parameters<GetCapabilitiesParams>,
    ) -> Result<CallToolResult, McpError> {
        let requested = if params.languages.is_empty() {
            self.grammar.available_ids()
        } else {
            params.languages
        };

        let available = self.grammar.available_ids();
        let statuses = requested
            .into_iter()
            .map(|id| self.language_status(&id, &available))
            .collect::<Vec<_>>();

        json_result(&statuses, "capabilities")
    }
}

impl crate::TreeSitterServer {
    fn language_status(&self, id: &str, available: &[String]) -> LanguageStatus {
        match self.grammar.load_language(id) {
            Ok(lang) => LanguageStatus::Loaded {
                info: LanguageInfo {
                    name: lang.id.clone(),
                    extensions: lang.extensions.iter().map(|e| e.to_string()).collect(),
                    capabilities: vec![],
                },
            },
            Err(GrammarError::UnknownLanguage(_)) => LanguageStatus::NotConfigured {
                suggestions: suggest_near(id, available),
            },
            Err(err) => LanguageStatus::LoadFailed {
                language: id.to_string(),
                reason: err.to_string(),
            },
        }
    }
}

/// Near-name matches for a not-configured id, for client self-correction.
fn suggest_near(id: &str, available: &[String]) -> Vec<String> {
    let mut scored: Vec<(usize, &String)> = available
        .iter()
        .map(|cand| (edit_distance(id, cand), cand))
        .collect();
    scored.sort_by_key(|(dist, _)| *dist);

    scored
        .into_iter()
        .filter(|(dist, _)| *dist <= 2)
        .map(|(_, cand)| cand.clone())
        .collect()
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();

    for i in 1..=a.len() {
        let mut cur = vec![i];
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            let min = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            cur.push(min);
        }
        prev = cur;
    }
    prev[b.len()]
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct GetCapabilitiesParams {
    #[schemars(
        description = "Language ids to load and report capabilities for. Empty = all configured languages."
    )]
    pub languages: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggest_near_ranks_close_matches() {
        let available = ["rust", "python", "typescript"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        assert_eq!(suggest_near("ruts", &available), vec!["rust"]);
        assert_eq!(suggest_near("pythonn", &available), vec!["python"]);
    }

    #[test]
    fn suggest_near_empty_when_far() {
        let available = ["rust"].iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(suggest_near("brainfuck", &available).is_empty());
    }
}
