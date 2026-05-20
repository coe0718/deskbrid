/// XKB keysyms for common characters and keys.
/// Reference: /usr/include/X11/keysymdef.h

// Modifier keys
pub const SHIFT_L: u32 = 0xFFE1;
pub const CTRL_L: u32 = 0xFFE3;
pub const ALT_L: u32 = 0xFFE9;
pub const SUPER_L: u32 = 0xFFEB;

// Special keys
pub const RETURN: u32 = 0xFF0D;
pub const TAB: u32 = 0xFF09;
pub const ESCAPE: u32 = 0xFF1B;
pub const BACKSPACE: u32 = 0xFF08;
pub const DELETE: u32 = 0xFFFF;
pub const UP: u32 = 0xFF52;
pub const DOWN: u32 = 0xFF54;
pub const LEFT: u32 = 0xFF51;
pub const RIGHT: u32 = 0xFF53;
pub const HOME: u32 = 0xFF50;
pub const END: u32 = 0xFF57;
pub const PAGE_UP: u32 = 0xFF55;
pub const PAGE_DOWN: u32 = 0xFF56;
pub const SPACE: u32 = 0x0020;

/// Map a key name string to its XKB keysym.
pub fn from_name(name: &str) -> Option<u32> {
    Some(match name.to_lowercase().as_str() {
        "return" | "enter" => RETURN,
        "tab" => TAB,
        "escape" | "esc" => ESCAPE,
        "backspace" => BACKSPACE,
        "delete" | "del" => DELETE,
        "up" => UP,
        "down" => DOWN,
        "left" => LEFT,
        "right" => RIGHT,
        "home" => HOME,
        "end" => END,
        "page_up" | "pgup" => PAGE_UP,
        "page_down" | "pgdn" => PAGE_DOWN,
        "space" => SPACE,
        // Modifier names
        "shift" | "shift_l" => SHIFT_L,
        "ctrl" | "control" | "control_l" => CTRL_L,
        "alt" | "alt_l" => ALT_L,
        "super" | "super_l" | "meta" | "win" | "windows" => SUPER_L,
        _ => return None,
    })
}

/// Map a printable ASCII character to its XKB keysym.
/// Returns (keysym, needs_shift).
pub fn from_char(c: char) -> Option<(u32, bool)> {
    match c {
        'a'..='z' => Some((0x0061 + (c as u32 - 'a' as u32), false)),
        'A'..='Z' => Some((0x0061 + (c as u32 - 'A' as u32), true)),
        '0'..='9' => Some((0x0030 + (c as u32 - '0' as u32), false)),
        ' ' => Some((0x0020, false)),
        '.' => Some((0x002E, false)),
        ',' => Some((0x002C, false)),
        ';' => Some((0x003B, false)),
        ':' => Some((0x003B, true)),
        '\'' => Some((0x0027, false)),
        '"' => Some((0x0027, true)),
        '/' => Some((0x002F, false)),
        '?' => Some((0x002F, true)),
        '\\' => Some((0x005C, false)),
        '|' => Some((0x005C, true)),
        '[' => Some((0x005B, false)),
        '{' => Some((0x005B, true)),
        ']' => Some((0x005D, false)),
        '}' => Some((0x005D, true)),
        '-' => Some((0x002D, false)),
        '_' => Some((0x002D, true)),
        '=' => Some((0x003D, false)),
        '+' => Some((0x003D, true)),
        '`' => Some((0x0060, false)),
        '~' => Some((0x0060, true)),
        '!' => Some((0x0031, true)),
        '@' => Some((0x0032, true)),
        '#' => Some((0x0033, true)),
        '$' => Some((0x0034, true)),
        '%' => Some((0x0035, true)),
        '^' => Some((0x0036, true)),
        '&' => Some((0x0037, true)),
        '*' => Some((0x0038, true)),
        '(' => Some((0x0039, true)),
        ')' => Some((0x0030, true)),
        '\n' => Some((RETURN, false)),
        '\t' => Some((TAB, false)),
        _ => None,
    }
}
