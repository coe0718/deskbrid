use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopType {
    Gnome,
    Kde,
    Wlroots,
    Other,
}

pub async fn detect_desktop() -> DesktopType {
    tokio::task::spawn_blocking(|| {
        if env_contains(
            &[
                "XDG_CURRENT_DESKTOP",
                "XDG_SESSION_DESKTOP",
                "DESKTOP_SESSION",
            ],
            "gnome",
        ) {
            return DesktopType::Gnome;
        }
        if env_contains(
            &[
                "XDG_CURRENT_DESKTOP",
                "XDG_SESSION_DESKTOP",
                "DESKTOP_SESSION",
            ],
            "kde",
        ) || env_contains(&["XDG_CURRENT_DESKTOP"], "plasma")
        {
            return DesktopType::Kde;
        }

        if process_exists(&["gnome-shell"]) {
            return DesktopType::Gnome;
        }
        if process_exists(&["kwin_wayland", "kwin_x11", "plasmashell"]) {
            return DesktopType::Kde;
        }
        if command_exists("sway", Some("--version"))
            || command_exists("hyprctl", None)
            || command_exists("riverctl", None)
            || command_exists("wayfire", Some("--version"))
        {
            return DesktopType::Wlroots;
        }

        DesktopType::Other
    })
    .await
    .unwrap_or(DesktopType::Other)
}

fn env_contains(vars: &[&str], needle: &str) -> bool {
    vars.iter().any(|key| {
        std::env::var(key)
            .map(|value| value.to_ascii_lowercase().contains(needle))
            .unwrap_or(false)
    })
}

fn process_exists(names: &[&str]) -> bool {
    let entries = match fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(_) => return false,
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let pid = match file_name.to_string_lossy().parse::<u32>() {
            Ok(pid) => pid,
            Err(_) => continue,
        };

        let comm_path = format!("/proc/{pid}/comm");
        let name = match fs::read_to_string(&comm_path) {
            Ok(name) => name.trim().to_ascii_lowercase(),
            Err(_) => continue,
        };

        if names.iter().any(|candidate| name == *candidate) {
            return true;
        }
    }

    false
}

fn command_exists(program: &str, arg: Option<&str>) -> bool {
    let mut command = Command::new(program);
    if let Some(arg) = arg {
        command.arg(arg);
    }

    command
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
