use super::*;

pub(super) async fn monitor_set_primary(
    _backend: &NiriBackend,
    _output: &str,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn monitor_set_resolution(
    _backend: &NiriBackend,
    _output: &str,
    _w: u32,
    _h: u32,
    _rr: Option<f64>,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn monitor_set_scale(
    _backend: &NiriBackend,
    _output: &str,
    _scale: f64,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn monitor_set_rotation(
    _backend: &NiriBackend,
    _output: &str,
    _rot: &str,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn monitor_set_enabled(
    _backend: &NiriBackend,
    _output: &str,
    _enabled: bool,
) -> anyhow::Result<()> {
    Ok(())
}
