use super::*;
use crate::protocol;
use crate::protocol::Action;

pub fn into_screenshot_action(cmd: Command) -> anyhow::Result<Action> {
    Ok(match cmd {
        Command::Color { cmd } => match cmd {
            ColorCmd::Pick { x, y, path } => Action::ColorPick { x, y, path },
        },

        Command::Screenshot {
            output: _,
            monitor,
            region,
            window,
        } => Action::Screenshot {
            monitor,
            region: region.map(|v| protocol::Region {
                x: v[0],
                y: v[1],
                width: v[2],
                height: v[3],
            }),
            window_id: window,
        },

        Command::Ocr {
            path,
            language,
            psm,
            boxes,
            monitor,
            region,
            window,
        } => Action::ScreenshotOcr {
            path,
            language,
            psm,
            bounding_boxes: boxes,
            monitor,
            region: region.map(|v| protocol::Region {
                x: v[0],
                y: v[1],
                width: v[2],
                height: v[3],
            }),
            window_id: window,
        },

        Command::ScreenshotDiff {
            before_path,
            after_path,
            tolerance,
            diff_path,
            save_diff,
            monitor,
            region,
            window,
        } => Action::ScreenshotDiff {
            before_path,
            after_path,
            tolerance,
            diff_path,
            save_diff,
            monitor,
            region: region.map(|v| protocol::Region {
                x: v[0],
                y: v[1],
                width: v[2],
                height: v[3],
            }),
            window_id: window,
        },

        _ => bail!(
            "unexpected command in client mode: {:?}",
            std::mem::discriminant(&cmd)
        ),
    })
}
