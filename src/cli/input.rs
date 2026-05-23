use clap::Subcommand;

#[derive(Subcommand)]
pub enum InputCmd {
    /// Type a string
    Type { text: String },
    /// Press a single key
    Key { key: String },
}

#[derive(Subcommand)]
pub enum MouseCmd {
    /// Move cursor to position
    Move { x: f64, y: f64 },
    /// Click: left, middle, right
    Click { button: String },
    /// Scroll: dx dy
    Scroll { dx: f64, dy: f64 },
    /// Drag from one coordinate to another
    Drag {
        from_x: f64,
        from_y: f64,
        to_x: f64,
        to_y: f64,
        #[arg(long, default_value = "left")]
        button: String,
        #[arg(long)]
        duration_ms: Option<u64>,
    },
}
