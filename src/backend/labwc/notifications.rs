use super::*;

pub(super) async fn notification_send(
    backend: &LabwcBackend,
    a: &str,
    t: &str,
    b: &str,
    u: &str,
) -> anyhow::Result<u32> {
    let out = backend
        .sh("notify-send", &["-a", a, "-u", u, "--print-id", t, b])
        .await?;
    Ok(out.parse().unwrap_or(0))
}

pub(super) async fn notification_close(backend: &LabwcBackend, id: u32) -> anyhow::Result<()> {
    backend
        .sh("makoctl", &["dismiss", "-n", &id.to_string()])
        .await
        .map(|_| ())
}
