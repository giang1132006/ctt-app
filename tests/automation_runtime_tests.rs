use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use ctt_app::domain::config::EmergencyStopHotkey;
use ctt_app::domain::keys::{InputSource, KeyState, KeyboardKey, ModifierMask, MouseButton};
use ctt_app::domain::macros::{
    Clip, ClipKind, LoopRegion, MacroDefinition, MacroId, MacroPlaybackMode, MacroTrigger,
};
use ctt_app::domain::mapping::{CompiledMappingTable, KeyBindingRule, MappingTarget};
use ctt_app::engine::command::EngineStatus;
use ctt_app::engine::macro_player::MacroPlayer;
use ctt_app::engine::remapper::RemapEngine;
use ctt_app::engine::runtime::InputDispatcher;
use ctt_app::platform::mock::MockInputSynthesizer;
use ctt_app::platform::{
    HookDecision, InputSynthesizer, PlatformError, RawInputEvent, RawKeyEvent, RawMouseButtonEvent,
    SyntheticOutput,
};

#[derive(Clone, Default)]
struct SharedMockSynth {
    events: Arc<Mutex<Vec<SyntheticOutput>>>,
}

impl SharedMockSynth {
    fn snapshot(&self) -> Vec<SyntheticOutput> {
        self.events.lock().unwrap().clone()
    }
}

impl InputSynthesizer for SharedMockSynth {
    fn synthesize(&mut self, output: SyntheticOutput) -> Result<(), PlatformError> {
        self.events.lock().unwrap().push(output);
        Ok(())
    }
}

#[test]
fn hold_loop_macro_on_mouse_side_button_loops_and_releases_cleanly_on_up() {
    let macro_sink = SharedMockSynth::default();
    let mut remap_sink = SharedMockSynth::default();
    let player = MacroPlayer::new(Box::new(macro_sink.clone()));
    let mut dispatcher = InputDispatcher::new(EmergencyStopHotkey::default(), player);

    let mac = MacroDefinition::new(
        MacroId(10),
        "Hold Fire",
        Some(MacroTrigger {
            source: InputSource::MouseButton(MouseButton::XButton2),
            mode: MacroPlaybackMode::HoldLoop,
        }),
        vec![Clip {
            id: 1,
            kind: ClipKind::Mouse {
                button: MouseButton::Left,
            },
            track: 0,
            start_ms: 0,
            dur_ms: 15,
        }],
        4,
        Some(LoopRegion {
            start_ms: 0,
            end_ms: 25,
        }),
    );

    dispatcher
        .sync_profile(CompiledMappingTable::default(), &[mac], &mut remap_sink)
        .unwrap();
    dispatcher.set_enabled(true, &mut remap_sink).unwrap();

    // Hold X2 -> should start looping
    assert_eq!(
        dispatcher.handle_raw_event(
            RawInputEvent::MouseButton(RawMouseButtonEvent::physical(
                MouseButton::XButton2,
                KeyState::Down
            )),
            &mut remap_sink,
        ),
        HookDecision::Suppress
    );

    // Wait for at least 2 clicks (4 events: down/up/down/up)
    let deadline = Instant::now() + Duration::from_millis(400);
    while macro_sink.snapshot().len() < 4 && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(
        macro_sink.snapshot().len() >= 4,
        "Expected HoldLoop to repeat multiple iterations"
    );

    // Release X2 -> stops macro immediately and balances Down/Up
    assert_eq!(
        dispatcher.handle_raw_event(
            RawInputEvent::MouseButton(RawMouseButtonEvent::physical(
                MouseButton::XButton2,
                KeyState::Up
            )),
            &mut remap_sink,
        ),
        HookDecision::Suppress
    );

    let deadline = Instant::now() + Duration::from_millis(300);
    while dispatcher.is_macro_playing() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(!dispatcher.is_macro_playing());

    let events = macro_sink.snapshot();
    let downs = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                SyntheticOutput::MouseButton {
                    button: MouseButton::Left,
                    state: KeyState::Down
                }
            )
        })
        .count();
    let ups = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                SyntheticOutput::MouseButton {
                    button: MouseButton::Left,
                    state: KeyState::Up
                }
            )
        })
        .count();
    assert_eq!(downs, ups, "Every MouseDown must have a matching MouseUp");
}

#[test]
fn injected_events_never_trigger_macro_hotkeys_or_remaps() {
    let macro_sink = SharedMockSynth::default();
    let mut remap_sink = SharedMockSynth::default();
    let player = MacroPlayer::new(Box::new(macro_sink.clone()));
    let mut dispatcher = InputDispatcher::new(EmergencyStopHotkey::default(), player);

    let rules = vec![KeyBindingRule::new(
        1,
        "A -> B",
        KeyboardKey::KeyA,
        MappingTarget::Keyboard(KeyboardKey::KeyB),
    )];
    let mac = MacroDefinition::new(
        MacroId(1),
        "RunOnce F2",
        Some(MacroTrigger {
            source: InputSource::Keyboard(KeyboardKey::F2),
            mode: MacroPlaybackMode::RunOnce,
        }),
        vec![Clip {
            id: 1,
            kind: ClipKind::Key {
                key: Some(KeyboardKey::KeyA),
            },
            track: 0,
            start_ms: 0,
            dur_ms: 10,
        }],
        4,
        None,
    );

    dispatcher
        .sync_profile(
            CompiledMappingTable::compile(&rules),
            &[mac],
            &mut remap_sink,
        )
        .unwrap();
    dispatcher.set_enabled(true, &mut remap_sink).unwrap();

    // Injected F2 must NOT trigger macro
    assert_eq!(
        dispatcher.handle_raw_event(
            RawInputEvent::Key(RawKeyEvent::injected(KeyboardKey::F2, KeyState::Down)),
            &mut remap_sink,
        ),
        HookDecision::PassThrough
    );
    assert!(!dispatcher.is_macro_playing());

    // Injected A must NOT trigger A -> B remap
    assert_eq!(
        dispatcher.handle_raw_event(
            RawInputEvent::Key(RawKeyEvent::injected(KeyboardKey::KeyA, KeyState::Down)),
            &mut remap_sink,
        ),
        HookDecision::PassThrough
    );
    assert!(remap_sink.snapshot().is_empty());
    assert!(macro_sink.snapshot().is_empty());
}

#[test]
fn pausing_or_switching_profile_cancels_running_macro_and_releases_all_holds() {
    let macro_sink = SharedMockSynth::default();
    let mut remap_sink = SharedMockSynth::default();
    let player = MacroPlayer::new(Box::new(macro_sink.clone()));
    let mut dispatcher = InputDispatcher::new(EmergencyStopHotkey::default(), player);

    let mac = MacroDefinition::new(
        MacroId(2),
        "Long Toggle",
        Some(MacroTrigger {
            source: InputSource::Keyboard(KeyboardKey::F4),
            mode: MacroPlaybackMode::ToggleLoop,
        }),
        vec![Clip {
            id: 1,
            kind: ClipKind::Key {
                key: Some(KeyboardKey::Space),
            },
            track: 0,
            start_ms: 0,
            dur_ms: 5_000,
        }],
        4,
        None,
    );

    dispatcher
        .sync_profile(CompiledMappingTable::default(), &[mac], &mut remap_sink)
        .unwrap();
    dispatcher.set_enabled(true, &mut remap_sink).unwrap();

    dispatcher.handle_raw_event(
        RawInputEvent::Key(RawKeyEvent::physical(KeyboardKey::F4, KeyState::Down)),
        &mut remap_sink,
    );
    let deadline = Instant::now() + Duration::from_millis(300);
    while macro_sink.snapshot().is_empty() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }

    // Pause engine -> cancels macro and emits Space Up
    dispatcher.set_enabled(false, &mut remap_sink).unwrap();
    assert_eq!(dispatcher.status(), EngineStatus::Paused);

    let deadline = Instant::now() + Duration::from_millis(300);
    while dispatcher.is_macro_playing() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        macro_sink.snapshot(),
        vec![
            SyntheticOutput::Key {
                key: KeyboardKey::Space,
                state: KeyState::Down
            },
            SyntheticOutput::Key {
                key: KeyboardKey::Space,
                state: KeyState::Up
            },
        ]
    );
}

#[test]
fn combo_synthesis_failure_rolls_back_partial_modifiers_without_phantom_hold() {
    let mut synth = MockInputSynthesizer::new();
    let mut engine = RemapEngine::new(EmergencyStopHotkey::default());
    let rules = vec![KeyBindingRule::new(
        1,
        "F1 -> Ctrl+C",
        KeyboardKey::F1,
        MappingTarget::KeyCombo {
            modifiers: ModifierMask::CTRL,
            key: KeyboardKey::KeyC,
        },
    )];
    engine
        .switch_table(CompiledMappingTable::compile(&rules), &mut synth)
        .unwrap();
    engine.set_enabled(true, &mut synth).unwrap();

    // Fail on the very first synthesis (Ctrl Down)
    synth.inject_next_error(PlatformError::SynthesisFailed("UIPI blocked".to_string()));
    let err = engine.handle_key_event(
        RawKeyEvent::physical(KeyboardKey::F1, KeyState::Down),
        &mut synth,
    );
    assert!(err.is_err());
    assert_eq!(engine.snapshot().active_holds_count, 0);
}
