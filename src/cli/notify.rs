use clap::Subcommand;

#[derive(Subcommand)]
pub enum NotifyCmd {
    /// Send a notification
    Send {
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: String,
        #[arg(long, default_value = "normal")]
        urgency: String,
    },
    /// Close a notification
    Close { notification_id: u32 },
    /// Show notification history
    History {
        /// Limit number of results (default: 50)
        #[arg(long)]
        limit: Option<u32>,
        /// Filter by app name
        #[arg(long)]
        app_name: Option<String>,
        /// Only show notifications since this Unix timestamp
        #[arg(long)]
        since: Option<u64>,
    },
    /// Clear notification history
    ClearHistory,
    /// Watch for new notifications (subscribe to D-Bus events)
    Watch,
}
