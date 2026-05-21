use serde_json::json;
use tokio::process::Command;

use super::command::ensure_arg;

pub async fn check_auth(
    action_id: &str,
    interactive: bool,
    reason: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    ensure_arg(action_id, "action_id")?;
    let pid = std::process::id().to_string();
    let mut args = vec!["--process", pid.as_str(), "--action-id", action_id];
    if interactive {
        args.push("--allow-user-interaction");
    }
    let result = Command::new("pkcheck").args(&args).output().await;
    let authorized = result.as_ref().map(|o| o.status.success()).unwrap_or(false);
    let detail = match &result {
        Ok(output) if output.status.success() => "authorized".to_string(),
        Ok(output) => String::from_utf8_lossy(&output.stderr).trim().to_string(),
        Err(err) => format!("pkcheck failed: {err}"),
    };
    Ok(json!({
        "action_id": action_id,
        "authorized": authorized,
        "interactive": interactive,
        "reason": reason,
        "detail": detail
    }))
}
