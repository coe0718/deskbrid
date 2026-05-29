use anyhow::{Context, bail};

pub(crate) async fn replace_binary(
    current: &std::path::Path,
    backup: &std::path::Path,
    new_binary: &std::path::Path,
) -> anyhow::Result<()> {
    match replace_binary_direct(current, backup, new_binary) {
        Ok(()) => Ok(()),
        Err(err) if is_permission_error(&err) => {
            replace_binary_with_sudo(current, new_binary).await
        }
        Err(err) => Err(err),
    }
}

fn replace_binary_direct(
    current: &std::path::Path,
    backup: &std::path::Path,
    new_binary: &std::path::Path,
) -> anyhow::Result<()> {
    if backup.exists() {
        std::fs::remove_file(backup).context("failed to remove old backup binary")?;
    }
    std::fs::rename(current, backup).context("failed to back up current binary")?;
    if let Err(err) = std::fs::copy(new_binary, current) {
        let _ = std::fs::rename(backup, current);
        return Err(err).context("failed to install new binary");
    }
    set_executable(current)?;
    Ok(())
}

async fn replace_binary_with_sudo(
    current: &std::path::Path,
    new_binary: &std::path::Path,
) -> anyhow::Result<()> {
    let backup = current.with_extension("old");
    let backup_status = tokio::process::Command::new("sudo")
        .arg("cp")
        .arg(current)
        .arg(&backup)
        .status()
        .await
        .context("failed to run sudo backup copy")?;
    if !backup_status.success() {
        bail!("sudo backup copy failed; rerun `sudo deskbrid update --force`");
    }

    let status = tokio::process::Command::new("sudo")
        .args(["install", "-m", "755"])
        .arg(new_binary)
        .arg(current)
        .status()
        .await
        .context("failed to run sudo install; rerun with sudo if needed")?;
    if !status.success() {
        bail!("sudo install failed; rerun `sudo deskbrid update --force`");
    }
    Ok(())
}

fn is_permission_error(err: &anyhow::Error) -> bool {
    err.chain().any(|e| {
        e.downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::PermissionDenied)
    })
}

fn set_executable(path: &std::path::Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

pub(crate) async fn restart_daemon_if_active() -> String {
    let active = tokio::process::Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", "deskbrid.service"])
        .status()
        .await;
    if !matches!(active, Ok(status) if status.success()) {
        return "not active; restart manually if needed".to_string();
    }
    match tokio::process::Command::new("systemctl")
        .args(["--user", "restart", "deskbrid.service"])
        .status()
        .await
    {
        Ok(status) if status.success() => "restarted systemd user service".to_string(),
        _ => "active but restart failed; restart manually".to_string(),
    }
}
