#!/usr/bin/env python3
"""Resolve ALL daemon.rs merge conflicts in-place."""
path = "/home/coemedia/projects/deskbrid/src/daemon.rs"
content = open(path).read()

replacements = [
    # 1. Focus event → keep main (theirs)
    ('<<<<<<< HEAD\n        Action::WindowsFocus(id) => Some(crate::protocol::DeskbridEvent::WindowFocused {\n            window_id: id.clone(),\n            timestamp: now,\n        }),\n=======\n        // Use the resolved window ID from the response data when available,\n        // so subscribers get the canonical ID, not the caller-provided selector.\n        Action::WindowsFocus(_) => {\n            let window_id = data\n                .get("focused")\n                .and_then(|v| v.as_str())\n                .unwrap_or("unknown")\n                .to_string();\n            Some(crate::protocol::DeskbridEvent::WindowFocused {\n                window_id,\n                timestamp: now,\n            })\n        }\n>>>>>>> origin/main',
     '        // Use the resolved window ID from the response data when available,\n        // so subscribers get the canonical ID, not the caller-provided selector.\n        Action::WindowsFocus(_) => {\n            let window_id = data\n                .get("focused")\n                .and_then(|v| v.as_str())\n                .unwrap_or("unknown")\n                .to_string();\n            Some(crate::protocol::DeskbridEvent::WindowFocused {\n                window_id,\n                timestamp: now,\n            })\n        }'),

    # 2. Capabilities init → keep HEAD (ours) — extended fields
    ('<<<<<<< HEAD\n                "reason": serde_json::Value::Null,\n                "requires": [],\n                "session": "any",\n                "degraded_modes": []\n=======\n                "reason": serde_json::Value::Null\n>>>>>>> origin/main',
     '                "reason": serde_json::Value::Null,\n                "requires": [],\n                "session": "any",\n                "degraded_modes": []'),

    # 3. GNOME requires/session
    ('<<<<<<< HEAD\n        set_requires(&mut actions, "windows.list", &["gnome-extension"]);\n        set_requires(&mut actions, "windows.focus", &["gnome-extension"]);\n        set_requires(&mut actions, "workspaces.list", &["gnome-extension"]);\n        set_requires(&mut actions, "workspaces.switch", &["gnome-extension"]);\n        set_session(&mut actions, "input.mouse", "wayland");\n=======\n>>>>>>> origin/main',
     '        set_requires(&mut actions, "windows.list", &["gnome-extension"]);\n        set_requires(&mut actions, "windows.focus", &["gnome-extension"]);\n        set_requires(&mut actions, "workspaces.list", &["gnome-extension"]);\n        set_requires(&mut actions, "workspaces.switch", &["gnome-extension"]);\n        set_session(&mut actions, "input.mouse", "wayland");'),

    # 4. KDE/Hyprland requires/session
    ('<<<<<<< HEAD\n        set_requires(&mut actions, "input.keyboard", &["ydotoold", "/dev/uinput"]);\n        set_requires(&mut actions, "input.mouse", &["ydotoold", "/dev/uinput"]);\n        set_session(&mut actions, "input.keyboard", "wayland");\n        set_session(&mut actions, "input.mouse", "wayland");\n=======\n>>>>>>> origin/main',
     '        set_requires(&mut actions, "input.keyboard", &["ydotoold", "/dev/uinput"]);\n        set_requires(&mut actions, "input.mouse", &["ydotoold", "/dev/uinput"]);\n        set_session(&mut actions, "input.keyboard", "wayland");\n        set_session(&mut actions, "input.mouse", "wayland");'),

    # 5. Bluetooth unsupported → keep main (theirs)
    ('<<<<<<< HEAD\n=======\n        "bluetooth.pair",\n        "bluetooth.forget",\n>>>>>>> origin/main',
     '        "bluetooth.pair",\n        "bluetooth.forget",'),

    # 6. Clipboard → keep main (theirs)
    ('<<<<<<< HEAD\n        deps.insert("wl_clipboard".to_string(), check_in_path("wl-copy"));\n=======\n        deps.insert("wl_clipboard".to_string(), check_clipboard_tools());\n>>>>>>> origin/main',
     '        deps.insert("wl_clipboard".to_string(), check_clipboard_tools());'),

    # 7. Ydotool KDE → keep main (theirs)
    ('<<<<<<< HEAD\n=======\n        deps.insert("ydotool".to_string(), check_in_path("ydotool"));\n>>>>>>> origin/main\n        deps.insert("uinput".to_string(), check_uinput());\n    } else if desktop.contains("hyprland") {',
     '        deps.insert("ydotool".to_string(), check_in_path("ydotool"));\n        deps.insert("uinput".to_string(), check_uinput());\n    } else if desktop.contains("hyprland") {'),

    # 8. Ydotool Hyprland → keep main (theirs)
    ('<<<<<<< HEAD\n=======\n        deps.insert("ydotool".to_string(), check_in_path("ydotool"));\n>>>>>>> origin/main\n        deps.insert("uinput".to_string(), check_uinput());',
     '        deps.insert("ydotool".to_string(), check_in_path("ydotool"));\n        deps.insert("uinput".to_string(), check_uinput());'),

    # 9. set_degraded → keep HEAD (ours) — extended fields
    ('<<<<<<< HEAD\n        serde_json::json!({\"supported\": true, \"degraded\": true, "reason": reason, "requires": [], "session": "any", "degraded_modes": [reason]}),\n=======\n        serde_json::json!({\"supported\": true, \"degraded\": true, "reason": reason}),\n>>>>>>> origin/main',
     '        serde_json::json!({\"supported\": true, \"degraded\": true, "reason": reason, "requires": [], "session": "any", "degraded_modes": [reason]}),'),

    # 10. These are the remaining 2 I need to handle manually
]

# Apply all replacements
for i, (old, new) in enumerate(replacements):
    if old in content:
        content = content.replace(old, new)
        print(f"  ✓ {i+1}")
    else:
        print(f"  ✗ {i+1} — not found")

# Now handle the tricky ones that may not perfectly match
# Let me check lines 909-960 (set_unsupported + helpers conflict)
remaining_count = content.count('<<<<<<< HEAD')
print(f"\nRemaining before manual fixup: {remaining_count}")

if remaining_count > 0:
    # Let me look at each remaining conflict section
    import re
    for m in re.finditer(r'<<<<<<< HEAD.*?(=======).*?>>>>>>> [^\n]+', content, re.DOTALL):
        start = m.start()
        snippet = content[start:start+400]
        lines = snippet.split('\n')
        print(f"\n=== Remaining conflict near char {start} ===")
        for l in lines[:20]:
            print(f"  {repr(l) if '<' in l or '>' in l or '=' in l else l[:100]}")
else:
    open(path, 'w').write(content)
    print("✓ ALL CONFLICTS RESOLVED — file saved!")
