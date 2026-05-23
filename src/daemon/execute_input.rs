use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

pub(crate) async fn execute_input(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        InputKeyboardType { ref text } => {
            backend.keyboard_type(text).await?;
            serde_json::json!({"typed": text.len()})
        }
        InputKeyboardKey { ref key } => {
            backend.keyboard_key(key).await?;
            serde_json::json!({"key": key})
        }
        InputKeyboardCombo { ref keys } => {
            backend.keyboard_combo(keys).await?;
            serde_json::json!({"combo": keys})
        }
        InputMouse {
            ref action,
            x,
            y,
            ref button,
            dx,
            dy,
        } => {
            match action.as_str() {
                "move" => {
                    backend
                        .mouse_move(x.unwrap_or(0.0), y.unwrap_or(0.0))
                        .await?
                }
                "click" => {
                    backend
                        .mouse_click(button.as_deref().unwrap_or("left"))
                        .await?
                }
                "scroll" => {
                    backend
                        .mouse_scroll(dx.unwrap_or(0.0), dy.unwrap_or(0.0))
                        .await?
                }
                _ => anyhow::bail!("unknown mouse action: {}", action),
            }
            serde_json::json!({"mouse": action})
        }
        InputMouseDrag {
            from_x,
            from_y,
            to_x,
            to_y,
            ref button,
            duration_ms,
        } => {
            let button = button.as_deref().unwrap_or("left");
            backend
                .mouse_drag(from_x, from_y, to_x, to_y, button, duration_ms)
                .await?;
            serde_json::json!({
                "dragged": true,
                "from": {"x": from_x, "y": from_y},
                "to": {"x": to_x, "y": to_y},
                "button": button,
                "duration_ms": duration_ms.unwrap_or(0)
            })
        }
        InputListLayouts => {
            let layouts = backend.keyboard_layout_list().await?;
            serde_json::to_value(layouts)?
        }
        InputGetLayout => {
            let layout = backend.keyboard_layout_get().await?;
            serde_json::to_value(layout)?
        }
        InputSetLayout {
            index,
            ref name,
            ref variant,
        } => {
            backend
                .keyboard_layout_set(index, name.as_deref(), variant.as_deref())
                .await?;
            serde_json::json!({"set": true})
        }
        InputAddLayout {
            ref name,
            ref variant,
        } => {
            backend
                .keyboard_layout_add(name, variant.as_deref())
                .await?;
            serde_json::json!({"added": name})
        }
        InputRemoveLayout { index } => {
            backend.keyboard_layout_remove(index).await?;
            serde_json::json!({"removed": index})
        }

        _ => unreachable!("not a input action"),
    })
}
