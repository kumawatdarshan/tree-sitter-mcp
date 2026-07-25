use rmcp::{
    ErrorData,
    model::{CallToolResult, ContentBlock},
    tool, tool_router,
};

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
        let summaries = self.grammar.language_summaries();
        let lines: Vec<String> = summaries
            .iter()
            .map(|s| {
                if s.loaded {
                    format!("{}: {}", s.id, s.extensions.join(", "))
                } else {
                    format!("{}: {} (grammar not loaded)", s.id, s.extensions.join(", "))
                }
            })
            .collect();

        Ok(CallToolResult::success(vec![ContentBlock::text(
            lines.join("\n"),
        )]))
    }
}
