//! Per-compositor a11y selector baselines.
//!
//! Each compositor exports AT-SPI roles differently. GNOME says `push_button`,
//! KDE says `button`, Hyprland might use either. A structural tree test passes
//! because the tree shape didn't change — but `a11y.click_element` silently
//! lands on the parent container instead of the widget.
//!
//! This module asserts on **resolved click targets**: role + name pairs that
//! must exist in the tree. When a compositor remaps a role, the baseline fails
//! loud instead of quietly producing wrong clicks.
//!
//! ## Usage
//!
//! ```rust,ignore
//! let baseline = SelectorBaseline::load("hyprland")?;
//! let tree = snapshot_tree(None, None, None, None).await?;
//! let failures = baseline.assert_against(&tree);
//! assert!(failures.is_empty(), "Baseline failures:\n{}", failures.join("\n"));
//! ```

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single selector assertion — this role + name combination must exist
/// in the accessibility tree for this compositor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorEntry {
    /// AT-SPI role (e.g., "push_button", "menu_item", "text")
    pub role: String,
    /// Accessible name (e.g., "OK", "File", "Search")
    pub name: String,
    /// Human-readable description of what this element is
    pub description: String,
}

/// Per-compositor baseline: a set of selectors that must resolve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorBaseline {
    /// Compositor name (hyprland, gnome, kde, sway, cosmic, niri, wayfire, labwc, x11)
    pub compositor: String,
    /// Compositor version at baseline creation time
    pub version: String,
    /// ISO date of last update
    pub last_updated: String,
    /// Selectors that must exist in the tree
    pub selectors: Vec<SelectorEntry>,
}

/// A single baseline failure — selector didn't resolve, or resolved to a
/// different role than expected.
#[derive(Debug, Clone)]
pub struct BaselineFailure {
    pub selector: SelectorEntry,
    pub reason: String,
}

impl std::fmt::Display for BaselineFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FAIL: role=\"{}\" name=\"{}\" ({}) — {}",
            self.selector.role, self.selector.name, self.selector.description, self.reason
        )
    }
}

impl SelectorBaseline {
    /// Load a baseline from the baselines directory.
    pub fn load(compositor: &str) -> anyhow::Result<Self> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("a11y_baselines")
            .join(format!("{compositor}.json"));
        let content = std::fs::read_to_string(&path)?;
        let baseline: Self = serde_json::from_str(&content)?;
        Ok(baseline)
    }

    /// Assert all selectors exist in the given tree JSON.
    ///
    /// Returns a vec of failures — empty means all selectors resolved.
    /// This is the LOUD failure: we don't just say "tree came back fine",
    /// we say exactly which selector broke and how.
    pub fn assert_against(&self, tree: &serde_json::Value) -> Vec<BaselineFailure> {
        let nodes = extract_nodes(tree);
        let mut failures = Vec::new();

        for selector in &self.selectors {
            // Find all nodes matching role + name
            let matches: Vec<&&serde_json::Value> = nodes
                .iter()
                .filter(|n| {
                    let role_match = n.get("role").and_then(|v| v.as_str()) == Some(&selector.role);
                    let name_match = n
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s == selector.name)
                        .unwrap_or(false);
                    role_match && name_match
                })
                .collect();

            if matches.is_empty() {
                // Check if the name exists under a different role (role remap!)
                let name_only_matches: Vec<&&serde_json::Value> = nodes
                    .iter()
                    .filter(|n| {
                        n.get("name")
                            .and_then(|v| v.as_str())
                            .map(|s| s == selector.name)
                            .unwrap_or(false)
                    })
                    .collect();

                let reason = if !name_only_matches.is_empty() {
                    let found_roles: Vec<&str> = name_only_matches
                        .iter()
                        .filter_map(|n| n.get("role").and_then(|v| v.as_str()))
                        .collect();
                    format!(
                        "name found but role changed: expected role=\"{}\", found role(s) {:?}",
                        selector.role, found_roles
                    )
                } else {
                    "not found in tree".to_string()
                };

                failures.push(BaselineFailure {
                    selector: selector.clone(),
                    reason,
                });
            }
        }

        failures
    }
}

/// Extract the flat list of nodes from a tree JSON value.
///
/// Supports both the AccessibilityNode array format (direct vector)
/// and the nested {"nodes": [...], "root": ...} format.
fn extract_nodes(tree: &serde_json::Value) -> Vec<&serde_json::Value> {
    if let Some(arr) = tree.as_array() {
        return arr.iter().collect();
    }
    if let Some(nodes) = tree.get("nodes").and_then(|v| v.as_array()) {
        return nodes.iter().collect();
    }
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn mock_tree() -> serde_json::Value {
        json!([
            {
                "index": 0,
                "parent_index": null,
                "depth": 0,
                "object_ref": "/org/a11y/atspi/accessible/root",
                "role": "frame",
                "name": "TestApp",
                "child_count": 2,
                "states": ["enabled", "visible"]
            },
            {
                "index": 1,
                "parent_index": 0,
                "depth": 1,
                "object_ref": "/org/a11y/atspi/accessible/1",
                "role": "push_button",
                "name": "OK",
                "child_count": 1,
                "states": ["enabled", "visible", "focusable"]
            },
            {
                "index": 2,
                "parent_index": 0,
                "depth": 1,
                "object_ref": "/org/a11y/atspi/accessible/2",
                "role": "menu_item",
                "name": "File",
                "child_count": 0,
                "states": ["enabled", "visible"]
            },
            {
                "index": 3,
                "parent_index": 1,
                "depth": 2,
                "object_ref": "/org/a11y/atspi/accessible/3",
                "role": "text",
                "name": "OK",
                "child_count": 0,
                "states": ["enabled"]
            }
        ])
    }

    fn test_baseline() -> SelectorBaseline {
        SelectorBaseline {
            compositor: "test".to_string(),
            version: "0.0.0".to_string(),
            last_updated: "2026-06-11".to_string(),
            selectors: vec![
                SelectorEntry {
                    role: "push_button".to_string(),
                    name: "OK".to_string(),
                    description: "Standard dialog OK button".to_string(),
                },
                SelectorEntry {
                    role: "menu_item".to_string(),
                    name: "File".to_string(),
                    description: "File menu item".to_string(),
                },
            ],
        }
    }

    #[test]
    fn all_selectors_resolve() {
        let baseline = test_baseline();
        let tree = mock_tree();
        let failures = baseline.assert_against(&tree);
        assert!(
            failures.is_empty(),
            "Expected no failures, got: {failures:#?}"
        );
    }

    #[test]
    fn missing_selector_fails() {
        let baseline = test_baseline();
        // Tree without the "File" menu item
        let tree = json!([
            {
                "index": 0,
                "role": "frame",
                "name": "TestApp",
                "child_count": 1,
                "states": ["enabled"]
            },
            {
                "index": 1,
                "role": "push_button",
                "name": "OK",
                "child_count": 0,
                "states": ["enabled"]
            }
        ]);
        let failures = baseline.assert_against(&tree);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].reason.contains("not found"));
    }

    #[test]
    fn role_remap_detected() {
        // Simulates GNOME changing push_button → button between versions
        let tree = json!([
            {
                "index": 0,
                "role": "frame",
                "name": "TestApp",
                "child_count": 2,
                "states": ["enabled"]
            },
            {
                "index": 1,
                "role": "button",
                "name": "OK",
                "child_count": 0,
                "states": ["enabled", "visible", "focusable"]
            },
            {
                "index": 2,
                "role": "menu_item",
                "name": "File",
                "child_count": 0,
                "states": ["enabled"]
            }
        ]);

        let failures = test_baseline().assert_against(&tree);
        assert_eq!(failures.len(), 1, "Expected exactly 1 role remap failure");
        assert!(
            failures[0].reason.contains("role changed"),
            "Should detect role remap, got: {}",
            failures[0].reason
        );
        assert!(failures[0].reason.contains("button"));
    }

    #[test]
    fn nested_nodes_format_supported() {
        let baseline = SelectorBaseline {
            compositor: "test".to_string(),
            version: "0.0.0".to_string(),
            last_updated: "2026-06-11".to_string(),
            selectors: vec![SelectorEntry {
                role: "push_button".to_string(),
                name: "OK".to_string(),
                description: "OK button".to_string(),
            }],
        };

        // Nested format: {"nodes": [...], "root": ...}
        let tree = json!({
            "nodes": [
                {
                    "index": 0,
                    "role": "frame",
                    "name": "TestApp",
                    "states": ["enabled"]
                },
                {
                    "index": 1,
                    "role": "push_button",
                    "name": "OK",
                    "states": ["enabled"]
                }
            ],
            "root": {"index": 0}
        });

        let failures = baseline.assert_against(&tree);
        assert!(failures.is_empty());
    }
}
