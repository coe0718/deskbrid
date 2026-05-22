use super::*;
use crate::protocol;

pub(super) async fn workspaces_list(
    _backend: &LabwcBackend,
) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
    Ok(vec![protocol::WorkspaceInfo {
        id: 1,
        name: "workspace-1".into(),
        is_active: true,
    }])
}

pub(super) async fn workspace_switch(_backend: &LabwcBackend, _id: u32) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn workspace_move_window(
    _backend: &LabwcBackend,
    _w: &str,
    _ws: u32,
    _follow: bool,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn keyboard_type(backend: &LabwcBackend, t: &str) -> anyhow::Result<()> {
    backend.ydotool(&["type", t]).await
}

pub(super) async fn keyboard_key(backend: &LabwcBackend, k: &str) -> anyhow::Result<()> {
    backend.ydotool(&["key", k]).await
}

pub(super) async fn keyboard_combo(backend: &LabwcBackend, keys: &[String]) -> anyhow::Result<()> {
    for k in keys {
        backend.ydotool(&["key", &format!("{}:1", k)]).await?;
    }
    for k in keys.iter().rev() {
        backend.ydotool(&["key", &format!("{}:0", k)]).await?;
    }
    Ok(())
}

pub(super) async fn mouse_move(backend: &LabwcBackend, x: f64, y: f64) -> anyhow::Result<()> {
    backend
        .ydotool(&["mousemove", "--absolute", &x.to_string(), &y.to_string()])
        .await
}

pub(super) async fn mouse_click(backend: &LabwcBackend, b: &str) -> anyhow::Result<()> {
    let btn: u8 = match b {
        "left" => 1,
        "middle" => 2,
        "right" => 3,
        _ => 1,
    };
    backend.ydotool(&["click", &btn.to_string()]).await
}

pub(super) async fn mouse_scroll(backend: &LabwcBackend, dx: f64, dy: f64) -> anyhow::Result<()> {
    if dy != 0.0 {
        backend
            .ydotool(&["mousemove", "--wheel", "0", &format!("{}", dy as i32)])
            .await?;
    }
    if dx != 0.0 {
        backend
            .ydotool(&["mousemove", "--wheel", &format!("{}", dx as i32), "0"])
            .await?;
    }
    Ok(())
}

pub(super) async fn clipboard_read(backend: &LabwcBackend) -> anyhow::Result<String> {
    backend.sh("wl-paste", &[]).await
}

pub(super) async fn clipboard_write(backend: &LabwcBackend, text: &str) -> anyhow::Result<()> {
    let mut cmd = Command::new("wl-copy");
    cmd.stdin(Stdio::piped()).stderr(Stdio::piped());
    backend.apply_env(&mut cmd);
    let mut child = cmd.spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(text.as_bytes()).await?;
    }
    let output = child.wait_with_output().await?;
    if !output.status.success() {
        anyhow::bail!(
            "wl-copy failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}
