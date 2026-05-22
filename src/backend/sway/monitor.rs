use super::*;

pub(super) async fn monitor_set_primary(backend: &SwayBackend, output: &str) -> anyhow::Result<()> {
    backend.swaymsg_raw(&["focus", "output", output]).await
}

pub(super) async fn monitor_set_resolution(
    backend: &SwayBackend,
    output: &str,
    width: u32,
    height: u32,
    refresh_rate: Option<f64>,
) -> anyhow::Result<()> {
    let mode = if let Some(rr) = refresh_rate {
        format!("{}x{}@{}Hz", width, height, rr)
    } else {
        format!("{}x{}", width, height)
    };
    backend
        .swaymsg_raw(&[&format!("output {} resolution {}", output, mode)])
        .await
}

pub(super) async fn monitor_set_scale(
    backend: &SwayBackend,
    output: &str,
    scale: f64,
) -> anyhow::Result<()> {
    backend
        .swaymsg_raw(&[&format!("output {} scale {:.2}", output, scale)])
        .await
}

pub(super) async fn monitor_set_rotation(
    backend: &SwayBackend,
    output: &str,
    rotation: &str,
) -> anyhow::Result<()> {
    let rot = match rotation {
        "normal" | "0" => "0",
        "left" | "90" => "90",
        "right" | "270" => "270",
        "inverted" | "180" => "180",
        _ => anyhow::bail!("unsupported rotation: {}", rotation),
    };
    backend
        .swaymsg_raw(&[&format!("output {} transform {}", output, rot)])
        .await
}

pub(super) async fn monitor_set_enabled(
    backend: &SwayBackend,
    output: &str,
    enabled: bool,
) -> anyhow::Result<()> {
    let action = if enabled { "enable" } else { "disable" };
    backend
        .swaymsg_raw(&[&format!("output {} {}", output, action)])
        .await
}
