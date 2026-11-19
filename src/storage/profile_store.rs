//! Schema-versioned profile serialization and atomic filesystem persistence.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::domain::config::{EmergencyStopHotkey, CURRENT_SCHEMA_VERSION};
use crate::domain::keys::InputSource;
use crate::domain::mapping::{KeyBindingRule, MappingTarget};
use crate::domain::profile::Profile;
use crate::domain::validation::validate_profile_rules;
use crate::storage::error::StorageError;

/// Handles lossless serialization and atomic file storage of [`Profile`] data.
pub struct ProfileStore;

impl ProfileStore {
    /// Serializes `profile` into a versioned line-based UTF-8 representation.
    pub fn serialize_profile(profile: &Profile) -> String {
        let mut out = String::new();
        out.push_str("# ctt-app profile\n");
        out.push_str(&format!("schema_version={}\n", profile.schema_version));
        out.push_str(&format!("id={}\n", profile.id));
        out.push_str(&format!("name={}\n", escape_field(&profile.name)));
        out.push_str(&format!("enabled={}\n", profile.enabled));

        for rule in &profile.rules {
            out.push_str(&format!(
                "rule={}|{}|{}|{}|{}\n",
                rule.id,
                rule.enabled,
                rule.source.short_code(),
                rule.target.to_serial_string(),
                escape_field(&rule.label)
            ));
        }

        out
    }

    /// Parses and validates a serialized [`Profile`], checking schema version and safety rules.
    pub fn deserialize_profile(
        content: &str,
        emergency_stop: EmergencyStopHotkey,
    ) -> Result<Profile, StorageError> {
        let mut schema_version: Option<u32> = None;
        let mut id: Option<u32> = None;
        let mut name: Option<String> = None;
        let mut enabled: Option<bool> = None;
        let mut rules: Vec<KeyBindingRule> = Vec::new();

        for (line_idx, raw_line) in content.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                return Err(StorageError::ParseError(format!(
                    "Line {}: expected key=value",
                    line_idx + 1
                )));
            };

            match key {
                "schema_version" => {
                    let parsed = value.parse::<u32>().map_err(|_| {
                        StorageError::ParseError(format!(
                            "Line {}: invalid schema_version '{value}'",
                            line_idx + 1
                        ))
                    })?;
                    schema_version = Some(parsed);
                }
                "id" => {
                    let parsed = value.parse::<u32>().map_err(|_| {
                        StorageError::ParseError(format!(
                            "Line {}: invalid id '{value}'",
                            line_idx + 1
                        ))
                    })?;
                    id = Some(parsed);
                }
                "name" => {
                    name = Some(unescape_field(value).ok_or_else(|| {
                        StorageError::ParseError(format!(
                            "Line {}: invalid escape sequence in name",
                            line_idx + 1
                        ))
                    })?);
                }
                "enabled" => {
                    let parsed = value.parse::<bool>().map_err(|_| {
                        StorageError::ParseError(format!(
                            "Line {}: invalid boolean '{value}'",
                            line_idx + 1
                        ))
                    })?;
                    enabled = Some(parsed);
                }
                "rule" => {
                    rules.push(parse_rule_line(value, line_idx + 1)?);
                }
                other => {
                    return Err(StorageError::ParseError(format!(
                        "Line {}: unknown field '{other}'",
                        line_idx + 1
                    )));
                }
            }
        }

        let found_version = schema_version.ok_or_else(|| {
            StorageError::ParseError("Missing required 'schema_version' field".to_string())
        })?;
        if found_version != CURRENT_SCHEMA_VERSION {
            return Err(StorageError::UnsupportedSchemaVersion {
                found: found_version,
                expected: CURRENT_SCHEMA_VERSION,
            });
        }

        let profile = Profile {
            id: id.ok_or_else(|| {
                StorageError::ParseError("Missing required 'id' field".to_string())
            })?,
            name: name.ok_or_else(|| {
                StorageError::ParseError("Missing required 'name' field".to_string())
            })?,
            schema_version: found_version,
            enabled: enabled.unwrap_or(true),
            rules,
        };

        let report = validate_profile_rules(&profile.rules, emergency_stop);
        if !report.is_valid() {
            return Err(StorageError::ValidationFailed(report.errors));
        }

        Ok(profile)
    }

    /// Writes `profile` atomically using a sibling temporary file + `sync_all` + rename.
    pub fn save_profile_atomic(path: &Path, profile: &Profile) -> Result<(), StorageError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let tmp_path = temp_sibling_path(path);
        let serialized = Self::serialize_profile(profile);

        {
            let mut file = File::create(&tmp_path)?;
            file.write_all(serialized.as_bytes())?;
            file.sync_all()?;
        }

        if let Err(err) = fs::rename(&tmp_path, path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(StorageError::Io(err));
        }

        Ok(())
    }

    /// Reads, parses, and validates a [`Profile`] from disk.
    pub fn load_profile(
        path: &Path,
        emergency_stop: EmergencyStopHotkey,
    ) -> Result<Profile, StorageError> {
        let content = fs::read_to_string(path)?;
        Self::deserialize_profile(&content, emergency_stop)
    }
}

fn parse_rule_line(raw: &str, line_num: usize) -> Result<KeyBindingRule, StorageError> {
    let parts: Vec<&str> = raw.splitn(5, '|').collect();
    if parts.len() != 5 {
        return Err(StorageError::ParseError(format!(
            "Line {line_num}: expected 5 '|' separated rule fields"
        )));
    }

    // Note: COMBO targets use `COMBO:MODS|KEY` internally, which contains one `|`.
    // Re-parse cleanly when field 3 starts with `COMBO:`.
    if parts[3].starts_with("COMBO:") {
        let combo_parts: Vec<&str> = raw.splitn(6, '|').collect();
        if combo_parts.len() != 6 {
            return Err(StorageError::ParseError(format!(
                "Line {line_num}: malformed COMBO target field"
            )));
        }
        let combined_target = format!("{}|{}", combo_parts[3], combo_parts[4]);
        return build_rule(
            combo_parts[0],
            combo_parts[1],
            combo_parts[2],
            &combined_target,
            combo_parts[5],
            line_num,
        );
    }

    build_rule(parts[0], parts[1], parts[2], parts[3], parts[4], line_num)
}

fn build_rule(
    id_str: &str,
    enabled_str: &str,
    source_str: &str,
    target_str: &str,
    label_escaped: &str,
    line_num: usize,
) -> Result<KeyBindingRule, StorageError> {
    let id = id_str.parse::<u32>().map_err(|_| {
        StorageError::ParseError(format!("Line {line_num}: invalid rule id '{id_str}'"))
    })?;
    let enabled = enabled_str.parse::<bool>().map_err(|_| {
        StorageError::ParseError(format!(
            "Line {line_num}: invalid rule enabled flag '{enabled_str}'"
        ))
    })?;
    let source = InputSource::from_short_code(source_str).ok_or_else(|| {
        StorageError::ParseError(format!(
            "Line {line_num}: unknown source input '{source_str}'"
        ))
    })?;
    let target = MappingTarget::from_serial_string(target_str).ok_or_else(|| {
        StorageError::ParseError(format!("Line {line_num}: invalid target '{target_str}'"))
    })?;
    let label = unescape_field(label_escaped).ok_or_else(|| {
        StorageError::ParseError(format!("Line {line_num}: invalid escape sequence in label"))
    })?;

    Ok(KeyBindingRule {
        id,
        label,
        enabled,
        source,
        target,
    })
}

fn temp_sibling_path(path: &Path) -> PathBuf {
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".tmp");
    PathBuf::from(tmp)
}

fn escape_field(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '|' => out.push_str("\\p"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out
}

fn unescape_field(raw: &str) -> Option<String> {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next()? {
                '\\' => out.push('\\'),
                'p' => out.push('|'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                _ => return None,
            }
        } else {
            out.push(ch);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::keys::{KeyboardKey, MouseButton, MouseWheel};

    #[test]
    fn serialize_and_deserialize_preserves_combos_mouse_and_special_chars() {
        let mut profile = Profile::new(7, "Profile | Test \\ v1");
        profile.set_or_add_binding(
            KeyboardKey::Space,
            MappingTarget::MouseButton(MouseButton::Left),
            "Jump | Fire",
        );
        profile.set_or_add_binding(
            MouseButton::XButton1,
            MappingTarget::Keyboard(KeyboardKey::KeyR),
            "Side1 -> R",
        );
        profile.set_or_add_binding(
            KeyboardKey::KeyQ,
            MappingTarget::KeyCombo {
                modifiers: crate::domain::keys::ModifierMask::CTRL,
                key: KeyboardKey::KeyC,
            },
            "Copy\\Fast",
        );
        profile.set_or_add_binding(
            KeyboardKey::PageDown,
            MappingTarget::MouseWheel(MouseWheel::ScrollDown),
            "Scroll",
        );

        let raw = ProfileStore::serialize_profile(&profile);
        let parsed = ProfileStore::deserialize_profile(&raw, EmergencyStopHotkey::default())
            .expect("Serialized profile should parse cleanly");
        assert_eq!(parsed, profile);
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let content = "schema_version=999\nid=1\nname=Future\nenabled=true\n";
        let err = ProfileStore::deserialize_profile(content, EmergencyStopHotkey::default())
            .expect_err("Should reject future schema version");
        assert!(matches!(
            err,
            StorageError::UnsupportedSchemaVersion {
                found: 999,
                expected: CURRENT_SCHEMA_VERSION
            }
        ));
    }
}
