use rmcp::{tool, tool_router};

use crate::TreeSitterServer;

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
    async fn ping(&self) -> String {
        "pong".into()
    }
}
