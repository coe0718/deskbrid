use super::*;

pub(super) async fn notification_send(
    backend: &CosmicBackend,
    _app_name: &str,
    title: &str,
    body: &str,
    urgency: &str,
) -> anyhow::Result<u32> {
    let u = match urgency {
        "low" => "low",
        "critical" => "critical",
        _ => "normal",
    };
    backend.sh("notify-send", &["-u", u, title, body]).await?;
    // notify-send doesn't return an ID; return 0
    Ok(0)
}

pub(super) async fn notification_close(_backend: &CosmicBackend, _id: u32) -> anyhow::Result<()> {
    // notify-send doesn't support close by ID
    Ok(())
}

// ─── System ─────────────────────────────────────────
