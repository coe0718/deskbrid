use super::*;

pub(super) async fn monitor_set_primary(
    _backend: &NiriBackend,
    _output: &str,
) -> anyhow::Result<()> {
    anyhow::bail!("Niri does not expose a primary monitor setting")
}

pub(super) async fn monitor_set_resolution(
    backend: &NiriBackend,
    output: &str,
    width: u32,
    height: u32,
    refresh_rate: Option<f64>,
) -> anyhow::Result<()> {
    run_wlr_randr(
        backend,
        crate::backend::wlr_randr::set_resolution_args(output, width, height, refresh_rate),
    )
    .await
}

pub(super) async fn monitor_set_scale(
    backend: &NiriBackend,
    output: &str,
    scale: f64,
) -> anyhow::Result<()> {
    run_wlr_randr(
        backend,
        crate::backend::wlr_randr::set_scale_args(output, scale),
    )
    .await
}

pub(super) async fn monitor_set_rotation(
    backend: &NiriBackend,
    output: &str,
    rotation: &str,
) -> anyhow::Result<()> {
    run_wlr_randr(
        backend,
        crate::backend::wlr_randr::set_rotation_args(output, rotation)?,
    )
    .await
}

pub(super) async fn monitor_set_enabled(
    backend: &NiriBackend,
    output: &str,
    enabled: bool,
) -> anyhow::Result<()> {
    run_wlr_randr(
        backend,
        crate::backend::wlr_randr::set_enabled_args(output, enabled),
    )
    .await
}

async fn run_wlr_randr(backend: &NiriBackend, args: Vec<String>) -> anyhow::Result<()> {
    let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    backend.sh("wlr-randr", &refs).await.map(|_| ())
}
