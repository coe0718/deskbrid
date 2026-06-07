use clap::Subcommand;

#[derive(Subcommand)]
pub enum SystemCmd {
    /// Show system info
    Info,
    /// Get idle seconds
    Idle,
    /// Power action
    Power { action: String },
    /// Battery status
    Battery,
    /// List all backlight devices
    BacklightList,
    /// Read backlight brightness from /sys/class/backlight
    BacklightGet { device: Option<String> },
    /// Set backlight brightness (absolute value or "50%")
    BacklightSet {
        value: String,
        #[arg(long)]
        device: Option<String>,
    },
    /// List printers
    PrintList,
    /// Get or set default printer
    PrintDefault {
        #[arg(long)]
        printer: Option<String>,
    },
    /// Send a file to a printer
    PrintFile { printer: String, path: String },
    /// List print jobs
    PrintJobs,
    /// Cancel a print job
    PrintJobCancel { job_id: String },
    /// Pause a print job
    PrintJobPause { job_id: String },
    /// Resume a paused print job
    PrintJobResume { job_id: String },
    /// Read system pressure (PSI) — CPU, memory, IO
    Pressure,
    /// Read thermal zones from /sys/class/thermal
    Thermal,
    /// Read CPU frequency details
    CpuFrequency,
    /// Read CPU frequency governors
    CpuGovernor,
    /// Set CPU frequency governor on all writable CPUs
    CpuSetGovernor { governor: String },
    /// Inhibit sleep/shutdown/idle while work is active
    Inhibit {
        what: String,
        #[arg(long, default_value = "deskbrid")]
        who: String,
        #[arg(long)]
        why: Option<String>,
        #[arg(long)]
        mode: Option<String>,
    },
    /// Release a Deskbrid-created inhibitor
    ReleaseInhibit { inhibitor_id: u32 },
    /// List logind sessions
    Sessions,
    /// Lock the current or specified logind session
    LockSession { session_id: Option<String> },
    /// Switch to another display-manager user
    SwitchUser { username: String },
    /// Check a polkit action without prompting
    CheckAuth { action_id: String },
    /// Request polkit authorization with user interaction
    Elevate {
        action_id: String,
        #[arg(long)]
        reason: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ServiceCmd {
    /// Show one unit's status
    Status { name: String },
    /// Start a unit
    Start { name: String },
    /// Stop a unit
    Stop { name: String },
    /// Restart a unit
    Restart { name: String },
    /// Enable a unit
    Enable {
        name: String,
        #[arg(long)]
        runtime: bool,
    },
    /// Disable a unit
    Disable {
        name: String,
        #[arg(long)]
        runtime: bool,
    },
    /// List units by type
    List { unit_type: Option<String> },
}

#[derive(Subcommand)]
pub enum JournalCmd {
    /// Query journald lines
    Query {
        #[arg(long)]
        since: Option<u64>,
        #[arg(long)]
        until: Option<u64>,
        #[arg(long)]
        unit: Option<String>,
        #[arg(long)]
        priority: Option<u8>,
        #[arg(long)]
        tail: Option<u32>,
    },
}

#[derive(Subcommand)]
pub enum TimerCmd {
    /// List systemd timers
    List,
    /// Start a timer
    Start { name: String },
    /// Stop a timer
    Stop { name: String },
}
