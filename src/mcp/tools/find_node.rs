use rmcp::{
    ErrorData,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock, ErrorCode},
    schemars, tool, tool_router,
};
use serde::Deserialize;

use crate::mcp::tools::grammar_error;

#[tool_router(router = find_node_router, vis = "pub")]
impl crate::TreeSitterServer {
    #[tool(
        description = "Find the smallest named node at a byte offset in a source file, \
                        returning it and its ancestor chain up to the root. \
                        Read-only; does not modify the file.",
        annotations(
            title = "Find Node At Offset",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn tree_sitter_find_node(
        &self,
        Parameters(params): Parameters<FindNodeParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = self
            .grammar
            .find_node(&params.path, params.language.as_deref(), params.byte)
            .map_err(grammar_error)?;

        let json = serde_json::to_string_pretty(&result).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("failed to serialize node info: {e}"),
                None,
            )
        })?;

        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FindNodeParams {
    #[schemars(description = "Absolute or workspace-relative path to the source file")]
    pub path: String,

    #[schemars(description = "Language id. Inferred from the file extension if omitted.")]
    pub language: Option<String>,

    #[schemars(description = "Byte offset into the file to locate")]
    pub byte: usize,
}
