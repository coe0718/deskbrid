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
//! ## Auto-normalization
//!
//! The real trap with baselines is the same one screenshot diffs have: a legit
//! role rename and an actual regression look identical. This module solves that
//! by defining **semantic role groups** — sets of AT-SPI roles that are
//! functionally equivalent (e.g., `push_button`, `button`, `toggle_button`
//! all represent clickable buttons). When a role remaps within the same group,
//! it's classified as a `Normalization` and auto-applied. Cross-group changes
//! are flagged as `Regressions` requiring human review.
//!
//! ## Usage
//!
//! ```rust,ignore
//! let baseline = SelectorBaseline::load("hyprland")?;
//! let tree = snapshot_tree(None, None, None, None).await?;
//! let assessment = baseline.assess(&tree);
//!
//! // Auto-fixes within same semantic group — ship it.
//! if !assessment.normalizations.is_empty() {
//!     println!("{} auto-normalizations applied", assessment.normalizations.len());
//!     baseline.apply_normalizations(&assessment.normalizations);
//!     baseline.save()?;
//! }
//!
//! // Cross-group changes — these need a human.
//! for regression in &assessment.regressions {
//!     eprintln!("REGRESSION: {regression}");
//! }
//! assert!(assessment.regressions.is_empty(), "Needs human review");
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

// ─── Role Groups ───────────────────────────────────────────────────────────

/// A named group of semantically equivalent AT-SPI roles.
///
/// When a role remaps within the same group (e.g., `push_button` → `button`),
/// it's a normalization. When it jumps groups (e.g., `push_button` → `text`),
/// it's a regression that needs human review.
#[derive(Debug, Clone)]
struct RoleGroup {
    name: &'static str,
    roles: &'static [&'static str],
}

/// All defined semantic role groups.
///
/// These map canonical AT-SPI role names to groups of equivalent roles.
/// If a role isn't in any group, it defaults to its own singleton group.
static ROLE_GROUPS: LazyLock<Vec<RoleGroup>> = LazyLock::new(|| {
    vec![
        RoleGroup {
            name: "button",
            roles: &[
                "push_button",
                "button",
                "toggle_button",
                "menu_button",
                "split_button",
            ],
        },
        RoleGroup {
            name: "text_container",
            roles: &["text", "paragraph", "label", "heading", "section"],
        },
        RoleGroup {
            name: "toggle",
            roles: &["check_box", "checkbox", "toggle", "radio_button", "switch"],
        },
        RoleGroup {
            name: "selector",
            roles: &["combo_box", "dropdown", "select", "list_box", "popup_menu"],
        },
        RoleGroup {
            name: "menu_entry",
            roles: &[
                "menu_item",
                "menuitem",
                "check_menu_item",
                "radio_menu_item",
            ],
        },
        RoleGroup {
            name: "input",
            roles: &["entry", "text_box", "input", "search_box", "spin_button"],
        },
        RoleGroup {
            name: "container",
            roles: &[
                "frame",
                "panel",
                "window",
                "dialog",
                "page",
                "grouping",
                "scroll_pane",
            ],
        },
        RoleGroup {
            name: "table_cell",
            roles: &[
                "table_cell",
                "cell",
                "grid_cell",
                "row_header",
                "column_header",
            ],
        },
        RoleGroup {
            name: "slider",
            roles: &["slider", "scroll_bar", "level_bar", "progress_bar"],
        },
    ]
});

/// Build a lookup map: role → group name.
fn role_group_map() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    for group in ROLE_GROUPS.iter() {
        for role in group.roles {
            map.insert(*role, group.name);
        }
    }
    map
}

/// Return the semantic group for a role, defaulting to the role itself
/// if not in any defined group.
fn semantic_group(role: &str) -> &str {
    static MAP: LazyLock<HashMap<&str, &str>> = LazyLock::new(role_group_map);
    MAP.get(role).copied().unwrap_or(role)
}

// ─── Data Types ────────────────────────────────────────────────────────────

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

/// Classification of a baseline discrepancy.
#[derive(Debug, Clone)]
pub enum Assessment {
    /// Role remapped within the same semantic group — auto-fixable.
    /// e.g., `push_button` → `button` (both in "button" group)
    Normalization {
        selector: SelectorEntry,
        found_role: String,
        group: String,
    },
    /// Role changed to a different semantic group, or element missing entirely.
    /// Needs a human to decide if this is a real regression or a new baseline.
    Regression {
        selector: SelectorEntry,
        reason: String,
    },
}

/// Result of assessing a tree against a baseline.
#[derive(Debug, Clone)]
pub struct AssessmentResult {
    /// Normalizations that can be auto-applied (same semantic group).
    pub normalizations: Vec<Assessment>,
    /// Regressions that need human review (cross-group change or missing element).
    pub regressions: Vec<Assessment>,
}

impl AssessmentResult {
    /// True when there are no failures at all — baseline matches perfectly.
    pub fn is_clean(&self) -> bool {
        self.normalizations.is_empty() && self.regressions.is_empty()
    }

    /// True when all failures are auto-fixable normalizations.
    pub fn is_auto_fixable(&self) -> bool {
        self.regressions.is_empty()
    }
}

impl std::fmt::Display for Assessment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Assessment::Normalization {
                selector,
                found_role,
                group,
            } => {
                write!(
                    f,
                    "NORM: role=\"{}\" → \"{}\" name=\"{}\" (group: {}) — {}",
                    selector.role, found_role, selector.name, group, selector.description
                )
            }
            Assessment::Regression { selector, reason } => {
                write!(
                    f,
                    "REGRESSION: role=\"{}\" name=\"{}\" ({}) — {}",
                    selector.role, selector.name, selector.description, reason
                )
            }
        }
    }
}

// ─── Core Logic ────────────────────────────────────────────────────────────

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

    /// Save the baseline back to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("a11y_baselines")
            .join(format!("{}.json", self.compositor));
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content + "\n")?;
        Ok(())
    }

    /// Assess the tree against this baseline, classifying every discrepancy
    /// as a Normalization (auto-fixable) or Regression (needs human).
    ///
    /// Use this instead of `assert_against()` when you want automatic
    /// baseline maintenance. Call `apply_normalizations()` after to
    /// auto-fix roles within the same semantic group.
    pub fn assess(&self, tree: &serde_json::Value) -> AssessmentResult {
        let nodes = extract_nodes(tree);
        let mut normalizations = Vec::new();
        let mut regressions = Vec::new();

        for selector in &self.selectors {
            // Look for exact role + name match
            let exact_match = nodes.iter().any(|n| {
                n.get("role").and_then(|v| v.as_str()) == Some(&selector.role)
                    && n.get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s == selector.name)
                        .unwrap_or(false)
            });

            if exact_match {
                continue; // Still good
            }

            // Look for name match under any role
            let name_matches: Vec<&&serde_json::Value> = nodes
                .iter()
                .filter(|n| {
                    n.get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s == selector.name)
                        .unwrap_or(false)
                })
                .collect();

            if name_matches.is_empty() {
                // Completely gone — regression
                regressions.push(Assessment::Regression {
                    selector: selector.clone(),
                    reason: "not found in tree".to_string(),
                });
                continue;
            }

            // Name found — check if the new role is in the same semantic group
            let expected_group = semantic_group(&selector.role);
            let found_roles: Vec<&str> = name_matches
                .iter()
                .filter_map(|n| n.get("role").and_then(|v| v.as_str()))
                .collect();

            // Use the first matching role that's in the same group, or fall
            // back to checking if ANY of the found roles share the group.
            let same_group_role = found_roles
                .iter()
                .find(|r| semantic_group(r) == expected_group);

            if let Some(found_role) = same_group_role {
                // Same semantic group — auto-normalize
                normalizations.push(Assessment::Normalization {
                    selector: selector.clone(),
                    found_role: (*found_role).to_string(),
                    group: expected_group.to_string(),
                });
            } else {
                // Different group — needs human
                regressions.push(Assessment::Regression {
                    selector: selector.clone(),
                    reason: format!(
                        "role changed across semantic groups: expected group=\"{}\" role=\"{}\", found role(s) {:?}",
                        expected_group, selector.role, found_roles
                    ),
                });
            }
        }

        AssessmentResult {
            normalizations,
            regressions,
        }
    }

    /// Apply normalizations in-place: update the role in each entry to the
    /// found role. Only call with normalizations from `assess()`.
    pub fn apply_normalizations(&mut self, normalizations: &[Assessment]) {
        for norm in normalizations {
            if let Assessment::Normalization {
                selector,
                found_role,
                ..
            } = norm
            {
                for entry in &mut self.selectors {
                    if entry.role == selector.role && entry.name == selector.name {
                        entry.role = found_role.clone();
                        break;
                    }
                }
            }
        }
    }

    // ─── Legacy API (pre-normalization) ────────────────────────────────────

    /// Assert all selectors exist in the given tree JSON.
    ///
    /// Returns a vec of failures — empty means all selectors resolved.
    /// This is the LOUD failure: we don't just say "tree came back fine",
    /// we say exactly which selector broke and how.
    ///
    /// Prefer `assess()` for new code — it distinguishes normalizations
    /// from regressions.
    #[deprecated(since = "1.0.1", note = "use `assess()` instead")]
    pub fn assert_against(&self, tree: &serde_json::Value) -> Vec<String> {
        let assessment = self.assess(tree);
        let mut failures: Vec<String> = Vec::new();

        for norm in &assessment.normalizations {
            failures.push(format!("NORMALIZATION (auto-fixable): {}", norm));
        }
        for regression in &assessment.regressions {
            failures.push(regression.to_string());
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

// ─── Tests ─────────────────────────────────────────────────────────────────

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

    // ─── Role Group Tests ──────────────────────────────────────────────

    #[test]
    fn same_group_roles_share_semantic_group() {
        assert_eq!(semantic_group("push_button"), "button");
        assert_eq!(semantic_group("button"), "button");
        assert_eq!(semantic_group("toggle_button"), "button");
    }

    #[test]
    fn different_group_roles_dont_share() {
        assert_eq!(semantic_group("push_button"), "button");
        assert_eq!(semantic_group("text"), "text_container");
        assert_ne!(semantic_group("push_button"), semantic_group("text"));
    }

    #[test]
    fn unknown_role_uses_itself_as_group() {
        assert_eq!(semantic_group("some_obscure_role"), "some_obscure_role");
    }

    // ─── Assessment Tests ─────────────────────────────────────────────

    #[test]
    fn clean_tree_passes_all() {
        let baseline = test_baseline();
        let tree = mock_tree();
        let assessment = baseline.assess(&tree);
        assert!(assessment.is_clean());
        assert!(assessment.is_auto_fixable());
    }

    #[test]
    fn missing_element_is_regression() {
        let baseline = test_baseline();
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
        let assessment = baseline.assess(&tree);
        assert_eq!(assessment.regressions.len(), 1);
        assert!(assessment.regressions[0].to_string().contains("not found"));
    }

    #[test]
    fn same_group_remap_is_normalization() {
        // push_button → button: both in "button" group
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

        let assessment = test_baseline().assess(&tree);
        assert_eq!(
            assessment.normalizations.len(),
            1,
            "Should be 1 normalization"
        );
        assert!(assessment.regressions.is_empty(), "No regressions expected");
        assert!(assessment.is_auto_fixable());

        let norm = &assessment.normalizations[0];
        let norm_str = norm.to_string();
        assert!(norm_str.contains("NORM"));
        assert!(norm_str.contains("push_button"));
        assert!(norm_str.contains("button"));
    }

    #[test]
    fn cross_group_remap_is_regression() {
        // push_button → text: button group → text_container group
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
                "role": "text",
                "name": "OK",
                "child_count": 0,
                "states": ["enabled"]
            },
            {
                "index": 2,
                "role": "menu_item",
                "name": "File",
                "child_count": 0,
                "states": ["enabled"]
            }
        ]);

        let assessment = test_baseline().assess(&tree);
        assert!(
            assessment.normalizations.is_empty(),
            "Cross-group change should NOT be a normalization"
        );
        assert_eq!(assessment.regressions.len(), 1);
        assert!(!assessment.is_auto_fixable());
        assert!(
            assessment.regressions[0]
                .to_string()
                .contains("across semantic groups")
        );
    }

    #[test]
    fn toggle_button_in_button_group_is_normalization() {
        // toggle_button is in the "button" group with push_button
        let baseline = test_baseline();
        // The OK button is push_button in the baseline
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
                "role": "toggle_button",
                "name": "OK",
                "child_count": 0,
                "states": ["enabled"]
            },
            {
                "index": 2,
                "role": "menu_item",
                "name": "File",
                "child_count": 0,
                "states": ["enabled"]
            }
        ]);

        let assessment = baseline.assess(&tree);
        assert_eq!(
            assessment.normalizations.len(),
            1,
            "toggle_button in button group should normalize"
        );
    }

    #[test]
    fn apply_normalizations_updates_roles() {
        let mut baseline = test_baseline();
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

        let assessment = baseline.assess(&tree);
        baseline.apply_normalizations(&assessment.normalizations);

        // OK button should now expect "button" instead of "push_button"
        let ok_entry = baseline.selectors.iter().find(|e| e.name == "OK").unwrap();
        assert_eq!(ok_entry.role, "button");

        // Re-assess should be clean
        let reassessment = baseline.assess(&tree);
        assert!(
            reassessment.is_clean(),
            "After normalization, should be clean"
        );
    }

    // ─── Legacy API Tests ─────────────────────────────────────────────

    #[test]
    fn assert_against_clean() {
        let baseline = test_baseline();
        let tree = mock_tree();
        #[allow(deprecated)]
        let failures = baseline.assert_against(&tree);
        assert!(failures.is_empty());
    }

    #[test]
    fn assert_against_detects_remap() {
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

        #[allow(deprecated)]
        let failures = test_baseline().assert_against(&tree);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("NORMALIZATION"));
        assert!(failures[0].contains("button"));
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

        let assessment = baseline.assess(&tree);
        assert!(assessment.is_clean());
    }
}
