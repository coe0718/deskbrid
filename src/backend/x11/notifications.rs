use super::*;

pub(super) async fn notification_send(
    backend: &X11Backend,
    app_name: &str,
    title: &str,
    body: &str,
    urgency: &str,
) -> anyhow::Result<u32> {
    backend
        .sh("notify-send", &["-a", app_name, "-u", urgency, title, body])
        .await?;
    Ok(0)
}

pub(super) async fn notification_close(_backend: &X11Backend, _id: u32) -> anyhow::Result<()> {
    anyhow::bail!("closing notifications is not supported by notify-send on X11")
}
