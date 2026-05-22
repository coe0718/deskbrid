use super::*;
use crate::protocol;

pub(super) async fn bluetooth_list(
    _backend: &CosmicBackend,
) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
    Ok(vec![])
}

pub(super) async fn bluetooth_scan(
    _backend: &CosmicBackend,
    _duration: Option<u32>,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn bluetooth_stop_scan(_backend: &CosmicBackend) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn bluetooth_connect(
    _backend: &CosmicBackend,
    _address: &str,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn bluetooth_disconnect(
    _backend: &CosmicBackend,
    _address: &str,
) -> anyhow::Result<()> {
    Ok(())
}

// ─── Files ──────────────────────────────────────────
