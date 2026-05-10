//! `deskbrid setup` — one-command desktop setup.
//! Auto-detects your desktop and only installs what's needed.

/// GNOME Shell extension metadata, baked into the binary at compile time.
pub const GNOME_EXTENSION_METADATA: &str =
    include_str!("../extensions/deskbrid@deskbrid/metadata.json");

/// GNOME Shell extension JavaScript, baked into the binary at compile time.
pub const GNOME_EXTENSION_JS: &str =
    include_str!("../extensions/deskbrid@deskbrid/extension.js");

enum DesktopEnv {
    Gnome,
    Hyprland,
    Kde,
    Unknown,
}

/// Detect the running desktop environment by checking env vars and processes.
fn detect_desktop() -> DesktopEnv {
    if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
        let lower = desktop.to_lowercase();
        if lower.contains("gnome") || lower.contains("unity") {
            return DesktopEnv::Gnome;
        }
        if lower.contains("hyprland") {
            return DesktopEnv::Hyprland;
        }
        if lower.contains("kde") || lower.contains("plasma") {
            return DesktopEnv::Kde;
        }
    }

    // Fallback: check running processes
    for (name, env) in [
        ("gnome-shell", DesktopEnv::Gnome),
        ("Hyprland", DesktopEnv::Hyprland),
        ("kwin_wayland", DesktopEnv::Kde),
    ] {
        if let Ok(out) = std::process::Command::new("pgrep")
            .args(["-x", name])
            .output()
        {
            if out.status.success() {
                return env;
            }
        }
    }

    DesktopEnv::Unknown
}

/// One-command setup: detect desktop and install what's needed.
pub async fn run() -> anyhow::Result<()> {
    match detect_desktop() {
        DesktopEnv::Gnome => setup_gnome().await,
        DesktopEnv::Hyprland => setup_hyprland().await,
        DesktopEnv::Kde => setup_kde().await,
        DesktopEnv::Unknown => {
            eprintln!("Could not detect your desktop environment.");
            eprintln!("Run `deskbrid setup` from within a desktop session, or check:");
            eprintln!("  $XDG_CURRENT_DESKTOP should be set on Wayland");
            Ok(())
        }
    }
}

/// Install and enable the GNOME Shell extension.
async fn setup_gnome() -> anyhow::Result<()> {
    eprintln!("Detected: GNOME Shell");
    let home = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("$HOME not set"))?;
    let ext_dir = format!(
        "{}/.local/share/gnome-shell/extensions/deskbrid@deskbrid",
        home
    );

    std::fs::create_dir_all(&ext_dir)?;
    std::fs::write(format!("{}/metadata.json", ext_dir), GNOME_EXTENSION_METADATA)?;
    std::fs::write(format!("{}/extension.js", ext_dir), GNOME_EXTENSION_JS)?;
    eprintln!("  ✓ Extension files written to {ext_dir}");

    match std::process::Command::new("gnome-extensions")
        .args(["enable", "deskbrid@deskbrid"])
        .output()
    {
        Ok(out) if out.status.success() => {
            eprintln!("  ✓ GNOME Shell extension enabled");
            eprintln!("  → Log out and back in (or Alt+F2 → r on X11) to activate");
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            eprintln!("  ⚠ Could not enable extension: {stderr}");
        }
        Err(e) => {
            eprintln!("  ⚠ gnome-extensions not found: {e}");
            eprintln!("  Extensions are installed — enable manually:");
            eprintln!("    gnome-extensions enable deskbrid@deskbrid");
        }
    }

    Ok(())
}

/// Print Hyprland setup tips.
async fn setup_hyprland() -> anyhow::Result<()> {
    eprintln!("Detected: Hyprland");
    eprintln!("  No extension needed — deskbrid uses hyprctl + ydotool directly.");
    eprintln!("  Make sure ydotoold is running:");
    eprintln!("    echo 'exec-once = ydotoold' >> ~/.config/hypr/hyprland.conf");
    eprintln!("  And /dev/uinput permissions are set (see README).");
    Ok(())
}

/// Print KDE setup tips.
async fn setup_kde() -> anyhow::Result<()> {
    eprintln!("Detected: KDE Plasma");
    eprintln!("  No extension needed — deskbrid uses KWin D-Bus + ydotool directly.");
    eprintln!("  Make sure ydotoold is running as your user (not root):");
    eprintln!("    ydotoold &");
    eprintln!("  Or add ydotoold to KDE autostart:");
    eprintln!("    mkdir -p ~/.config/autostart");
    eprintln!("    cat > ~/.config/autostart/ydotoold.desktop << 'EOF'");
    eprintln!("    [Desktop Entry]");
    eprintln!("    Type=Application");
    eprintln!("    Name=ydotoold");
    eprintln!("    Exec=ydotoold");
    eprintln!("    Terminal=false");
    eprintln!("    NoDisplay=true");
    eprintln!("    X-KDE-autostart-phase=2");
    eprintln!("    EOF");
    Ok(())
}
