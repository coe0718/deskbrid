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
    /// Filesystem usage for one path or all mounts
    StorageUsage {
        /// Optional path; omit to list every real mount
        path: Option<String>,
    },
    /// Largest files/directories under a path
    StorageScan {
        path: String,
        #[arg(long)]
        max_depth: Option<u32>,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Read thermal zones from /sys/class/thermal
    Thermal,
    /// List DDC/CI monitors
    DdcList,
    /// Read a VCP feature code from a monitor
    DdcGetVcp { bus: String, vcp_code: u16 },
    /// Set a VCP feature code on a monitor
    DdcSetVcp {
        bus: String,
        vcp_code: u16,
        value: u16,
    },
    /// Set DDC/CI monitor brightness (0-100%)
    DdcBrightness { bus: String, percent: f64 },
    /// Set DDC/CI monitor contrast (0-100%)
    DdcContrast { bus: String, percent: f64 },
    /// Set monitor power state via DDC/CI
    DdcPower { bus: String, state: String },
    /// Set monitor input source via DDC/CI
    DdcInput { bus: String, input: String },
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
