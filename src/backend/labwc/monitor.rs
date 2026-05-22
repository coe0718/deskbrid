use super::*;

pub(super) async fn monitor_set_primary(
    _backend: &LabwcBackend,
    _output: &str,
) -> anyhow::Result<()> {
    anyhow::bail!("monitor_set_primary not implemented on Labwc backend")
}

pub(super) async fn monitor_set_resolution(
    _backend: &LabwcBackend,
    _output: &str,
    _width: u32,
    _height: u32,
    _refresh_rate: Option<f64>,
) -> anyhow::Result<()> {
    anyhow::bail!("monitor_set_resolution not implemented on Labwc backend")
}

pub(super) async fn monitor_set_scale(
    _backend: &LabwcBackend,
    _output: &str,
    _scale: f64,
) -> anyhow::Result<()> {
    anyhow::bail!("monitor_set_scale not implemented on Labwc backend")
}

pub(super) async fn monitor_set_rotation(
    _backend: &LabwcBackend,
    _output: &str,
    _rotation: &str,
) -> anyhow::Result<()> {
    anyhow::bail!("monitor_set_rotation not implemented on Labwc backend")
}

pub(super) async fn monitor_set_enabled(
    _backend: &LabwcBackend,
    _output: &str,
    _enabled: bool,
) -> anyhow::Result<()> {
    anyhow::bail!("monitor_set_enabled not implemented on Labwc backend")
}
