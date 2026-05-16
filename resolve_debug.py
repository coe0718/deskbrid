#!/usr/bin/env python3
"""Resolve ALL merge conflicts in src/daemon.rs using simple string replace."""
import re

path = "/home/coemedia/projects/deskbrid/src/daemon.rs"
content = open(path).read()

# Read conflict markers directly from the file so exact strings match
# Strategy: read the file, find conflict sections, replace them

# 1. Focus event: keep main's (theirs) — the longer one with resolved ID
old = "<<<<<<< HEAD\n        Action::WindowsFocus(id) => Some(crate::protocol::DeskbridEvent::WindowFocused {\n            window_id: id.clone(),\n            timestamp: now,\n        }),\n=======\n        // Use the resolved window ID from the response data when available,\n        // so subscribers get the canonical ID, not the caller-provided selector.\n        Action::WindowsFocus(_) => {\n            let window_id = data\n                .get(\"focused\")\n                .and_then(|v| v.as_str())\n                .unwrap_or(\"unknown\")\n                .to_string();\n            Some(crate::protocol::DeskbridEvent::WindowFocused {\n                window_id,\n                timestamp: now,\n            })\n        }\n>>>>>>> origin/main"
new = "        // Use the resolved window ID from the response data when available,\n        // so subscribers get the canonical ID, not the caller-provided selector.\n        Action::WindowsFocus(_) => {\n            let window_id = data\n                .get(\"focused\")\n                .and_then(|v| v.as_str())\n                .unwrap_or(\"unknown\")\n                .to_string();\n            Some(crate::protocol::DeskbridEvent::WindowFocused {\n                window_id,\n                timestamp: now,\n            })\n        }"
if old in content:
    content = content.replace(old, new)
    print("✓ 1. Focus event resolved")
else:
    print("✗ 1. Focus event NOT FOUND")

# 2. Capabilities init: keep HEAD's extended
old = '<<<<<<< HEAD\n                "reason": serde_json::Value::Null,\n                "requires": [],\n                "session": "any",\n                "degraded_modes": []\n=======\n                "reason": serde_json::Value::Null\n>>>>>>> origin/main'
new = '                "reason": serde_json::Value::Null,\n                "requires": [],\n                "session": "any",\n                "degraded_modes": []'
if old in content:
    content = content.replace(old, new)
    print("✓ 2. Capabilities init resolved")
else:
    print("✗ 2. Capabilities init NOT FOUND")

# 3. GNOME requires: keep HEAD's
old = '<<<<<<< HEAD\n        set_requires(&mut actions, "windows.list", &["gnome-extension"]);\n        set_requires(&mut actions, "windows.focus", &["gnome-extension"]);\n        set_requires(&mut actions, "workspaces.list", &["gnome-extension"]);\n        set_requires(&mut actions, "workspaces.switch", &["gnome-extension"]);\n        set_session(&mut actions, "input.mouse", "wayland");\n=======\n>>>>>>> origin/main'
new = '        set_requires(&mut actions, "windows.list", &["gnome-extension"]);\n        set_requires(&mut actions, "windows.focus", &["gnome-extension"]);\n        set_requires(&mut actions, "workspaces.list", &["gnome-extension"]);\n        set_requires(&mut actions, "workspaces.switch", &["gnome-extension"]);\n        set_session(&mut actions, "input.mouse", "wayland");'
if old in content:
    content = content.replace(old, new)
    print("✓ 3. GNOME requires resolved")
else:
    print("✗ 3. GNOME requires NOT FOUND")

# 4. KDE requires: keep HEAD's
old = '<<<<<<< HEAD\n        set_requires(&mut actions, "input.keyboard", &["ydotoold", "/dev/uinput"]);\n        set_requires(&mut actions, "input.mouse", &["ydotoold", "/dev/uinput"]);\n        set_session(&mut actions, "input.keyboard", "wayland");\n        set_session(&mut actions, "input.mouse", "wayland");\n=======\n>>>>>>> origin/main'
new = '        set_requires(&mut actions, "input.keyboard", &["ydotoold", "/dev/uinput"]);\n        set_requires(&mut actions, "input.mouse", &["ydotoold", "/dev/uinput"]);\n        set_session(&mut actions, "input.keyboard", "wayland");\n        set_session(&mut actions, "input.mouse", "wayland");'
if old in content:
    content = content.replace(old, new)
    print("✓ 4. KDE requires resolved")
else:
    print("✗ 4. KDE requires NOT FOUND")

# 5. Bluetooth unsupported: keep main's additions
old = '<<<<<<< HEAD\n=======\n        "bluetooth.pair",\n        "bluetooth.forget",\n>>>>>>> origin/main'
new = '        "bluetooth.pair",\n        "bluetooth.forget",'
if old in content:
    content = content.replace(old, new)
    print("✓ 5. Bluetooth unsupported resolved")
else:
    print("✗ 5. Bluetooth unsupported NOT FOUND")

# 6. Clipboard Hyprland: keep main's check_clipboard_tools
old = '<<<<<<< HEAD\n        deps.insert("wl_clipboard".to_string(), check_in_path("wl-copy"));\n=======\n        deps.insert("wl_clipboard".to_string(), check_clipboard_tools());\n>>>>>>> origin/main'
new = '        deps.insert("wl_clipboard".to_string(), check_clipboard_tools());'
if old in content:
    content = content.replace(old, new)
    print("✓ 6. Clipboard resolved")
else:
    print("✗ 6. Clipboard NOT FOUND")

# 7. Ydotool KDE: keep main's
old = '<<<<<<< HEAD\n=======\n        deps.insert("ydotool".to_string(), check_in_path("ydotool"));\n>>>>>>> origin/main\n        deps.insert("uinput".to_string(), check_uinput());\n    } else if desktop.contains("hyprland") {'
new = '        deps.insert("ydotool".to_string(), check_in_path("ydotool"));\n        deps.insert("uinput".to_string(), check_uinput());\n    } else if desktop.contains("hyprland") {'
if old in content:
    content = content.replace(old, new)
    print("✓ 7. Ydotool KDE resolved")
else:
    print("✗ 7. Ydotool KDE NOT FOUND")

# 8. Ydotool Hyprland: keep main's
old = '<<<<<<< HEAD\n=======\n        deps.insert("ydotool".to_string(), check_in_path("ydotool"));\n>>>>>>> origin/main\n        deps.insert("uinput".to_string(), check_uinput());'
new = '        deps.insert("ydotool".to_string(), check_in_path("ydotool"));\n        deps.insert("uinput".to_string(), check_uinput());'
if old in content:
    content = content.replace(old, new)
    print("✓ 8. Ydotool Hyprland resolved")
else:
    print("✗ 8. Ydotool Hyprland NOT FOUND")

# 9. set_degraded format: keep HEAD's
old = '<<<<<<< HEAD\n        serde_json::json!({\"supported\": true, \"degraded\": true, \"reason\": reason, \"requires\": [], \"session\": \"any\", \"degraded_modes\": [reason]}),\n=======\n        serde_json::json!({\"supported\": true, \"degraded\": true, \"reason\": reason}),\n>>>>>>> origin/main'
new = '        serde_json::json!({\"supported\": true, \"degraded\": true, \"reason\": reason, \"requires\": [], \"session\": \"any\", \"degraded_modes\": [reason]}),'
if old in content:
    content = content.replace(old, new)
    print("✓ 9. set_degraded format resolved")
else:
    print("✗ 9. set_degraded format NOT FOUND")

# 10. set_unsupported format + helpers: keep HEAD's
old = '<<<<<<< HEAD\n        serde_json::json!({\"supported\": false, \"degraded\": false, "reason": reason, "requires": [], "session": "any", "degraded_modes": []}),\n    );\n}\n\nfn set_requires(\n    actions: &mut serde_json::Map<String, serde_json::Value>,\n    action: &str,\n    requires: &[&str],\n) {\n    if let Some(v) = actions.get_mut(action) {\n        v["requires"] = serde_json::json!(requires);\n    }\n}\n\nfn set_session(\n    actions: &mut serde_json::Map<String, serde_json::Value>,\n    action: &str,\n    session: &str,\n) {\n    if let Some(v) = actions.get_mut(action) {\n        v["session"] = serde_json::json!(session);\n    }\n}\n\n=======\n        serde_json::json!({\"supported\": false, \"degraded\": false, \"reason\": reason}),\n    );\n}\n\n>>>>>>> origin/main'
# This one is complex, let me match the exact content from the file
print("✗ 10. set_unsupported - checking raw content...")
# Find it by line numbers
lines = content.split('\n')
for i, line in enumerate(lines):
    if '<<<<<<< HEAD' in line and i > 900 and i < 960:
        print(f"  Conflict at line {i+1}: {line[:80]}")

# 11. check_clipboard_tools function: keep main's
# This should be the LAST conflict at the end of the file
print("\nChecking final conflict section...")
last_conflict_lines = []
in_conflict = False
for i, line in enumerate(lines):
    if '<<<<<<< HEAD' in line and i > 990:
        in_conflict = True
    if in_conflict:
        last_conflict_lines.append((i+1, line))
        if '>>>>>>>' in line:
            break

if last_conflict_lines:
    print(f"  Lines {last_conflict_lines[0][0]}-{last_conflict_lines[-1][0]}:")
    for ln, l in last_conflict_lines:
        print(f"    {ln}: {l[:100]}")
    print(f"  Total: {len(last_conflict_lines)} lines")

# Check remaining conflicts
remaining = content.count('<<<<<<< HEAD')
print(f"\nTotal remaining conflict markers: {remaining}")

if remaining == 0:
    open(path, "w").write(content)
    print("✓ ALL CONFLICTS RESOLVED — saved!")
