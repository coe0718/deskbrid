use super::*;
use crate::protocol;

pub(super) async fn audio_list_sinks(
    _backend: &CosmicBackend,
) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
    Ok(vec![])
}

pub(super) async fn audio_set_sink_volume(
    _backend: &CosmicBackend,
    _sink_id: u32,
    _volume: f64,
) -> anyhow::Result<()> {
    Ok(())
}

// ═══════════════════════════════════════════════════════
// MONITOR (via cosmic-randr)
// ═══════════════════════════════════════════════════════
