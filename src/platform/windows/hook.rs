//! Windows low-level keyboard hook (`WH_KEYBOARD_LL`) scaffolding with RAII cleanup.

use crate::platform::{InputInterceptor, PlatformError, CTT_SYNTHETIC_TAG};

/// Returns `true` when `KBDLLHOOKSTRUCT.dwExtraInfo` identifies an event emitted by `ctt-app`.
pub const fn is_synthetic_extra_info(extra_info: usize) -> bool {
    extra_info == CTT_SYNTHETIC_TAG
}

/// RAII handle for a Windows low-level keyboard hook.
///
/// Ensures hooks are always unhooked cleanly on stop or [`Drop`].
#[derive(Debug, Default)]
pub struct WindowsKeyboardHook {
    active: bool,
}

impl WindowsKeyboardHook {
    /// Creates an inactive hook controller.
    pub const fn new() -> Self {
        Self { active: false }
    }

    /// Registers the Windows low-level hook.
    ///
    /// Live Win32 FFI hook installation (`SetWindowsHookExW`) is deferred until Milestone 3 (`ADR-003`).
    pub fn start(&mut self) -> Result<(), PlatformError> {
        Err(PlatformError::HookRegistrationFailed(
            "Win32 WH_KEYBOARD_LL FFI backend pending ADR-003 selection".to_string(),
        ))
    }
}

impl InputInterceptor for WindowsKeyboardHook {
    fn is_active(&self) -> bool {
        self.active
    }

    fn stop(&mut self) -> Result<(), PlatformError> {
        self.active = false;
        Ok(())
    }
}

impl Drop for WindowsKeyboardHook {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_ctt_synthetic_extra_info_tag() {
        assert!(is_synthetic_extra_info(CTT_SYNTHETIC_TAG));
        assert!(!is_synthetic_extra_info(0));
        assert!(!is_synthetic_extra_info(0x1234));
    }
}
