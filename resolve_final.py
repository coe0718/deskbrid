import re

content = open("/home/coemedia/projects/deskbrid/src/daemon.rs").read()

# Conflict 1: keep HEAD's set_unsupported extended format + set_requires/set_session helpers
old1 = '''<<<<<<< HEAD
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

>>>>>>> origin/main'''

new1 = '''        serde_json::json!({\"supported\": false, \"degraded\": false, \"reason\": reason, \"requires\": [], \"session\": \"any\", \"degraded_modes\": []}),
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
}'''

content = content.replace(old1, new1)

# Conflict 2: keep main's check_clipboard_tools
old2 = '''<<<<<<< HEAD
=======

fn check_clipboard_tools() -> serde_json::Value {
    let copy = std::process::Command::new("sh")
        .arg("-c")
        .arg("command -v wl-copy >/dev/null 2>&1")
        .status();
    let paste = std::process::Command::new("sh")
        .arg("-c")
        .arg("command -v wl-paste >/dev/null 2>&1")
        .status();

    let copy_ok = copy.map(|s| s.success()).unwrap_or(false);
    let paste_ok = paste.map(|s| s.success()).unwrap_or(false);

    if copy_ok && paste_ok {
        serde_json::json!({\"ok\": true, \"details\": \"wl-copy and wl-paste present\"})
    } else {
        let mut missing = Vec::new();
        if !copy_ok {
            missing.push("wl-copy");
        }
        if !paste_ok {
            missing.push("wl-paste");
        }
        serde_json::json!({\"ok\": false, \"details\": format!(\"missing: {}\", missing.join(\", \"))})
    }
}
>>>>>>> origin/main'''

new2 = '''\n\nfn check_clipboard_tools() -> serde_json::Value {
    let copy = std::process::Command::new("sh")
        .arg("-c")
        .arg("command -v wl-copy >/dev/null 2>&1")
        .status();
    let paste = std::process::Command::new("sh")
        .arg("-c")
        .arg("command -v wl-paste >/dev/null 2>&1")
        .status();

    let copy_ok = copy.map(|s| s.success()).unwrap_or(false);
    let paste_ok = paste.map(|s| s.success()).unwrap_or(false);

    if copy_ok && paste_ok {
        serde_json::json!({\"ok\": true, \"details\": \"wl-copy and wl-paste present\"})
    } else {
        let mut missing = Vec::new();
        if !copy_ok {
            missing.push("wl-copy");
        }
        if !paste_ok {
            missing.push("wl-paste");
        }
        serde_json::json!({\"ok\": false, \"details\": format!(\"missing: {}\", missing.join(\", \"))})
    }
}'''

content = content.replace(old2, new2)

remaining = re.findall(r'<<<<<<< HEAD.*?>>>>>>> [^\n]+', content, re.DOTALL)
print(f"Remaining conflicts: {len(remaining)}")

if len(remaining) == 0:
    open("/home/coemedia/projects/deskbrid/src/daemon.rs", "w").write(content)
    print("ALL CONFLICTS RESOLVED")
else:
    for m in remaining:
        print(m[:300])
        print("---")
