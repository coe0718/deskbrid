use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

pub(crate) async fn execute_a11y(
    action: Action,
    _backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        A11yTree { depth } => crate::a11y::tree(depth).await?,
        A11yGetText {
            role,
            ref name,
            index,
        } => crate::a11y::get_text(role.as_deref(), name.as_deref(), index).await?,

        _ => unreachable!("not a a11y action"),
    })
}
