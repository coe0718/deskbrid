use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

pub(crate) async fn execute_vision(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        VisionFindElement {
            template_path,
            screenshot,
            min_confidence,
            max_results,
        } => {
            crate::visual::vision_find_element(
                backend,
                crate::visual::VisionFindElementRequest {
                    template_path,
                    screenshot,
                    min_confidence,
                    max_results,
                },
            )
            .await?
        }
        VisionFindByText { text, screenshot } => {
            crate::visual::vision_find_by_text(
                backend,
                crate::visual::VisionFindByTextRequest { text, screenshot },
            )
            .await?
        }
        VisionDetectState { screenshot, checks } => {
            crate::visual::vision_detect_state(
                backend,
                crate::visual::VisionDetectStateRequest { screenshot, checks },
            )
            .await?
        }
        _ => anyhow::bail!("internal dispatch error: not a vision action"),
    })
}
