use tokio::process::Command;

pub async fn run(cmd: &str, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new(cmd).args(args).output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{cmd} failed: {}", stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn ensure_unit(name: &str) -> anyhow::Result<()> {
    ensure_arg(name, "name")?;
    if name.split_whitespace().count() != 1 {
        anyhow::bail!("unit name must not contain whitespace");
    }
    Ok(())
}

pub fn ensure_arg(value: &str, field: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() || value.starts_with('-') || value.contains('\0') {
        anyhow::bail!("invalid {field}");
    }
    Ok(())
}
