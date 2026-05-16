#!/usr/bin/env python3
"""Resolve ALL merge conflicts in src/daemon.rs for PR #9."""
import re

path = "/home/coemedia/projects/deskbrid/src/daemon.rs"
content = open(path).read()

replacements = [
    # 1. Focus event: keep main's resolved window ID version
    (
        re.escape("<<<<<<< HEAD\n        Action::WindowsFocus(id) => Some(crate::protocol::DeskbridEvent::WindowFocused {\n            window_id: id.clone(),\n            timestamp: now,\n        }),\n=======\n        // Use the resolved window ID from the response data when available,\n        // so subscribers get the canonical ID, not the caller-provided selector.\n        Action::WindowsFocus(_) => {\n            let window_id = data\n                .get(\"focused\")\n                .and_then(|v| v.as_str())\n                .unwrap_or(\"unknown\")\n                .to_string();\n            Some(crate::protocol::DeskbridEvent::WindowFocused {\n                window_id,\n                timestamp: now,\n            })\n        }\n>>>>>>> origin/main"),
        "        // Use the resolved window ID from the response data when available,\n        // so subscribers get the canonical ID, not the caller-provided selector.\n        Action::WindowsFocus(_) => {\n            let window_id = data\n                .get(\"focused\")\n                .and_then(|v| v.as_str())\n                .unwrap_or(\"unknown\")\n                .to_string();\n            Some(crate::protocol::DeskbridEvent::WindowFocused {\n                window_id,\n                timestamp: now,\n            })\n        }"
    ),
    # 2. Capabilities init format: keep HEAD's extended fields
    (
        re.escape("<<<<<<< HEAD\n                \"reason\": serde_json::Value::Null,\n                \"requires\": [],\n                \"session\": \"any\",\n                \"degraded_modes\": []\n=======\n                \"reason\": serde_json::Value::Null\n>>>>>>> origin/main"),
        "                \"reason\": serde_json::Value::Null,\n                \"requires\": [],\n                \"session\": \"any\",\n                \"degraded_modes\": []"
    ),
    # 3. GNOME requires/session: keep HEAD's additions
    (
        re.escape("<<<<<<< HEAD\n        set_requires(&mut actions, \"windows.list\", &[\"gnome-extension\"]);\n        set_requires(&mut actions, \"windows.focus\", &[\"gnome-extension\"]);\n        set_requires(&mut actions, \"workspaces.list\", &[\"gnome-extension\"]);\n        set_requires(&mut actions, \"workspaces.switch\", &[\"gnome-extension\"]);\n        set_session(&mut actions, \"input.mouse\", \"wayland\");\n=======\n>>>>>>> origin/main"),
        "        set_requires(&mut actions, \"windows.list\", &[\"gnome-extension\"]);\n        set_requires(&mut actions, \"windows.focus\", &[\"gnome-extension\"]);\n        set_requires(&mut actions, \"workspaces.list\", &[\"gnome-extension\"]);\n        set_requires(&mut actions, \"workspaces.switch\", &[\"gnome-extension\"]);\n        set_session(&mut actions, \"input.mouse\", \"wayland\");"
    ),
    # 4. KDE/Hyprland requires/session: keep HEAD's additions
    (
        re.escape("<<<<<<< HEAD\n        set_requires(&mut actions, \"input.keyboard\", &[\"ydotoold\", \"/dev/uinput\"]);\n        set_requires(&mut actions, \"input.mouse\", &[\"ydotoold\", \"/dev/uinput\"]);\n        set_session(&mut actions, \"input.keyboard\", \"wayland\");\n        set_session(&mut actions, \"input.mouse\", \"wayland\");\n=======\n>>>>>>> origin/main"),
        "        set_requires(&mut actions, \"input.keyboard\", &[\"ydotoold\", \"/dev/uinput\"]);\n        set_requires(&mut actions, \"input.mouse\", &[\"ydotoold\", \"/dev/uinput\"]);\n        set_session(&mut actions, \"input.keyboard\", \"wayland\");\n        set_session(&mut actions, \"input.mouse\", \"wayland\");"
    ),
    # 5. Bluetooth unsupported: keep main's additions
    (
        re.escape("<<<<<<< HEAD\n=======\n        \"bluetooth.pair\",\n        \"bluetooth.forget\",\n>>>>>>> origin/main"),
        "        \"bluetooth.pair\",\n        \"bluetooth.forget\","
    ),
    # 6. Clipboard check (Hyprland): keep main's check_clipboard_tools
    (
        re.escape("<<<<<<< HEAD\n        deps.insert(\"wl_clipboard\".to_string(), check_in_path(\"wl-copy\"));\n=======\n        deps.insert(\"wl_clipboard\".to_string(), check_clipboard_tools());\n>>>>>>> origin/main"),
        "        deps.insert(\"wl_clipboard\".to_string(), check_clipboard_tools());"
    ),
    # 7. Ydotool KDE: keep main's addition
    (
        re.escape("<<<<<<< HEAD\n=======\n        deps.insert(\"ydotool\".to_string(), check_in_path(\"ydotool\"));\n>>>>>>> origin/main\n        deps.insert(\"uinput\".to_string(), check_uinput());\n    } else if desktop.contains(\"hyprland\") {\n"),
        "        deps.insert(\"ydotool\".to_string(), check_in_path(\"ydotool\"));\n        deps.insert(\"uinput\".to_string(), check_uinput());\n    } else if desktop.contains(\"hyprland\") {\n"
    ),
    # 8. Ydotool Hyprland: keep main's addition
    (
        re.escape("<<<<<<< HEAD\n=======\n        deps.insert(\"ydotool\".to_string(), check_in_path(\"ydotool\"));\n>>>>>>> origin/main\n        deps.insert(\"uinput\".to_string(), check_uinput());\n"),
        "        deps.insert(\"ydotool\".to_string(), check_in_path(\"ydotool\"));\n        deps.insert(\"uinput\".to_string(), check_uinput());\n"
    ),
    # 9. set_degraded format: keep HEAD's extended fields
    (
        re.escape("<<<<<<< HEAD\n        serde_json::json!({\"supported\": true, \"degraded\": true, \"reason\": reason, \"requires\": [], \"session\": \"any\", \"degraded_modes\": [reason]}),\n=======\n        serde_json::json!({\"supported\": true, \"degraded\": true, \"reason\": reason}),\n>>>>>>> origin/main"),
        "        serde_json::json!({\"supported\": true, \"degraded\": true, \"reason\": reason, \"requires\": [], \"session\": \"any\", \"degraded_modes\": [reason]}),"
    ),
    # 10. set_unsupported format: keep HEAD's extended fields (includes set_requires/set_session)
    (
        re.escape("<<<<<<< HEAD\n        serde_json::json!({\"supported\": false, \"degraded\": false, \"reason\": reason, \"requires\": [], \"session\": \"any\", \"degraded_modes\": []}),\n    );\n}\n\nfn set_requires(\n    actions: &mut serde_json::Map<String, serde_json::Value>,\n    action: &str,\n    requires: &[&str],\n) {\n    if let Some(v) = actions.get_mut(action) {\n        v[\"requires\"] = serde_json::json!(requires);\n    }\n}\n\nfn set_session(\n    actions: &mut serde_json::Map<String, serde_json::Value>,\n    action: &str,\n    session: &str,\n) {\n    if let Some(v) = actions.get_mut(action) {\n        v[\"session\"] = serde_json::json!(session);\n    }\n}\n\n=======\n        serde_json::json!({\"supported\": false, \"degraded\": false, \"reason\": reason}),\n    );\n}\n\n>>>>>>> origin/main\n"),
        "        serde_json::json!({\"supported\": false, \"degraded\": false, \"reason\": reason, \"requires\": [], \"session\": \"any\", \"degraded_modes\": []}),\n    );\n}\n\nfn set_requires(\n    actions: &mut serde_json::Map<String, serde_json::Value>,\n    action: &str,\n    requires: &[&str],\n) {\n    if let Some(v) = actions.get_mut(action) {\n        v[\"requires\"] = serde_json::json!(requires);\n    }\n}\n\nfn set_session(\n    actions: &mut serde_json::Map<String, serde_json::Value>,\n    action: &str,\n    session: &str,\n) {\n    if let Some(v) = actions.get_mut(action) {\n        v[\"session\"] = serde_json::json!(session);\n    }\n}\n"
    ),
    # 11. check_clipboard_tools: keep main's new function
    (
        re.escape("<<<<<<< HEAD\n=======\n\nfn check_clipboard_tools() -> serde_json::Value {\n    let copy = std::process::Command::new(\"sh\")\n        .arg(\"-c\")\n        .arg(\"command -v wl-copy >/dev/null 2>&1\")\n        .status();\n    let paste = std::process::Command::new(\"sh\")\n        .arg(\"-c\")\n        .arg(\"command -v wl-paste >/dev/null 2>&1\")\n        .status();\n\n    let copy_ok = copy.map(|s| s.success()).unwrap_or(false);\n    let paste_ok = paste.map(|s| s.success()).unwrap_or(false);\n\n    if copy_ok && paste_ok {\n        serde_json::json!({\"ok\": true, \"details\": \"wl-copy and wl-paste present\"})\n    } else {\n        let mut missing = Vec::new();\n        if !copy_ok {\n            missing.push(\"wl-copy\");\n        }\n        if !paste_ok {\n            missing.push(\"wl-paste\");\n        }\n        serde_json::json!({\"ok\": false, \"details\": format!(\"missing: {}\", missing.join(\", \"))})\n    }\n}\n>>>>>>> origin/main"),
        "\n\nfn check_clipboard_tools() -> serde_json::Value {\n    let copy = std::process::Command::new(\"sh\")\n        .arg(\"-c\")\n        .arg(\"command -v wl-copy >/dev/null 2>&1\")\n        .status();\n    let paste = std::process::Command::new(\"sh\")\n        .arg(\"-c\")\n        .arg(\"command -v wl-paste >/dev/null 2>&1\")\n        .status();\n\n    let copy_ok = copy.map(|s| s.success()).unwrap_or(false);\n    let paste_ok = paste.map(|s| s.success()).unwrap_or(false);\n\n    if copy_ok && paste_ok {\n        serde_json::json!({\"ok\": true, \"details\": \"wl-copy and wl-paste present\"})\n    } else {\n        let mut missing = Vec::new();\n        if !copy_ok {\n            missing.push(\"wl-copy\");\n        }\n        if !paste_ok {\n            missing.push(\"wl-paste\");\n        }\n        serde_json::json!({\"ok\": false, \"details\": format!(\"missing: {}\", missing.join(\", \"))})\n    }\n}"
    ),
]

for i, (old, new) in enumerate(replacements):
    count = content.count(old)
    if count > 0:
        content = content.replace(old, new)
        print(f"  ✓ Replacement {i+1}: applied ({count} occurrence(s))")
    else:
        print(f"  ✗ Replacement {i+1}: NOT FOUND")

# Final check
remaining = re.findall(r'<<<<<<< HEAD.*?>>>>>>> [^\n]+', content, re.DOTALL)
print(f"\nRemaining conflicts: {len(remaining)}")

if remaining:
    for m in remaining:
        first_line = m.split('\n')[0]
        print(f"  - {first_line[:80]}...")
else:
    open(path, "w").write(content)
    print("✓ ALL CONFLICTS RESOLVED — saved!")

