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
        } => {
            serde_json::json!(
                backend
                    .screenshot(monitor, region.clone(), window_id.clone())
                    .await?
            )
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

        _ => unreachable!("not a screenshot action"),
    })
}
