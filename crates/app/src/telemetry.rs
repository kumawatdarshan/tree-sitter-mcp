use tracing_subscriber::EnvFilter;

pub(crate) fn init_tracing() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    Ok(())
}
