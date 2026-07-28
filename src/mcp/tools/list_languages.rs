use rmcp::{ErrorData, model::CallToolResult, tool, tool_router};

use crate::{grammar::LanguageSummary, mcp::tools::text_result};

#[tool_router(router = list_languages_router, vis = "pub")]
impl crate::TreeSitterServer {
    #[tool(
        description = "List the languages with available tree-sitter grammars on this server, \
                        along with the file extensions used to infer each one.",
        annotations(
            title = "List Supported Languages",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn tree_sitter_list_languages(&self) -> Result<CallToolResult, ErrorData> {
        let lines: Vec<String> = self
            .grammar
            .entries
            .values()
            .map(LanguageSummary::from)
            .map(|s| s.to_string())
            .collect();

        Ok(text_result(lines.join("\n")))
    }
}
