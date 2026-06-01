use super::KdeBackend;

impl KdeBackend {
    async fn kreadconfig(&self, args: &[&str]) -> anyhow::Result<String> {
        let mut cmd = tokio::process::Command::new("kreadconfig5");
        cmd.args(args)
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());
        let out = cmd.output().await?;
        if !out.status.success() {
            anyhow::bail!(
                "kreadconfig5 failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(String::from_utf8(out.stdout)?.trim().to_string())
    }

    async fn kwriteconfig(&self, args: &[&str]) -> anyhow::Result<()> {
        let mut cmd = tokio::process::Command::new("kwriteconfig5");
        cmd.args(args)
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());
        let out = cmd.output().await?;
        if !out.status.success() {
            anyhow::bail!(
                "kwriteconfig5 failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(())
    }

    pub(super) async fn desktop_get_setting(
        &self,
        schema: &str,
        key: &str,
    ) -> anyhow::Result<String> {
        // KDE: schema = config file (e.g. "kdeglobals"), key = group/key (e.g. "General/ColorScheme")
        let (group, setting) = key.split_once('/').unwrap_or(("General", key));
        self.kreadconfig(&["--file", schema, "--group", group, "--key", setting])
            .await
    }

    pub(super) async fn desktop_set_setting(
        &self,
        schema: &str,
        key: &str,
        value: &str,
    ) -> anyhow::Result<()> {
        let (group, setting) = key.split_once('/').unwrap_or(("General", key));
        self.kwriteconfig(&["--file", schema, "--group", group, "--key", setting, value])
            .await
    }

    pub(super) async fn desktop_list_schemas(&self) -> anyhow::Result<Vec<String>> {
        let glob = std::fs::read_dir(&format!(
            "{}/.config",
            std::env::var("HOME").unwrap_or_default()
        ))?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with("rc"))
        .map(|e| {
            e.file_name()
                .to_string_lossy()
                .trim_end_matches("rc")
                .to_string()
        })
        .collect();
        Ok(glob)
    }
}
