use clap::Parser;
use picoflow::cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Restore default SIGPIPE handling so piping output into a pager that exits early
    // (e.g. `picoflow logs ... | head`) terminates quietly instead of panicking on a
    // broken pipe (which, with panic="abort", would abort the process).
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize logging
    cli.init_logging()?;

    // Execute command
    cli.execute().await?;

    Ok(())
}
