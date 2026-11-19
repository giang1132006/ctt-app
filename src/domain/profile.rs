//! Profile models grouping key binding rules and compilation helpers.

use crate::domain::config::{EmergencyStopHotkey, CURRENT_SCHEMA_VERSION};
use crate::domain::keys::InputSource;
use crate::domain::mapping::{CompiledMappingTable, KeyBindingRule, MappingTarget};
use crate::domain::validation::{validate_profile_rules, ValidationReport};

/// Named mapping profile holding an ordered list of key binding rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub id: u32,
    pub name: String,
    pub schema_version: u32,
    pub enabled: bool,
    pub rules: Vec<KeyBindingRule>,
}

impl Profile {
    /// Creates an empty enabled profile with the given ID and name.
    pub fn new(id: u32, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            schema_version: CURRENT_SCHEMA_VERSION,
            enabled: true,
            rules: Vec::new(),
        }
    }

    /// Compiles this profile into an `O(1)` lookup table if enabled, or an empty table otherwise.
    pub fn compile_table(&self) -> CompiledMappingTable {
        if !self.enabled {
            return CompiledMappingTable::default();
        }
        CompiledMappingTable::compile(&self.rules)
    }

    /// Validates this profile's rules against conflict and emergency-stop constraints.
    pub fn validate(&self, emergency_hotkey: EmergencyStopHotkey) -> ValidationReport {
        validate_profile_rules(&self.rules, emergency_hotkey)
    }

    /// Updates the existing rule for `source` or inserts a new enabled rule, returning its ID.
    pub fn set_or_add_binding(
        &mut self,
        source: impl Into<InputSource>,
        target: MappingTarget,
        label: impl Into<String>,
    ) -> u32 {
        let source = source.into();
        let label_str = label.into();
        if let Some(existing) = self.rules.iter_mut().find(|r| r.source == source) {
            existing.label = label_str;
            existing.target = target;
            existing.enabled = true;
            return existing.id;
        }

        let next_id = self.rules.iter().map(|r| r.id).max().unwrap_or(0) + 1;
        self.rules
            .push(KeyBindingRule::new(next_id, label_str, source, target));
        next_id
    }

    /// Removes all rules bound to `source`, returning `true` if at least one rule was removed.
    pub fn remove_binding_by_source(&mut self, source: impl Into<InputSource>) -> bool {
        let source = source.into();
        let before = self.rules.len();
        self.rules.retain(|r| r.source != source);
        self.rules.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::keys::{KeyboardKey, MouseButton};

    #[test]
    fn set_or_add_and_remove_binding_work_correctly() {
        let mut profile = Profile::new(10, "Gaming");
        let id1 = profile.set_or_add_binding(
            KeyboardKey::Space,
            MappingTarget::MouseButton(MouseButton::Left),
            "Jump -> Left Click",
        );
        assert_eq!(id1, 1);
        assert_eq!(profile.rules.len(), 1);

        // Updating the same source replaces in-place rather than duplicating.
        let id2 = profile.set_or_add_binding(
            KeyboardKey::Space,
            MappingTarget::MouseButton(MouseButton::Right),
            "Jump -> Right Click",
        );
        assert_eq!(id2, 1);
        assert_eq!(profile.rules.len(), 1);
        assert_eq!(
            profile.compile_table().lookup(KeyboardKey::Space),
            Some(MappingTarget::MouseButton(MouseButton::Right))
        );

        assert!(profile.remove_binding_by_source(KeyboardKey::Space));
        assert!(!profile.remove_binding_by_source(KeyboardKey::Space));
        assert!(profile.rules.is_empty());
    }
}
