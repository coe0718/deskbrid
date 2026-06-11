use super::Action;
use serde_json::json;

pub(super) fn serialize_screenshot(action: &Action, id: &str) -> serde_json::Value {
    match action {
        // Screenshot
        Action::Screenshot {
            monitor,
            region,
            window_id,
            output,
        } => {
            let mut obj = json!({"type": "screenshot", "id": id});
            if let Some(m) = monitor {
                obj["monitor"] = json!(m);
            }
            if let Some(r) = region {
                obj["region"] = json!(r);
            }
            if let Some(w) = window_id {
                obj["window_id"] = json!(w);
            }
            if let Some(o) = output {
                obj["output"] = json!(o);
            }
            obj
        }
        Action::ScreenshotOcr {
            path,
            language,
            psm,
            bounding_boxes,
            monitor,
            region,
            window_id,
        } => {
            let mut obj =
                json!({"type": "screenshot.ocr", "id": id, "bounding_boxes": bounding_boxes});
            if let Some(path) = path {
                obj["path"] = json!(path);
            }
            if let Some(language) = language {
                obj["language"] = json!(language);
            }
            if let Some(psm) = psm {
                obj["psm"] = json!(psm);
            }
            if let Some(monitor) = monitor {
                obj["monitor"] = json!(monitor);
            }
            if let Some(region) = region {
                obj["region"] = json!(region);
            }
            if let Some(window_id) = window_id {
                obj["window_id"] = json!(window_id);
            }
            obj
        }
        Action::ScreenshotDiff {
            before_path,
            after_path,
            tolerance,
            diff_path,
            save_diff,
            monitor,
            region,
            window_id,
        } => {
            let mut obj = json!({"type": "screenshot.diff", "id": id, "before_path": before_path, "save_diff": save_diff});
            if let Some(after_path) = after_path {
                obj["after_path"] = json!(after_path);
            }
            if let Some(tolerance) = tolerance {
                obj["tolerance"] = json!(tolerance);
            }
            if let Some(diff_path) = diff_path {
                obj["diff_path"] = json!(diff_path);
            }
            if let Some(monitor) = monitor {
                obj["monitor"] = json!(monitor);
            }
            if let Some(region) = region {
                obj["region"] = json!(region);
            }
            if let Some(window_id) = window_id {
                obj["window_id"] = json!(window_id);
            }
            obj
        }
        Action::ScreencastStart { output_path } => {
            json!({"type": "screencast.start", "id": id, "output_path": output_path})
        }
        Action::ScreencastStop => {
            json!({"type": "screencast.stop", "id": id})
        }
        Action::PortalScreenshot { interactive } => {
            json!({"type": "portal.screenshot", "id": id, "interactive": interactive})
        }
        Action::PortalScreencastStart { output_path } => {
            json!({"type": "portal.screencast_start", "id": id, "output_path": output_path})
        }
        Action::PortalScreencastStop => {
            json!({"type": "portal.screencast_stop", "id": id})
        }
        _ => serde_json::json!({"error": "not a screenshot action"}),
    }
}
