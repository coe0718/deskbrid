use clap::Subcommand;

#[derive(Subcommand)]
pub enum NetworkCmd {
    /// Connection status
    Status,
    /// List interfaces
    Interfaces,
    /// List active connections
    Connections,
    /// List saved connection profiles
    Profiles,
    /// Start a WiFi hotspot
    HotspotStart {
        /// SSID for the hotspot
        ssid: String,
        /// Optional WiFi password
        #[arg(long)]
        password: Option<String>,
    },
    /// Stop the active hotspot
    HotspotStop,
    /// Enable or disable WiFi
    WifiEnable {
        /// Set to "true" to enable, "false" to disable
        enabled: bool,
    },
    /// Enable or disable mobile broadband (WWAN)
    WwanEnable {
        /// Set to "true" to enable, "false" to disable
        enabled: bool,
    },
    /// Set custom DNS servers
    DnsSet {
        /// DNS server addresses (e.g., 8.8.8.8 1.1.1.1)
        dns: Vec<String>,
    },
    /// Reset DNS to auto-configuration from DHCP
    DnsReset,
    /// Connect to a VPN profile
    VpnConnect {
        /// Name of the VPN connection profile
        profile_name: String,
    },
    /// Disconnect all VPN connections
    VpnDisconnect,
}

#[derive(Subcommand)]
pub enum WifiCmd {
    /// Scan for networks
    Scan,
    /// Connect to a network
    Connect { ssid: String },
}
