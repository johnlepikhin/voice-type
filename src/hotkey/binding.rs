//! Hotkey binding: modifiers, keys, parsing, and display.

/// Modifier keys that can be combined with a hotkey.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Modifiers {
    /// Ctrl (left or right).
    pub ctrl: bool,
    /// Alt (left or right).
    pub alt: bool,
    /// Shift (left or right).
    pub shift: bool,
    /// Super / Meta / Win (left or right).
    pub super_: bool,
}

/// Platform-agnostic key representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Key {
    // ── Letters ──────────────────────────────────────────────────────────
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    // ── Digits ───────────────────────────────────────────────────────────
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    // ── Function keys ────────────────────────────────────────────────────
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    // ── Special keys ────────────────────────────────────────────────────
    Space,
    Tab,
    Escape,
    Backspace,
    ScrollLock,
    Pause,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
}

impl Key {
    /// Parse a key name (case-insensitive).
    ///
    /// # Errors
    /// Returns an error string if the key name is not recognized.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            // Letters
            "A" => Ok(Self::A),
            "B" => Ok(Self::B),
            "C" => Ok(Self::C),
            "D" => Ok(Self::D),
            "E" => Ok(Self::E),
            "F" => Ok(Self::F),
            "G" => Ok(Self::G),
            "H" => Ok(Self::H),
            "I" => Ok(Self::I),
            "J" => Ok(Self::J),
            "K" => Ok(Self::K),
            "L" => Ok(Self::L),
            "M" => Ok(Self::M),
            "N" => Ok(Self::N),
            "O" => Ok(Self::O),
            "P" => Ok(Self::P),
            "Q" => Ok(Self::Q),
            "R" => Ok(Self::R),
            "S" => Ok(Self::S),
            "T" => Ok(Self::T),
            "U" => Ok(Self::U),
            "V" => Ok(Self::V),
            "W" => Ok(Self::W),
            "X" => Ok(Self::X),
            "Y" => Ok(Self::Y),
            "Z" => Ok(Self::Z),
            // Digits
            "0" => Ok(Self::Num0),
            "1" => Ok(Self::Num1),
            "2" => Ok(Self::Num2),
            "3" => Ok(Self::Num3),
            "4" => Ok(Self::Num4),
            "5" => Ok(Self::Num5),
            "6" => Ok(Self::Num6),
            "7" => Ok(Self::Num7),
            "8" => Ok(Self::Num8),
            "9" => Ok(Self::Num9),
            // Function keys
            "F1" => Ok(Self::F1),
            "F2" => Ok(Self::F2),
            "F3" => Ok(Self::F3),
            "F4" => Ok(Self::F4),
            "F5" => Ok(Self::F5),
            "F6" => Ok(Self::F6),
            "F7" => Ok(Self::F7),
            "F8" => Ok(Self::F8),
            "F9" => Ok(Self::F9),
            "F10" => Ok(Self::F10),
            "F11" => Ok(Self::F11),
            "F12" => Ok(Self::F12),
            // Special keys
            "SPACE" => Ok(Self::Space),
            "TAB" => Ok(Self::Tab),
            "ESCAPE" | "ESC" => Ok(Self::Escape),
            "BACKSPACE" => Ok(Self::Backspace),
            "SCROLLLOCK" | "SCROLL_LOCK" => Ok(Self::ScrollLock),
            "PAUSE" => Ok(Self::Pause),
            "INSERT" => Ok(Self::Insert),
            "HOME" => Ok(Self::Home),
            "END" => Ok(Self::End),
            "PAGEUP" | "PAGE_UP" => Ok(Self::PageUp),
            "PAGEDOWN" | "PAGE_DOWN" => Ok(Self::PageDown),
            "DELETE" | "DEL" => Ok(Self::Delete),
            _ => Err(format!("Unknown key: {s}")),
        }
    }

    /// Convert to the corresponding `evdev::Key`.
    #[must_use]
    pub fn to_evdev(self) -> evdev::Key {
        match self {
            Self::A => evdev::Key::KEY_A,
            Self::B => evdev::Key::KEY_B,
            Self::C => evdev::Key::KEY_C,
            Self::D => evdev::Key::KEY_D,
            Self::E => evdev::Key::KEY_E,
            Self::F => evdev::Key::KEY_F,
            Self::G => evdev::Key::KEY_G,
            Self::H => evdev::Key::KEY_H,
            Self::I => evdev::Key::KEY_I,
            Self::J => evdev::Key::KEY_J,
            Self::K => evdev::Key::KEY_K,
            Self::L => evdev::Key::KEY_L,
            Self::M => evdev::Key::KEY_M,
            Self::N => evdev::Key::KEY_N,
            Self::O => evdev::Key::KEY_O,
            Self::P => evdev::Key::KEY_P,
            Self::Q => evdev::Key::KEY_Q,
            Self::R => evdev::Key::KEY_R,
            Self::S => evdev::Key::KEY_S,
            Self::T => evdev::Key::KEY_T,
            Self::U => evdev::Key::KEY_U,
            Self::V => evdev::Key::KEY_V,
            Self::W => evdev::Key::KEY_W,
            Self::X => evdev::Key::KEY_X,
            Self::Y => evdev::Key::KEY_Y,
            Self::Z => evdev::Key::KEY_Z,
            Self::Num0 => evdev::Key::KEY_0,
            Self::Num1 => evdev::Key::KEY_1,
            Self::Num2 => evdev::Key::KEY_2,
            Self::Num3 => evdev::Key::KEY_3,
            Self::Num4 => evdev::Key::KEY_4,
            Self::Num5 => evdev::Key::KEY_5,
            Self::Num6 => evdev::Key::KEY_6,
            Self::Num7 => evdev::Key::KEY_7,
            Self::Num8 => evdev::Key::KEY_8,
            Self::Num9 => evdev::Key::KEY_9,
            Self::F1 => evdev::Key::KEY_F1,
            Self::F2 => evdev::Key::KEY_F2,
            Self::F3 => evdev::Key::KEY_F3,
            Self::F4 => evdev::Key::KEY_F4,
            Self::F5 => evdev::Key::KEY_F5,
            Self::F6 => evdev::Key::KEY_F6,
            Self::F7 => evdev::Key::KEY_F7,
            Self::F8 => evdev::Key::KEY_F8,
            Self::F9 => evdev::Key::KEY_F9,
            Self::F10 => evdev::Key::KEY_F10,
            Self::F11 => evdev::Key::KEY_F11,
            Self::F12 => evdev::Key::KEY_F12,
            Self::Space => evdev::Key::KEY_SPACE,
            Self::Tab => evdev::Key::KEY_TAB,
            Self::Escape => evdev::Key::KEY_ESC,
            Self::Backspace => evdev::Key::KEY_BACKSPACE,
            Self::ScrollLock => evdev::Key::KEY_SCROLLLOCK,
            Self::Pause => evdev::Key::KEY_PAUSE,
            Self::Insert => evdev::Key::KEY_INSERT,
            Self::Home => evdev::Key::KEY_HOME,
            Self::End => evdev::Key::KEY_END,
            Self::PageUp => evdev::Key::KEY_PAGEUP,
            Self::PageDown => evdev::Key::KEY_PAGEDOWN,
            Self::Delete => evdev::Key::KEY_DELETE,
        }
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
            Self::F => "F",
            Self::G => "G",
            Self::H => "H",
            Self::I => "I",
            Self::J => "J",
            Self::K => "K",
            Self::L => "L",
            Self::M => "M",
            Self::N => "N",
            Self::O => "O",
            Self::P => "P",
            Self::Q => "Q",
            Self::R => "R",
            Self::S => "S",
            Self::T => "T",
            Self::U => "U",
            Self::V => "V",
            Self::W => "W",
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
            Self::Num0 => "0",
            Self::Num1 => "1",
            Self::Num2 => "2",
            Self::Num3 => "3",
            Self::Num4 => "4",
            Self::Num5 => "5",
            Self::Num6 => "6",
            Self::Num7 => "7",
            Self::Num8 => "8",
            Self::Num9 => "9",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
            Self::F11 => "F11",
            Self::F12 => "F12",
            Self::Space => "Space",
            Self::Tab => "Tab",
            Self::Escape => "Escape",
            Self::Backspace => "Backspace",
            Self::ScrollLock => "ScrollLock",
            Self::Pause => "Pause",
            Self::Insert => "Insert",
            Self::Home => "Home",
            Self::End => "End",
            Self::PageUp => "PageUp",
            Self::PageDown => "PageDown",
            Self::Delete => "Delete",
        };
        f.write_str(name)
    }
}

/// A parsed hotkey: modifiers + trigger key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedHotkey {
    /// The trigger key.
    pub key: Key,
    /// Active modifiers.
    pub modifiers: Modifiers,
}

impl std::fmt::Display for ParsedHotkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let m = &self.modifiers;
        if m.ctrl {
            f.write_str("Ctrl+")?;
        }
        if m.alt {
            f.write_str("Alt+")?;
        }
        if m.shift {
            f.write_str("Shift+")?;
        }
        if m.super_ {
            f.write_str("Super+")?;
        }
        write!(f, "{}", self.key)
    }
}

/// Parse a hotkey string like `"Super+F8"` or `"Ctrl+Alt+Delete"`.
///
/// Modifier names are case-insensitive. Supported modifiers:
/// `Ctrl`/`Control`, `Alt`, `Shift`, `Super`/`Meta`/`Win`.
///
/// # Errors
/// Returns an error string if parsing fails.
pub fn parse_hotkey(s: &str) -> Result<ParsedHotkey, String> {
    let parts: Vec<&str> = s.split('+').collect();

    if parts.is_empty() || (parts.len() == 1 && parts[0].is_empty()) {
        return Err("Empty hotkey string".to_owned());
    }

    let mut modifiers = Modifiers::default();

    for part in &parts[..parts.len() - 1] {
        match part.to_uppercase().as_str() {
            "SHIFT" => modifiers.shift = true,
            "CTRL" | "CONTROL" => modifiers.ctrl = true,
            "ALT" => modifiers.alt = true,
            "SUPER" | "META" | "WIN" => modifiers.super_ = true,
            _ => return Err(format!("Unknown modifier: {part}")),
        }
    }

    let key_str = parts[parts.len() - 1];
    let key = Key::parse(key_str)?;

    Ok(ParsedHotkey { key, modifiers })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_key() {
        let hk = parse_hotkey("F8").unwrap();
        assert_eq!(hk.key, Key::F8);
        assert_eq!(hk.modifiers, Modifiers::default());
    }

    #[test]
    fn parse_shift_modifier() {
        let hk = parse_hotkey("Shift+F8").unwrap();
        assert!(hk.modifiers.shift);
        assert!(!hk.modifiers.super_);
        assert_eq!(hk.key, Key::F8);
    }

    #[test]
    fn parse_super_modifier() {
        let hk = parse_hotkey("Super+F8").unwrap();
        assert!(hk.modifiers.super_);
        assert!(!hk.modifiers.shift);
        assert_eq!(hk.key, Key::F8);
    }

    #[test]
    fn parse_meta_alias() {
        let hk = parse_hotkey("Meta+F1").unwrap();
        assert!(hk.modifiers.super_);
    }

    #[test]
    fn parse_win_alias() {
        let hk = parse_hotkey("Win+F1").unwrap();
        assert!(hk.modifiers.super_);
    }

    #[test]
    fn parse_multiple_modifiers() {
        let hk = parse_hotkey("Ctrl+Alt+F1").unwrap();
        assert!(hk.modifiers.ctrl);
        assert!(hk.modifiers.alt);
        assert!(!hk.modifiers.shift);
        assert!(!hk.modifiers.super_);
        assert_eq!(hk.key, Key::F1);
    }

    #[test]
    fn parse_all_modifiers() {
        let hk = parse_hotkey("Ctrl+Alt+Shift+Super+F5").unwrap();
        assert!(hk.modifiers.ctrl);
        assert!(hk.modifiers.alt);
        assert!(hk.modifiers.shift);
        assert!(hk.modifiers.super_);
        assert_eq!(hk.key, Key::F5);
    }

    #[test]
    fn parse_case_insensitive() {
        let hk = parse_hotkey("SHIFT+f8").unwrap();
        assert!(hk.modifiers.shift);
        assert_eq!(hk.key, Key::F8);
    }

    #[test]
    fn parse_new_keys() {
        assert_eq!(parse_hotkey("Home").unwrap().key, Key::Home);
        assert_eq!(parse_hotkey("End").unwrap().key, Key::End);
        assert_eq!(parse_hotkey("PageUp").unwrap().key, Key::PageUp);
        assert_eq!(parse_hotkey("Delete").unwrap().key, Key::Delete);
    }

    #[test]
    fn parse_letter_keys() {
        assert_eq!(parse_hotkey("Super+I").unwrap().key, Key::I);
        assert_eq!(parse_hotkey("Ctrl+A").unwrap().key, Key::A);
        assert_eq!(parse_hotkey("Super+V").unwrap().key, Key::V);
    }

    #[test]
    fn parse_digit_keys() {
        assert_eq!(parse_hotkey("Super+1").unwrap().key, Key::Num1);
        assert_eq!(parse_hotkey("Ctrl+0").unwrap().key, Key::Num0);
    }

    #[test]
    fn parse_unknown_key() {
        assert!(parse_hotkey("Unknown").is_err());
        assert!(parse_hotkey("Ctrl+???").is_err());
    }

    #[test]
    fn parse_unknown_modifier() {
        assert!(parse_hotkey("Hyper+F8").is_err());
    }

    #[test]
    fn parse_empty() {
        assert!(parse_hotkey("").is_err());
    }

    #[test]
    fn display_simple() {
        let hk = parse_hotkey("F8").unwrap();
        assert_eq!(hk.to_string(), "F8");
    }

    #[test]
    fn display_with_super() {
        let hk = parse_hotkey("Super+F8").unwrap();
        assert_eq!(hk.to_string(), "Super+F8");
    }

    #[test]
    fn display_all_modifiers() {
        let hk = parse_hotkey("Ctrl+Alt+Shift+Super+F1").unwrap();
        assert_eq!(hk.to_string(), "Ctrl+Alt+Shift+Super+F1");
    }

    #[test]
    fn evdev_key_mapping() {
        assert_eq!(Key::F8.to_evdev(), evdev::Key::KEY_F8);
        assert_eq!(Key::Delete.to_evdev(), evdev::Key::KEY_DELETE);
        assert_eq!(Key::Home.to_evdev(), evdev::Key::KEY_HOME);
    }
}
