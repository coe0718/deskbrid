use tokio::process::Command;

/// Try xfconf-query first (XFCE), fall back to gsettings (MATE/Cinnamon/etc.)
pub(super) async fn desktop_get_setting(schema: &str, key: &str) -> anyhow::Result<String> {
    // Try xfconf-query first (XFCE)
    let output = Command::new("xfconf-query")
        .args(["-c", schema, "-p", key])
        .output()
        .await?;
    if output.status.success() {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !s.is_empty() {
            return Ok(s);
        }
    }

    // Fall back to gsettings (MATE, Cinnamon, etc.)
    let output = Command::new("gsettings")
        .args(["get", schema, key])
        .output()
        .await?;
    if output.status.success() {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Ok(s.trim_matches('\'').to_string());
    }

    anyhow::bail!("failed to get setting {} {}", schema, key)
}

pub(super) async fn desktop_set_setting(
    schema: &str,
    key: &str,
    value: &str,
) -> anyhow::Result<()> {
    // Try xfconf-query first
    let xfce = Command::new("xfconf-query")
        .args(["-c", schema, "-p", key, "-s", value])
        .output()
        .await?;
    if xfce.status.success() {
        return Ok(());
    }

    // Fall back to gsettings
    let gs = Command::new("gsettings")
        .args(["set", schema, key, value])
        .output()
        .await?;
    if gs.status.success() {
        return Ok(());
    }

    anyhow::bail!("failed to set setting {} {} = {}", schema, key, value)
}

pub(super) async fn desktop_list_schemas() -> anyhow::Result<Vec<String>> {
    // xfconf-query: list channels
    let output = Command::new("xfconf-query").args(["-l"]).output().await?;
    if output.status.success() {
        let channels: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.trim().to_string())
            .collect();
        if !channels.is_empty() {
            return Ok(channels);
        }
    }

    // gsettings: list schemas
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

    anyhow::bail!("no settings backend found (tried xfconf-query and gsettings)")
}
