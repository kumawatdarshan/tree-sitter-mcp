use rmcp::{model::CallToolResult, tool, tool_router};

use crate::{McpError, tools::text_result};

#[tool_router(router = list_languages_router, vis = "pub(crate)")]
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
    async fn tree_sitter_list_languages(&self) -> Result<CallToolResult, McpError> {
        let lines: Vec<String> = self
            .grammar
            .language_summaries()
            .map(|x| x.to_string())
            .collect();

        Ok(text_result(lines.join("\n")))
    }
}
