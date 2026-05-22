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
            crate::color::pick_color(backend, x, y, path.as_deref()).await?
        }

        _ => unreachable!("not a color action"),
    })
}
