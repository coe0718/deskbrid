use clap::Subcommand;

#[derive(Subcommand)]
pub enum PortalCmd {
    /// Take a screenshot via XDG Desktop Portal
    Screenshot {
        /// Show interactive picker to select area/window
        #[arg(long)]
        interactive: bool,
    },
    /// Start screencast via XDG Desktop Portal (requires PipeWire)
    ScreencastStart {
        /// Output file path for the recording
        output_path: String,
    },
    /// Stop portal screencast
    ScreencastStop,
}
