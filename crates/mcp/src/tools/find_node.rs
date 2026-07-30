use grammar::ParseSession;
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool, tool_router,
};
use serde::Deserialize;

use crate::{
    McpError,
    tools::{FileParams, json_result},
};

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
    ) -> Result<CallToolResult, McpError> {
        let source = std::fs::read_to_string(&params.file.path)?;
        let lang = self
            .grammar
            .resolve_language(&params.file.path, params.file.language.as_deref())?;
        let result = ParseSession::new(lang.clone(), source)?.find_node(params.byte)?;

        json_result(&result, "node info")
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct FindNodeParams {
    #[serde(flatten)]
    pub file: FileParams,

    #[schemars(description = "Byte offset into the file to locate")]
    pub byte: usize,
}
