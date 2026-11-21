//! Windows `SendInput` packet builder and live Win32 event synthesizer.

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

#[cfg(target_os = "windows")]
pub(crate) fn packet_to_win32_input(
    packet: Win32InputPacket,
) -> crate::platform::windows::ffi::INPUT {
    use crate::platform::windows::ffi::{
        INPUT, INPUT_KEYBOARD, INPUT_MOUSE, INPUT_UNION, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
        KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN,
        MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN,
        MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT,
        XBUTTON1, XBUTTON2,
    };

    match packet {
        Win32InputPacket::Keyboard {
            code,
            is_up,
            extra_info,
        } => {
            let mut dw_flags = 0_u32;
            if code.vk == 0 {
                dw_flags |= KEYEVENTF_SCANCODE;
            }
            if code.extended {
                dw_flags |= KEYEVENTF_EXTENDEDKEY;
            }
            if is_up {
                dw_flags |= KEYEVENTF_KEYUP;
            }
            INPUT {
                input_type: INPUT_KEYBOARD,
                union: INPUT_UNION {
                    ki: KEYBDINPUT {
                        w_vk: code.vk,
                        w_scan: code.scan_code,
                        dw_flags,
                        time: 0,
                        dw_extra_info: extra_info,
                    },
                },
            }
        }
        Win32InputPacket::MouseButton {
            button,
            is_up,
            extra_info,
        } => {
            let (dw_flags, mouse_data) = match (button, is_up) {
                (MouseButton::Left, false) => (MOUSEEVENTF_LEFTDOWN, 0),
                (MouseButton::Left, true) => (MOUSEEVENTF_LEFTUP, 0),
                (MouseButton::Right, false) => (MOUSEEVENTF_RIGHTDOWN, 0),
                (MouseButton::Right, true) => (MOUSEEVENTF_RIGHTUP, 0),
                (MouseButton::Middle, false) => (MOUSEEVENTF_MIDDLEDOWN, 0),
                (MouseButton::Middle, true) => (MOUSEEVENTF_MIDDLEUP, 0),
                (MouseButton::XButton1, false) => (MOUSEEVENTF_XDOWN, XBUTTON1),
                (MouseButton::XButton1, true) => (MOUSEEVENTF_XUP, XBUTTON1),
                (MouseButton::XButton2, false) => (MOUSEEVENTF_XDOWN, XBUTTON2),
                (MouseButton::XButton2, true) => (MOUSEEVENTF_XUP, XBUTTON2),
            };
            INPUT {
                input_type: INPUT_MOUSE,
                union: INPUT_UNION {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouse_data,
                        dw_flags,
                        time: 0,
                        dw_extra_info: extra_info,
                    },
                },
            }
        }
        Win32InputPacket::MouseWheel {
            direction,
            delta,
            extra_info,
        } => {
            let abs_delta = delta.abs();
            let (dw_flags, signed_data) = match direction {
                MouseWheel::ScrollUp => (MOUSEEVENTF_WHEEL, abs_delta),
                MouseWheel::ScrollDown => (MOUSEEVENTF_WHEEL, -abs_delta),
                MouseWheel::ScrollRight => (MOUSEEVENTF_HWHEEL, abs_delta),
                MouseWheel::ScrollLeft => (MOUSEEVENTF_HWHEEL, -abs_delta),
            };
            INPUT {
                input_type: INPUT_MOUSE,
                union: INPUT_UNION {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouse_data: signed_data as u32,
                        dw_flags,
                        time: 0,
                        dw_extra_info: extra_info,
                    },
                },
            }
        }
    }
}

/// Live Windows `SendInput` synthesizer stamping every event with [`CTT_SYNTHETIC_TAG`].
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsInputSynthesizer;

impl WindowsInputSynthesizer {
    /// Creates a new Windows input synthesizer handle.
    pub const fn new() -> Self {
        Self
    }
}

impl InputSynthesizer for WindowsInputSynthesizer {
    fn synthesize(&mut self, output: SyntheticOutput) -> Result<(), PlatformError> {
        let packet = build_win32_packet(output);

        #[cfg(target_os = "windows")]
        {
            use crate::platform::windows::ffi::{GetLastError, SendInput, INPUT};
            let input = packet_to_win32_input(packet);
            let cb_size = i32::try_from(std::mem::size_of::<INPUT>()).unwrap_or(40);
            // SAFETY: `input` is a valid initialized Win32 `INPUT` struct and `cb_size` matches its size.
            let inserted = unsafe { SendInput(1, &input, cb_size) };
            if inserted == 1 {
                Ok(())
            } else {
                // SAFETY: `GetLastError` is always safe to call after a failed Win32 call.
                let err_code = unsafe { GetLastError() };
                Err(PlatformError::SynthesisFailed(format!(
                    "SendInput returned 0 (Win32 error {err_code}; target window may be elevated via UIPI)"
                )))
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = packet;
            Err(PlatformError::UnsupportedPlatform)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::keys::{KeyboardKey, ScanCode};

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

    #[cfg(target_os = "windows")]
    #[test]
    fn win32_input_structs_encode_flags_wheel_and_synthetic_tag() {
        use crate::platform::windows::ffi::{
            INPUT_KEYBOARD, INPUT_MOUSE, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
            KEYEVENTF_SCANCODE, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN, XBUTTON2,
        };

        // Extended key release
        let k_up = packet_to_win32_input(build_win32_packet(SyntheticOutput::Key {
            key: KeyboardKey::RightCtrl,
            state: KeyState::Up,
        }));
        assert_eq!(k_up.input_type, INPUT_KEYBOARD);
        // SAFETY: input_type == INPUT_KEYBOARD
        let ki = unsafe { k_up.union.ki };
        assert_eq!(ki.w_vk, 0xA3);
        assert_eq!(ki.w_scan, 0x1D);
        assert_eq!(ki.dw_flags, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP);
        assert_eq!(ki.dw_extra_info, CTT_SYNTHETIC_TAG);

        // OEM scan code fallback sets KEYEVENTF_SCANCODE
        let oem = packet_to_win32_input(build_win32_packet(SyntheticOutput::Key {
            key: KeyboardKey::OemScanCode(ScanCode::new(0x70, true)),
            state: KeyState::Down,
        }));
        // SAFETY: input_type == INPUT_KEYBOARD
        let ki_oem = unsafe { oem.union.ki };
        assert_eq!(ki_oem.w_vk, 0);
        assert_eq!(ki_oem.w_scan, 0x70);
        assert_eq!(ki_oem.dw_flags, KEYEVENTF_SCANCODE | KEYEVENTF_EXTENDEDKEY);
        assert_eq!(ki_oem.dw_extra_info, CTT_SYNTHETIC_TAG);

        // Side mouse button X2
        let x2 = packet_to_win32_input(build_win32_packet(SyntheticOutput::MouseButton {
            button: MouseButton::XButton2,
            state: KeyState::Down,
        }));
        assert_eq!(x2.input_type, INPUT_MOUSE);
        // SAFETY: input_type == INPUT_MOUSE
        let mi_x2 = unsafe { x2.union.mi };
        assert_eq!(mi_x2.dw_flags, MOUSEEVENTF_XDOWN);
        assert_eq!(mi_x2.mouse_data, XBUTTON2);
        assert_eq!(mi_x2.dw_extra_info, CTT_SYNTHETIC_TAG);

        // ScrollDown uses negative delta; ScrollLeft uses negative horizontal delta
        let wheel_dn = packet_to_win32_input(build_win32_packet(SyntheticOutput::MouseWheel {
            direction: MouseWheel::ScrollDown,
            delta: 120,
        }));
        // SAFETY: input_type == INPUT_MOUSE
        let mi_dn = unsafe { wheel_dn.union.mi };
        assert_eq!(mi_dn.dw_flags, MOUSEEVENTF_WHEEL);
        assert_eq!(mi_dn.mouse_data as i32, -120);
        assert_eq!(mi_dn.dw_extra_info, CTT_SYNTHETIC_TAG);

        let wheel_left = packet_to_win32_input(build_win32_packet(SyntheticOutput::MouseWheel {
            direction: MouseWheel::ScrollLeft,
            delta: 120,
        }));
        // SAFETY: input_type == INPUT_MOUSE
        let mi_left = unsafe { wheel_left.union.mi };
        assert_eq!(mi_left.dw_flags, MOUSEEVENTF_HWHEEL);
        assert_eq!(mi_left.mouse_data as i32, -120);
    }
}
