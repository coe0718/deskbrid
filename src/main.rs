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
                // Start daemon + MCP TCP listener in parallel (both use rmcp transport)
                let daemon_handle = tokio::spawn(async { daemon::run().await });
                let mcp_handle =
                    tokio::spawn(
                        async move { deskbrid::mcp::server::run_mcp_tcp_on_port(port).await },
                    );
                let (daemon_result, mcp_result) = tokio::join!(daemon_handle, mcp_handle);
                daemon_result??;
                mcp_result??;
                Ok(())
            } else {
                daemon::run().await
            }
        }
        cli::Command::Status => client::send_one_shot(deskbrid::protocol::Action::Ping).await,
        cli::Command::Setup => deskbrid::setup::run().await,
        cli::Command::Mcp => {
            let event_tx = tokio::sync::broadcast::channel(256).0;
            let state = std::sync::Arc::new(deskbrid::DaemonState::new());
            let backend = deskbrid::backend::create_backend(event_tx).await?;
            *state.backend.write().await = Some(backend);
            deskbrid::mcp::server::run_mcp(state).await
        }
        _ => {
            let action = cli::into_action(args.command)?;
            client::send_one_shot_with_options(action, request_options).await
        }
    }
}
