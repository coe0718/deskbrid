use super::*;
use crate::protocol;

pub(super) async fn network_status(
    backend: &HyprBackend,
) -> anyhow::Result<protocol::NetworkStatusInfo> {
    let online = if backend
        .sh_ok("nmcli", &["networking", "connectivity", "check"])
        .await
    {
        true
    } else {
        backend
            .sh_ok("ping", &["-c", "1", "-W", "2", "8.8.8.8"])
            .await
    };
    Ok(protocol::NetworkStatusInfo {
        online,
        net_type: if online {
            "wifi_or_ethernet".into()
        } else {
            "offline".into()
        },
    })
}

pub(super) async fn network_interfaces(
    backend: &HyprBackend,
) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
    let output = backend
        .sh(
            "nmcli",
            &["-t", "-f", "DEVICE,STATE,IP4.ADDRESS", "dev", "status"],
        )
        .await
        .unwrap_or_default();
    let mut ifaces = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 2 {
            continue;
        }
        let name = parts[0].to_string();
        if name == "lo" || name.is_empty() {
            continue;
        }
        let state = match *parts.get(1).unwrap_or(&"") {
            "connected" => "connected".to_string(),
            "connecting" => "connecting".to_string(),
            _ => "disconnected".to_string(),
        };
        let ipv4 = parts
            .get(2)
            .filter(|s| !s.is_empty())
            .map(|s| s.split('/').next().unwrap_or(s).to_string());
        ifaces.push(protocol::NetworkInterfaceInfo {
            name,
            state,
            ipv4,
            ipv6: None,
        });
    }
    Ok(ifaces)
}

pub(super) async fn wifi_scan(
    backend: &HyprBackend,
) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
    backend.sh("nmcli", &["dev", "wifi", "rescan"]).await.ok();
    let output = backend
        .sh(
            "nmcli",
            &["-t", "-f", "SSID,SIGNAL,SECURITY", "dev", "wifi", "list"],
        )
        .await
        .unwrap_or_default();
    let mut networks = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.is_empty() || parts[0].is_empty() {
            continue;
        }
        let ssid = parts[0].to_string();
        let signal: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let security = parts.get(2).unwrap_or(&"").to_string();
        networks.push(protocol::WifiNetworkInfo {
            ssid,
            strength: signal,
            secured: !security.is_empty() && security != "--",
            frequency: None,
        });
    }
    Ok(networks)
}

pub(super) async fn wifi_connect(
    backend: &HyprBackend,
    ssid: &str,
    password: Option<&str>,
) -> anyhow::Result<()> {
    match password {
        Some(pw) => {
            // W24 (docs/CODE_REVIEW_VEX.md): pass password via stdin instead
            // of argv so it does not appear in /proc/<pid>/cmdline for any
            // user on the system to read.
            let mut child = std::process::Command::new("nmcli")
                .args(["device", "wifi", "connect", ssid, "--ask"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| anyhow::anyhow!("failed to spawn nmcli: {e}"))?;
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(pw.as_bytes());
                let _ = stdin.write_all(b"\n");
            }
            let output = tokio::task::spawn_blocking(move || child.wait_with_output())
                .await
                .map_err(|e| anyhow::anyhow!("nmcli wait task failed: {e}"))?
                .map_err(|e| anyhow::anyhow!("failed to read nmcli output: {e}"))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("nmcli wifi connect failed: {}", redact_wifi_error(&stderr));
            }
        }
        None => {
            backend
                .sh("nmcli", &["dev", "wifi", "connect", ssid])
                .await?;
        }
    }
    Ok(())
}

/// W24 (docs/CODE_REVIEW_VEX.md): scrub anything that looks like a password
/// from nmcli error output before propagating it. nmcli sometimes echoes
/// back the SSID or a fragment of the password in failure messages.
fn redact_wifi_error(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(*c, '"' | '\'' | '\\'))
        .collect::<String>()
        .lines()
        .filter(|line| !line.to_lowercase().contains("password"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[tokio::test(flavor = "current_thread")]
    async fn wifi_connect_password_wait_does_not_block_async_runtime() {
        let _env_guard = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let temp_dir = tempfile::tempdir().expect("create fake nmcli directory");
        let nmcli = temp_dir.path().join("nmcli");
        std::fs::write(&nmcli, "#!/bin/sh\nread -r password\nsleep 0.3\n")
            .expect("write fake nmcli");

        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&nmcli)
            .expect("read fake nmcli metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&nmcli, permissions).expect("make fake nmcli executable");

        let previous_path = std::env::var_os("PATH");
        let mut paths = vec![temp_dir.path().to_path_buf()];
        if let Some(path) = previous_path.as_ref() {
            paths.extend(std::env::split_paths(path));
        }
        let test_path = std::env::join_paths(paths).expect("build test PATH");
        // SAFETY: TEST_ENV_LOCK serializes all process environment mutations in tests.
        unsafe {
            std::env::set_var("PATH", test_path);
        }

        let (event_tx, _) = tokio::sync::broadcast::channel(1);
        let backend = HyprBackend {
            event_tx,
            watchers: Arc::new(Mutex::new(std::collections::HashMap::new())),
            last_mouse: Mutex::new((0.0, 0.0)),
            monitors: Mutex::new(Vec::new()),
            instance_sig: None,
            wl_socket: None,
            xdg_runtime: "/tmp".to_string(),
        };

        let heartbeat_ran = Arc::new(AtomicBool::new(false));
        let heartbeat_flag = Arc::clone(&heartbeat_ran);
        let heartbeat = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            heartbeat_flag.store(true, Ordering::SeqCst);
        });

        let connect_result = wifi_connect(&backend, "Deskbrid Test", Some("secret")).await;
        let runtime_stayed_responsive = heartbeat_ran.load(Ordering::SeqCst);
        heartbeat.await.expect("heartbeat task completes");

        match previous_path {
            Some(path) => unsafe {
                // SAFETY: TEST_ENV_LOCK is still held while restoring PATH.
                std::env::set_var("PATH", path);
            },
            None => unsafe {
                // SAFETY: TEST_ENV_LOCK is still held while restoring PATH.
                std::env::remove_var("PATH");
            },
        }

        connect_result.expect("fake nmcli succeeds");
        assert!(
            runtime_stayed_responsive,
            "nmcli wait blocked the single-threaded Tokio runtime"
        );
    }
}
