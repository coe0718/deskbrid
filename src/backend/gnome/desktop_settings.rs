use super::GnomeBackend;

impl GnomeBackend {
    async fn gsettings(&self, args: &[&str]) -> anyhow::Result<String> {
        let mut cmd = tokio::process::Command::new("gsettings");
        cmd.args(args)
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());
        let out = cmd.output().await?;
        if !out.status.success() {
            anyhow::bail!(
                "gsettings failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(String::from_utf8(out.stdout)?.trim().to_string())
    }

    pub(super) async fn desktop_get_setting(
        &self,
        schema: &str,
        key: &str,
    ) -> anyhow::Result<String> {
        self.gsettings(&["get", schema, key]).await
    }

    pub(super) async fn desktop_set_setting(
        &self,
        schema: &str,
        key: &str,
        value: &str,
    ) -> anyhow::Result<()> {
        self.gsettings(&["set", schema, key, value]).await?;
        Ok(())
    }

    pub(super) async fn desktop_list_schemas(&self) -> anyhow::Result<Vec<String>> {
        let out = self.gsettings(&["list-schemas"]).await?;
        Ok(out
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }
}
