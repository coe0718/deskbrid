use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

pub(crate) async fn execute_screenshot(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        Screenshot {
            monitor,
            ref region,
            ref window_id,
            ref output,
        } => {
            let result = backend
                .screenshot(monitor, region.clone(), window_id.clone())
                .await?;
            if let Some(out_path) = output {
                tokio::fs::copy(&result.path, out_path).await?;
            }
            serde_json::json!(result)
        }
        ScreenshotOcr {
            ref path,
            ref language,
            psm,
            bounding_boxes,
            monitor,
            ref region,
            ref window_id,
        } => {
            crate::ocr::screenshot_ocr(
                backend,
                crate::ocr::OcrRequest {
                    path: path.as_deref(),
                    language: language.as_deref(),
                    psm,
                    bounding_boxes,
                    monitor,
                    region: region.clone(),
                    window_id: window_id.clone(),
                },
            )
            .await?
        }
        ScreenshotDiff {
            ref before_path,
            ref after_path,
            tolerance,
            ref diff_path,
            save_diff,
            monitor,
            ref region,
            ref window_id,
        } => {
            crate::visual::screenshot_diff(
                backend,
                crate::visual::ScreenshotDiffRequest {
                    before_path,
                    after_path: after_path.as_deref(),
                    tolerance: tolerance.unwrap_or(0),
                    diff_path: diff_path.as_deref(),
                    save_diff,
                    monitor,
                    region: region.clone(),
                    window_id: window_id.clone(),
                },
            )
            .await?
        }

        _ => anyhow::bail!("internal dispatch error: not a screenshot action"),
    })
}

pub(crate) async fn execute_screencast(
    action: Action,
    backend: &dyn DesktopBackend,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        ScreencastStart { output_path } => {
            backend.start_screencast(&output_path).await?;
            serde_json::json!({ "ok": true, "output_path": output_path })
        }
        ScreencastStop => {
            backend.stop_screencast().await?;
            serde_json::json!({ "ok": true })
        }
        _ => anyhow::bail!("internal dispatch error: not a screencast action"),
    })
}
