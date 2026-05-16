import re

# ===== KDE RS =====
content = open("src/backend/kde.rs").read()

# Keep main's `--since "30 seconds ago"` version
content = content.replace(
    '''<<<<<<< HEAD
                    &["_COMM=kwin_wayland", "-o", "cat", "-n", "300"],
=======
                    &[
                        "--since",
                        "30 seconds ago",
                        "_COMM=kwin_wayland",
                        "-o",
                        "cat",
                        "-n",
                        "300",
                    ],
>>>>>>> origin/main''',
    '''                    &[
                        "--since",
                        "30 seconds ago",
                        "_COMM=kwin_wayland",
                        "-o",
                        "cat",
                        "-n",
                        "300",
                    ]'''
)
open("src/backend/kde.rs", "w").write(content)
print("kde.rs resolved")

# ===== GNOME RS =====
content = open("src/backend/gnome.rs").read()

content = content.replace(
    '''<<<<<<< HEAD
        self.ext_call_parsed("FocusWindow", &[&target.app_id, &target.title, "false"])
=======
        // Rust already matched deterministically above — pass exact=true so the
        // extension doesn't re-match by app_id and potentially pick the wrong window.
        self.ext_call_parsed("FocusWindow", &[&target.app_id, &target.title, "true"])
>>>>>>> origin/main''',
    '''        // Rust already matched deterministically above — pass exact=true so the
        // extension doesn't re-match by app_id and potentially pick the wrong window.
        self.ext_call_parsed("FocusWindow", &[&target.app_id, &target.title, "true"])'''
)
open("src/backend/gnome.rs", "w").write(content)
print("gnome.rs resolved")

# ===== DAEMON RS =====
content = open("src/daemon.rs").read()

# 1. Focus event: keep main's version
content = content.replace(
    '''<<<<<<< HEAD
        Action::WindowsFocus(id) => Some(crate::protocol::DeskbridEvent::WindowFocused {
            window_id: id.clone(),
            timestamp: now,
        }),
=======
        // Use the resolved window ID from the response data when available,
        // so subscribers get the canonical ID, not the caller-provided selector.
        Action::WindowsFocus(_) => {
            let window_id = data
                .get("focused")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            Some(crate::protocol::DeskbridEvent::WindowFocused {
                window_id,
                timestamp: now,
            })
        }
>>>>>>> origin/main''',
    '''        // Use the resolved window ID from the response data when available,
        // so subscribers get the canonical ID, not the caller-provided selector.
        Action::WindowsFocus(_) => {
            let window_id = data
                .get("focused")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            Some(crate::protocol::DeskbridEvent::WindowFocused {
                window_id,
                timestamp: now,
            })
        }'''
)

# 2. Capabilities init format: keep HEAD's extended fields
content = content.replace(
    '''<<<<<<< HEAD
                "reason": serde_json::Value::Null,
                "requires": [],
                "session": "any",
                "degraded_modes": []
=======
                "reason": serde_json::Value::Null
>>>>>>> origin/main''',
    '''                "reason": serde_json::Value::Null,
                "requires": [],
                "session": "any",
                "degraded_modes": []'''
)

# 3. GNOME requires/session: keep HEAD's additions
content = content.replace(
    '''<<<<<<< HEAD
        set_requires(&mut actions, "windows.list", &["gnome-extension"]);
        set_requires(&mut actions, "windows.focus", &["gnome-extension"]);
        set_requires(&mut actions, "workspaces.list", &["gnome-extension"]);
        set_requires(&mut actions, "workspaces.switch", &["gnome-extension"]);
        set_session(&mut actions, "input.mouse", "wayland");
=======
>>>>>>> origin/main''',
    '''        set_requires(&mut actions, "windows.list", &["gnome-extension"]);
        set_requires(&mut actions, "windows.focus", &["gnome-extension"]);
        set_requires(&mut actions, "workspaces.list", &["gnome-extension"]);
        set_requires(&mut actions, "workspaces.switch", &["gnome-extension"]);
        set_session(&mut actions, "input.mouse", "wayland");'''
)

# 4. KDE/Hyprland requires/session: keep HEAD's additions
content = content.replace(
    '''<<<<<<< HEAD
        set_requires(&mut actions, "input.keyboard", &["ydotoold", "/dev/uinput"]);
        set_requires(&mut actions, "input.mouse", &["ydotoold", "/dev/uinput"]);
        set_session(&mut actions, "input.keyboard", "wayland");
        set_session(&mut actions, "input.mouse", "wayland");
=======
>>>>>>> origin/main''',
    '''        set_requires(&mut actions, "input.keyboard", &["ydotoold", "/dev/uinput"]);
        set_requires(&mut actions, "input.mouse", &["ydotoold", "/dev/uinput"]);
        set_session(&mut actions, "input.keyboard", "wayland");
        set_session(&mut actions, "input.mouse", "wayland");'''
)

# 5. Bluetooth unsupported: keep main's additions
content = content.replace(
    '''<<<<<<< HEAD
=======
        "bluetooth.pair",
        "bluetooth.forget",
>>>>>>> origin/main''',
    '''        "bluetooth.pair",
        "bluetooth.forget",'''
)

# 6. Clipboard check (Hyprland): keep main's check_clipboard_tools
content = content.replace(
    '''<<<<<<< HEAD
        deps.insert("wl_clipboard".to_string(), check_in_path("wl-copy"));
=======
        deps.insert("wl_clipboard".to_string(), check_clipboard_tools());
>>>>>>> origin/main''',
    '''        deps.insert("wl_clipboard".to_string(), check_clipboard_tools());'''
)

# 7. Ydotool check (KDE): keep main's addition
content = content.replace(
    '''<<<<<<< HEAD
=======
        deps.insert("ydotool".to_string(), check_in_path("ydotool"));
>>>>>>> origin/main
        deps.insert("uinput".to_string(), check_uinput());
    } else if desktop.contains("hyprland") {''',
    '''        deps.insert("ydotool".to_string(), check_in_path("ydotool"));
        deps.insert("uinput".to_string(), check_uinput());
    } else if desktop.contains("hyprland") {'''
)

# 8. Ydotool check (Hyprland): keep main's addition
content = content.replace(
    '''<<<<<<< HEAD
=======
        deps.insert("ydotool".to_string(), check_in_path("ydotool"));
>>>>>>> origin/main
        deps.insert("uinput".to_string(), check_uinput());''',
    '''        deps.insert("ydotool".to_string(), check_in_path("ydotool"));
        deps.insert("uinput".to_string(), check_uinput());'''
)

# 9-11. set_degraded/set_unsupported/set_unsupported (second): keep HEAD's extended format
# set_degraded
content = content.replace(
    '''<<<<<<< HEAD
        serde_json::json!({\"supported\": true, \"degraded\": true, \"reason\": reason, \"requires\": [], \"session\": \"any\", \"degraded_modes\": [reason]}),
=======
        serde_json::json!({\"supported\": true, \"degraded\": true, \"reason\": reason}),
>>>>>>> origin/main''',
    '''        serde_json::json!({\"supported\": true, \"degraded\": true, \"reason\": reason, \"requires\": [], \"session\": \"any\", \"degraded_modes\": [reason]}),'''
)

# set_unsupported - note there are TWO of these, one is part of a larger conflict block
# First one (line 925)
content = content.replace(
    '''<<<<<<< HEAD
        serde_json::json!({\"supported\": false, \"degraded\": false, \"reason\": reason, \"requires\": [], \"session\": \"any\", \"degraded_modes\": []});
    }
=======
        serde_json::json!({\"supported\": false, \"degraded\": false, \"reason\": reason});
    }

>>>>>>> origin/main''',
    '''        serde_json::json!({\"supported\": false, \"degraded\": false, \"reason\": reason, \"requires\": [], \"session\": \"any\", \"degraded_modes\": []});
    }

'''
)

# Actually the set_unsupported conflict overlaps with a larger block that includes set_requires/set_session
# Let me handle this more carefully. The second set_unsupported conflict (line 950-955)
# is actually part of a bigger conflict. Let me check.

remaining = re.findall(r'<<<<<<< HEAD.*?>>>>>>> [^\n]+', content, re.DOTALL)
print(f"Remaining conflicts: {len(remaining)}")
for i, m in enumerate(remaining):
    print(f"\n--- Conflict {i+1} ---")
    lines = m.split('\n')
    for l in lines:
        print(l)

if len(remaining) == 0:
    open("src/daemon.rs", "w").write(content)
    print("\nALL CONFLICTS RESOLVED")
else:
    print(f"\n{len(remaining)} UNRESOLVED - not saving")
