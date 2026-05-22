use super::*;

pub(super) async fn monitor_set_primary(
    _backend: &WayfireBackend,
    _output: &str,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn monitor_set_resolution(
    _backend: &WayfireBackend,
    _output: &str,
    _w: u32,
    _h: u32,
    _rr: Option<f64>,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn monitor_set_scale(
    _backend: &WayfireBackend,
    _output: &str,
    _scale: f64,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn monitor_set_rotation(
    _backend: &WayfireBackend,
    _output: &str,
    _rotation: &str,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn monitor_set_enabled(
    _backend: &WayfireBackend,
    _output: &str,
    _enabled: bool,
) -> anyhow::Result<()> {
    Ok(())
}
