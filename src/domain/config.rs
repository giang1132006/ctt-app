//! Application configuration and safety settings.

use crate::domain::keys::{KeyboardKey, ModifierMask};

/// Current schema version for persisted profiles and configuration files.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Non-remappable emergency stop hotkey configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmergencyStopHotkey {
    pub modifiers: ModifierMask,
    pub trigger_key: KeyboardKey,
}

impl Default for EmergencyStopHotkey {
    fn default() -> Self {
        Self {
            modifiers: ModifierMask::CTRL.with(ModifierMask::SHIFT),
            trigger_key: KeyboardKey::F12,
        }
    }
}

impl EmergencyStopHotkey {
    /// Returns `true` when the active modifiers and pressed key match this emergency stop chord.
    pub fn matches(&self, active_modifiers: ModifierMask, key: KeyboardKey) -> bool {
        key == self.trigger_key && active_modifiers.contains(self.modifiers)
    }

    /// Formats the emergency stop hotkey for UI display.
    pub fn display_name(&self) -> String {
        format!(
            "{}{}",
            self.modifiers.display_prefix(),
            self.trigger_key.display_name()
        )
    }
}

/// Persisted top-level application configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub schema_version: u32,
    pub emergency_stop: EmergencyStopHotkey,
    pub mapping_enabled_on_start: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            emergency_stop: EmergencyStopHotkey::default(),
            mapping_enabled_on_start: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emergency_stop_defaults_to_ctrl_shift_f12() {
        let hotkey = EmergencyStopHotkey::default();
        assert_eq!(hotkey.display_name(), "Ctrl + Shift + F12");
        assert!(hotkey.matches(
            ModifierMask::CTRL.with(ModifierMask::SHIFT),
            KeyboardKey::F12
        ));
        assert!(!hotkey.matches(ModifierMask::CTRL, KeyboardKey::F12));
    }
}
