use deskbrid::cli;
use deskbrid::client;
use deskbrid::daemon;

#[tokio::main]
#[allow(unused_variables)]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("DESKBRID_LOG")
                .unwrap_or_else(|_| "warn".into()),
        )
        .init();

    let args = cli::parse();
    let request_options = deskbrid::protocol::RequestOptions {
        dry_run: args.dry_run,
        timeout_ms: args.timeout_ms,
    };

    match args.command {
        cli::Command::Daemon { verbose, mcp_port } => {
            if verbose {
                // SAFETY: called at startup before threads are spawned
                unsafe {
                    std::env::set_var("DESKBRID_LOG", "debug");
                }
            }
            if let Some(port) = mcp_port {
                #[cfg(feature = "mcp")]
                {
                    // Start daemon + MCP TCP listener in parallel
                    let daemon_handle = tokio::spawn(async { daemon::run().await });
                    let mcp_handle =
                        tokio::spawn(async move { deskbrid::mcp::run_mcp_tcp(port).await });
                    let (daemon_result, mcp_result) = tokio::join!(daemon_handle, mcp_handle);
                    daemon_result??;
                    mcp_result??;
                    Ok(())
                }
                #[cfg(not(feature = "mcp"))]
                anyhow::bail!(
                    "MCP server not compiled (enable 'mcp' feature: cargo build --features mcp)"
                )
            } else {
                daemon::run().await
            }
        }
        cli::Command::Status => client::send_one_shot(deskbrid::protocol::Action::Ping).await,
        cli::Command::Setup => deskbrid::setup::run().await,
        #[cfg(feature = "mcp")]
        cli::Command::Mcp => deskbrid::mcp::run_mcp_server().await,
        #[cfg(not(feature = "mcp"))]
        cli::Command::Mcp => {
            anyhow::bail!(
                "MCP server not compiled (enable 'mcp' feature: cargo build --features mcp)"
            )
        }
        _ => {
            let action = cli::into_action(args.command)?;
            client::send_one_shot_with_options(action, request_options).await
        }
    }
}
