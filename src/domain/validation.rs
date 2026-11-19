//! Validation rules for key mapping conflicts, cycles, swaps, and safety lockouts.

use std::collections::HashMap;
use std::fmt;

use crate::domain::config::EmergencyStopHotkey;
use crate::domain::keys::{InputSource, KeyboardKey};
use crate::domain::mapping::{KeyBindingRule, MappingTarget};

/// Blocking validation errors that prevent safely saving or activating a rule set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingError {
    NoActiveProfile,
    DuplicateSourceKey {
        key: InputSource,
        first_rule_id: u32,
        duplicate_rule_id: u32,
    },
    ReservedEmergencyStopKey {
        rule_id: u32,
        key: KeyboardKey,
    },
    EmptyRuleLabel {
        rule_id: u32,
    },
}

impl fmt::Display for MappingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoActiveProfile => {
                f.write_str("No active profile: create a profile before binding keys")
            }
            Self::DuplicateSourceKey {
                key,
                first_rule_id,
                duplicate_rule_id,
            } => write!(
                f,
                "Duplicate binding for source input '{}' (rules #{first_rule_id} and #{duplicate_rule_id})",
                key.display_name()
            ),
            Self::ReservedEmergencyStopKey { rule_id, key } => write!(
                f,
                "Rule #{rule_id} conflicts with the reserved Emergency Stop key '{}'",
                key.display_name()
            ),
            Self::EmptyRuleLabel { rule_id } => {
                write!(f, "Rule #{rule_id} must have a non-empty label")
            }
        }
    }
}

impl std::error::Error for MappingError {}

/// Non-blocking informational warnings about swaps, chains, or no-op bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingWarning {
    IdentityMapping {
        rule_id: u32,
        key: InputSource,
    },
    KeySwapDetected {
        rule_a_id: u32,
        rule_b_id: u32,
        key_a: InputSource,
        key_b: InputSource,
    },
    ChainedMappingDetected {
        from_rule_id: u32,
        to_rule_id: u32,
        via_key: InputSource,
    },
}

impl fmt::Display for MappingWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdentityMapping { rule_id, key } => write!(
                f,
                "Rule #{rule_id} maps '{}' to itself (no-op)",
                key.display_name()
            ),
            Self::KeySwapDetected {
                rule_a_id,
                rule_b_id,
                key_a,
                key_b,
            } => write!(
                f,
                "Rules #{rule_a_id} and #{rule_b_id} form a direct swap ('{}' <-> '{}')",
                key_a.display_name(),
                key_b.display_name()
            ),
            Self::ChainedMappingDetected {
                from_rule_id,
                to_rule_id,
                via_key,
            } => write!(
                f,
                "Rule #{from_rule_id} outputs '{}', which is remapped by rule #{to_rule_id} (only physical presses trigger rule #{to_rule_id})",
                via_key.display_name()
            ),
        }
    }
}

/// Complete validation outcome separating blocking errors from advisory warnings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationReport {
    pub errors: Vec<MappingError>,
    pub warnings: Vec<MappingWarning>,
}

impl ValidationReport {
    /// Returns `true` when zero blocking [`MappingError`]s were found.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validates active binding rules against conflict, cycle, and safety invariants.
pub fn validate_profile_rules(
    rules: &[KeyBindingRule],
    emergency_hotkey: EmergencyStopHotkey,
) -> ValidationReport {
    let mut report = ValidationReport::default();
    let mut seen_sources: HashMap<InputSource, &KeyBindingRule> = HashMap::new();

    let enabled_rules: Vec<&KeyBindingRule> = rules.iter().filter(|r| r.enabled).collect();

    for rule in &enabled_rules {
        if rule.label.trim().is_empty() {
            report
                .errors
                .push(MappingError::EmptyRuleLabel { rule_id: rule.id });
        }

        if rule.source == InputSource::Keyboard(emergency_hotkey.trigger_key)
            && (emergency_hotkey.modifiers.is_empty()
                || matches!(rule.target, MappingTarget::Disabled))
        {
            report.errors.push(MappingError::ReservedEmergencyStopKey {
                rule_id: rule.id,
                key: emergency_hotkey.trigger_key,
            });
        }

        if let Some(existing) = seen_sources.get(&rule.source) {
            report.errors.push(MappingError::DuplicateSourceKey {
                key: rule.source,
                first_rule_id: existing.id,
                duplicate_rule_id: rule.id,
            });
        } else {
            seen_sources.insert(rule.source, rule);
        }

        if matches!(
            rule.target,
            MappingTarget::Keyboard(_) | MappingTarget::MouseButton(_)
        ) && rule.target.as_input_source() == Some(rule.source)
        {
            report.warnings.push(MappingWarning::IdentityMapping {
                rule_id: rule.id,
                key: rule.source,
            });
        }
    }

    // Detect swaps (A <-> B) and chained mappings (A -> B, B -> C) across distinct enabled rules.
    for rule in &enabled_rules {
        let Some(target_source) = rule.target.as_input_source() else {
            continue;
        };
        if target_source == rule.source {
            continue;
        }
        let Some(next_rule) = seen_sources.get(&target_source) else {
            continue;
        };

        let is_direct_swap = next_rule.target.as_input_source() == Some(rule.source);
        if is_direct_swap {
            // Emit each symmetric swap pair once by ordering on rule ID.
            if rule.id < next_rule.id {
                report.warnings.push(MappingWarning::KeySwapDetected {
                    rule_a_id: rule.id,
                    rule_b_id: next_rule.id,
                    key_a: rule.source,
                    key_b: next_rule.source,
                });
            }
        } else {
            report
                .warnings
                .push(MappingWarning::ChainedMappingDetected {
                    from_rule_id: rule.id,
                    to_rule_id: next_rule.id,
                    via_key: target_source,
                });
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::keys::ModifierMask;

    #[test]
    fn duplicate_source_is_rejected() {
        let rules = vec![
            KeyBindingRule::new(
                1,
                "First",
                KeyboardKey::KeyQ,
                MappingTarget::Keyboard(KeyboardKey::KeyE),
            ),
            KeyBindingRule::new(
                2,
                "Second",
                KeyboardKey::KeyQ,
                MappingTarget::Keyboard(KeyboardKey::KeyR),
            ),
        ];

        let report = validate_profile_rules(&rules, EmergencyStopHotkey::default());
        assert!(!report.is_valid());
        assert_eq!(
            report.errors,
            vec![MappingError::DuplicateSourceKey {
                key: InputSource::Keyboard(KeyboardKey::KeyQ),
                first_rule_id: 1,
                duplicate_rule_id: 2,
            }]
        );
    }

    #[test]
    fn emergency_stop_trigger_cannot_be_disabled_or_unmodified_hijacked() {
        let disabled_f12 = vec![KeyBindingRule::new(
            1,
            "Disable F12",
            KeyboardKey::F12,
            MappingTarget::Disabled,
        )];
        let report = validate_profile_rules(&disabled_f12, EmergencyStopHotkey::default());
        assert!(!report.is_valid());
        assert!(matches!(
            report.errors.as_slice(),
            [MappingError::ReservedEmergencyStopKey { rule_id: 1, .. }]
        ));

        let bare_stop = EmergencyStopHotkey {
            modifiers: ModifierMask::NONE,
            trigger_key: KeyboardKey::Pause,
        };
        let hijack_pause = vec![KeyBindingRule::new(
            2,
            "Remap Pause",
            KeyboardKey::Pause,
            MappingTarget::Keyboard(KeyboardKey::Space),
        )];
        let report2 = validate_profile_rules(&hijack_pause, bare_stop);
        assert!(!report2.is_valid());
    }

    #[test]
    fn swap_and_chain_produce_warnings_without_blocking() {
        let rules = vec![
            KeyBindingRule::new(
                1,
                "Swap A->B",
                KeyboardKey::KeyA,
                MappingTarget::Keyboard(KeyboardKey::KeyB),
            ),
            KeyBindingRule::new(
                2,
                "Swap B->A",
                KeyboardKey::KeyB,
                MappingTarget::Keyboard(KeyboardKey::KeyA),
            ),
            KeyBindingRule::new(
                3,
                "Chain C->A",
                KeyboardKey::KeyC,
                MappingTarget::Keyboard(KeyboardKey::KeyA),
            ),
        ];

        let report = validate_profile_rules(&rules, EmergencyStopHotkey::default());
        assert!(report.is_valid());
        assert!(report.warnings.contains(&MappingWarning::KeySwapDetected {
            rule_a_id: 1,
            rule_b_id: 2,
            key_a: InputSource::Keyboard(KeyboardKey::KeyA),
            key_b: InputSource::Keyboard(KeyboardKey::KeyB),
        }));
        assert!(report
            .warnings
            .contains(&MappingWarning::ChainedMappingDetected {
                from_rule_id: 3,
                to_rule_id: 1,
                via_key: InputSource::Keyboard(KeyboardKey::KeyA),
            }));
    }
}
