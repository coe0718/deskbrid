use tokio::process::Command;

/// Shared gsettings fallback for compositors without a native settings daemon.
/// Used by Hyprland, Sway, Labwc, Niri, Wayfire, COSMIC, Cinnamon.
pub(super) async fn desktop_get_setting(schema: &str, key: &str) -> anyhow::Result<String> {
    let output = Command::new("gsettings")
        .args(["get", schema, key])
        .output()
        .await?;
    if output.status.success() {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Ok(s.trim_matches('\'').to_string());
    }
    anyhow::bail!("gsettings get failed for {} {}", schema, key)
}

pub(super) async fn desktop_set_setting(
    schema: &str,
    key: &str,
    value: &str,
) -> anyhow::Result<()> {
    let output = Command::new("gsettings")
        .args(["set", schema, key, value])
        .output()
        .await?;
    if output.status.success() {
        return Ok(());
    }
    anyhow::bail!("gsettings set failed for {} {} = {}", schema, key, value)
}

pub(super) async fn desktop_list_schemas() -> anyhow::Result<Vec<String>> {
    let output = Command::new("gsettings")
        .args(["list-schemas"])
        .output()
        .await?;
    if output.status.success() {
        let schemas: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.trim().to_string())
            .collect();
        return Ok(schemas);
    }
    anyhow::bail!("gsettings list-schemas failed")
}
