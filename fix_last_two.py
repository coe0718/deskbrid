# Fix the two remaining conflicts manually
path = "/home/coemedia/projects/deskbrid/src/daemon.rs"
content = open(path).read()

# 1. set_unsupported conflict (lines 925-955) - keep HEAD's extended format + helpers
old = '''<<<<<<< HEAD
        serde_json::json!({\"supported\": false, \"degraded\": false, \"reason\": reason, \"requires\": [], \"session\": \"any\", \"degraded_modes\": []}),
    );
}

fn set_requires(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    action: &str,
    requires: &[&str],
) {
    if let Some(v) = actions.get_mut(action) {
        v[\"requires\"] = serde_json::json!(requires);
    }
}

fn set_session(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    action: &str,
    session: &str,
) {
    if let Some(v) = actions.get_mut(action) {
        v[\"session\"] = serde_json::json!(session);
    }
}

=======
        serde_json::json!({\"supported\": false, \"degraded\": false, \"reason\": reason}),
    );
}

>>>>>>> origin/main
fn check_in_path'''

new = '''        serde_json::json!({\"supported\": false, \"degraded\": false, \"reason\": reason, \"requires\": [], \"session\": \"any\", \"degraded_modes\": []}),
    );
}

fn set_requires(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    action: &str,
    requires: &[&str],
) {
    if let Some(v) = actions.get_mut(action) {
        v[\"requires\"] = serde_json::json!(requires);
    }
}

fn set_session(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    action: &str,
    session: &str,
) {
    if let Some(v) = actions.get_mut(action) {
        v[\"session\"] = serde_json::json!(session);
    }
}

fn check_in_path'''

if old in content:
    content = content.replace(old, new)
    print("✓ Conflict 10 resolved (set_unsupported + helpers)")
else:
    print("✗ Conflict 10 NOT FOUND - checking raw bytes...")
    idx = content.find('<<<<<<< HEAD')
    # Find the last conflict section
    for i in range(920, 960):
        lines = content.split('\n')
        if '<<<<<<<' in lines[i]:
            print(f"  Line {i+1}: {lines[i][:80]}")

# 2. check_clipboard_tools conflict - keep main's version
remaining = content.count('<<<<<<< HEAD')
print(f"\nRemaining conflict markers: {remaining}")

if remaining == 1:
    # Find the last one
    idx = content.find('<<<<<<< HEAD')
    print(f"Last conflict at char {idx}")
    print(content[idx-50:idx+200])

open(path, 'w').write(content)
print("File saved")
