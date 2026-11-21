//! Deterministic key remapping state machine with stuck-key and loop protection.

use std::collections::{HashMap, HashSet};

use crate::domain::config::EmergencyStopHotkey;
use crate::domain::keys::{InputSource, KeyState, KeyboardKey, ModifierMask, MouseButton};
use crate::domain::mapping::{CompiledMappingTable, MappingTarget};
use crate::engine::command::{EngineSnapshot, EngineStatus};
use crate::platform::{
    HookDecision, InputSynthesizer, PlatformError, RawKeyEvent, RawMouseButtonEvent,
    SyntheticOutput,
};

/// Standard Windows wheel detent value (`WHEEL_DELTA`).
pub const DEFAULT_WHEEL_DELTA: i32 = 120;

/// Active output state currently held due to a physical key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldTarget {
    Keyboard(KeyboardKey),
    KeyCombo {
        modifiers: ModifierMask,
        key: KeyboardKey,
    },
    MouseButton(MouseButton),
}

/// Runtime key remapping engine enforcing `O(1)` dispatch and guaranteed key release.
#[derive(Debug, Clone)]
pub struct RemapEngine {
    status: EngineStatus,
    compiled_table: CompiledMappingTable,
    active_holds: HashMap<InputSource, HeldTarget>,
    hold_order: Vec<InputSource>,
    held_physical_modifiers: HashSet<KeyboardKey>,
    physical_modifiers: ModifierMask,
    emergency_stop: EmergencyStopHotkey,
}

impl RemapEngine {
    /// Creates a new remapping engine starting safely paused (explicit activation required).
    pub fn new(emergency_stop: EmergencyStopHotkey) -> Self {
        Self {
            status: EngineStatus::Paused,
            compiled_table: CompiledMappingTable::default(),
            active_holds: HashMap::new(),
            hold_order: Vec::new(),
            held_physical_modifiers: HashSet::new(),
            physical_modifiers: ModifierMask::NONE,
            emergency_stop,
        }
    }

    /// Returns the current engine status.
    pub const fn status(&self) -> EngineStatus {
        self.status
    }

    /// Returns a lightweight UI snapshot of current engine state.
    pub fn snapshot(&self) -> EngineSnapshot {
        EngineSnapshot {
            status: self.status,
            active_holds_count: self.active_holds.len(),
            active_rule_count: self.compiled_table.len(),
        }
    }

    /// Returns the currently configured emergency stop hotkey.
    pub const fn emergency_stop_hotkey(&self) -> EmergencyStopHotkey {
        self.emergency_stop
    }

    /// Updates the emergency stop hotkey.
    pub fn set_emergency_stop_hotkey(&mut self, hotkey: EmergencyStopHotkey) {
        self.emergency_stop = hotkey;
    }

    /// Enables or pauses key remapping, releasing any held virtual outputs when pausing.
    pub fn set_enabled(
        &mut self,
        enabled: bool,
        synth: &mut dyn InputSynthesizer,
    ) -> Result<(), PlatformError> {
        if enabled {
            self.status = EngineStatus::Active;
            Ok(())
        } else {
            self.release_all_holds(synth)?;
            self.status = EngineStatus::Paused;
            Ok(())
        }
    }

    /// Releases all currently held remapped outputs and installs a new compiled lookup table.
    pub fn switch_table(
        &mut self,
        new_table: CompiledMappingTable,
        synth: &mut dyn InputSynthesizer,
    ) -> Result<(), PlatformError> {
        self.release_all_holds(synth)?;
        self.compiled_table = new_table;
        if self.status == EngineStatus::Idle {
            self.status = EngineStatus::Active;
        }
        Ok(())
    }

    /// Immediately releases all active virtual holds and forces the engine into [`EngineStatus::EmergencyStopped`].
    pub fn emergency_stop_all(
        &mut self,
        synth: &mut dyn InputSynthesizer,
    ) -> Result<(), PlatformError> {
        self.status = EngineStatus::EmergencyStopped;
        self.release_all_holds(synth)
    }

    /// Updates physical modifier tracking for a non-injected key event and returns `true`
    /// if the event triggers the reserved Emergency Stop chord.
    pub fn track_physical_modifier_and_check_emergency_stop(&mut self, event: RawKeyEvent) -> bool {
        if event.is_injected {
            return false;
        }
        self.update_physical_modifiers(event.key, event.state);
        event.state == KeyState::Down
            && self
                .emergency_stop
                .matches(self.physical_modifiers, event.key)
    }

    /// Evaluates a single physical or injected keyboard event and emits mapped outputs.
    pub fn handle_key_event(
        &mut self,
        event: RawKeyEvent,
        synth: &mut dyn InputSynthesizer,
    ) -> Result<HookDecision, PlatformError> {
        // 1. Loop prevention: never remap self-synthesized or externally injected events.
        if event.is_injected {
            return Ok(HookDecision::PassThrough);
        }

        // 2 & 3. Track physical modifiers and prioritize Emergency Stop over all active rules.
        if self.track_physical_modifier_and_check_emergency_stop(event) {
            self.emergency_stop_all(synth)?;
            return Ok(HookDecision::Suppress);
        }

        self.handle_key_event_post_panic(event, synth)
    }

    /// Evaluates a physical keyboard event after physical modifiers and Emergency Stop have already been checked.
    pub fn handle_key_event_post_panic(
        &mut self,
        event: RawKeyEvent,
        synth: &mut dyn InputSynthesizer,
    ) -> Result<HookDecision, PlatformError> {
        if event.is_injected {
            return Ok(HookDecision::PassThrough);
        }

        let source = InputSource::Keyboard(event.key);
        match event.state {
            KeyState::Up => self.handle_source_up(source, synth),
            KeyState::Down => self.handle_source_down(source, synth),
        }
    }

    /// Evaluates a single physical or injected mouse button event and emits mapped outputs.
    pub fn handle_mouse_button_event(
        &mut self,
        event: RawMouseButtonEvent,
        synth: &mut dyn InputSynthesizer,
    ) -> Result<HookDecision, PlatformError> {
        if event.is_injected {
            return Ok(HookDecision::PassThrough);
        }

        let source = InputSource::MouseButton(event.button);
        match event.state {
            KeyState::Up => self.handle_source_up(source, synth),
            KeyState::Down => self.handle_source_down(source, synth),
        }
    }

    /// Releases every tracked virtual key or mouse button in reverse press order.
    pub fn release_all_holds(
        &mut self,
        synth: &mut dyn InputSynthesizer,
    ) -> Result<(), PlatformError> {
        let mut first_error: Option<PlatformError> = None;

        while let Some(source) = self.hold_order.pop() {
            if let Some(held) = self.active_holds.remove(&source) {
                if let Err(err) = emit_held_release(held, synth) {
                    if first_error.is_none() {
                        first_error = Some(err);
                    }
                }
            }
        }
        self.active_holds.clear();

        if let Some(err) = first_error {
            Err(err)
        } else {
            Ok(())
        }
    }

    fn handle_source_down(
        &mut self,
        source: InputSource,
        synth: &mut dyn InputSynthesizer,
    ) -> Result<HookDecision, PlatformError> {
        if self.status != EngineStatus::Active {
            return Ok(HookDecision::PassThrough);
        }

        // Handle OS key auto-repeat while a mapped target is already held down.
        if let Some(&held) = self.active_holds.get(&source) {
            match held {
                HeldTarget::MouseButton(_) => {
                    // Mouse buttons must stay held without spamming repeated MouseDown events.
                    return Ok(HookDecision::Suppress);
                }
                HeldTarget::Keyboard(key) | HeldTarget::KeyCombo { key, .. } => {
                    synth.synthesize(SyntheticOutput::Key {
                        key,
                        state: KeyState::Down,
                    })?;
                    return Ok(HookDecision::Suppress);
                }
            }
        }

        let Some(target) = self.compiled_table.lookup(source) else {
            return Ok(HookDecision::PassThrough);
        };

        match target {
            MappingTarget::Keyboard(key) => {
                synth.synthesize(SyntheticOutput::Key {
                    key,
                    state: KeyState::Down,
                })?;
                self.record_hold(source, HeldTarget::Keyboard(key));
                Ok(HookDecision::Suppress)
            }
            MappingTarget::KeyCombo { modifiers, key } => {
                let mut pressed_mods = Vec::with_capacity(4);
                for mod_key in modifiers.to_modifier_keys() {
                    if let Err(err) = synth.synthesize(SyntheticOutput::Key {
                        key: mod_key,
                        state: KeyState::Down,
                    }) {
                        for held_mod in pressed_mods.into_iter().rev() {
                            let _ = synth.synthesize(SyntheticOutput::Key {
                                key: held_mod,
                                state: KeyState::Up,
                            });
                        }
                        return Err(err);
                    }
                    pressed_mods.push(mod_key);
                }
                if let Err(err) = synth.synthesize(SyntheticOutput::Key {
                    key,
                    state: KeyState::Down,
                }) {
                    for held_mod in pressed_mods.into_iter().rev() {
                        let _ = synth.synthesize(SyntheticOutput::Key {
                            key: held_mod,
                            state: KeyState::Up,
                        });
                    }
                    return Err(err);
                }
                self.record_hold(source, HeldTarget::KeyCombo { modifiers, key });
                Ok(HookDecision::Suppress)
            }
            MappingTarget::MouseButton(button) => {
                synth.synthesize(SyntheticOutput::MouseButton {
                    button,
                    state: KeyState::Down,
                })?;
                self.record_hold(source, HeldTarget::MouseButton(button));
                Ok(HookDecision::Suppress)
            }
            MappingTarget::MouseWheel(direction) => {
                synth.synthesize(SyntheticOutput::MouseWheel {
                    direction,
                    delta: DEFAULT_WHEEL_DELTA,
                })?;
                Ok(HookDecision::Suppress)
            }
            MappingTarget::Disabled => Ok(HookDecision::Suppress),
        }
    }

    fn handle_source_up(
        &mut self,
        source: InputSource,
        synth: &mut dyn InputSynthesizer,
    ) -> Result<HookDecision, PlatformError> {
        // Always release active holds first even if the profile switched or mapping paused mid-hold.
        if let Some(held) = self.remove_hold(source) {
            emit_held_release(held, synth)?;
            return Ok(HookDecision::Suppress);
        }

        if self.status == EngineStatus::Active {
            if let Some(MappingTarget::Disabled | MappingTarget::MouseWheel(_)) =
                self.compiled_table.lookup(source)
            {
                return Ok(HookDecision::Suppress);
            }
        }

        Ok(HookDecision::PassThrough)
    }

    fn record_hold(&mut self, source: InputSource, target: HeldTarget) {
        if !self.active_holds.contains_key(&source) {
            self.hold_order.push(source);
        }
        self.active_holds.insert(source, target);
    }

    fn remove_hold(&mut self, source: InputSource) -> Option<HeldTarget> {
        let held = self.active_holds.remove(&source)?;
        self.hold_order.retain(|&s| s != source);
        Some(held)
    }

    fn update_physical_modifiers(&mut self, key: KeyboardKey, state: KeyState) {
        if key.modifier_flag().is_empty() {
            return;
        }
        match state {
            KeyState::Down => {
                self.held_physical_modifiers.insert(key);
            }
            KeyState::Up => {
                self.held_physical_modifiers.remove(&key);
            }
        }
        let mut mask = ModifierMask::NONE;
        for &held_key in &self.held_physical_modifiers {
            mask = mask.with(held_key.modifier_flag());
        }
        self.physical_modifiers = mask;
    }
}

fn emit_held_release(
    held: HeldTarget,
    synth: &mut dyn InputSynthesizer,
) -> Result<(), PlatformError> {
    let mut first_error: Option<PlatformError> = None;

    match held {
        HeldTarget::Keyboard(key) => {
            synth.synthesize(SyntheticOutput::Key {
                key,
                state: KeyState::Up,
            })?;
        }
        HeldTarget::KeyCombo { modifiers, key } => {
            if let Err(err) = synth.synthesize(SyntheticOutput::Key {
                key,
                state: KeyState::Up,
            }) {
                first_error = Some(err);
            }
            for mod_key in modifiers.to_modifier_keys().into_iter().rev() {
                if let Err(err) = synth.synthesize(SyntheticOutput::Key {
                    key: mod_key,
                    state: KeyState::Up,
                }) {
                    if first_error.is_none() {
                        first_error = Some(err);
                    }
                }
            }
        }
        HeldTarget::MouseButton(button) => {
            synth.synthesize(SyntheticOutput::MouseButton {
                button,
                state: KeyState::Up,
            })?;
        }
    }

    if let Some(err) = first_error {
        Err(err)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mapping::KeyBindingRule;
    use crate::platform::mock::MockInputSynthesizer;

    #[test]
    fn key_to_mouse_button_suppresses_autorepeat_and_releases_on_key_up() {
        let mut synth = MockInputSynthesizer::new();
        let mut engine = RemapEngine::new(EmergencyStopHotkey::default());

        let rules = vec![KeyBindingRule::new(
            1,
            "Space -> Left Click",
            KeyboardKey::Space,
            MappingTarget::MouseButton(MouseButton::Left),
        )];
        engine
            .switch_table(CompiledMappingTable::compile(&rules), &mut synth)
            .unwrap();
        engine.set_enabled(true, &mut synth).unwrap();

        // Initial press -> MouseDown
        let d1 = engine
            .handle_key_event(
                RawKeyEvent::physical(KeyboardKey::Space, KeyState::Down),
                &mut synth,
            )
            .unwrap();
        assert_eq!(d1, HookDecision::Suppress);

        // Auto-repeat KeyDown -> suppressed, no duplicate MouseDown
        let d2 = engine
            .handle_key_event(
                RawKeyEvent::physical(KeyboardKey::Space, KeyState::Down),
                &mut synth,
            )
            .unwrap();
        assert_eq!(d2, HookDecision::Suppress);

        // Release -> MouseUp
        let d3 = engine
            .handle_key_event(
                RawKeyEvent::physical(KeyboardKey::Space, KeyState::Up),
                &mut synth,
            )
            .unwrap();
        assert_eq!(d3, HookDecision::Suppress);

        assert_eq!(
            synth.events(),
            &[
                SyntheticOutput::MouseButton {
                    button: MouseButton::Left,
                    state: KeyState::Down,
                },
                SyntheticOutput::MouseButton {
                    button: MouseButton::Left,
                    state: KeyState::Up,
                },
            ]
        );
    }
}
