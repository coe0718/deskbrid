use super::*;

pub(super) async fn monitor_set_primary(
    backend: &CosmicBackend,
    output: &str,
) -> anyhow::Result<()> {
    // cosmic-randr has no "primary" concept — Wayland doesn't use it.
    // Use xwayland-primary as the closest equivalent.
    backend.sh("cosmic-randr", &["xwayland", output]).await?;
    Ok(())
}

pub(super) async fn monitor_set_resolution(
    backend: &CosmicBackend,
    output: &str,
    width: u32,
    height: u32,
    refresh_rate: Option<f64>,
) -> anyhow::Result<()> {
    let mut args = vec![
        "mode".to_string(),
        output.to_string(),
        width.to_string(),
        height.to_string(),
    ];
    if let Some(refresh) = refresh_rate {
        args.push("--refresh".to_string());
        args.push(format_monitor_float(refresh));
    }
    backend.sh_owned("cosmic-randr", args).await?;
    Ok(())
}

pub(super) async fn monitor_set_scale(
    backend: &CosmicBackend,
    output: &str,
    scale: f64,
) -> anyhow::Result<()> {
    // cosmic-randr mode --scale <value> — requires width+height too,
    // so we first list the current mode to preserve it.
    let list = backend
        .helper_json(&["list-monitors"])
        .await
        .unwrap_or_default();
    let current_w = list.get("width").and_then(|v| v.as_u64()).unwrap_or(1920) as u32;
    let current_h = list.get("height").and_then(|v| v.as_u64()).unwrap_or(1080) as u32;

    backend
        .sh_owned(
            "cosmic-randr",
            vec![
                "mode".to_string(),
                output.to_string(),
                current_w.to_string(),
                current_h.to_string(),
                "--scale".to_string(),
                format_monitor_float(scale),
            ],
        )
        .await?;
    Ok(())
}

pub(super) async fn monitor_set_rotation(
    backend: &CosmicBackend,
    output: &str,
    rotation: &str,
) -> anyhow::Result<()> {
    let transform = cosmic_transform(rotation)?;
    // cosmic-randr mode --transform <value> — needs width+height
    let list = backend
        .helper_json(&["list-monitors"])
        .await
        .unwrap_or_default();
    let current_w = list.get("width").and_then(|v| v.as_u64()).unwrap_or(1920) as u32;
    let current_h = list.get("height").and_then(|v| v.as_u64()).unwrap_or(1080) as u32;

    backend
        .sh_owned(
            "cosmic-randr",
            vec![
                "mode".to_string(),
                output.to_string(),
                current_w.to_string(),
                current_h.to_string(),
                "--transform".to_string(),
                transform.to_string(),
            ],
        )
        .await?;
    Ok(())
}

pub(super) async fn monitor_set_enabled(
    backend: &CosmicBackend,
    output: &str,
    enabled: bool,
) -> anyhow::Result<()> {
    let subcmd = if enabled { "enable" } else { "disable" };
    backend.sh("cosmic-randr", &[subcmd, output]).await?;
    Ok(())
}
