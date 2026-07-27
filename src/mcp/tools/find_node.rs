use rmcp::{
    ErrorData,
    handler::server::wrapper::Parameters,
    model::CallToolResult,
    schemars, tool, tool_router,
};
use serde::Deserialize;

use crate::mcp::tools::{grammar_error, json_result};

#[tool_router(router = find_node_router, vis = "pub(crate)")]
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

        json_result(&result, "node info")
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct FindNodeParams {
    #[schemars(description = "Absolute or workspace-relative path to the source file")]
    pub path: String,

    #[schemars(description = "Language id. Inferred from the file extension if omitted.")]
    pub language: Option<String>,

    #[schemars(description = "Byte offset into the file to locate")]
    pub byte: usize,
}
