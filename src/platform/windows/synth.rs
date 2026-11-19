//! Windows `SendInput` packet builder and synthetic event tagging adapter.

use crate::domain::keys::{KeyState, MouseButton, MouseWheel};
use crate::platform::windows::keymap::{keyboard_key_to_win32, Win32KeyCode};
use crate::platform::{InputSynthesizer, PlatformError, SyntheticOutput, CTT_SYNTHETIC_TAG};

/// Platform-neutral representation of a Win32 `INPUT` payload tagged with [`CTT_SYNTHETIC_TAG`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Win32InputPacket {
    Keyboard {
        code: Win32KeyCode,
        is_up: bool,
        extra_info: usize,
    },
    MouseButton {
        button: MouseButton,
        is_up: bool,
        extra_info: usize,
    },
    MouseWheel {
        direction: MouseWheel,
        delta: i32,
        extra_info: usize,
    },
}

/// Converts a [`SyntheticOutput`] into a tagged [`Win32InputPacket`].
pub fn build_win32_packet(output: SyntheticOutput) -> Win32InputPacket {
    match output {
        SyntheticOutput::Key { key, state } => Win32InputPacket::Keyboard {
            code: keyboard_key_to_win32(key),
            is_up: state == KeyState::Up,
            extra_info: CTT_SYNTHETIC_TAG,
        },
        SyntheticOutput::MouseButton { button, state } => Win32InputPacket::MouseButton {
            button,
            is_up: state == KeyState::Up,
            extra_info: CTT_SYNTHETIC_TAG,
        },
        SyntheticOutput::MouseWheel { direction, delta } => Win32InputPacket::MouseWheel {
            direction,
            delta,
            extra_info: CTT_SYNTHETIC_TAG,
        },
    }
}

/// Windows `SendInput` synthesizer adapter (live Win32 FFI wiring deferred to Milestone 3).
#[derive(Debug, Default)]
pub struct WindowsInputSynthesizer;

impl WindowsInputSynthesizer {
    /// Creates a new Windows input synthesizer handle.
    pub const fn new() -> Self {
        Self
    }
}

impl InputSynthesizer for WindowsInputSynthesizer {
    fn synthesize(&mut self, output: SyntheticOutput) -> Result<(), PlatformError> {
        let _packet = build_win32_packet(output);
        Err(PlatformError::UnsupportedPlatform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::keys::KeyboardKey;

    #[test]
    fn built_packets_always_carry_synthetic_tag() {
        let key_packet = build_win32_packet(SyntheticOutput::Key {
            key: KeyboardKey::NumpadEnter,
            state: KeyState::Down,
        });
        assert!(matches!(
            key_packet,
            Win32InputPacket::Keyboard {
                code: Win32KeyCode {
                    vk: 0x0D,
                    scan_code: 0x1C,
                    extended: true,
                },
                is_up: false,
                extra_info: CTT_SYNTHETIC_TAG,
            }
        ));

        let mouse_packet = build_win32_packet(SyntheticOutput::MouseButton {
            button: MouseButton::Left,
            state: KeyState::Up,
        });
        assert!(matches!(
            mouse_packet,
            Win32InputPacket::MouseButton {
                button: MouseButton::Left,
                is_up: true,
                extra_info: CTT_SYNTHETIC_TAG,
            }
        ));
    }
}
