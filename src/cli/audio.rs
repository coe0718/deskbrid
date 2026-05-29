use clap::Subcommand;

#[derive(Subcommand)]
pub enum AudioCmd {
    /// List audio sinks (output devices)
    Sinks,
    /// List audio sources (input devices)
    Sources,
    /// Get volume for a sink or source
    GetVolume {
        /// "sink" or "source"
        target: String,
        /// Device ID
        id: u32,
    },
    /// Set volume for a sink or source
    SetVolume {
        /// "sink" or "source"
        target: String,
        /// Device ID
        id: u32,
        /// Volume level 0.0-1.0
        volume: f64,
    },
    /// Set sink volume (legacy shorthand)
    Volume { sink_id: u32, volume: f64 },
    /// Mute or unmute a sink or source
    Mute {
        /// "sink" or "source"
        target: String,
        /// Device ID
        id: u32,
        /// true to mute, false to unmute
        mute: String,
    },
    /// Set default sink or source
    SetDefault {
        /// "sink" or "source"
        target: String,
        /// Device name
        name: String,
    },
}
