use tokio::process::Command;

pub async fn check_in_path(cmd: &str) -> serde_json::Value {
    match Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} >/dev/null 2>&1", cmd))
        .status()
        .await
    {
        Ok(status) if status.success() => serde_json::json!({"ok": true, "details": "present"}),
        Ok(_) => serde_json::json!({"ok": false, "details": "missing"}),
        Err(e) => serde_json::json!({"ok": false, "details": format!("check failed: {}", e)}),
    }
}

pub async fn check_process(proc_name: &str) -> serde_json::Value {
    match Command::new("pgrep").args(["-x", proc_name]).output().await {
        Ok(out) if out.status.success() => serde_json::json!({"ok": true, "details": "running"}),
        Ok(_) => serde_json::json!({"ok": false, "details": "not running"}),
        Err(e) => serde_json::json!({"ok": false, "details": format!("check failed: {}", e)}),
    }
}

pub async fn check_cmd(cmd: &str, args: &[&str]) -> serde_json::Value {
    match Command::new(cmd).args(args).output().await {
        Ok(out) if out.status.success() => {
            serde_json::json!({"ok": true, "details": "reachable"})
        }
        Ok(out) => {
            serde_json::json!({"ok": false, "details": format!("failed (code {:?})", out.status.code())})
        }
        Err(e) => serde_json::json!({"ok": false, "details": format!("check failed: {}", e)}),
    }
}

pub async fn check_uinput() -> serde_json::Value {
    let path = std::path::Path::new("/dev/uinput");
    if tokio::fs::metadata(path).await.is_err() {
        return serde_json::json!({"ok": false, "details": "missing /dev/uinput"});
    }
    match tokio::fs::OpenOptions::new().write(true).open(path).await {
        Ok(_) => serde_json::json!({"ok": true, "details": "write access"}),
        Err(e) => {
            serde_json::json!({"ok": false, "details": format!("no write access: {}", e)})
        }
    }
}

pub async fn check_clipboard_tools() -> serde_json::Value {
    let copy = check_in_path("wl-copy").await;
    let paste = check_in_path("wl-paste").await;
    let copy_ok = copy.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    let paste_ok = paste.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);

    if copy_ok && paste_ok {
        serde_json::json!({"ok": true, "details": "wl-copy and wl-paste present"})
    } else {
        let mut missing = Vec::new();
        if !copy_ok {
            missing.push("wl-copy");
        }
        if !paste_ok {
            missing.push("wl-paste");
        }
        serde_json::json!({"ok": false, "details": format!("missing: {}", missing.join(", "))})
    }
}
