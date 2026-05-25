use super::LabwcBackend;
use crate::protocol::KeyboardLayout;
use std::path::PathBuf;

/// Path to the labwc environment file for keyboard layout configuration.
fn env_file() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    PathBuf::from(home)
        .join(".config")
        .join("labwc")
        .join("environment")
}

/// Parse XKB_DEFAULT_LAYOUT from environment file content.
/// Format: XKB_DEFAULT_LAYOUT=us,se(dvorak)
fn parse_layouts(content: &str) -> Vec<KeyboardLayout> {
    for line in content.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("XKB_DEFAULT_LAYOUT=") {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if value.is_empty() {
                return vec![];
            }
            return value
                .split(',')
                .enumerate()
                .map(|(i, s)| {
                    let s = s.trim();
                    // Handle layout(variant) syntax
                    if let Some((name, variant)) = s.split_once('(') {
                        KeyboardLayout {
                            index: i as u32,
                            name: name.to_string(),
                            variant: Some(variant.trim_end_matches(')').to_string()),
                            display_name: None,
                        }
                    } else {
                        KeyboardLayout {
                            index: i as u32,
                            name: s.to_string(),
                            variant: None,
                            display_name: None,
                        }
                    }
                })
                .collect();
        }
    }
    vec![]
}

/// Parse the active layout index from XKB_DEFAULT_OPTIONS (grp: toggle tracking)
/// Returns 0 by default since we can't track runtime state.
fn parse_active_index(content: &str) -> u32 {
    for line in content.lines() {
        if line.trim().starts_with("XKB_DEFAULT_OPTIONS=") {
            // We can't track which layout is actually active at runtime,
            // but we know the first layout starts active on compositor start.
            return 0;
        }
    }
    0
}

impl LabwcBackend {
    /// Read the current environment file content.
    fn read_env_file(&self) -> anyhow::Result<String> {
        let path = env_file();
        if path.exists() {
            Ok(std::fs::read_to_string(&path)?)
        } else {
            Ok(String::new())
        }
    }

    /// Write environment file, preserving non-layout lines.
    fn write_env_file(
        &self,
        layouts: &[KeyboardLayout],
        options: Option<&str>,
    ) -> anyhow::Result<()> {
        let path = env_file();
        let existing = if path.exists() {
            std::fs::read_to_string(&path).unwrap_or_default()
        } else {
            String::new()
        };

        // Preserve all non-layout lines
        let mut lines: Vec<String> = existing
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("XKB_DEFAULT_LAYOUT=")
                    && !trimmed.starts_with("XKB_DEFAULT_OPTIONS=")
                    && !trimmed.starts_with('#')
                    && !trimmed.is_empty()
            })
            .map(|s| s.to_string())
            .collect();

        // Remove any leading blank lines from the filtered set
        while lines.first().is_some_and(|l| l.trim().is_empty()) {
            lines.remove(0);
        }

        // Add layout
        let layout_str: Vec<String> = layouts
            .iter()
            .map(|l| {
                if let Some(ref v) = l.variant {
                    format!("{}({})", l.name, v)
                } else {
                    l.name.clone()
                }
            })
            .collect();
        lines.push(format!("XKB_DEFAULT_LAYOUT={}", layout_str.join(",")));

        // Add options if present
        if let Some(opts) = options {
            lines.push(format!("XKB_DEFAULT_OPTIONS={}", opts));
        }

        // Ensure parent directory
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, lines.join("\n") + "\n")?;
        Ok(())
    }

    pub(super) async fn keyboard_layout_list(&self) -> anyhow::Result<Vec<KeyboardLayout>> {
        let content = self.read_env_file()?;
        Ok(parse_layouts(&content))
    }

    pub(super) async fn keyboard_layout_get(&self) -> anyhow::Result<KeyboardLayout> {
        let content = self.read_env_file()?;
        let layouts = parse_layouts(&content);
        let active = parse_active_index(&content);
        layouts
            .into_iter()
            .find(|l| l.index == active)
            .or_else(|| {
                // Fallback: return first layout
                parse_layouts(&content).into_iter().next()
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No keyboard layouts configured. Set XKB_DEFAULT_LAYOUT in {}",
                    env_file().display()
                )
            })
    }

    pub(super) async fn keyboard_layout_set(
        &self,
        index: Option<u32>,
        name: Option<&str>,
        variant: Option<&str>,
    ) -> anyhow::Result<()> {
        let content = self.read_env_file()?;
        let mut layouts = parse_layouts(&content);

        if let Some(idx) = index {
            // Set by index: make that layout first in the list
            let idx = idx as usize;
            if idx < layouts.len() {
                let selected = layouts.remove(idx);
                layouts.insert(0, selected);
            }
        } else if let Some(name) = name {
            // Set by name: find it and make it first, or add it
            let pos = layouts.iter().position(|l| l.name == name);
            if let Some(idx) = pos {
                let selected = layouts.remove(idx);
                layouts.insert(0, selected);
            } else {
                layouts.insert(
                    0,
                    KeyboardLayout {
                        index: 0,
                        name: name.to_string(),
                        variant: variant.map(|v| v.to_string()),
                        display_name: None,
                    },
                );
            }
            // Update variant if specified
            if let Some(v) = variant {
                layouts[0].variant = Some(v.to_string());
            }
        }

        // Re-index
        for (i, layout) in layouts.iter_mut().enumerate() {
            layout.index = i as u32;
        }

        // Preserve existing options
        let options = parse_options(&content);
        self.write_env_file(&layouts, options.as_deref())?;

        tracing::info!(
            "Keyboard layouts updated. Restart labwc ('labwc -e' then re-login) to apply."
        );
        Ok(())
    }

    pub(super) async fn keyboard_layout_add(
        &self,
        name: &str,
        variant: Option<&str>,
    ) -> anyhow::Result<()> {
        let content = self.read_env_file()?;
        let mut layouts = parse_layouts(&content);

        // Don't add duplicates
        if layouts
            .iter()
            .any(|l| l.name == name && l.variant.as_deref() == variant)
        {
            return Ok(());
        }

        let idx = layouts.len() as u32;
        layouts.push(KeyboardLayout {
            index: idx,
            name: name.to_string(),
            variant: variant.map(|v| v.to_string()),
            display_name: None,
        });

        let options = parse_options(&content);
        self.write_env_file(&layouts, options.as_deref())?;
        tracing::info!("Keyboard layout added. Restart labwc to apply.");
        Ok(())
    }

    pub(super) async fn keyboard_layout_remove(&self, index: u32) -> anyhow::Result<()> {
        let content = self.read_env_file()?;
        let mut layouts = parse_layouts(&content);

        if index as usize >= layouts.len() {
            anyhow::bail!(
                "layout index {} out of range ({} layouts)",
                index,
                layouts.len()
            );
        }
        if layouts.len() <= 1 {
            anyhow::bail!("cannot remove the last keyboard layout");
        }

        layouts.remove(index as usize);

        // Re-index
        for (i, layout) in layouts.iter_mut().enumerate() {
            layout.index = i as u32;
        }

        let options = parse_options(&content);
        self.write_env_file(&layouts, options.as_deref())?;
        tracing::info!("Keyboard layout removed. Restart labwc to apply.");
        Ok(())
    }
}

fn parse_options(content: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(value) = line.trim().strip_prefix("XKB_DEFAULT_OPTIONS=") {
            return Some(
                value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string(),
            );
        }
    }
    None
}
