#!/usr/bin/env python3
"""Re-resolve daemon.rs conflicts with correct bottom-up numbering."""
path = "/home/coemedia/projects/deskbrid/src/daemon.rs"

with open(path, 'rb') as f:
    content = f.read().decode('utf-8')

import re

# Find all conflict start positions (bottom-up = last conflict is #1)
positions = [m.start() for m in re.finditer(r'<<<<<<< HEAD', content)]
n = len(positions)
print(f"Found {n} conflicts")

# Bottom-up numbering:
# 1 = check_clipboard_tools (bottom of file) → theirs
# 2 = set_unsupported + helpers → ours  
# 3 = ydotool Hyprland → theirs
# 4 = ydotool KDE → theirs
# 5 = clipboard Hyprland → theirs
# 6 = bluetooth unsupported → theirs
# 7 = KDE requires → ours
# 8 = GNOME requires → ours
# 9 = capabilities init → ours
# 10 = focus event (top of file) → theirs

keep_ours = {2, 7, 8, 9}  # indices where HEAD's version should be kept

# Sort positions by descending file position (bottom-up = first to process)
sorted_positions = sorted(positions, reverse=True)

idx = 0
for start_pos in sorted_positions:
    idx += 1
    
    # Find the divider and end marker
    end_marker = content.find('>>>>>>> origin/main', start_pos)
    if end_marker == -1:
        print(f"  ✗ {idx}: no >>>>>>>> found")
        continue
    
    # Find divider
    divider = content.find('=======\n', start_pos)
    if divider == -1:
        divider = content.find('=======', start_pos)
    if divider == -1 or divider > end_marker:
        print(f"  ✗ {idx}: no ======= found")
        continue
    
    ours = content[start_pos + len('<<<<<<< HEAD\n'):divider]
    theirs = content[divider + len('=======\n'):end_marker]
    
    if idx in keep_ours:
        replacement = ours.rstrip('\n')
        choice = "ours"
    else:
        replacement = theirs.rstrip('\n')
        choice = "theirs"
    
    # Full conflict region
    conflict_end = end_marker + len('>>>>>>> origin/main')
    conflict = content[start_pos:conflict_end]
    
    content = content[:start_pos] + replacement + content[conflict_end:]
    
    first_line = conflict.split('\n')[0]
    print(f"  {idx}: {choice} - {first_line[:50]}")

# Verify
remaining = content.count('<<<<<<< HEAD')
print(f"\nRemaining: {remaining}")

if remaining == 0:
    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)
    print("✓ SAVED!")
else:
    pos = content.find('<<<<<<< HEAD')
    print(f"✗ Still has {remaining} — first at char {pos}")
    print(content[pos:pos+200])
