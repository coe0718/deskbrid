# A11y Selector Baselines

Per-compositor baselines of role+name selectors that **must** exist in the
accessibility tree. Used by `src/a11y/selector_baseline.rs` to catch silent
role remapping bugs — when a compositor quietly changes `push_button` → `button`
and tests stay green while clicks land on the wrong element.

## How to populate

1. Start Deskbrid daemon on target compositor
2. Open a standard GTK/Qt app (gedit, kate, etc.)
3. Capture the tree:
   ```bash
   deskbrid a11y tree > tests/a11y_baselines/hyprland_capture.json
   ```
4. Extract key selectors (common UI elements that agents click):
   - Dialog buttons: OK, Cancel, Apply
   - Menu items: File, Edit, View, Help
   - Input fields: Search, Name, URL
   - Widgets: checkboxes, combo boxes, sliders

## Adding a compositor

Copy an existing baseline, update `compositor` and `version`, and populate
selectors from a real tree capture on that desktop.

## Why not structural tests?

Structural tests ("tree came back with N nodes") pass even when a role silently
changes. A click on `push_button` that becomes `button` will still "succeed" —
it just clicks the parent container instead. The selector baseline fails LOUD
when the expected role+name pair doesn't exist, and tells you what it found instead.
