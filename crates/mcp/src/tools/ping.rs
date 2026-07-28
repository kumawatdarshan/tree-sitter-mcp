use rmcp::{model::CallToolResult, tool, tool_router};

use crate::{McpError, TreeSitterServer, tools::text_result};

#[tool_router(router = ping_router, vis = "pub(crate)")]
impl TreeSitterServer {
    #[tool(
        description = "Ping the server to verify connectivity",
        annotations(
            title = "Ping",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn ping(&self) -> Result<CallToolResult, McpError> {
        Ok(text_result("pong"))
    }
}
