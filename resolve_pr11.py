#!/usr/bin/env python3
"""Resolve ALL merge conflicts for PR #11 across 5 files."""
import re

files_rules = {
    "src/backend/gnome.rs": "theirs",   # FocusWindow false→true
    "src/backend/kde.rs": "theirs",     # journalctl --since  
    "src/permissions.rs": "ours",       # SystemRemediate/NormalizeCoords
    "src/protocol.rs": "ours",          # SystemRemediate/NormalizeCoords
}

for path, default_choice in files_rules.items():
    full_path = f"/home/coemedia/projects/deskbrid/{path}"
    with open(full_path, 'rb') as f:
        content = f.read().decode('utf-8')
    
    positions = sorted([m.start() for m in re.finditer(r'<<<<<<< HEAD', content)], reverse=True)
    if not positions:
        print(f"{path}: 0 conflicts")
        continue
    
    print(f"{path}: {len(positions)} conflicts")
    
    for start_pos in positions:
        end_marker = content.find('>>>>>>> origin/main', start_pos)
        if end_marker == -1:
            continue
        
        divider = content.find('=======\n', start_pos)
        if divider == -1:
            divider = content.find('=======', start_pos)
        if divider == -1 or divider > end_marker:
            continue
        
        if default_choice == 'theirs':
            replacement = content[divider + 8:end_marker].rstrip('\n')
        else:
            replacement = content[start_pos + 13:divider].rstrip('\n')
        
        conflict_end = end_marker + 19
        content = content[:start_pos] + replacement + content[conflict_end:]
    
    remaining = content.count('<<<<<<< HEAD')
    if remaining == 0:
        with open(full_path, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"  → Saved (all resolved)")
    else:
        print(f"  ✗ {remaining} remaining!")

# Now daemon.rs — needs more careful handling like PR #10
print("\ndaemon.rs: resolving...")
path = "/home/coemedia/projects/deskbrid/src/daemon.rs"
with open(path, 'rb') as f:
    content = f.read().decode('utf-8')

positions = sorted([m.start() for m in re.finditer(r'<<<<<<< HEAD', content)], reverse=True)
print(f"  {len(positions)} conflicts")

for idx, start_pos in enumerate(positions, 1):
    end_marker = content.find('>>>>>>> origin/main', start_pos)
    if end_marker == -1:
        print(f"  ✗ {idx}: no >>>>>>>>")
        continue
    
    divider = content.find('=======\n', start_pos)
    if divider == -1:
        divider = content.find('=======', start_pos)
    if divider == -1 or divider > end_marker:
        print(f"  ✗ {idx}: no =======")
        continue
    
    # Peek at the conflict type
    snippet = content[start_pos:start_pos+300]
    
    # Decide: same pattern as PR #10
    if any(x in snippet for x in ['Action::WindowsFocus(', 'Action::WindowsGet(ref', 'check_in_path("wl-copy")', 'check_in_path("ydotool")', 'bluetooth.pair', 'bluetooth.forget', 'check_clipboard_tools']):
        choice = 'theirs'
    elif any(x in snippet for x in ['set_requires(', 'set_session(', '"reason": serde_json::Value::Null', 'degraded', 'fn set_session', 'fn set_requires']):
        choice = 'ours'
    elif 'SystemRemediate' in snippet or 'SystemNormalizeCoords' in snippet:
        choice = 'ours'
    elif 'SystemIdle' in snippet:
        # SystemIdle exists in both - if it's a replacement match arm, check if ours has same
        ours_text = content[start_pos + 13:divider]
        if 'normalize_coords' in ours_text or 'SystemNormalizeCoords' in ours_text:
            choice = 'ours'
        else:
            choice = 'theirs'
    else:
        # Unknown - check what we're dealing with
        first_ours = snippet.split('\n')[0] if '\n' in snippet else snippet
        print(f"  ? {idx}: UNKNOWN - {first_ours[:60]}")
        choice = 'theirs'  # default safe
    
    # Extract chosen side
    if choice == 'theirs':
        replacement = content[divider + 8:end_marker].rstrip('\n')
    else:
        replacement = content[start_pos + 13:divider].rstrip('\n')
    
    conflict_end = end_marker + 19
    content = content[:start_pos] + replacement + content[conflict_end:]

remaining = content.count('<<<<<<< HEAD')
if remaining == 0:
    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f"  → Saved (all {len(positions)} resolved)")
else:
    print(f"  ✗ {remaining} remaining!")
