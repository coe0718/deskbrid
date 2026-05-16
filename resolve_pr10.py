#!/usr/bin/env python3
"""Resolve ALL merge conflicts in PR #10's daemon.rs."""
import re

path = "/home/coemedia/projects/deskbrid/src/daemon.rs"
with open(path, 'rb') as f:
    content = f.read().decode('utf-8')

# Find all conflict positions (bottom-up for safe replacement)
positions = [(m.start(), '<<<<<<< HEAD') for m in re.finditer(r'<<<<<<< HEAD', content)]
n = len(positions)
print(f"Found {n} conflicts")

# Bottom-up numbering: last conflict in file = #1
# Strategy by position:
keep = {}  # conflict_number -> 'ours' or 'theirs'

# I'll decide based on what I know about each section.
# Let me check what each conflict is about by examining it.

sorted_pos = sorted(positions, reverse=True)
for idx, (start_pos, _) in enumerate(sorted_pos, 1):
    end_marker = content.find('>>>>>>> origin/main', start_pos)
    if end_marker == -1:
        print(f"  ✗ {idx}: no >>>>>>>> found")
        continue
    
    # Find the section type by looking at nearby content
    snippet = content[start_pos:start_pos+200]
    first_ours = snippet.split('\n')[0] if '\n' in snippet else snippet
    first_lines = [l.strip() for l in snippet.split('\n')[:5]]
    
    # Determine what kind of conflict this is
    if any('Action::WindowsFocus' in l for l in first_lines):
        decide = 'theirs'  # focus event: keep main's resolved ID
    elif any('Action::WindowsGet(' in l for l in first_lines):
        decide = 'theirs'  # windows_get action: check
    elif any('"reason": serde_json::Value::Null' in l for l in first_lines):
        decide = 'ours'  # capabilities init format: keep HEAD's extended
    elif any('set_requires(' in l for l in first_lines):
        decide = 'ours'  # requires/session calls: keep HEAD's
    elif any('"bluetooth.pair"' in l for l in first_lines):
        decide = 'theirs'  # bluetooth unsupported: keep main's
    elif any('wl_clipboard' in l for l in first_lines):
        decide = 'theirs'  # clipboard: keep main's check_clipboard_tools
    elif any('ydotool' in l for l in first_lines):
        decide = 'theirs'  # ydotool checks: keep main's
    elif any('"supported": true, "degraded": true' in l for l in first_lines):
        decide = 'ours'  # set_degraded format: keep HEAD's extended
    elif any('"supported": false, "degraded": false' in l for l in first_lines):
        decide = 'ours'  # set_unsupported format: keep HEAD's extended
    elif any('fn check_clipboard_tools' in l for l in first_lines):
        decide = 'theirs'  # clipboard_tools function: keep main's
    elif any('Action::FilesWatch' in l for l in first_lines):
        # This is in the execute_action match - probably window_get from PR #8 style fix
        decide = 'theirs'
    else:
        # Let me check more context
        context = content[start_pos:start_pos+400]
        if 'set_requires' in context or 'set_session' in context:
            decide = 'ours'
        elif 'bluetooth.pair' in context or 'bluetooth.forget' in context:
            if '=======\n' in context:
                lines = context.split('\n')
                mid = [i for i, l in enumerate(lines) if l == '======='][0]
                after = [l for l in lines[mid+1:mid+5] if l.strip() and not l.startswith('>>>>')]
                if any('bluetooth' in l for l in after):
                    decide = 'theirs'
                else:
                    decide = 'theirs'
            else:
                decide = 'theirs'
        elif 'check_clipboard_tools' in context:
            decide = 'theirs'
        elif 'wd = ' in context or 'WindowGet' in context or 'window_get' in context:
            decide = 'theirs'
        else:
            print(f"  ? {idx}: UNKNOWN TYPE - checking...")
            print(f"    First lines: {first_lines[:3]}")
            # Check both sides
            divider = content.find('=======\n', start_pos)
            if divider != -1 and divider < end_marker:
                theirs = content[divider+8:divider+200]
                print(f"    Theirs: {theirs[:100]}")
            decide = 'theirs'  # default to theirs for safety
    
    # Now resolve the conflict
    divider = content.find('=======\n', start_pos)
    if divider == -1:
        divider = content.find('=======', start_pos)
    if divider == -1 or divider > end_marker:
        print(f"  ✗ {idx}: no ======= found")
        continue
    
    ours = content[start_pos + 13:divider].rstrip('\n')
    theirs = content[divider + 8:end_marker].rstrip('\n')
    
    if decide == 'ours':
        replacement = ours
    else:
        replacement = theirs
    
    conflict_end = end_marker + 19  # len('>>>>>>> origin/main')
    content = content[:start_pos] + replacement + content[conflict_end:]
    
    # Log what we did
    marker = ''
    if 'Action::WindowsFocus' in first_ours[:100]:
        marker = 'focus event'
    elif 'set_requires' in first_ours[:100]:
        marker = 'requires/session calls'
    elif '"reason": serde_json::Value::Null' in first_ours[:100]:
        marker = 'capabilities format'
    elif 'bluetooth.pair' in first_ours[:100]:
        marker = 'bluetooth unsupported'
    elif 'wl_clipboard' in first_ours[:100]:
        marker = 'clipboard check'
    elif 'ydotool' in first_ours[:100]:
        marker = 'ydotool check'
    elif 'degraded' in first_ours[:100]:
        marker = 'set_degraded format'
    elif 'check_clipboard_tools' in first_ours[:100]:
        marker = 'clipboard_tools fn'
    elif 'fn set_requires' in first_ours[:100] or 'fn set_session' in first_ours[:100]:
        marker = 'set_requires/set_session defs'
    elif 'FilesWatch' in first_ours[:100] or 'FilesUnwatch' in first_ours[:100]:
        marker = 'files actions'
    elif 'WindowGet' in first_ours[:100]:
        marker = 'window_get action'
    else:
        marker = first_ours[:50]
    
    print(f"  ✓ {idx}: {decide} ({marker})")

# Verify no remaining
remaining = content.count('<<<<<<< HEAD')
print(f"\nRemaining: {remaining}")
if remaining == 0:
    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)
    print("✓ SAVED!")
else:
    pos = content.find('<<<<<<< HEAD')
    print(f"✗ First remaining at char {pos}")
    print(content[pos:pos+200])
