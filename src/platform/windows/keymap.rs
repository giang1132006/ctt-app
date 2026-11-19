//! Bidirectional Win32 virtual key / scan code mapping to domain [`KeyboardKey`].

use crate::domain::keys::{KeyboardKey, ScanCode};

/// Windows key identity combining Virtual-Key code, hardware scan code, and `LLKHF_EXTENDED`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Win32KeyCode {
    pub vk: u16,
    pub scan_code: u16,
    pub extended: bool,
}

impl Win32KeyCode {
    pub const fn new(vk: u16, scan_code: u16, extended: bool) -> Self {
        Self {
            vk,
            scan_code,
            extended,
        }
    }
}

struct Win32MappingEntry {
    key: KeyboardKey,
    vk: u16,
    scan_code: u16,
    extended: bool,
}

const WIN32_KEY_TABLE: &[Win32MappingEntry] = &[
    // Function Row
    Win32MappingEntry {
        key: KeyboardKey::Escape,
        vk: 0x1B,
        scan_code: 0x01,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F1,
        vk: 0x70,
        scan_code: 0x3B,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F2,
        vk: 0x71,
        scan_code: 0x3C,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F3,
        vk: 0x72,
        scan_code: 0x3D,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F4,
        vk: 0x73,
        scan_code: 0x3E,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F5,
        vk: 0x74,
        scan_code: 0x3F,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F6,
        vk: 0x75,
        scan_code: 0x40,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F7,
        vk: 0x76,
        scan_code: 0x41,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F8,
        vk: 0x77,
        scan_code: 0x42,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F9,
        vk: 0x78,
        scan_code: 0x43,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F10,
        vk: 0x79,
        scan_code: 0x44,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F11,
        vk: 0x7A,
        scan_code: 0x57,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F12,
        vk: 0x7B,
        scan_code: 0x58,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F13,
        vk: 0x7C,
        scan_code: 0x64,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F14,
        vk: 0x7D,
        scan_code: 0x65,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F15,
        vk: 0x7E,
        scan_code: 0x66,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F16,
        vk: 0x7F,
        scan_code: 0x67,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F17,
        vk: 0x80,
        scan_code: 0x68,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F18,
        vk: 0x81,
        scan_code: 0x69,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F19,
        vk: 0x82,
        scan_code: 0x6A,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F20,
        vk: 0x83,
        scan_code: 0x6B,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F21,
        vk: 0x84,
        scan_code: 0x6C,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F22,
        vk: 0x85,
        scan_code: 0x6D,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F23,
        vk: 0x86,
        scan_code: 0x6E,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::F24,
        vk: 0x87,
        scan_code: 0x76,
        extended: false,
    },
    // Number Row
    Win32MappingEntry {
        key: KeyboardKey::Grave,
        vk: 0xC0,
        scan_code: 0x29,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Digit1,
        vk: 0x31,
        scan_code: 0x02,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Digit2,
        vk: 0x32,
        scan_code: 0x03,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Digit3,
        vk: 0x33,
        scan_code: 0x04,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Digit4,
        vk: 0x34,
        scan_code: 0x05,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Digit5,
        vk: 0x35,
        scan_code: 0x06,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Digit6,
        vk: 0x36,
        scan_code: 0x07,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Digit7,
        vk: 0x37,
        scan_code: 0x08,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Digit8,
        vk: 0x38,
        scan_code: 0x09,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Digit9,
        vk: 0x39,
        scan_code: 0x0A,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Digit0,
        vk: 0x30,
        scan_code: 0x0B,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Minus,
        vk: 0xBD,
        scan_code: 0x0C,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Equal,
        vk: 0xBB,
        scan_code: 0x0D,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Backspace,
        vk: 0x08,
        scan_code: 0x0E,
        extended: false,
    },
    // Letters
    Win32MappingEntry {
        key: KeyboardKey::KeyA,
        vk: 0x41,
        scan_code: 0x1E,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyB,
        vk: 0x42,
        scan_code: 0x30,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyC,
        vk: 0x43,
        scan_code: 0x2E,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyD,
        vk: 0x44,
        scan_code: 0x20,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyE,
        vk: 0x45,
        scan_code: 0x12,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyF,
        vk: 0x46,
        scan_code: 0x21,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyG,
        vk: 0x47,
        scan_code: 0x22,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyH,
        vk: 0x48,
        scan_code: 0x23,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyI,
        vk: 0x49,
        scan_code: 0x17,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyJ,
        vk: 0x4A,
        scan_code: 0x24,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyK,
        vk: 0x4B,
        scan_code: 0x25,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyL,
        vk: 0x4C,
        scan_code: 0x26,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyM,
        vk: 0x4D,
        scan_code: 0x32,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyN,
        vk: 0x4E,
        scan_code: 0x31,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyO,
        vk: 0x4F,
        scan_code: 0x18,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyP,
        vk: 0x50,
        scan_code: 0x19,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyQ,
        vk: 0x51,
        scan_code: 0x10,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyR,
        vk: 0x52,
        scan_code: 0x13,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyS,
        vk: 0x53,
        scan_code: 0x1F,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyT,
        vk: 0x54,
        scan_code: 0x14,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyU,
        vk: 0x55,
        scan_code: 0x16,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyV,
        vk: 0x56,
        scan_code: 0x2F,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyW,
        vk: 0x57,
        scan_code: 0x11,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyX,
        vk: 0x58,
        scan_code: 0x2D,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyY,
        vk: 0x59,
        scan_code: 0x15,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::KeyZ,
        vk: 0x5A,
        scan_code: 0x2C,
        extended: false,
    },
    // Whitespace & Punctuation
    Win32MappingEntry {
        key: KeyboardKey::Tab,
        vk: 0x09,
        scan_code: 0x0F,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::CapsLock,
        vk: 0x14,
        scan_code: 0x3A,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Enter,
        vk: 0x0D,
        scan_code: 0x1C,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Space,
        vk: 0x20,
        scan_code: 0x39,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::LeftBracket,
        vk: 0xDB,
        scan_code: 0x1A,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::RightBracket,
        vk: 0xDD,
        scan_code: 0x1B,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Backslash,
        vk: 0xDC,
        scan_code: 0x2B,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Semicolon,
        vk: 0xBA,
        scan_code: 0x27,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Quote,
        vk: 0xDE,
        scan_code: 0x28,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Comma,
        vk: 0xBC,
        scan_code: 0x33,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Period,
        vk: 0xBE,
        scan_code: 0x34,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Slash,
        vk: 0xBF,
        scan_code: 0x35,
        extended: false,
    },
    // Modifiers & Menu
    Win32MappingEntry {
        key: KeyboardKey::LeftShift,
        vk: 0xA0,
        scan_code: 0x2A,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::RightShift,
        vk: 0xA1,
        scan_code: 0x36,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::LeftCtrl,
        vk: 0xA2,
        scan_code: 0x1D,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::RightCtrl,
        vk: 0xA3,
        scan_code: 0x1D,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::LeftAlt,
        vk: 0xA4,
        scan_code: 0x38,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::RightAlt,
        vk: 0xA5,
        scan_code: 0x38,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::LeftMeta,
        vk: 0x5B,
        scan_code: 0x5B,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::RightMeta,
        vk: 0x5C,
        scan_code: 0x5C,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::ContextMenu,
        vk: 0x5D,
        scan_code: 0x5D,
        extended: true,
    },
    // System & Navigation
    Win32MappingEntry {
        key: KeyboardKey::PrintScreen,
        vk: 0x2C,
        scan_code: 0x37,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::ScrollLock,
        vk: 0x91,
        scan_code: 0x46,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Pause,
        vk: 0x13,
        scan_code: 0x45,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Insert,
        vk: 0x2D,
        scan_code: 0x52,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::Delete,
        vk: 0x2E,
        scan_code: 0x53,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::Home,
        vk: 0x24,
        scan_code: 0x47,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::End,
        vk: 0x23,
        scan_code: 0x4F,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::PageUp,
        vk: 0x21,
        scan_code: 0x49,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::PageDown,
        vk: 0x22,
        scan_code: 0x51,
        extended: true,
    },
    // Arrows
    Win32MappingEntry {
        key: KeyboardKey::ArrowUp,
        vk: 0x26,
        scan_code: 0x48,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::ArrowDown,
        vk: 0x28,
        scan_code: 0x50,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::ArrowLeft,
        vk: 0x25,
        scan_code: 0x4B,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::ArrowRight,
        vk: 0x27,
        scan_code: 0x4D,
        extended: true,
    },
    // Full Numpad
    Win32MappingEntry {
        key: KeyboardKey::NumLock,
        vk: 0x90,
        scan_code: 0x45,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::NumpadDivide,
        vk: 0x6F,
        scan_code: 0x35,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::NumpadMultiply,
        vk: 0x6A,
        scan_code: 0x37,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::NumpadSubtract,
        vk: 0x6D,
        scan_code: 0x4A,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::NumpadAdd,
        vk: 0x6B,
        scan_code: 0x4E,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::NumpadEnter,
        vk: 0x0D,
        scan_code: 0x1C,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::NumpadDecimal,
        vk: 0x6E,
        scan_code: 0x53,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Numpad0,
        vk: 0x60,
        scan_code: 0x52,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Numpad1,
        vk: 0x61,
        scan_code: 0x4F,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Numpad2,
        vk: 0x62,
        scan_code: 0x50,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Numpad3,
        vk: 0x63,
        scan_code: 0x51,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Numpad4,
        vk: 0x64,
        scan_code: 0x4B,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Numpad5,
        vk: 0x65,
        scan_code: 0x4C,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Numpad6,
        vk: 0x66,
        scan_code: 0x4D,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Numpad7,
        vk: 0x67,
        scan_code: 0x47,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Numpad8,
        vk: 0x68,
        scan_code: 0x48,
        extended: false,
    },
    Win32MappingEntry {
        key: KeyboardKey::Numpad9,
        vk: 0x69,
        scan_code: 0x49,
        extended: false,
    },
    // Media Keys
    Win32MappingEntry {
        key: KeyboardKey::VolumeMute,
        vk: 0xAD,
        scan_code: 0x20,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::VolumeDown,
        vk: 0xAE,
        scan_code: 0x2E,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::VolumeUp,
        vk: 0xAF,
        scan_code: 0x30,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::MediaPlayPause,
        vk: 0xB3,
        scan_code: 0x22,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::MediaStop,
        vk: 0xB2,
        scan_code: 0x24,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::MediaNext,
        vk: 0xB0,
        scan_code: 0x19,
        extended: true,
    },
    Win32MappingEntry {
        key: KeyboardKey::MediaPrev,
        vk: 0xB1,
        scan_code: 0x10,
        extended: true,
    },
];

/// Converts a domain [`KeyboardKey`] into its corresponding Win32 `(vk, scan_code, extended)` tuple.
pub fn keyboard_key_to_win32(key: KeyboardKey) -> Win32KeyCode {
    if let KeyboardKey::OemScanCode(sc) = key {
        return Win32KeyCode::new(0, sc.code, sc.extended);
    }

    WIN32_KEY_TABLE
        .iter()
        .find(|entry| entry.key == key)
        .map(|entry| Win32KeyCode::new(entry.vk, entry.scan_code, entry.extended))
        .unwrap_or(Win32KeyCode::new(0, 0, false))
}

/// Converts a raw Win32 `(vkCode, scanCode, LLKHF_EXTENDED)` tuple into a domain [`KeyboardKey`].
pub fn win32_to_keyboard_key(vk: u16, scan_code: u16, extended: bool) -> KeyboardKey {
    // Disambiguate Enter vs. NumpadEnter explicitly via extended flag.
    if vk == 0x0D {
        return if extended {
            KeyboardKey::NumpadEnter
        } else {
            KeyboardKey::Enter
        };
    }

    // Disambiguate generic modifier VKs when hooks report VK_SHIFT / VK_CONTROL / VK_MENU.
    match vk {
        0x10 => {
            return if scan_code == 0x36 {
                KeyboardKey::RightShift
            } else {
                KeyboardKey::LeftShift
            };
        }
        0x11 => {
            return if extended {
                KeyboardKey::RightCtrl
            } else {
                KeyboardKey::LeftCtrl
            };
        }
        0x12 => {
            return if extended {
                KeyboardKey::RightAlt
            } else {
                KeyboardKey::LeftAlt
            };
        }
        _ => {}
    }

    // Prefer exact match on (vk, extended), then fallback to vk-only match for standard keys.
    if let Some(entry) = WIN32_KEY_TABLE
        .iter()
        .find(|entry| entry.vk == vk && entry.extended == extended)
    {
        return entry.key;
    }

    if let Some(entry) = WIN32_KEY_TABLE.iter().find(|entry| entry.vk == vk) {
        return entry.key;
    }

    KeyboardKey::OemScanCode(ScanCode::new(scan_code, extended))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_standard_keys_round_trip_through_win32_codes() {
        assert_eq!(WIN32_KEY_TABLE.len(), KeyboardKey::ALL_STANDARD.len());
        for &key in KeyboardKey::ALL_STANDARD {
            let win32 = keyboard_key_to_win32(key);
            assert_ne!(win32.vk, 0, "Missing Win32 VK for {key}");
            assert_eq!(
                win32_to_keyboard_key(win32.vk, win32.scan_code, win32.extended),
                key,
                "Win32 round-trip failed for {key}"
            );
        }
    }

    #[test]
    fn enter_and_numpad_enter_disambiguate_via_extended_flag() {
        assert_eq!(win32_to_keyboard_key(0x0D, 0x1C, false), KeyboardKey::Enter);
        assert_eq!(
            win32_to_keyboard_key(0x0D, 0x1C, true),
            KeyboardKey::NumpadEnter
        );
    }

    #[test]
    fn generic_modifier_vks_disambiguate_left_and_right() {
        assert_eq!(
            win32_to_keyboard_key(0x10, 0x2A, false),
            KeyboardKey::LeftShift
        );
        assert_eq!(
            win32_to_keyboard_key(0x10, 0x36, false),
            KeyboardKey::RightShift
        );
        assert_eq!(
            win32_to_keyboard_key(0x11, 0x1D, false),
            KeyboardKey::LeftCtrl
        );
        assert_eq!(
            win32_to_keyboard_key(0x11, 0x1D, true),
            KeyboardKey::RightCtrl
        );
        assert_eq!(
            win32_to_keyboard_key(0x12, 0x38, false),
            KeyboardKey::LeftAlt
        );
        assert_eq!(
            win32_to_keyboard_key(0x12, 0x38, true),
            KeyboardKey::RightAlt
        );
    }
}
