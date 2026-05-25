use crate::protocol::KeyboardLayout;

use super::CosmicBackend;

impl CosmicBackend {
    /// Run gsettings get
    async fn gsettings_get(&self, schema: &str, key: &str) -> anyhow::Result<String> {
        self.sh("gsettings", &["get", schema, key]).await
    }

    /// Run gsettings set
    async fn gsettings_set(&self, schema: &str, key: &str, value: &str) -> anyhow::Result<()> {
        self.sh("gsettings", &["set", schema, key, value]).await?;
        Ok(())
    }

    /// Parse GNOME/COSMIC input sources GVariant: [('xkb', 'us'), ('xkb', 'ru')]
    fn parse_sources(raw: &str) -> Vec<KeyboardLayout> {
        let mut layouts = Vec::new();
        let mut i = 0u32;
        let trimmed = raw.trim();
        let chars: Vec<char> = trimmed.chars().collect();
        let mut pos = 0;
        while pos < chars.len() {
            while pos < chars.len() && chars[pos] != '(' {
                pos += 1;
            }
            if pos >= chars.len() {
                break;
            }
            pos += 1; // skip '('
            let name = Self::read_gvariant_string(&chars, &mut pos);
            pos += 1; // skip ','
            let layout_str = Self::read_gvariant_string(&chars, &mut pos);
            while pos < chars.len() && chars[pos] != ')' {
                pos += 1;
            }
            pos += 1; // skip ')'

            if !name.is_empty() && !layout_str.is_empty() {
                layouts.push(KeyboardLayout {
                    index: i,
                    name: layout_str,
                    variant: None,
                    display_name: Some(name),
                });
                i += 1;
            }
        }
        layouts
    }

    fn read_gvariant_string(chars: &[char], pos: &mut usize) -> String {
        while *pos < chars.len() && chars[*pos] != '\'' && chars[*pos] != '"' {
            *pos += 1;
        }
        if *pos >= chars.len() {
            return String::new();
        }
        let quote = chars[*pos];
        *pos += 1;
        let mut result = String::new();
        while *pos < chars.len() && chars[*pos] != quote {
            result.push(chars[*pos]);
            *pos += 1;
        }
        *pos += 1;
        result
    }

    pub(super) async fn keyboard_layout_list_inner(&self) -> anyhow::Result<Vec<KeyboardLayout>> {
        let raw = self
            .gsettings_get("org.gnome.desktop.input-sources", "sources")
            .await?;
        Ok(Self::parse_sources(&raw))
    }

    pub(super) async fn keyboard_layout_get_inner(&self) -> anyhow::Result<KeyboardLayout> {
        let raw = self
            .gsettings_get("org.gnome.desktop.input-sources", "current")
            .await?;
        let current: u32 = raw
            .split_whitespace()
            .last()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let layouts = self.keyboard_layout_list_inner().await?;
        layouts
            .into_iter()
            .find(|l| l.index == current)
            .ok_or_else(|| anyhow::anyhow!("current layout index {} out of range", current))
    }

    pub(super) async fn keyboard_layout_set_inner(
        &self,
        index: Option<u32>,
        _name: Option<&str>,
        _variant: Option<&str>,
    ) -> anyhow::Result<()> {
        if let Some(idx) = index {
            self.gsettings_set(
                "org.gnome.desktop.input-sources",
                "current",
                &idx.to_string(),
            )
            .await
        } else {
            anyhow::bail!("COSMIC layout switching requires an index")
        }
    }

    pub(super) async fn keyboard_layout_add_inner(
        &self,
        name: &str,
        variant: Option<&str>,
    ) -> anyhow::Result<()> {
        let layouts = self.keyboard_layout_list_inner().await?;
        let source = match variant {
            Some(v) => format!("('xkb', '{}+{}')", name, v),
            None => format!("('xkb', '{}')", name),
        };
        let mut parts: Vec<String> = layouts
            .iter()
            .map(|l| format!("('xkb', '{}')", l.name))
            .collect();
        parts.push(source);
        let value = format!("[{}]", parts.join(", "));
        self.gsettings_set("org.gnome.desktop.input-sources", "sources", &value)
            .await
    }

    pub(super) async fn keyboard_layout_remove_inner(&self, index: u32) -> anyhow::Result<()> {
        let layouts = self.keyboard_layout_list_inner().await?;
        let parts: Vec<String> = layouts
            .iter()
            .filter(|l| l.index != index)
            .map(|l| format!("('xkb', '{}')", l.name))
            .collect();
        let value = format!("[{}]", parts.join(", "));
        self.gsettings_set("org.gnome.desktop.input-sources", "sources", &value)
            .await
    }
}
