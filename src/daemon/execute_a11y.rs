use crate::DaemonState;
use crate::protocol::Action;
use serde_json::Value;

pub(crate) async fn execute_a11y(action: Action, _state: &DaemonState) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        // --- Legacy (role/name based) ---
        A11yTree { depth } => crate::a11y::tree(depth).await?,
        A11yGetElement { role, name, index } => {
            crate::a11y::get_element(role.as_deref(), name.as_deref(), index).await?
        }
        A11yClickElement { role, name, index } => {
            crate::a11y::click_element(role.as_deref(), name.as_deref(), index).await?
        }
        A11yGetText {
            role,
            ref name,
            index,
        } => crate::a11y::get_text(role.as_deref(), name.as_deref(), index).await?,

        // --- Expanded (object_ref based) ---
        A11ySnapshotTree {
            app_name,
            pid,
            max_nodes,
            max_depth,
        } => {
            crate::a11y::tree::snapshot_tree(app_name.as_deref(), pid, max_nodes, max_depth).await?
        }

        A11yPerformAction {
            object_ref,
            action_name,
        } => crate::a11y::actions::perform_action(&object_ref, action_name.as_deref()).await?,

        A11ySetValue { object_ref, value } => {
            crate::a11y::value::set_element_value(&object_ref, &value).await?
        }

        A11yGetElementText {
            object_ref,
            max_chars,
        } => crate::a11y::value::get_element_text(&object_ref, max_chars).await?,

        A11yListApps { limit } => {
            let apps = crate::a11y::list_apps(limit).await?;
            serde_json::json!({ "apps": apps, "count": apps.len() })
        }

        A11yDoctor => crate::a11y::setup::doctor_report().await,
        A11ySetupAccessibility => {
            let enabled = crate::a11y::setup::enable_accessibility().await?;
            serde_json::json!({ "enabled": enabled })
        }

        A11yClickElementByRef { object_ref } => {
            crate::a11y::actions::click_element(&object_ref).await?
        }

        _ => anyhow::bail!("internal dispatch error: not an a11y action"),
    })
}
