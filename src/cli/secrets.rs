use clap::Subcommand;

#[derive(Subcommand)]
pub enum SecretsCmd {
    /// List available keyring collections
    List,
    /// Look up a secret by its attributes
    Lookup {
        /// Key=value pairs (e.g. "service=github" "username=tuck")
        attributes: Vec<String>,
    },
    /// Store a secret in the keyring
    Store {
        /// Key=value pairs that identify the secret (e.g. "service=github")
        attributes: Vec<String>,
        /// The secret value to store
        secret: String,
        /// Optional human-readable label
        #[arg(long)]
        label: Option<String>,
        /// Optional collection path
        #[arg(long)]
        collection: Option<String>,
    },
}
