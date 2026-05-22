use crate::protocol::Action;
use crate::DaemonState;
use crate::backend::DesktopBackend;
use serde_json::Value;

pub(crate) async fn execute_audit(action: Action, backend: &dyn DesktopBackend, state: &DaemonState) -> anyhow::Result<Value> {
    let _ = (action, backend, state);
    anyhow::bail!("audit actions are handled by the daemon dispatcher")
}
