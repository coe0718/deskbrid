use super::*;
use crate::protocol;

pub(super) async fn screenshot(
    backend: &SwayBackend,
    monitor: Option<u32>,
    region: Option<protocol::Region>,
    _window_id: Option<String>,
) -> anyhow::Result<protocol::ScreenshotResult> {
    let path = crate::daemon::helpers::screenshot_temp_path();

    let output_name = if let Some(monitor_id) = monitor {
        let outputs = backend.swaymsg_json(&["-t", "get_outputs"]).await?;
        let monitors = parse_sway_outputs(&outputs);
        monitors
            .get(monitor_id as usize)
            .map(|m| m.name.clone())
            .unwrap_or_default()
    } else {
        let outputs = backend.swaymsg_json(&["-t", "get_outputs"]).await?;
        let monitors = parse_sway_outputs(&outputs);
        monitors
            .iter()
            .find(|m| m.primary)
            .map(|m| m.name.clone())
            .unwrap_or_default()
    };

    let mut grim_args: Vec<String> = vec!["-t".into(), "png".into()];
    grim_args.push(format!("-o{}", output_name));
    if let Some(region) = region {
        grim_args.push("-g".into());
        grim_args.push(format!(
            "{},{} {}x{}",
            region.x, region.y, region.width, region.height
        ));
    }
    grim_args.push(path.clone());

    let mut cmd = Command::new("grim");
    cmd.args(&grim_args)
        .stdin(Stdio::null())
        .stderr(Stdio::piped());
    backend.apply_env(&mut cmd);
    let out = cmd.output().await?;
    if !out.status.success() {
        anyhow::bail!(
            "grim failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }

    let dims = backend
        .sh("identify", &["-format", "%w %h", &path])
        .await
        .ok();
    let (width, height) = if let Some(ref dim) = dims {
        let parts: Vec<&str> = dim.split_whitespace().collect();
        (
            parts.first().and_then(|s| s.parse().ok()).unwrap_or(0),
            parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
        )
    } else {
        (0, 0)
    };

    Ok(protocol::ScreenshotResult {
        path,
        width,
        height,
        format: "png".into(),
    })
}
