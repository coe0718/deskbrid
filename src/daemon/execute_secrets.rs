use crate::protocol::Action;
use serde_json::Value;
use tokio::process::Command;
use zeroize::Zeroize;

/// A secret value that zeroizes itself on drop.
#[derive(Zeroize)]
struct SecretString(String);

impl Drop for SecretString {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

impl std::ops::Deref for SecretString {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

/// Execute a secrets action. All reads and writes are confirmed via the
/// existing confirmation pipeline before reaching this executor.
pub(crate) async fn execute_secrets_action(
    action: Action,
    _state: &crate::DaemonState,
) -> anyhow::Result<Value> {
    match action {
        Action::SecretsListCollections => list_collections().await,
        Action::SecretsGetSecret { attributes } => get_secret(&attributes).await,
        Action::SecretsStoreSecret {
            attributes,
            secret,
            label,
            collection,
        } => {
            store_secret(
                &attributes,
                &secret,
                label.as_deref(),
                collection.as_deref(),
            )
            .await
        }
        _ => anyhow::bail!("not a secrets action"),
    }
}

/// Check if the given action is a secrets action.
pub(crate) fn is_secrets_action(action: &Action) -> bool {
    matches!(
        action,
        Action::SecretsListCollections
            | Action::SecretsGetSecret { .. }
            | Action::SecretsStoreSecret { .. }
    )
}

async fn list_collections() -> anyhow::Result<Value> {
    let output = Command::new("secret-tool")
        .arg("search")
        .arg("--all")
        .arg("--unlock")
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not found") || stderr.contains("Cannot autolaunch") {
            return Ok(serde_json::json!({
                "error": "service_unavailable",
                "message": "Secret Service is not available on this system"
            }));
        }
        anyhow::bail!("secret-tool search failed: {stderr}");
    }

    // Parse secret-tool output to extract unique collection paths.
    // Output format: [/<collection_path>]\nlabel = ...\nsecret = ...\n...
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut collections: Vec<Value> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for line in stdout.lines() {
        if line.starts_with('[') && line.ends_with(']') {
            let path = line[1..line.len() - 1].to_string();
            if seen.insert(path.clone()) {
                collections.push(serde_json::json!({
                    "path": path,
                    "label": path.rsplit('/').next().unwrap_or(&path),
                    "locked": null  // secret-tool can't report lock state directly
                }));
            }
        }
    }

    Ok(serde_json::json!({ "collections": collections }))
}

async fn get_secret(
    attributes: &std::collections::HashMap<String, String>,
) -> anyhow::Result<Value> {
    let mut cmd = Command::new("secret-tool");
    cmd.arg("lookup");
    for (k, v) in attributes {
        cmd.arg(k);
        cmd.arg(v);
    }

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not found") || stderr.contains("Cannot autolaunch") {
            return Ok(serde_json::json!({
                "error": "not_found",
                "message": "no secret matching those attributes"
            }));
        }
        // Exit code 1 with empty stderr = not found (secret-tool behavior)
        if output.status.code() == Some(1) && stderr.is_empty() {
            return Ok(serde_json::json!({
                "error": "not_found",
                "message": "no secret matching those attributes"
            }));
        }
        anyhow::bail!("secret-tool lookup failed: {stderr}");
    }

    let mut secret = SecretString(String::from_utf8(output.stdout)?);
    let trimmed = secret.0.trim_end_matches('\n').to_string();
    secret.0.zeroize();
    drop(secret);

    Ok(serde_json::json!({ "secret": trimmed }))
}

async fn store_secret(
    attributes: &std::collections::HashMap<String, String>,
    secret: &str,
    label: Option<&str>,
    collection: Option<&str>,
) -> anyhow::Result<Value> {
    let mut cmd = Command::new("secret-tool");
    cmd.arg("store");
    if let Some(c) = collection {
        cmd.arg("--collection").arg(c);
    }
    if let Some(l) = label {
        cmd.arg("--label").arg(l);
    }
    for (k, v) in attributes {
        cmd.arg(k);
        cmd.arg(v);
    }

    // Pass secret via stdin
    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    use tokio::io::AsyncWriteExt;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(secret.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
    }

    let output = child.wait_with_output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("secret-tool store failed: {stderr}");
    }

    Ok(serde_json::json!({ "success": true }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_string_zeroizes() {
        let mut s = SecretString("test-value".to_string());
        assert_eq!(&*s, "test-value");
        s.0.zeroize();
        assert_eq!(&*s, ""); // zeroized
    }

    #[test]
    fn secret_string_debug_redacted() {
        let s = SecretString("secret123".to_string());
        assert_eq!(format!("{:?}", s), "<redacted>");
    }
}
