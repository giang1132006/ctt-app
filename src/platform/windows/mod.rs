//! Windows-specific key translation, low-level hooks (`WH_KEYBOARD_LL` / `WH_MOUSE_LL`),
//! and `SendInput` packet synthesis tagged with [`crate::platform::CTT_SYNTHETIC_TAG`].

#[cfg(target_os = "windows")]
pub mod ffi;
pub mod hook;
pub mod keymap;
pub mod synth;
