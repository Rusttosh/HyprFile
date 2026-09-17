use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "hyprfiles")]
#[command(about = "A modern keyboard-first file manager for Linux")]
struct Args {
    /// Initial path to open
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Open in picker mode
    #[arg(long)]
    picker: bool,

    /// Open in dual-pane mode
    #[arg(long)]
    dual_pane: bool,

    /// Custom config file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Theme to use
    #[arg(long, value_name = "THEME")]
    theme: Option<String>,

    /// Select a specific file on open
    #[arg(long, value_name = "FILE")]
    select: Option<PathBuf>,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,

    /// Disable Hyprland-specific features
    #[arg(long)]
    no_hyprland: bool,

    /// Print selected file to stdout and exit (use with --picker)
    #[arg(long)]
    stdout: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.debug {
        tracing_subscriber::fmt()
            .with_env_filter("debug")
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter("info")
            .init();
    }

    tracing::info!("HyprFile starting with args: {:?}", args);

    // TODO: wire up to UI runtime once framework is chosen.
    println!("HyprFile is under construction. Args: {:?}", args);

    Ok(())
}
