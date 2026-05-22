use super::*;

pub(super) async fn notification_send(
    backend: &HyprBackend,
    app_name: &str,
    title: &str,
    body: &str,
    urgency: &str,
) -> anyhow::Result<u32> {
    let urgency_byte = match urgency {
        "low" => "low",
        "normal" => "normal",
        "critical" => "critical",
        _ => "normal",
    };
    let output = backend
        .sh(
            "notify-send",
            &[
                "--app-name",
                app_name,
                "--urgency",
                urgency_byte,
                "--print-id",
                title,
                body,
            ],
        )
        .await?;
    Ok(output.parse().unwrap_or(0))
}

pub(super) async fn notification_close(backend: &HyprBackend, id: u32) -> anyhow::Result<()> {
    if backend
        .sh_ok("makoctl", &["dismiss", &id.to_string()])
        .await
    {
        return Ok(());
    }
    Ok(())
}
