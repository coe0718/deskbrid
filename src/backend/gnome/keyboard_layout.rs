use super::GnomeBackend;
use crate::protocol::KeyboardLayout;

impl GnomeBackend {
    /// Run gsettings and return stdout
    async fn gsettings_get(&self, schema: &str, key: &str) -> anyhow::Result<String> {
        self.sh("gsettings", &["get", schema, key]).await
    }

    /// Run gsettings set
    async fn gsettings_set(&self, schema: &str, key: &str, value: &str) -> anyhow::Result<()> {
        self.sh("gsettings", &["set", schema, key, value]).await?;
        Ok(())
    }

    /// Parse GNOME input sources GVariant: [('xkb', 'us'), ('xkb', 'ru')]
    fn parse_sources(raw: &str) -> Vec<KeyboardLayout> {
        let mut layouts = Vec::new();
        let mut i = 0u32;
        let trimmed = raw.trim();
        // Walk through GVariant tuple pairs: ('xkb', 'us')
        let mut pos = 0;
        let chars: Vec<char> = trimmed.chars().collect();
        while pos < chars.len() {
            // Skip until '('
            while pos < chars.len() && chars[pos] != '(' {
                pos += 1;
            }
            if pos >= chars.len() {
                break;
            }
            pos += 1; // skip '('
            // Read type string inside first quote
            let name = Self::read_gvariant_string(&chars, &mut pos);
            pos += 1; // skip ','
            // Read layout string inside second quote
            let layout_str = Self::read_gvariant_string(&chars, &mut pos);
            // Skip past ')'
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
        *pos += 1; // skip opening quote
        let mut result = String::new();
        while *pos < chars.len() && chars[*pos] != quote {
            result.push(chars[*pos]);
            *pos += 1;
        }
        *pos += 1; // skip closing quote
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
        let current: u32 = raw.trim().parse().unwrap_or(0);
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
            anyhow::bail!("GNOME layout switching requires an index")
        }
    }

    pub(super) async fn keyboard_layout_add_inner(
        &self,
        name: &str,
        variant: Option<&str>,
    ) -> anyhow::Result<()> {
        let layouts = self.keyboard_layout_list_inner().await?;
        // Build new GVariant source string
        let source = match variant {
            Some(v) => format!("('xkb', '{}'+'{}')", name, v),
            None => format!("('xkb', '{}')", name),
        };
        // Rebuild GVariant array from existing layouts
        let mut parts: Vec<String> = Vec::new();
        for l in &layouts {
            parts.push(format!("('xkb', '{}')", l.name));
        }
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
