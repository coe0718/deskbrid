pub fn check_in_path(cmd: &str) -> serde_json::Value {
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} >/dev/null 2>&1", cmd))
        .status()
    {
        Ok(status) if status.success() => serde_json::json!({"ok": true, "details": "present"}),
        Ok(_) => serde_json::json!({"ok": false, "details": "missing"}),
        Err(e) => serde_json::json!({"ok": false, "details": format!("check failed: {}", e)}),
    }
}

pub async fn check_process(proc_name: &str) -> serde_json::Value {
    match tokio::process::Command::new("pgrep")
        .args(["-x", proc_name])
        .output()
        .await
    {
        Ok(out) if out.status.success() => serde_json::json!({"ok": true, "details": "running"}),
        Ok(_) => serde_json::json!({"ok": false, "details": "not running"}),
        Err(e) => serde_json::json!({"ok": false, "details": format!("check failed: {}", e)}),
    }
}

pub fn check_cmd(cmd: &str, args: &[&str]) -> serde_json::Value {
    match std::process::Command::new(cmd).args(args).output() {
        Ok(out) if out.status.success() => {
            serde_json::json!({"ok": true, "details": "reachable"})
        }
        Ok(out) => {
            serde_json::json!({"ok": false, "details": format!("failed (code {:?})", out.status.code())})
        }
        Err(e) => serde_json::json!({"ok": false, "details": format!("check failed: {}", e)}),
    }
}

pub fn check_uinput() -> serde_json::Value {
    let path = std::path::Path::new("/dev/uinput");
    if !path.exists() {
        return serde_json::json!({"ok": false, "details": "missing /dev/uinput"});
    }
    match std::fs::OpenOptions::new().write(true).open(path) {
        Ok(_) => serde_json::json!({"ok": true, "details": "write access"}),
        Err(e) => {
            serde_json::json!({"ok": false, "details": format!("no write access: {}", e)})
        }
    }
}

pub fn check_clipboard_tools() -> serde_json::Value {
    let copy = std::process::Command::new("sh")
        .arg("-c")
        .arg("command -v wl-copy >/dev/null 2>&1")
        .status();
    let paste = std::process::Command::new("sh")
        .arg("-c")
        .arg("command -v wl-paste >/dev/null 2>&1")
        .status();

    let copy_ok = copy.map(|s| s.success()).unwrap_or(false);
    let paste_ok = paste.map(|s| s.success()).unwrap_or(false);

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
