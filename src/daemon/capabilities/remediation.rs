pub fn health_remediation() -> serde_json::Value {
    serde_json::json!({
        "ydotoold": "Start ydotoold in your user session (e.g. autostart entry).",
        "uinput": "Configure udev: KERNEL==\"uinput\", GROUP=\"input\", MODE=\"0660\" and add your user to input group.",
        "gnome-extension": "Install/enable deskbrid GNOME extension, then restart shell/session.",
        "grim": "Install grim package for screenshots.",
        "spectacle": "Install spectacle package for KDE screenshots."
    })
}

pub async fn run_system_remediation(check: &str, apply: bool) -> anyhow::Result<serde_json::Value> {
    match check {
        "ydotoold" => remediate_ydotoold(apply).await,
        "kde_ydotoold_autostart" => remediate_kde_ydotoold_autostart(apply).await,
        _ => Ok(serde_json::json!({
            "check": check,
            "applied": false,
            "error": "unknown check"
        })),
    }
}

async fn remediate_ydotoold(apply: bool) -> anyhow::Result<serde_json::Value> {
    if !apply {
        return Ok(serde_json::json!({
            "check":"ydotoold",
            "applied": false,
            "command":"ydotoold &",
            "note":"Set apply=true to start ydotoold in current user session"
        }));
    }
    tokio::process::Command::new("sh")
        .arg("-c")
        .arg("pgrep -x ydotoold >/dev/null 2>&1 || (nohup ydotoold >/tmp/deskbrid-ydotoold.log 2>&1 &)")
        .output()
        .await?;
    let running = tokio::process::Command::new("pgrep")
        .args(["-x", "ydotoold"])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);
    Ok(serde_json::json!({
        "check":"ydotoold",
        "applied": running,
        "details": if running { "started_or_already_running" } else { "failed_to_start" }
    }))
}

async fn remediate_kde_ydotoold_autostart(apply: bool) -> anyhow::Result<serde_json::Value> {
    let home = std::env::var("HOME").unwrap_or_default();
    let path = format!("{}/.config/autostart/ydotoold.desktop", home);
    if !apply {
        return Ok(serde_json::json!({
            "check":"kde_ydotoold_autostart",
            "applied":false,
            "path":path
        }));
    }
    tokio::fs::create_dir_all(format!("{}/.config/autostart", home)).await?;
    let desktop = "[Desktop Entry]\nType=Application\nExec=ydotoold\nHidden=false\nNoDisplay=false\nX-GNOME-Autostart-enabled=true\nName=Deskbrid ydotool Daemon\nComment=Auto-start ydotoold for input injection\n";
    tokio::fs::write(&path, desktop).await?;
    Ok(serde_json::json!({
        "check":"kde_ydotoold_autostart",
        "applied":true,
        "path":path
    }))
}
