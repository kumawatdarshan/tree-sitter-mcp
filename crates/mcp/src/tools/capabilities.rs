use grammar::{LanguageId, LanguageInfo, LanguageStatus};
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
            params
                .languages
                .into_iter()
                .filter_map(|s| LanguageId::new(s).ok())
                .collect::<Vec<_>>()
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
    fn language_status(&self, id: &LanguageId, available: &[LanguageId]) -> LanguageStatus {
        let meta = match self.grammar.language(id) {
            Some(meta) => meta,
            None => {
                return LanguageStatus::NotConfigured {
                    suggestions: suggest_near(id, available),
                };
            }
        };

        match self.grammar.load_language(id) {
            Ok(_) => LanguageStatus::Loaded {
                info: LanguageInfo::from(&meta),
            },
            Err(err) => LanguageStatus::LoadFailed {
                language: id.to_string(),
                reason: err.to_string(),
            },
        }
    }
}

/// Near-name matches for a not-configured id, for client self-correction.
fn suggest_near(id: &LanguageId, available: &[LanguageId]) -> Vec<String> {
    let mut scored: Vec<(usize, &LanguageId)> = available
        .iter()
        .map(|cand| (edit_distance(&id.to_string(), &cand.to_string()), cand))
        .collect();
    scored.sort_by_key(|(dist, _)| *dist);

    scored
        .into_iter()
        .filter(|(dist, _)| *dist <= 2)
        .map(|(_, cand)| cand.to_string())
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
            .filter_map(|s| LanguageId::new(*s).ok())
            .collect::<Vec<_>>();
        assert_eq!(
            suggest_near(&LanguageId::new("ruts").unwrap(), &available),
            vec!["rust"]
        );
    }

    #[test]
    fn suggest_near_empty_when_far() {
        let available = ["rust"]
            .iter()
            .filter_map(|s| LanguageId::new(*s).ok())
            .collect::<Vec<_>>();
        assert!(suggest_near(&LanguageId::new("brainfuck").unwrap(), &available).is_empty());
    }
}
