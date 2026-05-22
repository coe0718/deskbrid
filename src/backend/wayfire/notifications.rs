use super::*;

pub(super) async fn notification_send(
    backend: &WayfireBackend,
    app_name: &str,
    title: &str,
    body: &str,
    urgency: &str,
) -> anyhow::Result<u32> {
    let out = backend
        .sh(
            "notify-send",
            &["-a", app_name, "-u", urgency, "--print-id", title, body],
        )
        .await?;
    Ok(out.parse().unwrap_or(0))
}

pub(super) async fn notification_close(backend: &WayfireBackend, id: u32) -> anyhow::Result<()> {
    backend
        .sh("makoctl", &["dismiss", "-n", &id.to_string()])
        .await
        .map(|_| ())
}
