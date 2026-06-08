use super::*;
use crate::protocol;

pub(super) async fn screenshot(
    backend: &LabwcBackend,
    _m: Option<u32>,
    region: Option<protocol::Region>,
    _w: Option<String>,
) -> anyhow::Result<protocol::ScreenshotResult> {
    let path = crate::daemon::helpers::screenshot_temp_path();
    let mut args: Vec<String> = vec!["-t".into(), "png".into()];
    if let Some(r) = region {
        args.push("-g".into());
        args.push(format!("{},{} {}x{}", r.x, r.y, r.width, r.height));
    }
    args.push(path.clone());
    let mut cmd = Command::new("grim");
    cmd.args(&args).stdin(Stdio::null()).stderr(Stdio::piped());
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
    let (w, h) = if let Some(d) = dims {
        let p: Vec<&str> = d.split_whitespace().collect();
        (
            p.first().and_then(|s| s.parse().ok()).unwrap_or(0),
            p.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
        )
    } else {
        (0, 0)
    };
    Ok(protocol::ScreenshotResult {
        path,
        width: w,
        height: h,
        format: "png".into(),
    })
}
