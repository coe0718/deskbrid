use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

pub(crate) async fn execute_desktop(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        DesktopGetSetting {
            ref schema,
            ref key,
        } => {
            let value = backend.desktop_get_setting(schema, key).await?;
            serde_json::json!({ "schema": schema, "key": key, "value": value })
        }
        DesktopSetSetting {
            ref schema,
            ref key,
            ref value,
        } => {
            backend.desktop_set_setting(schema, key, value).await?;
            serde_json::json!({ "schema": schema, "key": key, "set": true })
        }
        DesktopListSchemas => {
            let schemas = backend.desktop_list_schemas().await?;
            serde_json::json!({ "schemas": schemas, "count": schemas.len() })
        }
        _ => unreachable!("not a desktop settings action"),
    })
}
