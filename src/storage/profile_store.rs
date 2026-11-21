//! Schema-versioned profile serialization and atomic filesystem persistence.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::domain::config::{EmergencyStopHotkey, CURRENT_SCHEMA_VERSION};
use crate::domain::keys::{InputSource, KeyboardKey, MouseButton};
use crate::domain::macros::{
    compile_clips, Clip, ClipKind, LoopRegion, MacroDefinition, MacroId, MacroPlaybackMode,
    MacroTrigger, MIN_TRACKS,
};
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

        for mac in &profile.macros {
            let trigger_str = match mac.trigger {
                Some(trig) => format!("{}~{}", trig.source.short_code(), trig.mode.short_code()),
                None => "NONE".to_string(),
            };
            let loop_str = match mac.loop_region {
                Some(lp) => format!("{}-{}", lp.start_ms, lp.end_ms),
                None => "NONE".to_string(),
            };
            let clips_str = mac
                .clips
                .iter()
                .map(serialize_clip_token)
                .collect::<Vec<_>>()
                .join(",");
            out.push_str(&format!(
                "macro={}|{}|{}|{}|{}|{}|{}\n",
                mac.id.0,
                mac.enabled,
                escape_field(&mac.name),
                trigger_str,
                loop_str,
                mac.tracks,
                clips_str
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
        let mut macros: Vec<MacroDefinition> = Vec::new();

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
                "macro" => {
                    macros.push(parse_macro_line(
                        value,
                        found_version_or_default(schema_version),
                        line_idx + 1,
                    )?);
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
            macros,
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

    /// Resolves the default directory for storing persisted profiles.
    ///
    /// Prefers a portable `profiles/` folder alongside the executable if it already exists,
    /// falling back to `%APPDATA%\ctt-app\profiles` on Windows or `./profiles`.
    pub fn default_profiles_dir() -> PathBuf {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let portable_dir = exe_dir.join("profiles");
                if portable_dir.is_dir() {
                    return portable_dir;
                }
            }
        }
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("ctt-app").join("profiles");
        }
        PathBuf::from("profiles")
    }

    /// Returns the deterministic file path for `profile_id` inside `dir`.
    pub fn profile_file_path(dir: &Path, profile_id: u32) -> PathBuf {
        dir.join(format!("profile_{profile_id}.ctt"))
    }

    /// Saves `profile` atomically into `dir/profile_<id>.ctt`.
    pub fn save_profile_to_dir(dir: &Path, profile: &Profile) -> Result<PathBuf, StorageError> {
        let path = Self::profile_file_path(dir, profile.id);
        Self::save_profile_atomic(&path, profile)?;
        Ok(path)
    }

    /// Deletes the persisted file for `profile_id` from `dir` if it exists.
    pub fn delete_profile_from_dir(dir: &Path, profile_id: u32) -> Result<(), StorageError> {
        let path = Self::profile_file_path(dir, profile_id);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(StorageError::Io(err)),
        }
    }

    /// Loads all valid `.ctt` profile files from `dir`, sorted by [`Profile::id`].
    ///
    /// Malformed or unreadable files are skipped and reported in the returned warning vector
    /// rather than panicking or blocking startup.
    pub fn load_all_profiles(
        dir: &Path,
        emergency_stop: EmergencyStopHotkey,
    ) -> (Vec<Profile>, Vec<String>) {
        let mut profiles = Vec::new();
        let mut warnings = Vec::new();

        let entries = match fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return (profiles, warnings);
            }
            Err(err) => {
                warnings.push(format!("Không thể đọc thư mục profile: {err}"));
                return (profiles, warnings);
            }
        };

        for entry_res in entries {
            let Ok(entry) = entry_res else {
                continue;
            };
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("ctt") {
                continue;
            }
            match Self::load_profile(&path, emergency_stop) {
                Ok(profile) => profiles.push(profile),
                Err(err) => {
                    let fname = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    warnings.push(format!("Bỏ qua file '{fname}': {err}"));
                }
            }
        }

        profiles.sort_by_key(|p| p.id);
        (profiles, warnings)
    }

    /// Persists the currently selected `active_profile_id` atomically into `dir/app_state.cfg`.
    pub fn save_active_profile_id(
        dir: &Path,
        active_profile_id: Option<u32>,
    ) -> Result<(), StorageError> {
        fs::create_dir_all(dir)?;
        let path = dir.join("app_state.cfg");
        let tmp_path = temp_sibling_path(&path);
        let value = active_profile_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "NONE".to_string());
        let payload = format!(
            "# ctt-app state\nschema_version={CURRENT_SCHEMA_VERSION}\nactive_profile_id={value}\n"
        );

        {
            let mut file = File::create(&tmp_path)?;
            file.write_all(payload.as_bytes())?;
            file.sync_all()?;
        }

        if let Err(err) = fs::rename(&tmp_path, &path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(StorageError::Io(err));
        }

        Ok(())
    }

    /// Reads the last selected `active_profile_id` from `dir/app_state.cfg`, if present and valid.
    pub fn load_active_profile_id(dir: &Path) -> Option<u32> {
        let content = fs::read_to_string(dir.join("app_state.cfg")).ok()?;
        for raw_line in content.lines() {
            let line = raw_line.trim();
            if let Some(val) = line.strip_prefix("active_profile_id=") {
                if val.eq_ignore_ascii_case("NONE") {
                    return None;
                }
                return val.parse::<u32>().ok();
            }
        }
        None
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

fn found_version_or_default(version: Option<u32>) -> u32 {
    version.unwrap_or(CURRENT_SCHEMA_VERSION)
}

fn serialize_clip_token(clip: &Clip) -> String {
    match clip.kind {
        ClipKind::Key { key } => {
            let key_code = key
                .map(KeyboardKey::short_code)
                .unwrap_or_else(|| "NONE".to_string());
            format!(
                "K:{}:{}:{}:{}:{}",
                clip.id, clip.track, clip.start_ms, clip.dur_ms, key_code
            )
        }
        ClipKind::Mouse { button } => format!(
            "M:{}:{}:{}:{}:{}",
            clip.id,
            clip.track,
            clip.start_ms,
            clip.dur_ms,
            button.short_code()
        ),
    }
}

fn parse_clip_token(raw: &str, line_num: usize) -> Result<Clip, StorageError> {
    let parts: Vec<&str> = raw.splitn(6, ':').collect();
    if parts.len() != 6 {
        return Err(StorageError::ParseError(format!(
            "Line {line_num}: malformed macro clip token '{raw}'"
        )));
    }
    let id = parts[1].parse::<u32>().map_err(|_| {
        StorageError::ParseError(format!("Line {line_num}: invalid clip id '{}'", parts[1]))
    })?;
    let track = parts[2].parse::<usize>().map_err(|_| {
        StorageError::ParseError(format!(
            "Line {line_num}: invalid clip track '{}'",
            parts[2]
        ))
    })?;
    let start_ms = parts[3].parse::<u64>().map_err(|_| {
        StorageError::ParseError(format!(
            "Line {line_num}: invalid clip start_ms '{}'",
            parts[3]
        ))
    })?;
    let dur_ms = parts[4]
        .parse::<u64>()
        .map_err(|_| {
            StorageError::ParseError(format!(
                "Line {line_num}: invalid clip dur_ms '{}'",
                parts[4]
            ))
        })?
        .max(1);

    let kind = match parts[0] {
        "K" => {
            let key = if parts[5].eq_ignore_ascii_case("NONE") {
                None
            } else {
                Some(KeyboardKey::from_short_code(parts[5]).ok_or_else(|| {
                    StorageError::ParseError(format!(
                        "Line {line_num}: unknown keyboard key '{}'",
                        parts[5]
                    ))
                })?)
            };
            ClipKind::Key { key }
        }
        "M" => {
            let button = MouseButton::from_short_code(parts[5]).ok_or_else(|| {
                StorageError::ParseError(format!(
                    "Line {line_num}: unknown mouse button '{}'",
                    parts[5]
                ))
            })?;
            ClipKind::Mouse { button }
        }
        other => {
            return Err(StorageError::ParseError(format!(
                "Line {line_num}: unknown clip kind '{other}'"
            )))
        }
    };

    Ok(Clip {
        id,
        kind,
        track,
        start_ms,
        dur_ms,
    })
}

fn parse_macro_line(
    raw: &str,
    schema_version: u32,
    line_num: usize,
) -> Result<MacroDefinition, StorageError> {
    let parts: Vec<&str> = raw.splitn(7, '|').collect();
    if parts.len() != 7 {
        return Err(StorageError::ParseError(format!(
            "Line {line_num}: expected 7 '|' separated macro fields"
        )));
    }

    let id = parts[0].parse::<u32>().map_err(|_| {
        StorageError::ParseError(format!("Line {line_num}: invalid macro id '{}'", parts[0]))
    })?;
    let enabled = parts[1].parse::<bool>().map_err(|_| {
        StorageError::ParseError(format!(
            "Line {line_num}: invalid macro enabled flag '{}'",
            parts[1]
        ))
    })?;
    let name = unescape_field(parts[2]).ok_or_else(|| {
        StorageError::ParseError(format!(
            "Line {line_num}: invalid escape sequence in macro name"
        ))
    })?;

    let trigger = if parts[3].eq_ignore_ascii_case("NONE") {
        None
    } else {
        let (src_raw, mode_raw) = parts[3].split_once('~').ok_or_else(|| {
            StorageError::ParseError(format!(
                "Line {line_num}: malformed macro trigger '{}'",
                parts[3]
            ))
        })?;
        let source = InputSource::from_short_code(src_raw).ok_or_else(|| {
            StorageError::ParseError(format!(
                "Line {line_num}: unknown macro trigger source '{src_raw}'"
            ))
        })?;
        let mode = MacroPlaybackMode::from_short_code(mode_raw).ok_or_else(|| {
            StorageError::ParseError(format!(
                "Line {line_num}: unknown macro playback mode '{mode_raw}'"
            ))
        })?;
        Some(MacroTrigger { source, mode })
    };

    let loop_region = if parts[4].eq_ignore_ascii_case("NONE") {
        None
    } else {
        let (ls_raw, le_raw) = parts[4].split_once('-').ok_or_else(|| {
            StorageError::ParseError(format!(
                "Line {line_num}: malformed loop region '{}'",
                parts[4]
            ))
        })?;
        let start_ms = ls_raw.parse::<u64>().map_err(|_| {
            StorageError::ParseError(format!("Line {line_num}: invalid loop start '{ls_raw}'"))
        })?;
        let end_ms = le_raw.parse::<u64>().map_err(|_| {
            StorageError::ParseError(format!("Line {line_num}: invalid loop end '{le_raw}'"))
        })?;
        Some(LoopRegion { start_ms, end_ms })
    };

    let tracks = parts[5]
        .parse::<usize>()
        .map_err(|_| {
            StorageError::ParseError(format!(
                "Line {line_num}: invalid track count '{}'",
                parts[5]
            ))
        })?
        .max(MIN_TRACKS);

    let mut clips = Vec::new();
    if !parts[6].trim().is_empty() {
        for tok in parts[6].split(',') {
            clips.push(parse_clip_token(tok, line_num)?);
        }
    }

    let compiled_steps = compile_clips(&clips, loop_region);
    Ok(MacroDefinition {
        id: MacroId(id),
        name,
        schema_version,
        enabled,
        trigger,
        clips,
        tracks,
        loop_region,
        compiled_steps,
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
    fn serialize_and_deserialize_preserves_multitrack_macros() {
        let mut profile = Profile::new(9, "Macro | Profile");
        let mac = MacroDefinition::new(
            MacroId(1),
            "Jump | Attack",
            Some(MacroTrigger {
                source: InputSource::MouseButton(MouseButton::XButton1),
                mode: MacroPlaybackMode::HoldLoop,
            }),
            vec![
                Clip {
                    id: 1,
                    kind: ClipKind::Mouse {
                        button: MouseButton::Left,
                    },
                    track: 0,
                    start_ms: 0,
                    dur_ms: 250,
                },
                Clip {
                    id: 2,
                    kind: ClipKind::Key {
                        key: Some(KeyboardKey::KeyE),
                    },
                    track: 1,
                    start_ms: 80,
                    dur_ms: 100,
                },
            ],
            4,
            Some(LoopRegion {
                start_ms: 0,
                end_ms: 250,
            }),
        );
        profile.upsert_macro(mac);

        let raw = ProfileStore::serialize_profile(&profile);
        let parsed = ProfileStore::deserialize_profile(&raw, EmergencyStopHotkey::default())
            .expect("Serialized profile with macros should parse cleanly");
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
