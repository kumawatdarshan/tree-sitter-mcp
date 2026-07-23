use crate::telemetry::init_tracing;

pub mod telemetry;

fn main() -> anyhow::Result<()> {
    init_tracing()?;

    tracing::info!("Starting Tree-Sitter MCP Server");
    Ok(())
}
