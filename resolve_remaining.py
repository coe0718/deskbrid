#!/usr/bin/env python3
"""Resolve conflicts in gnome.rs, kde.rs, permissions.rs, protocol.rs for PR #10."""
import re

files = [
    ("src/backend/gnome.rs", "theirs"),   # FocusWindow false→true
    ("src/backend/kde.rs", "theirs"),     # journalctl --since
    ("src/permissions.rs", "ours"),       # SystemRemediate/SystemNormalizeCoords
    ("src/protocol.rs", "ours"),          # SystemRemediate/SystemNormalizeCoords (5 conflicts)
]

for path, default_choice in files:
    full_path = f"/home/coemedia/projects/deskbrid/{path}"
    with open(full_path, 'rb') as f:
        content = f.read().decode('utf-8')
    
    positions = [(m.start(), m) for m in re.finditer(r'<<<<<<< HEAD', content)]
    n = len(positions)
    print(f"{path}: {n} conflicts")
    
    if n == 0:
        continue
    
    for idx, (start_pos, _) in enumerate(sorted(positions, reverse=True), 1):
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
        
        # For gnome/kde, we know both conflicts are "keep theirs" (main's version)
        # For permissions/protocol, all conflicts are "keep ours" (branch additions)
        if default_choice == 'theirs':
            replacement = content[divider + 8:end_marker].rstrip('\n')
        else:
            replacement = content[start_pos + 13:divider].rstrip('\n')
        
        conflict_end = end_marker + 19
        content = content[:start_pos] + replacement + content[conflict_end:]
        
        snippet = content[start_pos:start_pos+80]
        print(f"  ✓ {idx}: {default_choice}")
    
    remaining = content.count('<<<<<<< HEAD')
    if remaining == 0:
        with open(full_path, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"  → SAVED")
    else:
        print(f"  ✗ {remaining} remaining!")
