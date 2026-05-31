// TESTING_NEEDED: This feature requires manual testing with actual GitHub releases
//! Self-update command: download latest release and replace the deskbrid binary.

mod github;
mod install;

use anyhow::Context;
use serde_json::{Value, json};

const GITHUB_REPO: &str = "coe0718/deskbrid";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn run(check_only: bool, force: bool) -> anyhow::Result<()> {
    let result = run_json(check_only, force).await?;
    print_update_result(&result);
    Ok(())
}

pub async fn run_json(check_only: bool, force: bool) -> anyhow::Result<Value> {
    let client = reqwest::Client::builder()
        .user_agent("deskbrid-update")
        .build()?;
    let release = github::fetch_latest_release(&client, GITHUB_REPO).await?;
    let latest_tag = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);
    let update_available = force || latest_tag != CURRENT_VERSION;

    if check_only || !update_available {
        return Ok(json!({
            "current_version": CURRENT_VERSION,
            "latest_version": latest_tag,
            "update_available": update_available,
            "checked_only": true,
            "updated": false
        }));
    }

    let arch = std::env::consts::ARCH;
    let asset_name = format!("deskbrid-{arch}-unknown-linux-gnu.tar.gz");
    let asset = release.find_asset(&asset_name, arch)?;

    let tmp_dir = tempfile::tempdir().context("failed to create temp directory")?;
    let archive_path = tmp_dir.path().join(&asset.name);
    github::download(&client, &asset.browser_download_url, &archive_path).await?;

    let checksum =
        github::verify_checksum_if_available(&client, &release, asset, &archive_path).await?;
    extract_tarball(&archive_path, tmp_dir.path()).await?;
    let new_binary = find_binary(tmp_dir.path())
        .await
        .context("no deskbrid binary found in archive")?;

    let current_exe = std::env::current_exe().context("failed to determine running binary path")?;
    let backup_path = current_exe.with_extension("old");
    install::replace_binary(&current_exe, &backup_path, &new_binary).await?;
    let restart_status = install::restart_daemon_if_active().await;

    Ok(json!({
        "current_version": CURRENT_VERSION,
        "latest_version": latest_tag,
        "asset": asset.name,
        "updated": true,
        "backup_path": backup_path,
        "checksum": checksum,
        "restart": restart_status,
    }))
}

fn print_update_result(result: &Value) {
    println!(
        "Current version: v{}",
        result["current_version"]
            .as_str()
            .unwrap_or(CURRENT_VERSION)
    );
    println!(
        "Latest version:  v{}",
        result["latest_version"].as_str().unwrap_or("unknown")
    );

    if result["updated"].as_bool().unwrap_or(false) {
        println!("✓ Updated deskbrid");
        if let Some(path) = result["backup_path"].as_str() {
            println!("Backup: {path}");
        }
        if let Some(checksum) = result["checksum"].as_str() {
            println!("Checksum: {checksum}");
        }
        if let Some(restart) = result["restart"].as_str() {
            println!("Daemon restart: {restart}");
        }
    } else if result["update_available"].as_bool().unwrap_or(false) {
        println!("Update available. Run `deskbrid update` to apply.");
    } else {
        println!("Already up to date.");
    }
}

async fn extract_tarball(archive: &std::path::Path, dest: &std::path::Path) -> anyhow::Result<()> {
    let output = tokio::process::Command::new("tar")
        .args([
            "-xzf",
            archive.to_str().unwrap_or(""),
            "-C",
            dest.to_str().unwrap_or(""),
        ])
        .output()
        .await
        .context("failed to run tar")?;
    if !output.status.success() {
        anyhow::bail!(
            "tar extraction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

async fn find_binary(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut entries = tokio::fs::read_dir(dir).await.ok()?;
    loop {
        let entry = match entries.next_entry().await {
            Ok(Some(e)) => e,
            _ => break,
        };
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = Box::pin(find_binary(&path)).await {
                return Some(found);
            }
        } else if path.file_name().is_some_and(|name| name == "deskbrid") {
            return Some(path);
        }
    }
    None
}
