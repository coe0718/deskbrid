use super::Action;
use serde_json::json;

pub(super) fn serialize_a11y(action: &Action, id: &str) -> serde_json::Value {
    match action {
        // Accessibility
        Action::A11yTree { depth } => {
            let mut obj = json!({"type": "a11y.tree", "id": id});
            if let Some(d) = depth {
                obj["depth"] = json!(d);
            }
            obj
        }
        Action::A11yGetElement { role, name, index } => {
            let mut obj = json!({"type": "a11y.get_element", "id": id});
            if let Some(r) = role {
                obj["role"] = json!(r);
            }
            if let Some(n) = name {
                obj["name"] = json!(n);
            }
            if let Some(i) = index {
                obj["index"] = json!(i);
            }
            obj
        }
        Action::A11yClickElement { role, name, index } => {
            let mut obj = json!({"type": "a11y.click_element", "id": id});
            if let Some(r) = role {
                obj["role"] = json!(r);
            }
            if let Some(n) = name {
                obj["name"] = json!(n);
            }
            if let Some(i) = index {
                obj["index"] = json!(i);
            }
            obj
        }
        Action::A11yGetText { role, name, index } => {
            let mut obj = json!({"type": "a11y.get_text", "id": id});
            if let Some(r) = role {
                obj["role"] = json!(r);
            }
            if let Some(n) = name {
                obj["name"] = json!(n);
            }
            if let Some(i) = index {
                obj["index"] = json!(i);
            }
            obj
        }
        Action::A11ySnapshotTree {
            app_name,
            pid,
            max_nodes,
            max_depth,
        } => {
            let mut obj = json!({"type": "a11y.snapshot_tree", "id": id});
            if let Some(a) = app_name {
                obj["app_name"] = json!(a);
            }
            if let Some(p) = pid {
                obj["pid"] = json!(p);
            }
            if let Some(m) = max_nodes {
                obj["max_nodes"] = json!(m);
            }
            if let Some(d) = max_depth {
                obj["max_depth"] = json!(d);
            }
            obj
        }
        Action::A11yPerformAction {
            object_ref,
            action_name,
        } => {
            let mut obj =
                json!({"type": "a11y.perform_action", "id": id, "object_ref": object_ref});
            if let Some(a) = action_name {
                obj["action_name"] = json!(a);
            }
            obj
        }
        Action::A11ySetValue { object_ref, value } => {
            json!({"type": "a11y.set_value", "id": id, "object_ref": object_ref, "value": value})
        }
        Action::A11yGetElementText {
            object_ref,
            max_chars,
        } => {
            let mut obj =
                json!({"type": "a11y.get_element_text", "id": id, "object_ref": object_ref});
            if let Some(m) = max_chars {
                obj["max_chars"] = json!(m);
            }
            obj
        }
        Action::A11yListApps { limit } => {
            let mut obj = json!({"type": "a11y.list_apps", "id": id});
            if let Some(l) = limit {
                obj["limit"] = json!(l);
            }
            obj
        }
        Action::A11yDoctor => json!({"type": "a11y.doctor", "id": id}),
        Action::A11ySetupAccessibility => json!({"type": "a11y.setup_accessibility", "id": id}),
        Action::A11yClickElementByRef { object_ref } => {
            json!({"type": "a11y.click_element_by_ref", "id": id, "object_ref": object_ref})
        }
        _ => serde_json::json!({"error": "not a a11y action"}),
    }
}
