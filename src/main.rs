//! deskbrid entry point.

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let config = deskbrid::config::Config::from_env();
    let socket_path = config
        .socket_path
        .clone()
        .unwrap_or_else(deskbrid::default_socket_path);
    let cli = deskbrid::cli::Cli::parse();
    deskbrid::cli::run(cli.command, socket_path).await
}
