use super::*;

pub(super) async fn keyboard_type(backend: &HyprBackend, text: &str) -> anyhow::Result<()> {
    backend.sh("ydotool", &["type", text]).await?;
    Ok(())
}

pub(super) async fn keyboard_key(backend: &HyprBackend, key: &str) -> anyhow::Result<()> {
    let k = ydotool_key_name(key);
    backend.sh("ydotool", &["key", &k]).await?;
    Ok(())
}

pub(super) async fn keyboard_combo(backend: &HyprBackend, keys: &[String]) -> anyhow::Result<()> {
    if keys.is_empty() {
        return Ok(());
    }
    let combo: Vec<String> = keys.iter().map(|k| ydotool_key_name(k)).collect();
    for (i, key) in combo.iter().enumerate() {
        if i < combo.len() - 1 {
            backend
                .sh("ydotool", &["key", &format!("{}:1", key)])
                .await?;
        } else {
            backend.sh("ydotool", &["key", key]).await?;
        }
    }
    for key in combo.iter().take(combo.len().saturating_sub(1)) {
        backend
            .sh("ydotool", &["key", &format!("{}:0", key)])
            .await?;
    }
    Ok(())
}

pub(super) async fn mouse_move(backend: &HyprBackend, x: f64, y: f64) -> anyhow::Result<()> {
    let (last_x, last_y) = {
        let pos = backend.last_mouse.lock().unwrap();
        *pos
    };
    let _dx = x - last_x;
    let _dy = y - last_y;
    {
        let mut pos = backend.last_mouse.lock().unwrap();
        *pos = (x, y);
    }
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

pub(super) async fn mouse_click(backend: &HyprBackend, button: &str) -> anyhow::Result<()> {
    let btn_id = match button {
        "left" => "0",
        "middle" => "1",
        "right" => "2",
        _ => anyhow::bail!("unknown button: {}", button),
    };
    backend.sh("ydotool", &["click", btn_id]).await?;
    Ok(())
}

pub(super) async fn mouse_scroll(backend: &HyprBackend, dx: f64, dy: f64) -> anyhow::Result<()> {
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

pub(super) async fn mouse_drag(
    backend: &HyprBackend,
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
    button: &str,
    duration_ms: Option<u64>,
) -> anyhow::Result<()> {
    let btn = ydotool_mouse_button(button)?;
    mouse_move(backend, from_x, from_y).await?;
    backend.sh("ydotool", &["mousedown", btn]).await?;
    if let Some(duration_ms) = duration_ms.filter(|duration| *duration > 0) {
        tokio::time::sleep(std::time::Duration::from_millis(duration_ms.min(5_000))).await;
    }
    mouse_move(backend, to_x, to_y).await?;
    backend.sh("ydotool", &["mouseup", btn]).await?;
    Ok(())
}

fn ydotool_mouse_button(button: &str) -> anyhow::Result<&'static str> {
    match button {
        "left" => Ok("1"),
        "middle" => Ok("2"),
        "right" => Ok("3"),
        _ => anyhow::bail!("unknown button: {}", button),
    }
}
