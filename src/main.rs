use deskbrid::cli;
use deskbrid::client;
use deskbrid::daemon;

fn main() -> anyhow::Result<()> {
    // S1 (Vex review): load .env files BEFORE parsing CLI so values
    // like DESKBRID_LOG / DESKBRID_RATE_LIMIT_PER_SEC show up via env.
    // Search order: $XDG_CONFIG_HOME/deskbrid/.env, then ~/.config/deskbrid/.env.
    // Missing files are silently ignored — .env is optional.
    load_dotenv_files();
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

/// S1: try to load .env files from the standard config locations.
/// Both paths are optional — dotenvy silently returns Ok(()) when the
/// file doesn't exist. We probe both because the XDG variable might
/// not be set on a fresh install.
fn load_dotenv_files() {
    // 1) $XDG_CONFIG_HOME/deskbrid/.env (if XDG_CONFIG_HOME is set)
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        let path = std::path::PathBuf::from(xdg).join("deskbrid").join(".env");
        match dotenvy::from_path(&path) {
            Ok(_) => {}
            Err(dotenvy::Error::Io(ref io)) if io.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => tracing::warn!("failed to load {}: {e}", path.display()),
        }
    }
    // 2) ~/.config/deskbrid/.env (always probed — handles systems
    // without XDG_CONFIG_HOME and Linux distros where the user's
    // HOME doesn't match getuid-derived dirs).
    if let Some(home) = dirs::config_dir() {
        let path = home.join("deskbrid").join(".env");
        match dotenvy::from_path_override(&path) {
            Ok(_) => {}
            Err(dotenvy::Error::Io(ref io)) if io.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => tracing::warn!("failed to load {}: {e}", path.display()),
        }
    }
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
        require_confirmation: None,
    };

    match args.command {
        cli::Command::Daemon {
            verbose: _,
            mock,
            mcp_port,
            no_dashboard,
            dashboard_bind,
            dashboard_token,
            tcp_port,
            tcp_token,
            mcp_token,
        } => {
            daemon::run(
                no_dashboard,
                dashboard_bind,
                dashboard_token,
                tcp_port,
                tcp_token,
                mcp_port,
                mcp_token,
                mock,
            )
            .await
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
        cli::Command::Macro { cmd } => {
            let action = into_macro_action(&cmd);
            client::send_one_shot(action).await
        }
        cli::Command::Mcp => {
            let event_tx = tokio::sync::broadcast::channel(256).0;
            let state = std::sync::Arc::new(deskbrid::DaemonState::new());
            state.load_persistent_state().await;
            match deskbrid::backend::create_backend(event_tx).await {
                Ok(backend) => *state.backend.write().await = Some(backend),
                Err(e) => tracing::warn!(
                    "No desktop backend (Docker/headless): {e:#}. Desktop tools will be unavailable."
                ),
            }
            deskbrid::mcp::server::run_mcp(state).await
        }
        cli::Command::Repl { sock } => deskbrid::cli::repl::run(sock).await,
        _ => {
            let action = cli::into_action(args.command)?;
            client::send_one_shot_with_options(action, request_options).await
        }
    }
}

fn into_macro_action(cmd: &cli::MacroCmd) -> deskbrid::protocol::Action {
    match cmd {
        cli::MacroCmd::Record { name, description } => {
            deskbrid::protocol::Action::MacroRecordStart {
                name: name.clone(),
                description: description.clone(),
            }
        }
        cli::MacroCmd::Stop => deskbrid::protocol::Action::MacroRecordStop,
        cli::MacroCmd::Replay {
            name,
            mode,
            loop_count,
            stop_on_error,
        } => deskbrid::protocol::Action::MacroReplay {
            name: name.clone(),
            mode: Some(mode.clone()),
            loop_count: Some(*loop_count),
            stop_on_error: Some(*stop_on_error),
        },
        cli::MacroCmd::List => deskbrid::protocol::Action::MacroList,
        cli::MacroCmd::Get { name } => deskbrid::protocol::Action::MacroGet { name: name.clone() },
        cli::MacroCmd::Delete { name } => {
            deskbrid::protocol::Action::MacroDelete { name: name.clone() }
        }
        cli::MacroCmd::Export { name } => {
            deskbrid::protocol::Action::MacroExport { name: name.clone() }
        }
        cli::MacroCmd::Import { name, data } => deskbrid::protocol::Action::MacroImport {
            name: name.clone(),
            data: data.clone(),
        },
    }
}
