# Accessibility

Read the AT-SPI/accessibility tree and perform element-level actions on the focused or selected application.

Use cases: inspect UI state without screenshots, target buttons by name, automate accessibility-aware workflows.

## Protocol

`a11y.tree` returns an accessibility tree. `a11y.do` performs an action on an accessibility element by name.

## Requirements

- AT-SPI is required for tree inspection and element actions.
- If `a11y.tree` returns no results, the application may not expose an accessibility interface.

## Actions

- `a11y.tree`
- `a11y.do`

## Notes

- Some apps expose partial accessibility trees.
- Deeply nested trees may be truncated by the requested depth.
- Accessibility elements are identified by accessible name and role.
