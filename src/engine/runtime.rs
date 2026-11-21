//! Unified automation runtime coordinating [`RemapEngine`], [`MacroPlayer`],
//! macro trigger routing, Emergency Stop, and live Windows OS hooks.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::domain::config::EmergencyStopHotkey;
use crate::domain::keys::{InputSource, KeyState};
use crate::domain::macros::{MacroDefinition, MacroId, MacroPlaybackMode};
use crate::domain::mapping::CompiledMappingTable;
use crate::engine::command::EngineStatus;
use crate::engine::macro_player::MacroPlayer;
use crate::engine::remapper::RemapEngine;
use crate::platform::windows::hook::WindowsKeyboardHook;
use crate::platform::windows::synth::WindowsInputSynthesizer;
use crate::platform::{
    HookDecision, InputInterceptor, InputSynthesizer, PlatformError, RawInputEvent,
};

/// Lightweight state snapshot polled periodically by the UI thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePollUpdate {
    pub status: EngineStatus,
    pub active_macro_name: Option<String>,
    pub emergency_stop_fired: bool,
    pub error_message: Option<String>,
}

/// Deterministic input router uniting key/mouse remapping with hotkey-triggered macro playback.
///
/// Decoupled from OS hooks so all routing, auto-repeat suppression, hold/toggle macro triggers,
/// capture suspension, and Emergency Stop behaviors are unit-testable with mock synthesizers.
#[derive(Debug)]
pub struct InputDispatcher {
    remapper: RemapEngine,
    macro_player: MacroPlayer,
    macro_triggers: HashMap<InputSource, MacroDefinition>,
    held_macro_triggers: HashMap<InputSource, (MacroId, MacroPlaybackMode)>,
    suspended_for_capture: bool,
    emergency_stop_notified: Arc<AtomicBool>,
    last_error: Option<String>,
}

impl InputDispatcher {
    /// Creates a paused dispatcher with the given Emergency Stop hotkey and [`MacroPlayer`].
    pub fn new(emergency_stop: EmergencyStopHotkey, macro_player: MacroPlayer) -> Self {
        Self {
            remapper: RemapEngine::new(emergency_stop),
            macro_player,
            macro_triggers: HashMap::new(),
            held_macro_triggers: HashMap::new(),
            suspended_for_capture: false,
            emergency_stop_notified: Arc::new(AtomicBool::new(false)),
            last_error: None,
        }
    }

    /// Returns the current engine status (`Paused`, `Active`, `EmergencyStopped`, etc.).
    pub fn status(&self) -> EngineStatus {
        self.remapper.status()
    }

    /// Returns a read-only reference to the inner [`RemapEngine`].
    pub fn remapper(&self) -> &RemapEngine {
        &self.remapper
    }

    /// Returns the display name of the currently running macro, if any.
    pub fn active_macro_name(&self) -> Option<String> {
        self.macro_player.active_macro_name()
    }

    /// Returns `true` if a macro is currently executing.
    pub fn is_macro_playing(&self) -> bool {
        self.macro_player.is_playing()
    }

    /// Enables or pauses automation (key remapping + macro triggers).
    pub fn set_enabled(
        &mut self,
        enabled: bool,
        synth: &mut dyn InputSynthesizer,
    ) -> Result<(), PlatformError> {
        if !enabled {
            self.macro_player.stop_all();
            self.held_macro_triggers.clear();
        }
        self.remapper.set_enabled(enabled, synth)
    }

    /// Suspends or resumes interception while the user is capturing keys or live-recording in the UI.
    ///
    /// Enforces mutual exclusion between recording/capture and playback/remapping while keeping
    /// the reserved Emergency Stop chord active at all times.
    pub fn set_capture_suspended(
        &mut self,
        suspended: bool,
        synth: &mut dyn InputSynthesizer,
    ) -> Result<(), PlatformError> {
        if self.suspended_for_capture == suspended {
            return Ok(());
        }
        self.suspended_for_capture = suspended;
        if suspended {
            self.macro_player.stop_all();
            self.held_macro_triggers.clear();
            self.remapper.release_all_holds(synth)?;
        }
        Ok(())
    }

    /// Replaces the active profile's compiled key-mapping table and enabled macro triggers safely.
    pub fn sync_profile(
        &mut self,
        table: CompiledMappingTable,
        macros: &[MacroDefinition],
        synth: &mut dyn InputSynthesizer,
    ) -> Result<(), PlatformError> {
        self.macro_player.stop_all();
        self.held_macro_triggers.clear();

        self.macro_triggers.clear();
        for mac in macros {
            if !mac.enabled || mac.compiled_steps.is_empty() {
                continue;
            }
            if let Some(trigger) = mac.trigger {
                self.macro_triggers
                    .entry(trigger.source)
                    .or_insert_with(|| mac.clone());
            }
        }

        self.remapper.switch_table(table, synth)
    }

    /// Immediately cancels all running macros, releases all virtual holds, and enters `EmergencyStopped`.
    pub fn emergency_stop_all(
        &mut self,
        synth: &mut dyn InputSynthesizer,
    ) -> Result<(), PlatformError> {
        self.macro_player.stop_all();
        self.held_macro_triggers.clear();
        self.remapper.emergency_stop_all(synth)
    }

    /// Evaluates a single raw keyboard or mouse button event coming from an OS hook.
    pub fn handle_raw_event(
        &mut self,
        event: RawInputEvent,
        synth: &mut dyn InputSynthesizer,
    ) -> HookDecision {
        match event {
            RawInputEvent::Key(key_ev) => {
                if key_ev.is_injected {
                    return HookDecision::PassThrough;
                }

                // Emergency Stop ALWAYS takes top priority — even when paused or capturing.
                if self
                    .remapper
                    .track_physical_modifier_and_check_emergency_stop(key_ev)
                {
                    if let Err(err) = self.emergency_stop_all(synth) {
                        self.last_error = Some(err.to_string());
                    }
                    self.emergency_stop_notified.store(true, Ordering::SeqCst);
                    return HookDecision::Suppress;
                }

                if self.suspended_for_capture {
                    return HookDecision::PassThrough;
                }

                let source = InputSource::Keyboard(key_ev.key);
                if let Some(decision) = self.try_handle_macro_trigger(source, key_ev.state) {
                    return decision;
                }

                match self.remapper.handle_key_event_post_panic(key_ev, synth) {
                    Ok(decision) => decision,
                    Err(err) => {
                        self.last_error = Some(err.to_string());
                        HookDecision::PassThrough
                    }
                }
            }
            RawInputEvent::MouseButton(mouse_ev) => {
                if mouse_ev.is_injected || self.suspended_for_capture {
                    return HookDecision::PassThrough;
                }

                let source = InputSource::MouseButton(mouse_ev.button);
                if let Some(decision) = self.try_handle_macro_trigger(source, mouse_ev.state) {
                    return decision;
                }

                match self.remapper.handle_mouse_button_event(mouse_ev, synth) {
                    Ok(decision) => decision,
                    Err(err) => {
                        self.last_error = Some(err.to_string());
                        HookDecision::PassThrough
                    }
                }
            }
        }
    }

    /// Consumes pending background status transitions / errors for the UI thread.
    pub fn poll_update(&mut self) -> RuntimePollUpdate {
        let emergency_stop_fired = self.emergency_stop_notified.swap(false, Ordering::SeqCst);
        let error_message = self
            .last_error
            .take()
            .or_else(|| self.macro_player.take_last_error());

        RuntimePollUpdate {
            status: self.remapper.status(),
            active_macro_name: self.macro_player.active_macro_name(),
            emergency_stop_fired,
            error_message,
        }
    }

    fn try_handle_macro_trigger(
        &mut self,
        source: InputSource,
        state: KeyState,
    ) -> Option<HookDecision> {
        match state {
            KeyState::Up => {
                if let Some((macro_id, mode)) = self.held_macro_triggers.remove(&source) {
                    if mode == MacroPlaybackMode::HoldLoop {
                        self.macro_player.stop_macro(macro_id);
                    }
                    return Some(HookDecision::Suppress);
                }
                None
            }
            KeyState::Down => {
                if self.remapper.status() != EngineStatus::Active {
                    return None;
                }

                // Suppress OS auto-repeat while a macro trigger key is held down.
                if self.held_macro_triggers.contains_key(&source) {
                    return Some(HookDecision::Suppress);
                }

                let mac = self.macro_triggers.get(&source)?.clone();
                let mode = mac
                    .trigger
                    .map(|t| t.mode)
                    .unwrap_or(MacroPlaybackMode::RunOnce);

                self.held_macro_triggers.insert(source, (mac.id, mode));

                match mode {
                    MacroPlaybackMode::HoldLoop => {
                        self.macro_player.start_macro(mac);
                    }
                    MacroPlaybackMode::ToggleLoop => {
                        if self.macro_player.active_macro_id() == Some(mac.id) {
                            self.macro_player.stop_macro(mac.id);
                        } else {
                            self.macro_player.start_macro(mac);
                        }
                    }
                    MacroPlaybackMode::RunOnce | MacroPlaybackMode::RepeatCount(_) => {
                        if self.macro_player.active_macro_id() != Some(mac.id) {
                            self.macro_player.start_macro(mac);
                        }
                    }
                }

                Some(HookDecision::Suppress)
            }
        }
    }
}

struct LiveRuntimeState {
    dispatcher: InputDispatcher,
    synth: WindowsInputSynthesizer,
}

/// Live OS automation backend connecting global Windows low-level hooks (`WH_KEYBOARD_LL` + `WH_MOUSE_LL`),
/// `SendInput` synthesis, [`RemapEngine`], and [`MacroPlayer`].
pub struct LiveAutomationBackend {
    state: Arc<Mutex<LiveRuntimeState>>,
    hooks: WindowsKeyboardHook,
    hook_error: Option<String>,
}

impl std::fmt::Debug for LiveAutomationBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveAutomationBackend")
            .field("hooks_active", &self.hooks.is_active())
            .field("hook_error", &self.hook_error)
            .finish()
    }
}

impl LiveAutomationBackend {
    /// Initializes the live Windows automation runtime and installs global hooks in `Paused` state.
    pub fn start(emergency_stop: EmergencyStopHotkey) -> Self {
        let player = MacroPlayer::new(Box::new(WindowsInputSynthesizer::new()));
        let dispatcher = InputDispatcher::new(emergency_stop, player);
        let state = Arc::new(Mutex::new(LiveRuntimeState {
            dispatcher,
            synth: WindowsInputSynthesizer::new(),
        }));

        let mut hooks = WindowsKeyboardHook::new();
        let cb_state = Arc::clone(&state);
        let hook_error = hooks
            .start_with_callback(Arc::new(move |raw_event| {
                let Ok(mut guard) = cb_state.lock() else {
                    return HookDecision::PassThrough;
                };
                let LiveRuntimeState { dispatcher, synth } = &mut *guard;
                dispatcher.handle_raw_event(raw_event, synth)
            }))
            .err()
            .map(|e| e.to_string());

        Self {
            state,
            hooks,
            hook_error,
        }
    }

    /// Returns `true` if live Windows low-level hooks are installed and pumping events.
    pub fn hooks_active(&self) -> bool {
        self.hooks.is_active()
    }

    /// Returns any hook initialization error encountered at startup.
    pub fn hook_error(&self) -> Option<&str> {
        self.hook_error.as_deref()
    }

    /// Synchronizes the active profile's key bindings and macros to the live runtime.
    pub fn sync_profile(
        &self,
        table: CompiledMappingTable,
        macros: &[MacroDefinition],
    ) -> Result<(), PlatformError> {
        let Ok(mut guard) = self.state.lock() else {
            return Ok(());
        };
        let LiveRuntimeState { dispatcher, synth } = &mut *guard;
        dispatcher.sync_profile(table, macros, synth)
    }

    /// Enables or pauses live key remapping and macro hotkey triggering.
    pub fn set_enabled(&self, enabled: bool) -> Result<(), PlatformError> {
        if enabled && !self.hooks.is_active() {
            return Err(PlatformError::HookRegistrationFailed(
                self.hook_error
                    .clone()
                    .unwrap_or_else(|| "Windows input hooks are not active".to_string()),
            ));
        }
        let Ok(mut guard) = self.state.lock() else {
            return Ok(());
        };
        let LiveRuntimeState { dispatcher, synth } = &mut *guard;
        dispatcher.set_enabled(enabled, synth)
    }

    /// Suspends or resumes live interception during UI key/hotkey capture or live recording.
    pub fn set_capture_suspended(&self, suspended: bool) {
        if let Ok(mut guard) = self.state.lock() {
            let LiveRuntimeState { dispatcher, synth } = &mut *guard;
            let _ = dispatcher.set_capture_suspended(suspended, synth);
        }
    }

    /// Immediately stops all macros, releases all virtual holds, and enters `EmergencyStopped`.
    pub fn emergency_stop_all(&self) -> Result<(), PlatformError> {
        let Ok(mut guard) = self.state.lock() else {
            return Ok(());
        };
        let LiveRuntimeState { dispatcher, synth } = &mut *guard;
        dispatcher.emergency_stop_all(synth)
    }

    /// Polls live status transitions, active macro playback, and any background errors.
    pub fn poll_update(&self) -> RuntimePollUpdate {
        let Ok(mut guard) = self.state.lock() else {
            return RuntimePollUpdate {
                status: EngineStatus::Paused,
                active_macro_name: None,
                emergency_stop_fired: false,
                error_message: None,
            };
        };
        guard.dispatcher.poll_update()
    }

    /// Stops all macros, releases all held virtual keys/buttons, and cleanly unhooks OS hooks.
    pub fn shutdown(&mut self) {
        if let Ok(mut guard) = self.state.lock() {
            let LiveRuntimeState { dispatcher, synth } = &mut *guard;
            let _ = dispatcher.set_enabled(false, synth);
        }
        let _ = self.hooks.stop();
    }
}

impl Drop for LiveAutomationBackend {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::keys::{KeyboardKey, MouseButton};
    use crate::domain::macros::{Clip, ClipKind, MacroTrigger};
    use crate::domain::mapping::{KeyBindingRule, MappingTarget};
    use crate::platform::{RawKeyEvent, RawMouseButtonEvent, SyntheticOutput};
    use std::thread;
    use std::time::{Duration, Instant};

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
    fn toggle_macro_ignores_autorepeat_and_stops_on_second_press() {
        let macro_sink = SharedMockSynth::default();
        let mut remap_sink = SharedMockSynth::default();
        let player = MacroPlayer::new(Box::new(macro_sink.clone()));
        let mut dispatcher = InputDispatcher::new(EmergencyStopHotkey::default(), player);

        let mac = MacroDefinition::new(
            MacroId(1),
            "Toggle E",
            Some(MacroTrigger {
                source: InputSource::Keyboard(KeyboardKey::F2),
                mode: MacroPlaybackMode::ToggleLoop,
            }),
            vec![Clip {
                id: 1,
                kind: ClipKind::Key {
                    key: Some(KeyboardKey::KeyE),
                },
                track: 0,
                start_ms: 0,
                dur_ms: 500,
            }],
            4,
            None,
        );

        dispatcher
            .sync_profile(CompiledMappingTable::default(), &[mac], &mut remap_sink)
            .unwrap();
        dispatcher.set_enabled(true, &mut remap_sink).unwrap();
        assert_eq!(dispatcher.status(), EngineStatus::Active);

        // Press F2 -> starts ToggleLoop macro
        assert_eq!(
            dispatcher.handle_raw_event(
                RawInputEvent::Key(RawKeyEvent::physical(KeyboardKey::F2, KeyState::Down)),
                &mut remap_sink,
            ),
            HookDecision::Suppress
        );
        assert!(dispatcher.is_macro_playing());

        // OS auto-repeat F2 Down while still held MUST NOT toggle it off
        assert_eq!(
            dispatcher.handle_raw_event(
                RawInputEvent::Key(RawKeyEvent::physical(KeyboardKey::F2, KeyState::Down)),
                &mut remap_sink,
            ),
            HookDecision::Suppress
        );
        assert!(dispatcher.is_macro_playing());

        // Release F2 -> stays playing in ToggleLoop mode
        assert_eq!(
            dispatcher.handle_raw_event(
                RawInputEvent::Key(RawKeyEvent::physical(KeyboardKey::F2, KeyState::Up)),
                &mut remap_sink,
            ),
            HookDecision::Suppress
        );
        assert!(dispatcher.is_macro_playing());

        // Wait for KeyE Down
        let deadline = Instant::now() + Duration::from_millis(300);
        while macro_sink.snapshot().is_empty() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }

        // Second press of F2 -> toggles macro OFF and releases KeyE
        assert_eq!(
            dispatcher.handle_raw_event(
                RawInputEvent::Key(RawKeyEvent::physical(KeyboardKey::F2, KeyState::Down)),
                &mut remap_sink,
            ),
            HookDecision::Suppress
        );
        let deadline = Instant::now() + Duration::from_millis(300);
        while dispatcher.is_macro_playing() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(!dispatcher.is_macro_playing());
        assert_eq!(
            macro_sink.snapshot(),
            vec![
                SyntheticOutput::Key {
                    key: KeyboardKey::KeyE,
                    state: KeyState::Down
                },
                SyntheticOutput::Key {
                    key: KeyboardKey::KeyE,
                    state: KeyState::Up
                },
            ]
        );
    }

    #[test]
    fn emergency_stop_cancels_active_macro_and_releases_remapped_hold_simultaneously() {
        let macro_sink = SharedMockSynth::default();
        let mut remap_sink = SharedMockSynth::default();
        let player = MacroPlayer::new(Box::new(macro_sink.clone()));
        let mut dispatcher = InputDispatcher::new(EmergencyStopHotkey::default(), player);

        let rules = vec![KeyBindingRule::new(
            1,
            "Space -> LeftClick",
            KeyboardKey::Space,
            MappingTarget::MouseButton(MouseButton::Left),
        )];
        let mac = MacroDefinition::new(
            MacroId(5),
            "Hold R",
            Some(MacroTrigger {
                source: InputSource::MouseButton(MouseButton::XButton1),
                mode: MacroPlaybackMode::HoldLoop,
            }),
            vec![Clip {
                id: 1,
                kind: ClipKind::Key {
                    key: Some(KeyboardKey::KeyR),
                },
                track: 0,
                start_ms: 0,
                dur_ms: 3_000,
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

        // Hold Space (remapped to MouseLeft) AND hold Mouse X1 (macro holding KeyR)
        dispatcher.handle_raw_event(
            RawInputEvent::Key(RawKeyEvent::physical(KeyboardKey::Space, KeyState::Down)),
            &mut remap_sink,
        );
        dispatcher.handle_raw_event(
            RawInputEvent::MouseButton(RawMouseButtonEvent::physical(
                MouseButton::XButton1,
                KeyState::Down,
            )),
            &mut remap_sink,
        );

        let deadline = Instant::now() + Duration::from_millis(300);
        while macro_sink.snapshot().is_empty() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }

        // Trigger global Emergency Stop (Ctrl + Shift + F12)
        dispatcher.handle_raw_event(
            RawInputEvent::Key(RawKeyEvent::physical(KeyboardKey::LeftCtrl, KeyState::Down)),
            &mut remap_sink,
        );
        dispatcher.handle_raw_event(
            RawInputEvent::Key(RawKeyEvent::physical(
                KeyboardKey::LeftShift,
                KeyState::Down,
            )),
            &mut remap_sink,
        );
        let decision = dispatcher.handle_raw_event(
            RawInputEvent::Key(RawKeyEvent::physical(KeyboardKey::F12, KeyState::Down)),
            &mut remap_sink,
        );
        assert_eq!(decision, HookDecision::Suppress);

        let deadline = Instant::now() + Duration::from_millis(300);
        while dispatcher.is_macro_playing() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }

        let update = dispatcher.poll_update();
        assert!(update.emergency_stop_fired);
        assert_eq!(update.status, EngineStatus::EmergencyStopped);
        assert!(!dispatcher.is_macro_playing());

        // Remapped MouseLeft released
        assert_eq!(
            remap_sink.snapshot(),
            vec![
                SyntheticOutput::MouseButton {
                    button: MouseButton::Left,
                    state: KeyState::Down
                },
                SyntheticOutput::MouseButton {
                    button: MouseButton::Left,
                    state: KeyState::Up
                },
            ]
        );
        // Macro's KeyR released
        assert_eq!(
            macro_sink.snapshot(),
            vec![
                SyntheticOutput::Key {
                    key: KeyboardKey::KeyR,
                    state: KeyState::Down
                },
                SyntheticOutput::Key {
                    key: KeyboardKey::KeyR,
                    state: KeyState::Up
                },
            ]
        );
    }

    #[test]
    fn capture_suspension_bypasses_remaps_and_stops_macros_while_keeping_emergency_stop() {
        let macro_sink = SharedMockSynth::default();
        let mut remap_sink = SharedMockSynth::default();
        let player = MacroPlayer::new(Box::new(macro_sink));
        let mut dispatcher = InputDispatcher::new(EmergencyStopHotkey::default(), player);

        let rules = vec![KeyBindingRule::new(
            1,
            "Q -> W",
            KeyboardKey::KeyQ,
            MappingTarget::Keyboard(KeyboardKey::KeyW),
        )];
        dispatcher
            .sync_profile(CompiledMappingTable::compile(&rules), &[], &mut remap_sink)
            .unwrap();
        dispatcher.set_enabled(true, &mut remap_sink).unwrap();

        dispatcher
            .set_capture_suspended(true, &mut remap_sink)
            .unwrap();
        assert_eq!(
            dispatcher.handle_raw_event(
                RawInputEvent::Key(RawKeyEvent::physical(KeyboardKey::KeyQ, KeyState::Down)),
                &mut remap_sink,
            ),
            HookDecision::PassThrough
        );
        assert!(remap_sink.snapshot().is_empty());

        dispatcher
            .set_capture_suspended(false, &mut remap_sink)
            .unwrap();
        assert_eq!(
            dispatcher.handle_raw_event(
                RawInputEvent::Key(RawKeyEvent::physical(KeyboardKey::KeyQ, KeyState::Down)),
                &mut remap_sink,
            ),
            HookDecision::Suppress
        );
        assert_eq!(remap_sink.snapshot().len(), 1);
    }
}
