use super::*;
use crate::protocol;
use std::process::Stdio;
use tokio::process::Command;

pub(super) async fn workspaces_list(
    backend: &KdeBackend,
) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
    let js = r#"
var manager = workspace.virtualDesktopManager;
if (manager) {
    var desktops = manager.desktops;
    for (var i = 0; i < desktops.length; i++) {
        var d = desktops[i];
        print(JSON.stringify({
            id: Number(d.x11DesktopNumber || (i + 1)),
            name: String(d.name || "")
        }));
    }
}
"#;
    let lines = backend.kwin_js(js).await?;
    let mut workspaces = Vec::new();
    for line in &lines {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line.trim()) {
            workspaces.push(protocol::WorkspaceInfo {
                id: val["id"].as_i64().unwrap_or(1) as u32,
                name: val["name"].as_str().unwrap_or("").to_string(),
                is_active: false,
            });
        }
    }
    if workspaces.is_empty() {
        workspaces.push(protocol::WorkspaceInfo {
            id: 1,
            name: "Desktop 1".into(),
            is_active: false,
        });
    }
    Ok(workspaces)
}

pub(super) async fn workspace_switch(backend: &KdeBackend, id: u32) -> anyhow::Result<()> {
    backend
        .qdbus(
            "org.kde.KWin",
            "/KWin",
            "org.kde.KWin.setCurrentDesktop",
            &[&id.to_string()],
        )
        .await?;
    Ok(())
}

pub(super) async fn workspace_move_window(
    backend: &KdeBackend,
    window_id: &str,
    workspace_id: u32,
    follow: bool,
) -> anyhow::Result<()> {
    KdeBackend::ensure_window_id(window_id)?;
    let wid = window_id.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        r#"
var windows = workspace.windowList();
for (var i = 0; i < windows.length; i++) {{
    var w = windows[i];
    if (String(w.internalId) === "{}" || String(w.resourceClass) === "{}") {{
        var manager = workspace.virtualDesktopManager;
        if (manager && manager.desktops && manager.desktops.length > {}) {{
            w.desktops = [manager.desktops[{}]];
            print("MOVED:true");
        }}
        break;
    }}
}}
"#,
        wid,
        wid,
        workspace_id - 1,
        workspace_id - 1
    );
    backend.kwin_js(&js).await?;
    if follow {
        workspace_switch(backend, workspace_id).await?;
    }
    Ok(())
}

pub(super) async fn keyboard_type(backend: &KdeBackend, text: &str) -> anyhow::Result<()> {
    backend
        .sh("ydotool", &["type", &text.replace('\n', "\\n")])
        .await?;
    Ok(())
}

pub(super) async fn keyboard_key(backend: &KdeBackend, key: &str) -> anyhow::Result<()> {
    backend.sh("ydotool", &["key", key]).await?;
    Ok(())
}

pub(super) async fn keyboard_combo(backend: &KdeBackend, keys: &[String]) -> anyhow::Result<()> {
    backend.sh("ydotool", &["key", &keys.join("+")]).await?;
    Ok(())
}

pub(super) async fn mouse_move(backend: &KdeBackend, x: f64, y: f64) -> anyhow::Result<()> {
    backend
        .sh(
            "ydotool",
            &[
                "mousemove",
                "--absolute",
                &format!("{}", x as i32),
                &format!("{}", y as i32),
            ],
        )
        .await?;
    Ok(())
}

pub(super) async fn mouse_click(backend: &KdeBackend, button: &str) -> anyhow::Result<()> {
    let btn_id = match button {
        "left" => "0x1",
        "middle" => "0x2",
        "right" => "0x3",
        _ => anyhow::bail!("unknown button: {button}"),
    };
    backend.sh("ydotool", &["click", btn_id]).await?;
    Ok(())
}

pub(super) async fn mouse_scroll(backend: &KdeBackend, dx: f64, dy: f64) -> anyhow::Result<()> {
    if dx == 0.0 && dy == 0.0 {
        return Ok(());
    }
    backend
        .sh(
            "ydotool",
            &[
                "mousemove",
                "--wheel",
                &format!("{}", dx as i32),
                &format!("{}", dy as i32),
            ],
        )
        .await?;
    Ok(())
}

pub(super) async fn clipboard_read(backend: &KdeBackend) -> anyhow::Result<String> {
    backend.sh("wl-paste", &[]).await
}

pub(super) async fn clipboard_write(backend: &KdeBackend, text: &str) -> anyhow::Result<()> {
    use tokio::io::AsyncWriteExt;
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .env("XDG_RUNTIME_DIR", &backend.xdg_runtime)
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).await?;
    }
    child.wait().await?;
    Ok(())
}
