use deskbrid::cli;
use deskbrid::client;
use deskbrid::daemon;

fn main() -> anyhow::Result<()> {
    let args = cli::parse();
    if let cli::Command::Daemon { verbose: true, .. } = &args.command {
        // SAFETY: called in single-threaded fn main before tokio runtime starts
        unsafe {
            std::env::set_var("DESKBRID_LOG", "debug");
        }
    }
    ensure_xdg_runtime_dir();
    runtime(args)
}

fn ensure_xdg_runtime_dir() {
    if std::env::var_os("XDG_RUNTIME_DIR").is_some() {
        return;
    }

    let runtime_dir = format!("/run/user/{}", unsafe { libc::geteuid() });
    if std::path::Path::new(&runtime_dir).is_dir() {
        // SAFETY: called in single-threaded fn main before tokio runtime starts
        unsafe {
            std::env::set_var("XDG_RUNTIME_DIR", runtime_dir);
        }
    }
}

#[tokio::main]
async fn runtime(args: cli::Args) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("DESKBRID_LOG")
                .unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let request_options = deskbrid::protocol::RequestOptions {
        dry_run: args.dry_run,
        timeout_ms: args.timeout_ms,
    };

    match args.command {
        cli::Command::Daemon {
            verbose: _,
            mcp_port,
            no_dashboard,
            tcp_port,
            tcp_token,
        } => {
            if let Some(port) = mcp_port {
                // Start daemon + MCP TCP listener in parallel (both use rmcp transport)
                let no_dash = no_dashboard;
                let tcp_bind = tcp_port;
                let tcp_tok = tcp_token;
                let daemon_handle =
                    tokio::spawn(async move { daemon::run(no_dash, tcp_bind, tcp_tok).await });
                let mcp_handle =
                    tokio::spawn(
                        async move { deskbrid::mcp::server::run_mcp_tcp_on_port(port).await },
                    );
                let (daemon_result, mcp_result) = tokio::join!(daemon_handle, mcp_handle);
                daemon_result??;
                mcp_result??;
                Ok(())
            } else {
                daemon::run(no_dashboard, tcp_port, tcp_token).await
            }
        }
        cli::Command::Status => client::send_one_shot(deskbrid::protocol::Action::Ping).await,
        cli::Command::Setup => deskbrid::setup::run().await,
        cli::Command::Update { check, force } => deskbrid::cmd::update::run(check, force).await,
        cli::Command::Tray => deskbrid::tray::run().await,
        cli::Command::DbusCall {
            bus,
            service,
            path,
            interface,
            method,
            args,
        } => {
            let action = deskbrid::protocol::Action::DbusCall {
                bus: Some(bus),
                service,
                path,
                interface,
                method,
                args: args.and_then(|a| serde_json::from_str(&a).ok()),
            };
            client::send_one_shot(action).await
        }
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
