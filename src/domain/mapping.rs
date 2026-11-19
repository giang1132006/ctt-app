//! Key binding rules, target actions, and compiled `O(1)` lookup tables.

use std::collections::HashMap;
use std::fmt;

use crate::domain::keys::{InputSource, KeyboardKey, ModifierMask, MouseButton, MouseWheel};

/// Target action executed when a physical [`InputSource`] is pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MappingTarget {
    /// Remaps the source key to another single keyboard key.
    Keyboard(KeyboardKey),
    /// Remaps the source key to a modifier chord plus keyboard key.
    KeyCombo {
        modifiers: ModifierMask,
        key: KeyboardKey,
    },
    /// Remaps the source key to hold/release a mouse button.
    MouseButton(MouseButton),
    /// Remaps the source key to emit a mouse scroll wheel tick.
    MouseWheel(MouseWheel),
    /// Suppresses the source key without emitting any output.
    Disabled,
}

impl MappingTarget {
    /// Returns a human-readable representation for UI display.
    pub fn display_name(self) -> String {
        match self {
            Self::Keyboard(key) => key.display_name(),
            Self::KeyCombo { modifiers, key } => {
                format!("{}{}", modifiers.display_prefix(), key.display_name())
            }
            Self::MouseButton(button) => button.display_name().to_string(),
            Self::MouseWheel(wheel) => wheel.display_name().to_string(),
            Self::Disabled => "Disabled".to_string(),
        }
    }

    /// Serializes this target into a stable ASCII token.
    pub fn to_serial_string(self) -> String {
        match self {
            Self::Keyboard(key) => format!("KEY:{}", key.short_code()),
            Self::KeyCombo { modifiers, key } => {
                format!("COMBO:{}|{}", modifiers.to_serial_code(), key.short_code())
            }
            Self::MouseButton(button) => format!("MOUSE:{}", button.short_code()),
            Self::MouseWheel(wheel) => format!("WHEEL:{}", wheel.short_code()),
            Self::Disabled => "DISABLED".to_string(),
        }
    }

    /// Parses a [`MappingTarget`] from a serialized token produced by [`Self::to_serial_string`].
    pub fn from_serial_string(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("DISABLED") {
            return Some(Self::Disabled);
        }
        if let Some(code) = trimmed.strip_prefix("KEY:") {
            return KeyboardKey::from_short_code(code).map(Self::Keyboard);
        }
        if let Some(rest) = trimmed.strip_prefix("COMBO:") {
            let (mods_raw, key_raw) = rest.split_once('|')?;
            let modifiers = ModifierMask::from_serial_code(mods_raw)?;
            let key = KeyboardKey::from_short_code(key_raw)?;
            return Some(Self::KeyCombo { modifiers, key });
        }
        if let Some(code) = trimmed.strip_prefix("MOUSE:") {
            return MouseButton::from_short_code(code).map(Self::MouseButton);
        }
        if let Some(code) = trimmed.strip_prefix("WHEEL:") {
            return MouseWheel::from_short_code(code).map(Self::MouseWheel);
        }
        None
    }

    /// Returns the primary keyboard key if this target outputs a single key or combo key.
    pub const fn primary_keyboard_key(self) -> Option<KeyboardKey> {
        match self {
            Self::Keyboard(key) | Self::KeyCombo { key, .. } => Some(key),
            Self::MouseButton(_) | Self::MouseWheel(_) | Self::Disabled => None,
        }
    }

    /// Returns the corresponding [`InputSource`] when this target emits a key or mouse button.
    pub const fn as_input_source(self) -> Option<InputSource> {
        match self {
            Self::Keyboard(key) | Self::KeyCombo { key, .. } => Some(InputSource::Keyboard(key)),
            Self::MouseButton(button) => Some(InputSource::MouseButton(button)),
            Self::MouseWheel(_) | Self::Disabled => None,
        }
    }
}

impl fmt::Display for MappingTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display_name())
    }
}

/// A single key binding rule mapping one physical [`InputSource`] to a [`MappingTarget`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBindingRule {
    pub id: u32,
    pub label: String,
    pub enabled: bool,
    pub source: InputSource,
    pub target: MappingTarget,
}

impl KeyBindingRule {
    /// Creates an enabled [`KeyBindingRule`].
    pub fn new(
        id: u32,
        label: impl Into<String>,
        source: impl Into<InputSource>,
        target: MappingTarget,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            enabled: true,
            source: source.into(),
            target,
        }
    }
}

/// Pre-compiled `O(1)` lookup table evaluated by [`crate::engine::remapper::RemapEngine`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompiledMappingTable {
    entries: HashMap<InputSource, MappingTarget>,
}

impl CompiledMappingTable {
    /// Compiles all enabled rules into an allocation-free runtime lookup map.
    ///
    /// If duplicate enabled rules exist for the same source input in an unvalidated list,
    /// the first enabled rule wins deterministically.
    pub fn compile(rules: &[KeyBindingRule]) -> Self {
        let mut entries = HashMap::with_capacity(rules.len());
        for rule in rules.iter().filter(|r| r.enabled) {
            entries.entry(rule.source).or_insert(rule.target);
        }
        Self { entries }
    }

    /// Looks up the active target for `source` in `O(1)` expected time.
    pub fn lookup(&self, source: impl Into<InputSource>) -> Option<MappingTarget> {
        self.entries.get(&source.into()).copied()
    }

    /// Returns the number of active source bindings in this compiled table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if no active bindings are present.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapping_target_serialization_round_trips() {
        let targets = [
            MappingTarget::Keyboard(KeyboardKey::NumpadEnter),
            MappingTarget::KeyCombo {
                modifiers: ModifierMask::CTRL.with(ModifierMask::SHIFT),
                key: KeyboardKey::KeyC,
            },
            MappingTarget::MouseButton(MouseButton::XButton2),
            MappingTarget::MouseWheel(MouseWheel::ScrollDown),
            MappingTarget::Disabled,
        ];

        for target in targets {
            let serialized = target.to_serial_string();
            assert_eq!(
                MappingTarget::from_serial_string(&serialized),
                Some(target),
                "Failed round-trip for {serialized}"
            );
        }
    }

    #[test]
    fn compiled_table_ignores_disabled_rules_and_keeps_first_enabled() {
        let mut disabled = KeyBindingRule::new(
            1,
            "Disabled",
            KeyboardKey::KeyA,
            MappingTarget::MouseButton(MouseButton::Left),
        );
        disabled.enabled = false;

        let first = KeyBindingRule::new(
            2,
            "First",
            KeyboardKey::KeyA,
            MappingTarget::Keyboard(KeyboardKey::KeyB),
        );
        let second = KeyBindingRule::new(
            3,
            "Second",
            KeyboardKey::KeyA,
            MappingTarget::Keyboard(KeyboardKey::KeyC),
        );

        let table = CompiledMappingTable::compile(&[disabled, first, second]);
        assert_eq!(table.len(), 1);
        assert_eq!(
            table.lookup(KeyboardKey::KeyA),
            Some(MappingTarget::Keyboard(KeyboardKey::KeyB))
        );
    }
}
