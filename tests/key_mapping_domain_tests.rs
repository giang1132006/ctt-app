use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use ctt_app::domain::config::EmergencyStopHotkey;
use ctt_app::domain::keys::{
    InputSource, KeyboardKey, ModifierMask, MouseButton, MouseWheel, ScanCode,
};
use ctt_app::domain::mapping::{CompiledMappingTable, KeyBindingRule, MappingTarget};
use ctt_app::domain::profile::Profile;
use ctt_app::domain::validation::{validate_profile_rules, MappingError, MappingWarning};
use ctt_app::platform::windows::keymap::{keyboard_key_to_win32, win32_to_keyboard_key};
use ctt_app::storage::profile_store::ProfileStore;

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), nanos));
    fs::create_dir_all(&dir).expect("Create temporary directory for test");
    dir
}

#[test]
fn every_keyboard_key_has_valid_identity_and_win32_round_trip() {
    assert_eq!(KeyboardKey::ALL_STANDARD.len(), 123);

    let mut short_codes = HashSet::new();
    for &key in KeyboardKey::ALL_STANDARD {
        assert!(!key.display_name().is_empty());
        let code = key.short_code();
        assert!(short_codes.insert(code.clone()));
        assert_eq!(KeyboardKey::from_short_code(&code), Some(key));

        let win32 = keyboard_key_to_win32(key);
        assert_eq!(
            win32_to_keyboard_key(win32.vk, win32.scan_code, win32.extended),
            key
        );
    }

    let oem = KeyboardKey::OemScanCode(ScanCode::new(0x007E, true));
    assert_eq!(KeyboardKey::from_short_code(&oem.short_code()), Some(oem));
}

#[test]
fn every_keyboard_key_can_bind_to_any_keyboard_key_combo_or_mouse_target() {
    let mut all_sources: Vec<InputSource> = InputSource::all_standard();
    all_sources.push(InputSource::Keyboard(KeyboardKey::OemScanCode(
        ScanCode::new(0x0070, false),
    )));

    let mut keys_with_oem = KeyboardKey::ALL_STANDARD.to_vec();
    keys_with_oem.push(KeyboardKey::OemScanCode(ScanCode::new(0x0070, false)));

    for &source in &all_sources {
        // 1. Bind to every keyboard key
        for &target_key in &keys_with_oem {
            let rule =
                KeyBindingRule::new(1, "Src->Key", source, MappingTarget::Keyboard(target_key));
            let table = CompiledMappingTable::compile(&[rule]);
            assert_eq!(
                table.lookup(source),
                Some(MappingTarget::Keyboard(target_key))
            );
        }

        // 2. Bind to KeyCombo
        let combo = MappingTarget::KeyCombo {
            modifiers: ModifierMask::CTRL.with(ModifierMask::SHIFT),
            key: KeyboardKey::KeyK,
        };
        let combo_table =
            CompiledMappingTable::compile(&[KeyBindingRule::new(2, "Src->Combo", source, combo)]);
        assert_eq!(combo_table.lookup(source), Some(combo));

        // 3. Bind to every MouseButton
        for &btn in MouseButton::ALL {
            let target = MappingTarget::MouseButton(btn);
            let table = CompiledMappingTable::compile(&[KeyBindingRule::new(
                3,
                "Src->Mouse",
                source,
                target,
            )]);
            assert_eq!(table.lookup(source), Some(target));
        }

        // 4. Bind to every MouseWheel direction
        for &wheel in MouseWheel::ALL {
            let target = MappingTarget::MouseWheel(wheel);
            let table = CompiledMappingTable::compile(&[KeyBindingRule::new(
                4,
                "Src->Wheel",
                source,
                target,
            )]);
            assert_eq!(table.lookup(source), Some(target));
        }

        // 5. Bind to Disabled
        let disabled_table = CompiledMappingTable::compile(&[KeyBindingRule::new(
            5,
            "Disable",
            source,
            MappingTarget::Disabled,
        )]);
        assert_eq!(disabled_table.lookup(source), Some(MappingTarget::Disabled));
    }
}

#[test]
fn validation_catches_duplicates_emergency_stop_identity_swaps_and_chains() {
    let rules = vec![
        KeyBindingRule::new(
            1,
            "Duplicate 1",
            KeyboardKey::KeyQ,
            MappingTarget::Keyboard(KeyboardKey::KeyW),
        ),
        KeyBindingRule::new(
            2,
            "Duplicate 2",
            KeyboardKey::KeyQ,
            MappingTarget::MouseButton(MouseButton::Left),
        ),
        KeyBindingRule::new(3, "Disable F12", KeyboardKey::F12, MappingTarget::Disabled),
        KeyBindingRule::new(
            4,
            "Identity",
            KeyboardKey::KeyZ,
            MappingTarget::Keyboard(KeyboardKey::KeyZ),
        ),
        KeyBindingRule::new(
            5,
            "Swap W->E",
            KeyboardKey::KeyW,
            MappingTarget::Keyboard(KeyboardKey::KeyE),
        ),
        KeyBindingRule::new(
            6,
            "Swap E->W",
            KeyboardKey::KeyE,
            MappingTarget::Keyboard(KeyboardKey::KeyW),
        ),
        KeyBindingRule::new(
            7,
            "Chain R->W",
            KeyboardKey::KeyR,
            MappingTarget::Keyboard(KeyboardKey::KeyW),
        ),
    ];

    let report = validate_profile_rules(&rules, EmergencyStopHotkey::default());
    assert!(!report.is_valid());
    assert!(report.errors.contains(&MappingError::DuplicateSourceKey {
        key: InputSource::Keyboard(KeyboardKey::KeyQ),
        first_rule_id: 1,
        duplicate_rule_id: 2,
    }));
    assert!(report
        .errors
        .contains(&MappingError::ReservedEmergencyStopKey {
            rule_id: 3,
            key: KeyboardKey::F12,
        }));
    assert!(report.warnings.contains(&MappingWarning::IdentityMapping {
        rule_id: 4,
        key: InputSource::Keyboard(KeyboardKey::KeyZ),
    }));
    assert!(report.warnings.contains(&MappingWarning::KeySwapDetected {
        rule_a_id: 5,
        rule_b_id: 6,
        key_a: InputSource::Keyboard(KeyboardKey::KeyW),
        key_b: InputSource::Keyboard(KeyboardKey::KeyE),
    }));
    assert!(report
        .warnings
        .contains(&MappingWarning::ChainedMappingDetected {
            from_rule_id: 7,
            to_rule_id: 5,
            via_key: InputSource::Keyboard(KeyboardKey::KeyW),
        }));
}

#[test]
fn profile_store_atomic_save_and_load_round_trips() {
    let dir = unique_temp_dir("ctt_app_store_test");
    let file_path = dir.join("gaming_profile.ctt");

    let mut profile = Profile::new(42, "FPS Gaming | Pro");
    profile.set_or_add_binding(
        KeyboardKey::CapsLock,
        MappingTarget::Keyboard(KeyboardKey::Escape),
        "Caps -> Esc",
    );
    profile.set_or_add_binding(
        KeyboardKey::Space,
        MappingTarget::MouseButton(MouseButton::Left),
        "Space -> Click",
    );
    profile.set_or_add_binding(
        KeyboardKey::KeyF,
        MappingTarget::KeyCombo {
            modifiers: ModifierMask::CTRL.with(ModifierMask::ALT),
            key: KeyboardKey::Delete,
        },
        "Combo",
    );
    profile.set_or_add_binding(
        KeyboardKey::PageUp,
        MappingTarget::MouseWheel(MouseWheel::ScrollUp),
        "Wheel Up",
    );

    ProfileStore::save_profile_atomic(&file_path, &profile).expect("Atomic save should succeed");
    let loaded = ProfileStore::load_profile(&file_path, EmergencyStopHotkey::default())
        .expect("Load should validate and parse saved profile");

    assert_eq!(loaded, profile);
    assert!(!dir.join("gaming_profile.ctt.tmp").exists());

    let _ = fs::remove_dir_all(&dir);
}
