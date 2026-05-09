use deskbrid::cli;
use deskbrid::client;
use deskbrid::daemon;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("DESKBRID_LOG")
                .unwrap_or_else(|_| "warn".into()),
        )
        .init();

    let args = cli::parse();

    match args.command {
        cli::Command::Daemon { verbose } => {
            if verbose {
                std::env::set_var("DESKBRID_LOG", "debug");
            }
            daemon::run().await
        }
        cli::Command::Status => client::send_one_shot(deskbrid::protocol::Action::Ping).await,
        _ => {
            let action = cli::into_action(args.command)?;
            client::send_one_shot(action).await
        }
    }
}
