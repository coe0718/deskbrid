import re

# Resolve daemon.rs conflicts
# Strategy: focus event -> keep main (resolved ID), capabilities -> keep HEAD (extended format)
content = open("src/daemon.rs").read()

# 1. Focus event (line 288): keep main's version (resolved window ID via data["focused"])
focus_pat = r'''<<<<<<< HEAD
        Action::WindowsFocus\(id\) => Some\(crate::protocol::DeskbridEvent::WindowFocused \{
            window_id: id\.clone\(\),
            timestamp: now,
        \}\)?
=======
        // Use the resolved window ID from the response data when available,
        // so subscribers get the canonical ID, not the caller-provided selector\.
        Action::WindowsFocus\(_\) => \{
            let window_id = data
                \.get\("focused"\)
                \.and_then\(\|v\| v\.as_str\(\)\)
                \.unwrap_or\("unknown"\)
                \.to_string\(\);
            Some\(crate::protocol::DeskbridEvent::WindowFocused \{
                window_id,
                timestamp: now,
            \}\)
        \}
>>>>>>> origin/main'''

# Replace with main's version
main_focus = '''        // Use the resolved window ID from the response data when available,
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

content = re.sub(focus_pat, main_focus, content)

# 2. Capabilities format conflicts (multiple): keep HEAD's extended format with requires/session/degraded_modes
# Find all remaining <<<<<<< HEAD sections and resolve by keeping HEAD
# Pattern for capability fields
cap_pat = r'(<<<<<<< HEAD\n\s+"reason": serde_json::Value::Null,\n\s+"requires": \[\],\n\s+"session": "any",\n\s+"degraded_modes": \[\]\n=======\n\s+"reason": serde_json::Value::Null\n>>>>>>> origin/main)'

cap_replacement = r'            "reason": serde_json::Value::Null,\n            "requires": [],\n            "session": "any",\n            "degraded_modes": []'

content = re.sub(cap_pat, cap_replacement, content)

# 3. Check for any other conflicts
remaining = re.findall(r'<<<<<<< HEAD.*?>>>>>>> [^\n]+', content, re.DOTALL)
print(f"Remaining conflicts: {len(remaining)}")
for i, m in enumerate(remaining):
    print(f"\n--- Conflict {i+1} ---")
    print(m[:300])

open("src/daemon.rs", "w").write(content)
print("\ndaemon.rs written")
