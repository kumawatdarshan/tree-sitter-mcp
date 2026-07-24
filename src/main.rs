pub mod config;
pub mod runtime;
pub mod telemetry;

use crate::telemetry::init_tracing;

fn main() -> anyhow::Result<()> {
    init_tracing()?;

    tracing::info!("Starting Tree-Sitter MCP Server (grammars configured)",);

    Ok(())
}
