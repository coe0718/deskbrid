use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

pub(crate) async fn execute_color(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        ColorPick { x, y, ref path } => {
            if let Some(image_path) = path {
                crate::color::pick_color_from_image(image_path, x, y).await?
            } else {
                backend.pick_color(x, y).await?
            }
        }

        _ => anyhow::bail!("internal dispatch error: not a color action"),
    })
}
