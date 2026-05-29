use anyhow::{Context, bail};

#[derive(serde::Deserialize)]
pub(crate) struct GitHubRelease {
    pub(crate) tag_name: String,
    pub(crate) assets: Vec<GitHubAsset>,
}

impl GitHubRelease {
    pub(crate) fn find_asset(&self, asset_name: &str, arch: &str) -> anyhow::Result<&GitHubAsset> {
        self.assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .or_else(|| self.assets.iter().find(|asset| asset.name.contains(arch)))
            .context(format!(
                "no release asset for architecture '{arch}'. Available: {:?}",
                self.assets.iter().map(|a| &a.name).collect::<Vec<_>>()
            ))
    }
}

#[derive(serde::Deserialize)]
pub(crate) struct GitHubAsset {
    pub(crate) name: String,
    pub(crate) browser_download_url: String,
}

pub(crate) async fn fetch_latest_release(
    client: &reqwest::Client,
    repo: &str,
) -> anyhow::Result<GitHubRelease> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let response = client
        .get(url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .context("failed to query GitHub API")?;
    if response.status() == reqwest::StatusCode::FORBIDDEN {
        bail!("GitHub API rate limited. Try again later or set GITHUB_TOKEN.");
    }
    if !response.status().is_success() {
        bail!("GitHub API returned status: {}", response.status());
    }
    response
        .json()
        .await
        .context("failed to parse GitHub release JSON")
}

pub(crate) async fn download(
    client: &reqwest::Client,
    url: &str,
    path: &std::path::Path,
) -> anyhow::Result<()> {
    let response = client
        .get(url)
        .send()
        .await
        .context("download request failed")?;
    if !response.status().is_success() {
        bail!("download failed with status: {}", response.status());
    }
    let bytes = response
        .bytes()
        .await
        .context("failed to read download body")?;
    std::fs::write(path, &bytes).context("failed to write downloaded archive")?;
    Ok(())
}

pub(crate) async fn verify_checksum_if_available(
    client: &reqwest::Client,
    release: &GitHubRelease,
    asset: &GitHubAsset,
    archive_path: &std::path::Path,
) -> anyhow::Result<String> {
    let checksum_name = format!("{}.sha256", asset.name);
    let Some(checksum_asset) = release.assets.iter().find(|a| a.name == checksum_name) else {
        return Ok("no checksum asset published; skipped".to_string());
    };

    let checksum_path = archive_path.with_extension("tar.gz.sha256");
    download(client, &checksum_asset.browser_download_url, &checksum_path).await?;
    let status = tokio::process::Command::new("sha256sum")
        .arg("-c")
        .arg(&checksum_path)
        .current_dir(
            archive_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new(".")),
        )
        .status()
        .await
        .context("failed to run sha256sum")?;
    if !status.success() {
        bail!("checksum verification failed for {}", asset.name);
    }
    Ok("verified".to_string())
}
