use clap::Subcommand;

#[derive(Subcommand)]
pub enum SessionCmd {
    /// Create a new named session
    Create {
        /// Session name
        name: String,
        /// Clone an existing session's variables
        #[arg(long)]
        clone_from: Option<String>,
    },
    /// Destroy a named session
    Destroy {
        /// Session name
        name: String,
    },
    /// List all named sessions
    List,
    /// Switch to a named session (connect alias)
    Switch {
        /// Session name
        name: String,
    },
    /// Manage session variables
    #[command(name = "var")]
    Var {
        #[command(subcommand)]
        cmd: VarCmd,
    },
}

#[derive(Subcommand)]
pub enum VarCmd {
    /// Set a session variable
    Set {
        /// Variable name
        name: String,
        /// Variable value
        value: String,
    },
    /// Get a session variable
    Get {
        /// Variable name
        name: String,
    },
    /// List all variables in current session
    List,
}
