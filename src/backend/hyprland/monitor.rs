use super::*;

pub(super) async fn monitor_set_primary(
    _backend: &HyprBackend,
    _output: &str,
) -> anyhow::Result<()> {
    anyhow::bail!("Hyprland does not expose a primary monitor setting")
}

pub(super) async fn monitor_set_resolution(
    backend: &HyprBackend,
    output: &str,
    width: u32,
    height: u32,
    refresh_rate: Option<f64>,
) -> anyhow::Result<()> {
    let mut config = backend.monitor_config(output).await?;
    config.width = width;
    config.height = height;
    if let Some(refresh_rate) = refresh_rate {
        config.refresh_rate = refresh_rate;
    }
    backend.apply_monitor_config(&config).await
}

pub(super) async fn monitor_set_scale(
    backend: &HyprBackend,
    output: &str,
    scale: f64,
) -> anyhow::Result<()> {
    let mut config = backend.monitor_config(output).await?;
    config.scale = scale;
    backend.apply_monitor_config(&config).await
}

pub(super) async fn monitor_set_rotation(
    backend: &HyprBackend,
    output: &str,
    rotation: &str,
) -> anyhow::Result<()> {
    let mut config = backend.monitor_config(output).await?;
    config.transform = rotation_to_hypr_transform(rotation)?;
    backend.apply_monitor_config(&config).await
}

pub(super) async fn monitor_set_enabled(
    backend: &HyprBackend,
    output: &str,
    enabled: bool,
) -> anyhow::Result<()> {
    let value = if enabled {
        format!("{},preferred,auto,1", output)
    } else {
        format!("{},disable", output)
    };
    backend.hyprctl_keyword("monitor", &value).await?;
    backend.refresh_monitors_cache().await;
    Ok(())
}
