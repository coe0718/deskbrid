use super::GnomeBackend;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;

impl GnomeBackend {
    pub(super) async fn clipboard_read_inner(&self) -> anyhow::Result<String> {
        self.sh("wl-paste", &[]).await
    }

    pub(super) async fn clipboard_write_inner(&self, text: &str) -> anyhow::Result<()> {
        let mut child = tokio::process::Command::new("wl-copy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).await?;
        }
        let status = child.wait().await?;
        if !status.success() {
            anyhow::bail!("wl-copy failed");
        }
        Ok(())
    }
}
