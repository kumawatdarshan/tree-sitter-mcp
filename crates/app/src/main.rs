pub mod telemetry;

use std::sync::Arc;

use crate::telemetry::init_tracing;
use grammar::GrammarEngine;
use mcp::TreeSitterServer;
use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing()?;
    tracing::info!("Starting Tree-Sitter MCP Server");

    let grammar = Arc::new(GrammarEngine::load_default()?);
    let server = TreeSitterServer::new(grammar);

    let transport = (stdin(), stdout());
    let service = server.serve(transport).await?;

    service.waiting().await?;
    Ok(())
}
