use clap::Parser;
use picoflow::cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize logging
    cli.init_logging()?;

    // Execute command
    cli.execute().await?;

    Ok(())
}
