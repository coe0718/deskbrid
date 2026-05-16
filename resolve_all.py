#!/usr/bin/env python3
"""Read daemon.rs, resolve all 11 merge conflicts, save."""
path = "/home/coemedia/projects/deskbrid/src/daemon.rs"

# Read file raw (don't interpret escape sequences)
with open(path, 'rb') as f:
    raw = f.read()
content = raw.decode('utf-8')

# Define ALL replacements as literal strings
# I'll identify each by its position in the file
conflicts = []

# Strategy: find each conflict region by position, decide which side to keep
keep_ours = {2, 3, 4, 9, 10}  # HEAD's version (extended capabilities fields, requires/session, set_degraded format, set_unsupported format)
keep_theirs = {1, 5, 6, 7, 8, 11}  # origin/main version (focus event, bluetooth unsupported, clipboard, ydotool, clipboard_tools fn)

import re

# Find all conflict start positions
positions = [(m.start(), '<<<<<<< HEAD') for m in re.finditer(r'<<<<<<< HEAD', content)]

print(f"Found {len(positions)} conflict markers")

# For each conflict, resolve it
# We work from bottom up to preserve positions
idx = 0
results = []
for start_pos, _ in sorted(positions, reverse=True):
    idx += 1
    # Find the conflict boundaries
    end_marker = content.find('>>>>>>> origin/main', start_pos)
    if end_marker == -1:
        print(f"  ✗ Conflict {idx}: no '>>>>>>>' found")
        continue
    
    # Find the divider
    divider = content.find('=======\n', start_pos)
    if divider == -1:
        divider = content.find('=======', start_pos)
    if divider == -1 or divider > end_marker:
        print(f"  ✗ Conflict {idx}: no '=======' found")
        continue
    
    # Extract both sides
    ours_start = start_pos + len('<<<<<<< HEAD\n')
    ours_end = divider
    theirs_start = divider + len('=======\n')
    theirs_end = end_marker
    
    ours = content[ours_start:divider]
    theirs = content[theirs_start:end_marker]
    
    # Remove trailing newline from divider side
    if ours.endswith('\n'):
        ours = ours.rstrip('\n')
    if theirs.startswith('\n'):
        theirs = theirs.lstrip('\n')
    
    # Decide which side to keep (odd = conflict number)
    if idx in keep_ours:
        replacement = ours
        choice = "ours"
    else:
        replacement = theirs
        choice = "theirs"
    
    # Build the full conflict string to replace
    conflict_str = content[start_pos:end_marker + len('>>>>>>> origin/main')]
    
    results.append((idx, choice, conflict_str[:60]))
    
    # Replace - but also clean up the trailing newline after the end marker
    after_marker = content[end_marker + len('>>>>>>> origin/main'):]
    if after_marker.startswith('\n'):
        replacement_end = replacement + after_marker
    else:
        replacement_end = replacement
    
    content = content[:start_pos] + replacement + content[end_marker + len('>>>>>>> origin/main'):]

print(f"\nResolved {len(results)} conflicts:")
for idx, choice, snippet in sorted(results):
    print(f"  {idx}: {choice} - {snippet}")

# Verify no remaining
remaining = content.count('<<<<<<< HEAD')
print(f"\nRemaining conflict markers: {remaining}")

if remaining == 0:
    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)
    print("✓ FILE SAVED!")
else:
    print("✗ Still have conflicts!")
