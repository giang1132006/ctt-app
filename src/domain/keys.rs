//! Strongly typed keyboard and mouse input identifiers.
//!
//! Separates hardware [`ScanCode`] values from logical [`KeyboardKey`] positions
//! and mouse targets ([`MouseButton`], [`MouseWheel`]) with stable string codes
//! for cross-version profile serialization.

use std::fmt;

/// Hardware scan code with extended-key (`0xE0`) disambiguation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScanCode {
    pub code: u16,
    pub extended: bool,
}

impl ScanCode {
    /// Creates a new hardware scan code descriptor.
    pub const fn new(code: u16, extended: bool) -> Self {
        Self { code, extended }
    }
}

impl fmt::Display for ScanCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.extended {
            write!(f, "0x{:02X} (Ext)", self.code)
        } else {
            write!(f, "0x{:02X}", self.code)
        }
    }
}

/// Physical or synthetic key/button transition state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyState {
    Down,
    Up,
}

/// Bitmask of modifier keys held for chord targets or hotkeys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ModifierMask(pub u8);

impl ModifierMask {
    pub const NONE: Self = Self(0);
    pub const CTRL: Self = Self(1 << 0);
    pub const SHIFT: Self = Self(1 << 1);
    pub const ALT: Self = Self(1 << 2);
    pub const META: Self = Self(1 << 3);

    /// Returns `true` if `self` contains all modifier bits set in `other`.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Returns a new mask with all bits of `other` enabled.
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Returns a new mask with all bits of `other` cleared.
    pub const fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    /// Returns `true` when no modifiers are set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Formats active modifiers as a `"Ctrl + Shift + "` prefix string.
    pub fn display_prefix(self) -> String {
        let mut prefix = String::new();
        for (flag, name) in [
            (Self::CTRL, "Ctrl"),
            (Self::SHIFT, "Shift"),
            (Self::ALT, "Alt"),
            (Self::META, "Win"),
        ] {
            if self.contains(flag) {
                prefix.push_str(name);
                prefix.push_str(" + ");
            }
        }
        prefix
    }

    /// Returns the ordered list of canonical modifier keys for press emission.
    pub fn to_modifier_keys(self) -> Vec<KeyboardKey> {
        let mut keys = Vec::with_capacity(4);
        if self.contains(Self::CTRL) {
            keys.push(KeyboardKey::LeftCtrl);
        }
        if self.contains(Self::SHIFT) {
            keys.push(KeyboardKey::LeftShift);
        }
        if self.contains(Self::ALT) {
            keys.push(KeyboardKey::LeftAlt);
        }
        if self.contains(Self::META) {
            keys.push(KeyboardKey::LeftMeta);
        }
        keys
    }

    /// Serializes the modifier mask to a stable symbolic string (e.g. `"CTRL+SHIFT"`).
    pub fn to_serial_code(self) -> String {
        let mut parts = Vec::with_capacity(4);
        if self.contains(Self::CTRL) {
            parts.push("CTRL");
        }
        if self.contains(Self::SHIFT) {
            parts.push("SHIFT");
        }
        if self.contains(Self::ALT) {
            parts.push("ALT");
        }
        if self.contains(Self::META) {
            parts.push("META");
        }
        if parts.is_empty() {
            "NONE".to_string()
        } else {
            parts.join("+")
        }
    }

    /// Parses a symbolic modifier string produced by [`ModifierMask::to_serial_code`].
    pub fn from_serial_code(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("NONE") {
            return Some(Self::NONE);
        }
        let mut mask = Self::NONE;
        for token in trimmed.split('+') {
            match token.trim().to_ascii_uppercase().as_str() {
                "CTRL" => mask = mask.with(Self::CTRL),
                "SHIFT" => mask = mask.with(Self::SHIFT),
                "ALT" => mask = mask.with(Self::ALT),
                "META" | "WIN" => mask = mask.with(Self::META),
                _ => return None,
            }
        }
        Some(mask)
    }
}

/// All bindable mouse buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    XButton1,
    XButton2,
}

impl MouseButton {
    /// Complete slice of all supported mouse buttons.
    pub const ALL: &'static [Self] = &[
        Self::Left,
        Self::Right,
        Self::Middle,
        Self::XButton1,
        Self::XButton2,
    ];

    /// Human-readable label for UI display.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Left => "Mouse Left",
            Self::Right => "Mouse Right",
            Self::Middle => "Mouse Middle",
            Self::XButton1 => "Mouse Back (X1)",
            Self::XButton2 => "Mouse Forward (X2)",
        }
    }

    /// Stable serialization token.
    pub const fn short_code(self) -> &'static str {
        match self {
            Self::Left => "MOUSE_LEFT",
            Self::Right => "MOUSE_RIGHT",
            Self::Middle => "MOUSE_MIDDLE",
            Self::XButton1 => "MOUSE_X1",
            Self::XButton2 => "MOUSE_X2",
        }
    }

    /// Parses a mouse button from its serialization token.
    pub fn from_short_code(code: &str) -> Option<Self> {
        match code.trim() {
            "MOUSE_LEFT" => Some(Self::Left),
            "MOUSE_RIGHT" => Some(Self::Right),
            "MOUSE_MIDDLE" => Some(Self::Middle),
            "MOUSE_X1" => Some(Self::XButton1),
            "MOUSE_X2" => Some(Self::XButton2),
            _ => None,
        }
    }
}

impl fmt::Display for MouseButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

/// All bindable mouse scroll wheel directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseWheel {
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

impl MouseWheel {
    /// Complete slice of all supported wheel directions.
    pub const ALL: &'static [Self] = &[
        Self::ScrollUp,
        Self::ScrollDown,
        Self::ScrollLeft,
        Self::ScrollRight,
    ];

    /// Human-readable label for UI display.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::ScrollUp => "Wheel Up",
            Self::ScrollDown => "Wheel Down",
            Self::ScrollLeft => "Wheel Left",
            Self::ScrollRight => "Wheel Right",
        }
    }

    /// Stable serialization token.
    pub const fn short_code(self) -> &'static str {
        match self {
            Self::ScrollUp => "WHEEL_UP",
            Self::ScrollDown => "WHEEL_DOWN",
            Self::ScrollLeft => "WHEEL_LEFT",
            Self::ScrollRight => "WHEEL_RIGHT",
        }
    }

    /// Parses a mouse wheel direction from its serialization token.
    pub fn from_short_code(code: &str) -> Option<Self> {
        match code.trim() {
            "WHEEL_UP" => Some(Self::ScrollUp),
            "WHEEL_DOWN" => Some(Self::ScrollDown),
            "WHEEL_LEFT" => Some(Self::ScrollLeft),
            "WHEEL_RIGHT" => Some(Self::ScrollRight),
            _ => None,
        }
    }
}

impl fmt::Display for MouseWheel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

/// Exhaustive catalog of every standard keyboard key plus an OEM scan code fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyboardKey {
    // Function Row (25)
    Escape,
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
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    // Number Row (14)
    Grave,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Digit0,
    Minus,
    Equal,
    Backspace,

    // Letters (26)
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,

    // Whitespace & Punctuation (12)
    Tab,
    CapsLock,
    Enter,
    Space,
    LeftBracket,
    RightBracket,
    Backslash,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,

    // Left/Right Modifiers & Menu (9)
    LeftShift,
    RightShift,
    LeftCtrl,
    RightCtrl,
    LeftAlt,
    RightAlt,
    LeftMeta,
    RightMeta,
    ContextMenu,

    // System & Navigation Cluster (9)
    PrintScreen,
    ScrollLock,
    Pause,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,

    // Arrow Cluster (4)
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,

    // Full Numpad (17)
    NumLock,
    NumpadDivide,
    NumpadMultiply,
    NumpadSubtract,
    NumpadAdd,
    NumpadEnter,
    NumpadDecimal,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,

    // Media & Extra Keys (7)
    VolumeMute,
    VolumeDown,
    VolumeUp,
    MediaPlayPause,
    MediaStop,
    MediaNext,
    MediaPrev,

    // Non-standard / OEM fallback
    OemScanCode(ScanCode),
}

struct KeyMetadata {
    key: KeyboardKey,
    short_code: &'static str,
    display_name: &'static str,
    scan_code: u16,
    extended: bool,
}

const STANDARD_KEY_TABLE: &[KeyMetadata] = &[
    // Function Row (25)
    KeyMetadata {
        key: KeyboardKey::Escape,
        short_code: "ESC",
        display_name: "Esc",
        scan_code: 0x01,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F1,
        short_code: "F1",
        display_name: "F1",
        scan_code: 0x3B,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F2,
        short_code: "F2",
        display_name: "F2",
        scan_code: 0x3C,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F3,
        short_code: "F3",
        display_name: "F3",
        scan_code: 0x3D,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F4,
        short_code: "F4",
        display_name: "F4",
        scan_code: 0x3E,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F5,
        short_code: "F5",
        display_name: "F5",
        scan_code: 0x3F,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F6,
        short_code: "F6",
        display_name: "F6",
        scan_code: 0x40,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F7,
        short_code: "F7",
        display_name: "F7",
        scan_code: 0x41,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F8,
        short_code: "F8",
        display_name: "F8",
        scan_code: 0x42,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F9,
        short_code: "F9",
        display_name: "F9",
        scan_code: 0x43,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F10,
        short_code: "F10",
        display_name: "F10",
        scan_code: 0x44,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F11,
        short_code: "F11",
        display_name: "F11",
        scan_code: 0x57,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F12,
        short_code: "F12",
        display_name: "F12",
        scan_code: 0x58,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F13,
        short_code: "F13",
        display_name: "F13",
        scan_code: 0x64,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F14,
        short_code: "F14",
        display_name: "F14",
        scan_code: 0x65,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F15,
        short_code: "F15",
        display_name: "F15",
        scan_code: 0x66,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F16,
        short_code: "F16",
        display_name: "F16",
        scan_code: 0x67,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F17,
        short_code: "F17",
        display_name: "F17",
        scan_code: 0x68,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F18,
        short_code: "F18",
        display_name: "F18",
        scan_code: 0x69,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F19,
        short_code: "F19",
        display_name: "F19",
        scan_code: 0x6A,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F20,
        short_code: "F20",
        display_name: "F20",
        scan_code: 0x6B,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F21,
        short_code: "F21",
        display_name: "F21",
        scan_code: 0x6C,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F22,
        short_code: "F22",
        display_name: "F22",
        scan_code: 0x6D,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F23,
        short_code: "F23",
        display_name: "F23",
        scan_code: 0x6E,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::F24,
        short_code: "F24",
        display_name: "F24",
        scan_code: 0x76,
        extended: false,
    },
    // Number Row (14)
    KeyMetadata {
        key: KeyboardKey::Grave,
        short_code: "GRAVE",
        display_name: "`",
        scan_code: 0x29,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Digit1,
        short_code: "1",
        display_name: "1",
        scan_code: 0x02,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Digit2,
        short_code: "2",
        display_name: "2",
        scan_code: 0x03,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Digit3,
        short_code: "3",
        display_name: "3",
        scan_code: 0x04,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Digit4,
        short_code: "4",
        display_name: "4",
        scan_code: 0x05,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Digit5,
        short_code: "5",
        display_name: "5",
        scan_code: 0x06,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Digit6,
        short_code: "6",
        display_name: "6",
        scan_code: 0x07,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Digit7,
        short_code: "7",
        display_name: "7",
        scan_code: 0x08,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Digit8,
        short_code: "8",
        display_name: "8",
        scan_code: 0x09,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Digit9,
        short_code: "9",
        display_name: "9",
        scan_code: 0x0A,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Digit0,
        short_code: "0",
        display_name: "0",
        scan_code: 0x0B,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Minus,
        short_code: "MINUS",
        display_name: "-",
        scan_code: 0x0C,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Equal,
        short_code: "EQUAL",
        display_name: "=",
        scan_code: 0x0D,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Backspace,
        short_code: "BACKSPACE",
        display_name: "Backspace",
        scan_code: 0x0E,
        extended: false,
    },
    // Letters (26)
    KeyMetadata {
        key: KeyboardKey::KeyA,
        short_code: "A",
        display_name: "A",
        scan_code: 0x1E,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyB,
        short_code: "B",
        display_name: "B",
        scan_code: 0x30,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyC,
        short_code: "C",
        display_name: "C",
        scan_code: 0x2E,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyD,
        short_code: "D",
        display_name: "D",
        scan_code: 0x20,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyE,
        short_code: "E",
        display_name: "E",
        scan_code: 0x12,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyF,
        short_code: "F",
        display_name: "F",
        scan_code: 0x21,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyG,
        short_code: "G",
        display_name: "G",
        scan_code: 0x22,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyH,
        short_code: "H",
        display_name: "H",
        scan_code: 0x23,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyI,
        short_code: "I",
        display_name: "I",
        scan_code: 0x17,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyJ,
        short_code: "J",
        display_name: "J",
        scan_code: 0x24,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyK,
        short_code: "K",
        display_name: "K",
        scan_code: 0x25,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyL,
        short_code: "L",
        display_name: "L",
        scan_code: 0x26,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyM,
        short_code: "M",
        display_name: "M",
        scan_code: 0x32,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyN,
        short_code: "N",
        display_name: "N",
        scan_code: 0x31,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyO,
        short_code: "O",
        display_name: "O",
        scan_code: 0x18,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyP,
        short_code: "P",
        display_name: "P",
        scan_code: 0x19,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyQ,
        short_code: "Q",
        display_name: "Q",
        scan_code: 0x10,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyR,
        short_code: "R",
        display_name: "R",
        scan_code: 0x13,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyS,
        short_code: "S",
        display_name: "S",
        scan_code: 0x1F,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyT,
        short_code: "T",
        display_name: "T",
        scan_code: 0x14,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyU,
        short_code: "U",
        display_name: "U",
        scan_code: 0x16,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyV,
        short_code: "V",
        display_name: "V",
        scan_code: 0x2F,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyW,
        short_code: "W",
        display_name: "W",
        scan_code: 0x11,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyX,
        short_code: "X",
        display_name: "X",
        scan_code: 0x2D,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyY,
        short_code: "Y",
        display_name: "Y",
        scan_code: 0x15,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::KeyZ,
        short_code: "Z",
        display_name: "Z",
        scan_code: 0x2C,
        extended: false,
    },
    // Whitespace & Punctuation (12)
    KeyMetadata {
        key: KeyboardKey::Tab,
        short_code: "TAB",
        display_name: "Tab",
        scan_code: 0x0F,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::CapsLock,
        short_code: "CAPS_LOCK",
        display_name: "Caps Lock",
        scan_code: 0x3A,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Enter,
        short_code: "ENTER",
        display_name: "Enter",
        scan_code: 0x1C,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Space,
        short_code: "SPACE",
        display_name: "Space",
        scan_code: 0x39,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::LeftBracket,
        short_code: "LBRACKET",
        display_name: "[",
        scan_code: 0x1A,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::RightBracket,
        short_code: "RBRACKET",
        display_name: "]",
        scan_code: 0x1B,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Backslash,
        short_code: "BACKSLASH",
        display_name: "\\",
        scan_code: 0x2B,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Semicolon,
        short_code: "SEMICOLON",
        display_name: ";",
        scan_code: 0x27,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Quote,
        short_code: "QUOTE",
        display_name: "'",
        scan_code: 0x28,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Comma,
        short_code: "COMMA",
        display_name: ",",
        scan_code: 0x33,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Period,
        short_code: "PERIOD",
        display_name: ".",
        scan_code: 0x34,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Slash,
        short_code: "SLASH",
        display_name: "/",
        scan_code: 0x35,
        extended: false,
    },
    // Modifiers & Menu (9)
    KeyMetadata {
        key: KeyboardKey::LeftShift,
        short_code: "LSHIFT",
        display_name: "Left Shift",
        scan_code: 0x2A,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::RightShift,
        short_code: "RSHIFT",
        display_name: "Right Shift",
        scan_code: 0x36,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::LeftCtrl,
        short_code: "LCTRL",
        display_name: "Left Ctrl",
        scan_code: 0x1D,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::RightCtrl,
        short_code: "RCTRL",
        display_name: "Right Ctrl",
        scan_code: 0x1D,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::LeftAlt,
        short_code: "LALT",
        display_name: "Left Alt",
        scan_code: 0x38,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::RightAlt,
        short_code: "RALT",
        display_name: "Right Alt",
        scan_code: 0x38,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::LeftMeta,
        short_code: "LMETA",
        display_name: "Left Win",
        scan_code: 0x5B,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::RightMeta,
        short_code: "RMETA",
        display_name: "Right Win",
        scan_code: 0x5C,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::ContextMenu,
        short_code: "MENU",
        display_name: "Menu",
        scan_code: 0x5D,
        extended: true,
    },
    // System & Navigation (9)
    KeyMetadata {
        key: KeyboardKey::PrintScreen,
        short_code: "PRINT_SCREEN",
        display_name: "Print Screen",
        scan_code: 0x37,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::ScrollLock,
        short_code: "SCROLL_LOCK",
        display_name: "Scroll Lock",
        scan_code: 0x46,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Pause,
        short_code: "PAUSE",
        display_name: "Pause",
        scan_code: 0x45,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Insert,
        short_code: "INSERT",
        display_name: "Insert",
        scan_code: 0x52,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::Delete,
        short_code: "DELETE",
        display_name: "Delete",
        scan_code: 0x53,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::Home,
        short_code: "HOME",
        display_name: "Home",
        scan_code: 0x47,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::End,
        short_code: "END",
        display_name: "End",
        scan_code: 0x4F,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::PageUp,
        short_code: "PAGE_UP",
        display_name: "Page Up",
        scan_code: 0x49,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::PageDown,
        short_code: "PAGE_DOWN",
        display_name: "Page Down",
        scan_code: 0x51,
        extended: true,
    },
    // Arrow Cluster (4)
    KeyMetadata {
        key: KeyboardKey::ArrowUp,
        short_code: "UP",
        display_name: "Arrow Up",
        scan_code: 0x48,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::ArrowDown,
        short_code: "DOWN",
        display_name: "Arrow Down",
        scan_code: 0x50,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::ArrowLeft,
        short_code: "LEFT",
        display_name: "Arrow Left",
        scan_code: 0x4B,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::ArrowRight,
        short_code: "RIGHT",
        display_name: "Arrow Right",
        scan_code: 0x4D,
        extended: true,
    },
    // Full Numpad (17)
    KeyMetadata {
        key: KeyboardKey::NumLock,
        short_code: "NUM_LOCK",
        display_name: "Num Lock",
        scan_code: 0x45,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::NumpadDivide,
        short_code: "NUM_DIV",
        display_name: "Numpad /",
        scan_code: 0x35,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::NumpadMultiply,
        short_code: "NUM_MUL",
        display_name: "Numpad *",
        scan_code: 0x37,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::NumpadSubtract,
        short_code: "NUM_SUB",
        display_name: "Numpad -",
        scan_code: 0x4A,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::NumpadAdd,
        short_code: "NUM_ADD",
        display_name: "Numpad +",
        scan_code: 0x4E,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::NumpadEnter,
        short_code: "NUM_ENTER",
        display_name: "Numpad Enter",
        scan_code: 0x1C,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::NumpadDecimal,
        short_code: "NUM_DEC",
        display_name: "Numpad .",
        scan_code: 0x53,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Numpad0,
        short_code: "NUM_0",
        display_name: "Numpad 0",
        scan_code: 0x52,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Numpad1,
        short_code: "NUM_1",
        display_name: "Numpad 1",
        scan_code: 0x4F,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Numpad2,
        short_code: "NUM_2",
        display_name: "Numpad 2",
        scan_code: 0x50,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Numpad3,
        short_code: "NUM_3",
        display_name: "Numpad 3",
        scan_code: 0x51,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Numpad4,
        short_code: "NUM_4",
        display_name: "Numpad 4",
        scan_code: 0x4B,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Numpad5,
        short_code: "NUM_5",
        display_name: "Numpad 5",
        scan_code: 0x4C,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Numpad6,
        short_code: "NUM_6",
        display_name: "Numpad 6",
        scan_code: 0x4D,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Numpad7,
        short_code: "NUM_7",
        display_name: "Numpad 7",
        scan_code: 0x47,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Numpad8,
        short_code: "NUM_8",
        display_name: "Numpad 8",
        scan_code: 0x48,
        extended: false,
    },
    KeyMetadata {
        key: KeyboardKey::Numpad9,
        short_code: "NUM_9",
        display_name: "Numpad 9",
        scan_code: 0x49,
        extended: false,
    },
    // Media Keys (7)
    KeyMetadata {
        key: KeyboardKey::VolumeMute,
        short_code: "VOL_MUTE",
        display_name: "Volume Mute",
        scan_code: 0x20,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::VolumeDown,
        short_code: "VOL_DOWN",
        display_name: "Volume Down",
        scan_code: 0x2E,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::VolumeUp,
        short_code: "VOL_UP",
        display_name: "Volume Up",
        scan_code: 0x30,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::MediaPlayPause,
        short_code: "MEDIA_PLAY_PAUSE",
        display_name: "Media Play/Pause",
        scan_code: 0x22,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::MediaStop,
        short_code: "MEDIA_STOP",
        display_name: "Media Stop",
        scan_code: 0x24,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::MediaNext,
        short_code: "MEDIA_NEXT",
        display_name: "Media Next",
        scan_code: 0x19,
        extended: true,
    },
    KeyMetadata {
        key: KeyboardKey::MediaPrev,
        short_code: "MEDIA_PREV",
        display_name: "Media Prev",
        scan_code: 0x10,
        extended: true,
    },
];

impl KeyboardKey {
    /// Complete ordered catalog of all 123 standard physical keyboard keys.
    pub const ALL_STANDARD: &'static [Self] = &[
        Self::Escape,
        Self::F1,
        Self::F2,
        Self::F3,
        Self::F4,
        Self::F5,
        Self::F6,
        Self::F7,
        Self::F8,
        Self::F9,
        Self::F10,
        Self::F11,
        Self::F12,
        Self::F13,
        Self::F14,
        Self::F15,
        Self::F16,
        Self::F17,
        Self::F18,
        Self::F19,
        Self::F20,
        Self::F21,
        Self::F22,
        Self::F23,
        Self::F24,
        Self::Grave,
        Self::Digit1,
        Self::Digit2,
        Self::Digit3,
        Self::Digit4,
        Self::Digit5,
        Self::Digit6,
        Self::Digit7,
        Self::Digit8,
        Self::Digit9,
        Self::Digit0,
        Self::Minus,
        Self::Equal,
        Self::Backspace,
        Self::KeyA,
        Self::KeyB,
        Self::KeyC,
        Self::KeyD,
        Self::KeyE,
        Self::KeyF,
        Self::KeyG,
        Self::KeyH,
        Self::KeyI,
        Self::KeyJ,
        Self::KeyK,
        Self::KeyL,
        Self::KeyM,
        Self::KeyN,
        Self::KeyO,
        Self::KeyP,
        Self::KeyQ,
        Self::KeyR,
        Self::KeyS,
        Self::KeyT,
        Self::KeyU,
        Self::KeyV,
        Self::KeyW,
        Self::KeyX,
        Self::KeyY,
        Self::KeyZ,
        Self::Tab,
        Self::CapsLock,
        Self::Enter,
        Self::Space,
        Self::LeftBracket,
        Self::RightBracket,
        Self::Backslash,
        Self::Semicolon,
        Self::Quote,
        Self::Comma,
        Self::Period,
        Self::Slash,
        Self::LeftShift,
        Self::RightShift,
        Self::LeftCtrl,
        Self::RightCtrl,
        Self::LeftAlt,
        Self::RightAlt,
        Self::LeftMeta,
        Self::RightMeta,
        Self::ContextMenu,
        Self::PrintScreen,
        Self::ScrollLock,
        Self::Pause,
        Self::Insert,
        Self::Delete,
        Self::Home,
        Self::End,
        Self::PageUp,
        Self::PageDown,
        Self::ArrowUp,
        Self::ArrowDown,
        Self::ArrowLeft,
        Self::ArrowRight,
        Self::NumLock,
        Self::NumpadDivide,
        Self::NumpadMultiply,
        Self::NumpadSubtract,
        Self::NumpadAdd,
        Self::NumpadEnter,
        Self::NumpadDecimal,
        Self::Numpad0,
        Self::Numpad1,
        Self::Numpad2,
        Self::Numpad3,
        Self::Numpad4,
        Self::Numpad5,
        Self::Numpad6,
        Self::Numpad7,
        Self::Numpad8,
        Self::Numpad9,
        Self::VolumeMute,
        Self::VolumeDown,
        Self::VolumeUp,
        Self::MediaPlayPause,
        Self::MediaStop,
        Self::MediaNext,
        Self::MediaPrev,
    ];

    fn metadata(self) -> Option<&'static KeyMetadata> {
        STANDARD_KEY_TABLE.iter().find(|entry| entry.key == self)
    }

    /// Returns a human-readable key label for UI display.
    pub fn display_name(self) -> String {
        match self {
            Self::OemScanCode(sc) => {
                if sc.extended {
                    format!("OEM 0x{:02X} (Ext)", sc.code)
                } else {
                    format!("OEM 0x{:02X}", sc.code)
                }
            }
            _ => self
                .metadata()
                .map(|m| m.display_name.to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
        }
    }

    /// Returns a stable string identifier suitable for persistence and serialization.
    pub fn short_code(self) -> String {
        match self {
            Self::OemScanCode(sc) => {
                format!(
                    "OEM_{:04X}_{}",
                    sc.code,
                    if sc.extended { 'E' } else { 'N' }
                )
            }
            _ => self
                .metadata()
                .map(|m| m.short_code.to_string())
                .unwrap_or_else(|| "UNKNOWN".to_string()),
        }
    }

    /// Parses a [`KeyboardKey`] from its stable string identifier.
    pub fn from_short_code(code: &str) -> Option<Self> {
        let trimmed = code.trim();
        if let Some(rest) = trimmed.strip_prefix("OEM_") {
            let (hex_part, ext_part) = rest.split_once('_')?;
            let scan_code = u16::from_str_radix(hex_part, 16).ok()?;
            let extended = match ext_part {
                "E" => true,
                "N" => false,
                _ => return None,
            };
            return Some(Self::OemScanCode(ScanCode::new(scan_code, extended)));
        }

        STANDARD_KEY_TABLE
            .iter()
            .find(|entry| entry.short_code.eq_ignore_ascii_case(trimmed))
            .map(|entry| entry.key)
    }

    /// Returns the hardware Set-1 [`ScanCode`] associated with this key.
    pub fn default_scancode(self) -> ScanCode {
        match self {
            Self::OemScanCode(sc) => sc,
            _ => self
                .metadata()
                .map(|m| ScanCode::new(m.scan_code, m.extended))
                .unwrap_or(ScanCode::new(0, false)),
        }
    }

    /// Maps a modifier key to its corresponding [`ModifierMask`] bit, or [`ModifierMask::NONE`].
    pub const fn modifier_flag(self) -> ModifierMask {
        match self {
            Self::LeftCtrl | Self::RightCtrl => ModifierMask::CTRL,
            Self::LeftShift | Self::RightShift => ModifierMask::SHIFT,
            Self::LeftAlt | Self::RightAlt => ModifierMask::ALT,
            Self::LeftMeta | Self::RightMeta => ModifierMask::META,
            _ => ModifierMask::NONE,
        }
    }
}

impl fmt::Display for KeyboardKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display_name())
    }
}

/// Any physical trigger input that can be bound (keyboard key or mouse button).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputSource {
    Keyboard(KeyboardKey),
    MouseButton(MouseButton),
}

impl From<KeyboardKey> for InputSource {
    fn from(key: KeyboardKey) -> Self {
        Self::Keyboard(key)
    }
}

impl From<MouseButton> for InputSource {
    fn from(button: MouseButton) -> Self {
        Self::MouseButton(button)
    }
}

impl InputSource {
    /// Returns all standard bindable source inputs (5 mouse buttons followed by 123 keyboard keys).
    pub fn all_standard() -> Vec<Self> {
        let mut all = Vec::with_capacity(MouseButton::ALL.len() + KeyboardKey::ALL_STANDARD.len());
        for &btn in MouseButton::ALL {
            all.push(Self::MouseButton(btn));
        }
        for &key in KeyboardKey::ALL_STANDARD {
            all.push(Self::Keyboard(key));
        }
        all
    }

    /// Returns a human-readable source name for UI display.
    pub fn display_name(self) -> String {
        match self {
            Self::Keyboard(key) => key.display_name(),
            Self::MouseButton(button) => button.display_name().to_string(),
        }
    }

    /// Returns a stable serialization identifier for this source input.
    pub fn short_code(self) -> String {
        match self {
            Self::Keyboard(key) => key.short_code(),
            Self::MouseButton(button) => button.short_code().to_string(),
        }
    }

    /// Parses an [`InputSource`] from its stable serialization token.
    pub fn from_short_code(code: &str) -> Option<Self> {
        if let Some(button) = MouseButton::from_short_code(code) {
            return Some(Self::MouseButton(button));
        }
        KeyboardKey::from_short_code(code).map(Self::Keyboard)
    }

    /// Maps a keyboard modifier source to its [`ModifierMask`], or [`ModifierMask::NONE`].
    pub const fn modifier_flag(self) -> ModifierMask {
        match self {
            Self::Keyboard(key) => key.modifier_flag(),
            Self::MouseButton(_) => ModifierMask::NONE,
        }
    }
}

impl fmt::Display for InputSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn all_standard_keys_have_unique_codes_and_scancodes() {
        assert_eq!(KeyboardKey::ALL_STANDARD.len(), 123);
        assert_eq!(STANDARD_KEY_TABLE.len(), 123);

        let mut short_codes = HashSet::new();
        let mut scancodes = HashSet::new();

        for &key in KeyboardKey::ALL_STANDARD {
            let code = key.short_code();
            assert!(
                short_codes.insert(code.clone()),
                "Duplicate short code detected: {code}"
            );
            assert_eq!(KeyboardKey::from_short_code(&code), Some(key));

            let sc = key.default_scancode();
            assert!(
                scancodes.insert(sc),
                "Duplicate default scancode detected for {key}: {sc}"
            );
        }
    }

    #[test]
    fn oem_scancode_round_trips() {
        let key = KeyboardKey::OemScanCode(ScanCode::new(0x0073, true));
        let serialized = key.short_code();
        assert_eq!(serialized, "OEM_0073_E");
        assert_eq!(KeyboardKey::from_short_code(&serialized), Some(key));
    }

    #[test]
    fn mouse_buttons_and_wheel_round_trip() {
        for &btn in MouseButton::ALL {
            assert_eq!(MouseButton::from_short_code(btn.short_code()), Some(btn));
        }
        for &wheel in MouseWheel::ALL {
            assert_eq!(MouseWheel::from_short_code(wheel.short_code()), Some(wheel));
        }
    }

    #[test]
    fn input_source_catalog_includes_mouse_buttons_and_keyboard_keys() {
        let sources = InputSource::all_standard();
        assert_eq!(sources.len(), 128);
        for &btn in MouseButton::ALL {
            let src = InputSource::MouseButton(btn);
            assert!(sources.contains(&src));
            assert_eq!(InputSource::from_short_code(&src.short_code()), Some(src));
        }
    }
}
