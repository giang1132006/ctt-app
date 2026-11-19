use ctt_app::domain::config::EmergencyStopHotkey;
use ctt_app::domain::keys::{KeyState, KeyboardKey, ModifierMask, MouseButton, MouseWheel};
use ctt_app::domain::mapping::{CompiledMappingTable, KeyBindingRule, MappingTarget};
use ctt_app::engine::command::EngineStatus;
use ctt_app::engine::remapper::{RemapEngine, DEFAULT_WHEEL_DELTA};
use ctt_app::platform::mock::MockInputSynthesizer;
use ctt_app::platform::{HookDecision, RawKeyEvent, RawMouseButtonEvent, SyntheticOutput};

fn build_active_engine(rules: Vec<KeyBindingRule>) -> (RemapEngine, MockInputSynthesizer) {
    let mut synth = MockInputSynthesizer::new();
    let mut engine = RemapEngine::new(EmergencyStopHotkey::default());
    engine
        .switch_table(CompiledMappingTable::compile(&rules), &mut synth)
        .expect("Switching table should succeed");
    engine
        .set_enabled(true, &mut synth)
        .expect("Enabling engine should succeed");
    (engine, synth)
}

#[test]
fn every_standard_key_dispatches_and_releases_cleanly_in_engine() {
    for (idx, &source) in KeyboardKey::ALL_STANDARD.iter().enumerate() {
        let target = KeyboardKey::ALL_STANDARD[(idx + 1) % KeyboardKey::ALL_STANDARD.len()];
        let rules = vec![KeyBindingRule::new(
            1,
            "All Keys Test",
            source,
            MappingTarget::Keyboard(target),
        )];
        let (mut engine, mut synth) = build_active_engine(rules);

        assert_eq!(
            engine
                .handle_key_event(RawKeyEvent::physical(source, KeyState::Down), &mut synth)
                .unwrap(),
            HookDecision::Suppress
        );
        // OS auto-repeat re-emits target KeyDown
        assert_eq!(
            engine
                .handle_key_event(RawKeyEvent::physical(source, KeyState::Down), &mut synth)
                .unwrap(),
            HookDecision::Suppress
        );
        assert_eq!(
            engine
                .handle_key_event(RawKeyEvent::physical(source, KeyState::Up), &mut synth)
                .unwrap(),
            HookDecision::Suppress
        );

        assert_eq!(
            synth.events(),
            &[
                SyntheticOutput::Key {
                    key: target,
                    state: KeyState::Down,
                },
                SyntheticOutput::Key {
                    key: target,
                    state: KeyState::Down,
                },
                SyntheticOutput::Key {
                    key: target,
                    state: KeyState::Up,
                },
            ]
        );
    }
}

#[test]
fn key_combo_presses_repeats_and_releases_modifiers_in_reverse_order() {
    let rules = vec![KeyBindingRule::new(
        1,
        "F1 -> Ctrl+Shift+C",
        KeyboardKey::F1,
        MappingTarget::KeyCombo {
            modifiers: ModifierMask::CTRL.with(ModifierMask::SHIFT),
            key: KeyboardKey::KeyC,
        },
    )];
    let (mut engine, mut synth) = build_active_engine(rules);

    engine
        .handle_key_event(
            RawKeyEvent::physical(KeyboardKey::F1, KeyState::Down),
            &mut synth,
        )
        .unwrap();
    engine
        .handle_key_event(
            RawKeyEvent::physical(KeyboardKey::F1, KeyState::Down),
            &mut synth,
        )
        .unwrap();
    engine
        .handle_key_event(
            RawKeyEvent::physical(KeyboardKey::F1, KeyState::Up),
            &mut synth,
        )
        .unwrap();

    assert_eq!(
        synth.events(),
        &[
            SyntheticOutput::Key {
                key: KeyboardKey::LeftCtrl,
                state: KeyState::Down,
            },
            SyntheticOutput::Key {
                key: KeyboardKey::LeftShift,
                state: KeyState::Down,
            },
            SyntheticOutput::Key {
                key: KeyboardKey::KeyC,
                state: KeyState::Down,
            },
            // Auto-repeat only repeats the primary key while modifiers stay held
            SyntheticOutput::Key {
                key: KeyboardKey::KeyC,
                state: KeyState::Down,
            },
            // Release primary key first, then modifiers in reverse order
            SyntheticOutput::Key {
                key: KeyboardKey::KeyC,
                state: KeyState::Up,
            },
            SyntheticOutput::Key {
                key: KeyboardKey::LeftShift,
                state: KeyState::Up,
            },
            SyntheticOutput::Key {
                key: KeyboardKey::LeftCtrl,
                state: KeyState::Up,
            },
        ]
    );
}

#[test]
fn key_to_all_mouse_buttons_suppress_autorepeat_and_release_on_up() {
    let bindings = [
        (KeyboardKey::Digit1, MouseButton::Left),
        (KeyboardKey::Digit2, MouseButton::Right),
        (KeyboardKey::Digit3, MouseButton::Middle),
        (KeyboardKey::Digit4, MouseButton::XButton1),
        (KeyboardKey::Digit5, MouseButton::XButton2),
    ];

    let rules = bindings
        .iter()
        .enumerate()
        .map(|(i, &(src, btn))| {
            KeyBindingRule::new(i as u32 + 1, "Mouse", src, MappingTarget::MouseButton(btn))
        })
        .collect();
    let (mut engine, mut synth) = build_active_engine(rules);

    for (src, btn) in bindings {
        synth.drain_events();

        engine
            .handle_key_event(RawKeyEvent::physical(src, KeyState::Down), &mut synth)
            .unwrap();
        // Repeat presses while held must NOT emit extra MouseDown events
        engine
            .handle_key_event(RawKeyEvent::physical(src, KeyState::Down), &mut synth)
            .unwrap();
        engine
            .handle_key_event(RawKeyEvent::physical(src, KeyState::Up), &mut synth)
            .unwrap();

        assert_eq!(
            synth.events(),
            &[
                SyntheticOutput::MouseButton {
                    button: btn,
                    state: KeyState::Down,
                },
                SyntheticOutput::MouseButton {
                    button: btn,
                    state: KeyState::Up,
                },
            ]
        );
    }
}

#[test]
fn key_to_all_mouse_wheel_directions_emit_ticks_on_press_and_repeat() {
    let bindings = [
        (KeyboardKey::PageUp, MouseWheel::ScrollUp),
        (KeyboardKey::PageDown, MouseWheel::ScrollDown),
        (KeyboardKey::Home, MouseWheel::ScrollLeft),
        (KeyboardKey::End, MouseWheel::ScrollRight),
    ];

    let rules = bindings
        .iter()
        .enumerate()
        .map(|(i, &(src, wheel))| {
            KeyBindingRule::new(i as u32 + 1, "Wheel", src, MappingTarget::MouseWheel(wheel))
        })
        .collect();
    let (mut engine, mut synth) = build_active_engine(rules);

    for (src, wheel) in bindings {
        synth.drain_events();

        assert_eq!(
            engine
                .handle_key_event(RawKeyEvent::physical(src, KeyState::Down), &mut synth)
                .unwrap(),
            HookDecision::Suppress
        );
        assert_eq!(
            engine
                .handle_key_event(RawKeyEvent::physical(src, KeyState::Down), &mut synth)
                .unwrap(),
            HookDecision::Suppress
        );
        assert_eq!(
            engine
                .handle_key_event(RawKeyEvent::physical(src, KeyState::Up), &mut synth)
                .unwrap(),
            HookDecision::Suppress
        );

        assert_eq!(
            synth.events(),
            &[
                SyntheticOutput::MouseWheel {
                    direction: wheel,
                    delta: DEFAULT_WHEEL_DELTA,
                },
                SyntheticOutput::MouseWheel {
                    direction: wheel,
                    delta: DEFAULT_WHEEL_DELTA,
                },
            ]
        );
    }
}

#[test]
fn profile_switch_and_disable_mid_hold_release_all_held_outputs() {
    let rules = vec![
        KeyBindingRule::new(
            1,
            "Q -> W",
            KeyboardKey::KeyQ,
            MappingTarget::Keyboard(KeyboardKey::KeyW),
        ),
        KeyBindingRule::new(
            2,
            "Space -> LeftClick",
            KeyboardKey::Space,
            MappingTarget::MouseButton(MouseButton::Left),
        ),
    ];
    let (mut engine, mut synth) = build_active_engine(rules);

    // Hold both Q and Space
    engine
        .handle_key_event(
            RawKeyEvent::physical(KeyboardKey::KeyQ, KeyState::Down),
            &mut synth,
        )
        .unwrap();
    engine
        .handle_key_event(
            RawKeyEvent::physical(KeyboardKey::Space, KeyState::Down),
            &mut synth,
        )
        .unwrap();
    assert_eq!(engine.snapshot().active_holds_count, 2);

    synth.drain_events();
    // Switch to an empty profile mid-hold -> must release Space (MouseLeft) and Q (KeyW) in reverse order
    engine
        .switch_table(CompiledMappingTable::default(), &mut synth)
        .unwrap();

    assert_eq!(engine.snapshot().active_holds_count, 0);
    assert_eq!(
        synth.events(),
        &[
            SyntheticOutput::MouseButton {
                button: MouseButton::Left,
                state: KeyState::Up,
            },
            SyntheticOutput::Key {
                key: KeyboardKey::KeyW,
                state: KeyState::Up,
            },
        ]
    );
}

#[test]
fn injected_events_pass_through_to_prevent_remap_loops() {
    let rules = vec![
        KeyBindingRule::new(
            1,
            "A -> B",
            KeyboardKey::KeyA,
            MappingTarget::Keyboard(KeyboardKey::KeyB),
        ),
        KeyBindingRule::new(
            2,
            "B -> A",
            KeyboardKey::KeyB,
            MappingTarget::Keyboard(KeyboardKey::KeyA),
        ),
    ];
    let (mut engine, mut synth) = build_active_engine(rules);

    // Physical A emits B
    assert_eq!(
        engine
            .handle_key_event(
                RawKeyEvent::physical(KeyboardKey::KeyA, KeyState::Down),
                &mut synth
            )
            .unwrap(),
        HookDecision::Suppress
    );
    assert_eq!(
        synth.events(),
        &[SyntheticOutput::Key {
            key: KeyboardKey::KeyB,
            state: KeyState::Down,
        }]
    );

    // The resulting injected B from OS hook must pass through untouched (no B -> A loop)
    synth.drain_events();
    assert_eq!(
        engine
            .handle_key_event(
                RawKeyEvent::injected(KeyboardKey::KeyB, KeyState::Down),
                &mut synth
            )
            .unwrap(),
        HookDecision::PassThrough
    );
    assert!(synth.events().is_empty());
}

#[test]
fn emergency_stop_hotkey_releases_held_targets_and_stops_engine() {
    let rules = vec![KeyBindingRule::new(
        1,
        "Space -> RightClick",
        KeyboardKey::Space,
        MappingTarget::MouseButton(MouseButton::Right),
    )];
    let (mut engine, mut synth) = build_active_engine(rules);

    engine
        .handle_key_event(
            RawKeyEvent::physical(KeyboardKey::Space, KeyState::Down),
            &mut synth,
        )
        .unwrap();
    assert_eq!(engine.snapshot().active_holds_count, 1);

    // Press Ctrl + Shift + F12 physically
    engine
        .handle_key_event(
            RawKeyEvent::physical(KeyboardKey::LeftCtrl, KeyState::Down),
            &mut synth,
        )
        .unwrap();
    engine
        .handle_key_event(
            RawKeyEvent::physical(KeyboardKey::LeftShift, KeyState::Down),
            &mut synth,
        )
        .unwrap();

    synth.drain_events();
    let stop_decision = engine
        .handle_key_event(
            RawKeyEvent::physical(KeyboardKey::F12, KeyState::Down),
            &mut synth,
        )
        .unwrap();

    assert_eq!(stop_decision, HookDecision::Suppress);
    assert_eq!(engine.status(), EngineStatus::EmergencyStopped);
    assert_eq!(engine.snapshot().active_holds_count, 0);
    assert_eq!(
        synth.events(),
        &[SyntheticOutput::MouseButton {
            button: MouseButton::Right,
            state: KeyState::Up,
        }]
    );
}

#[test]
fn mouse_button_sources_dispatch_and_release_keyboard_combo_and_mouse_targets() {
    let rules = vec![
        KeyBindingRule::new(
            1,
            "X1 -> R",
            MouseButton::XButton1,
            MappingTarget::Keyboard(KeyboardKey::KeyR),
        ),
        KeyBindingRule::new(
            2,
            "X2 -> Ctrl+V",
            MouseButton::XButton2,
            MappingTarget::KeyCombo {
                modifiers: ModifierMask::CTRL,
                key: KeyboardKey::KeyV,
            },
        ),
        KeyBindingRule::new(
            3,
            "Middle -> RightClick",
            MouseButton::Middle,
            MappingTarget::MouseButton(MouseButton::Right),
        ),
    ];
    let (mut engine, mut synth) = build_active_engine(rules);

    // Injected mouse button events must pass through untouched
    assert_eq!(
        engine
            .handle_mouse_button_event(
                RawMouseButtonEvent::injected(MouseButton::XButton1, KeyState::Down),
                &mut synth,
            )
            .unwrap(),
        HookDecision::PassThrough
    );
    assert!(synth.events().is_empty());

    // Physical Mouse X1 -> KeyR down & up
    assert_eq!(
        engine
            .handle_mouse_button_event(
                RawMouseButtonEvent::physical(MouseButton::XButton1, KeyState::Down),
                &mut synth,
            )
            .unwrap(),
        HookDecision::Suppress
    );
    assert_eq!(
        engine
            .handle_mouse_button_event(
                RawMouseButtonEvent::physical(MouseButton::XButton1, KeyState::Up),
                &mut synth,
            )
            .unwrap(),
        HookDecision::Suppress
    );
    assert_eq!(
        synth.drain_events(),
        vec![
            SyntheticOutput::Key {
                key: KeyboardKey::KeyR,
                state: KeyState::Down,
            },
            SyntheticOutput::Key {
                key: KeyboardKey::KeyR,
                state: KeyState::Up,
            },
        ]
    );

    // Physical Mouse X2 -> Ctrl+V down & up
    engine
        .handle_mouse_button_event(
            RawMouseButtonEvent::physical(MouseButton::XButton2, KeyState::Down),
            &mut synth,
        )
        .unwrap();
    engine
        .handle_mouse_button_event(
            RawMouseButtonEvent::physical(MouseButton::XButton2, KeyState::Up),
            &mut synth,
        )
        .unwrap();
    assert_eq!(
        synth.drain_events(),
        vec![
            SyntheticOutput::Key {
                key: KeyboardKey::LeftCtrl,
                state: KeyState::Down,
            },
            SyntheticOutput::Key {
                key: KeyboardKey::KeyV,
                state: KeyState::Down,
            },
            SyntheticOutput::Key {
                key: KeyboardKey::KeyV,
                state: KeyState::Up,
            },
            SyntheticOutput::Key {
                key: KeyboardKey::LeftCtrl,
                state: KeyState::Up,
            },
        ]
    );

    // Hold Middle -> RightClick and trigger Emergency Stop (Ctrl+Shift+F12) mid-hold
    engine
        .handle_mouse_button_event(
            RawMouseButtonEvent::physical(MouseButton::Middle, KeyState::Down),
            &mut synth,
        )
        .unwrap();
    assert_eq!(engine.snapshot().active_holds_count, 1);

    synth.drain_events();
    engine.emergency_stop_all(&mut synth).unwrap();
    assert_eq!(engine.snapshot().active_holds_count, 0);
    assert_eq!(
        synth.events(),
        &[SyntheticOutput::MouseButton {
            button: MouseButton::Right,
            state: KeyState::Up,
        }]
    );
}
