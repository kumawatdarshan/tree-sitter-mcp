pub mod config;
pub mod server;
pub mod telemetry;

use crate::server::TreeSitterServer;
use crate::telemetry::init_tracing;
use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing()?;
    tracing::info!("Starting Tree-Sitter MCP Server");

    let transport = (stdin(), stdout());

    let server = TreeSitterServer::new();
    let service = server.serve(transport).await?;

    service.waiting().await?;
    Ok(())
}
