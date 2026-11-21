//! Background macro playback worker with monotonic timing, sub-millisecond cancellation,
//! zero-delay loop protection, and guaranteed virtual key/mouse release on stop.

use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::domain::keys::{InputSource, KeyState};
use crate::domain::macros::{MacroDefinition, MacroId, MacroPlaybackMode, MacroStep};
use crate::platform::{InputSynthesizer, PlatformError, SyntheticOutput};

/// Minimum delay enforced at the end of a looping iteration when the loop body contains
/// zero explicit wait steps, preventing CPU or OS input-queue starvation.
pub const MIN_LOOP_DELAY_MS: u64 = 10;

#[derive(Debug, Default)]
struct PlayerShared {
    generation: u64,
    pending: Option<MacroDefinition>,
    running_macro_id: Option<MacroId>,
    running_macro_name: Option<String>,
    last_error: Option<String>,
    shutdown: bool,
}

/// Manages a dedicated background worker thread executing compiled [`MacroStep`] sequences.
pub struct MacroPlayer {
    shared: Arc<(Mutex<PlayerShared>, Condvar)>,
    worker: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for MacroPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacroPlayer")
            .field("is_playing", &self.is_playing())
            .field("active_macro_id", &self.active_macro_id())
            .finish()
    }
}

impl MacroPlayer {
    /// Spawns a background playback worker using `synth` for emitting keyboard and mouse events.
    pub fn new(mut synth: Box<dyn InputSynthesizer>) -> Self {
        let shared = Arc::new((Mutex::new(PlayerShared::default()), Condvar::new()));
        let worker_shared = Arc::clone(&shared);

        let worker = thread::Builder::new()
            .name("ctt-macro-player".to_string())
            .spawn(move || {
                run_worker_loop(&worker_shared, synth.as_mut());
            })
            .ok();

        Self { shared, worker }
    }

    /// Starts (or restarts) playback of `macro_def`, cancelling any previously running macro cleanly.
    pub fn start_macro(&self, macro_def: MacroDefinition) {
        let (lock, cvar) = &*self.shared;
        let Ok(mut state) = lock.lock() else {
            return;
        };
        if state.shutdown {
            return;
        }
        state.generation = state.generation.wrapping_add(1);
        state.pending = Some(macro_def);
        state.last_error = None;
        cvar.notify_all();
    }

    /// Stops playback if the currently running or queued macro matches `macro_id`.
    pub fn stop_macro(&self, macro_id: MacroId) {
        let (lock, cvar) = &*self.shared;
        let Ok(mut state) = lock.lock() else {
            return;
        };
        let matches_running = state.running_macro_id == Some(macro_id);
        let matches_pending = state.pending.as_ref().is_some_and(|m| m.id == macro_id);
        if matches_running || matches_pending {
            state.generation = state.generation.wrapping_add(1);
            if matches_pending {
                state.pending = None;
            }
            cvar.notify_all();
        }
    }

    /// Immediately cancels any active or pending macro playback and triggers stuck-key release.
    pub fn stop_all(&self) {
        let (lock, cvar) = &*self.shared;
        let Ok(mut state) = lock.lock() else {
            return;
        };
        state.generation = state.generation.wrapping_add(1);
        state.pending = None;
        cvar.notify_all();
    }

    /// Returns the [`MacroId`] of the currently active or pending macro, if any.
    pub fn active_macro_id(&self) -> Option<MacroId> {
        let (lock, _) = &*self.shared;
        let Ok(state) = lock.lock() else {
            return None;
        };
        state
            .pending
            .as_ref()
            .map(|m| m.id)
            .or(state.running_macro_id)
    }

    /// Returns the display name of the currently active or pending macro, if any.
    pub fn active_macro_name(&self) -> Option<String> {
        let (lock, _) = &*self.shared;
        let Ok(state) = lock.lock() else {
            return None;
        };
        state
            .pending
            .as_ref()
            .map(|m| m.name.clone())
            .or_else(|| state.running_macro_name.clone())
    }

    /// Returns `true` if a macro is currently playing or queued to start.
    pub fn is_playing(&self) -> bool {
        self.active_macro_id().is_some()
    }

    /// Consumes and returns the most recent playback synthesis error, if any.
    pub fn take_last_error(&self) -> Option<String> {
        let (lock, _) = &*self.shared;
        let Ok(mut state) = lock.lock() else {
            return None;
        };
        state.last_error.take()
    }

    /// Stops playback and joins the worker thread cleanly.
    pub fn shutdown(&mut self) {
        {
            let (lock, cvar) = &*self.shared;
            if let Ok(mut state) = lock.lock() {
                state.shutdown = true;
                state.generation = state.generation.wrapping_add(1);
                state.pending = None;
                cvar.notify_all();
            }
        }
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for MacroPlayer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run_worker_loop(shared: &Arc<(Mutex<PlayerShared>, Condvar)>, synth: &mut dyn InputSynthesizer) {
    let (lock, cvar) = &**shared;

    loop {
        let (macro_def, run_gen) = {
            let Ok(mut state) = lock.lock() else {
                return;
            };
            while !state.shutdown && state.pending.is_none() {
                match cvar.wait(state) {
                    Ok(guard) => state = guard,
                    Err(_) => return,
                }
            }
            if state.shutdown {
                return;
            }
            let Some(mac) = state.pending.take() else {
                continue;
            };
            let gen = state.generation;
            state.running_macro_id = Some(mac.id);
            state.running_macro_name = Some(mac.name.clone());
            (mac, gen)
        };

        let result = execute_macro_definition(&macro_def, run_gen, synth, shared);

        if let Ok(mut state) = lock.lock() {
            state.running_macro_id = None;
            state.running_macro_name = None;
            if let Err(err) = result {
                state.last_error = Some(err.to_string());
            }
        }
    }
}

/// Finds `(loop_start_idx, loop_end_idx)` if a valid loop region exists in `steps`.
fn find_loop_bounds(steps: &[MacroStep]) -> Option<(usize, usize)> {
    let ls = steps
        .iter()
        .position(|s| matches!(s, MacroStep::LoopStart))?;
    let le = steps
        .iter()
        .rposition(|s| matches!(s, MacroStep::LoopEnd))?;
    (ls < le).then_some((ls, le))
}

fn slice_has_positive_wait(steps: &[MacroStep]) -> bool {
    steps
        .iter()
        .any(|s| matches!(s, MacroStep::Wait(ms) if *ms > 0))
}

fn is_cancelled(shared: &Arc<(Mutex<PlayerShared>, Condvar)>, run_gen: u64) -> bool {
    let (lock, _) = &**shared;
    match lock.lock() {
        Ok(state) => state.shutdown || state.generation != run_gen,
        Err(_) => true,
    }
}

/// Sleeps until `deadline` using [`Condvar::wait_timeout`] so cancellation interrupts immediately.
///
/// Returns `true` if the wait elapsed normally, or `false` if cancelled.
fn wait_until_or_cancel(
    shared: &Arc<(Mutex<PlayerShared>, Condvar)>,
    run_gen: u64,
    deadline: Instant,
) -> bool {
    let (lock, cvar) = &**shared;
    let Ok(mut state) = lock.lock() else {
        return false;
    };

    loop {
        if state.shutdown || state.generation != run_gen {
            return false;
        }
        let now = Instant::now();
        if now >= deadline {
            return true;
        }
        let remaining = deadline.saturating_duration_since(now);
        match cvar.wait_timeout(state, remaining) {
            Ok((guard, _)) => state = guard,
            Err(_) => return false,
        }
    }
}

fn execute_macro_definition(
    macro_def: &MacroDefinition,
    run_gen: u64,
    synth: &mut dyn InputSynthesizer,
    shared: &Arc<(Mutex<PlayerShared>, Condvar)>,
) -> Result<(), PlatformError> {
    if macro_def.compiled_steps.is_empty() {
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    let _timer_guard = crate::platform::windows::ffi::HighResTimerGuard::acquire(1);

    let mode = macro_def
        .trigger
        .map(|t| t.mode)
        .unwrap_or(MacroPlaybackMode::RunOnce);

    let steps = &macro_def.compiled_steps;
    let mut held_inputs: Vec<InputSource> = Vec::new();
    let mut schedule_cursor = Instant::now();

    let run_result = (|| -> Result<(), PlatformError> {
        match mode {
            MacroPlaybackMode::RunOnce => {
                execute_slice(
                    steps,
                    run_gen,
                    synth,
                    shared,
                    &mut held_inputs,
                    &mut schedule_cursor,
                )?;
            }
            MacroPlaybackMode::RepeatCount(count) => {
                let reps = count.max(1);
                if let Some((ls, le)) = find_loop_bounds(steps) {
                    let intro = &steps[..ls];
                    let body = &steps[(ls + 1)..le];
                    let outro = &steps[(le + 1)..];
                    let needs_floor = !slice_has_positive_wait(body);

                    if !execute_slice(
                        intro,
                        run_gen,
                        synth,
                        shared,
                        &mut held_inputs,
                        &mut schedule_cursor,
                    )? {
                        return Ok(());
                    }

                    for _ in 0..reps {
                        if !execute_slice(
                            body,
                            run_gen,
                            synth,
                            shared,
                            &mut held_inputs,
                            &mut schedule_cursor,
                        )? {
                            return Ok(());
                        }
                        if needs_floor {
                            schedule_cursor = advance_and_wait(
                                shared,
                                run_gen,
                                schedule_cursor,
                                MIN_LOOP_DELAY_MS,
                            )
                            .unwrap_or(schedule_cursor);
                            if is_cancelled(shared, run_gen) {
                                return Ok(());
                            }
                        }
                    }

                    let _ = execute_slice(
                        outro,
                        run_gen,
                        synth,
                        shared,
                        &mut held_inputs,
                        &mut schedule_cursor,
                    )?;
                } else {
                    let needs_floor = !slice_has_positive_wait(steps);
                    for _ in 0..reps {
                        if !execute_slice(
                            steps,
                            run_gen,
                            synth,
                            shared,
                            &mut held_inputs,
                            &mut schedule_cursor,
                        )? {
                            return Ok(());
                        }
                        if needs_floor {
                            schedule_cursor = advance_and_wait(
                                shared,
                                run_gen,
                                schedule_cursor,
                                MIN_LOOP_DELAY_MS,
                            )
                            .unwrap_or(schedule_cursor);
                            if is_cancelled(shared, run_gen) {
                                return Ok(());
                            }
                        }
                    }
                }
            }
            MacroPlaybackMode::HoldLoop | MacroPlaybackMode::ToggleLoop => {
                if let Some((ls, le)) = find_loop_bounds(steps) {
                    let intro = &steps[..ls];
                    let body = &steps[(ls + 1)..le];
                    let needs_floor = !slice_has_positive_wait(body);

                    if !execute_slice(
                        intro,
                        run_gen,
                        synth,
                        shared,
                        &mut held_inputs,
                        &mut schedule_cursor,
                    )? {
                        return Ok(());
                    }

                    while !is_cancelled(shared, run_gen) {
                        if !execute_slice(
                            body,
                            run_gen,
                            synth,
                            shared,
                            &mut held_inputs,
                            &mut schedule_cursor,
                        )? {
                            break;
                        }
                        if needs_floor {
                            match advance_and_wait(
                                shared,
                                run_gen,
                                schedule_cursor,
                                MIN_LOOP_DELAY_MS,
                            ) {
                                Some(next) => schedule_cursor = next,
                                None => break,
                            }
                        }
                    }
                } else {
                    let needs_floor = !slice_has_positive_wait(steps);
                    while !is_cancelled(shared, run_gen) {
                        if !execute_slice(
                            steps,
                            run_gen,
                            synth,
                            shared,
                            &mut held_inputs,
                            &mut schedule_cursor,
                        )? {
                            break;
                        }
                        if needs_floor {
                            match advance_and_wait(
                                shared,
                                run_gen,
                                schedule_cursor,
                                MIN_LOOP_DELAY_MS,
                            ) {
                                Some(next) => schedule_cursor = next,
                                None => break,
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    })();

    // Safety guarantee: release every pressed virtual key/button in reverse press order.
    let release_result = release_held_inputs(&mut held_inputs, synth);
    run_result.and(release_result)
}

fn advance_and_wait(
    shared: &Arc<(Mutex<PlayerShared>, Condvar)>,
    run_gen: u64,
    mut schedule_cursor: Instant,
    delay_ms: u64,
) -> Option<Instant> {
    let now = Instant::now();
    // Re-anchor if the system paused/slept significantly behind schedule.
    if now.saturating_duration_since(schedule_cursor) > Duration::from_millis(100) {
        schedule_cursor = now;
    }
    let deadline = schedule_cursor + Duration::from_millis(delay_ms);
    if wait_until_or_cancel(shared, run_gen, deadline) {
        Some(deadline)
    } else {
        None
    }
}

/// Executes a slice of [`MacroStep`]s, returning `Ok(true)` if completed or `Ok(false)` if cancelled.
fn execute_slice(
    steps: &[MacroStep],
    run_gen: u64,
    synth: &mut dyn InputSynthesizer,
    shared: &Arc<(Mutex<PlayerShared>, Condvar)>,
    held_inputs: &mut Vec<InputSource>,
    schedule_cursor: &mut Instant,
) -> Result<bool, PlatformError> {
    for &step in steps {
        if is_cancelled(shared, run_gen) {
            return Ok(false);
        }

        match step {
            MacroStep::KeyDown(key) => {
                synth.synthesize(SyntheticOutput::Key {
                    key,
                    state: KeyState::Down,
                })?;
                let src = InputSource::Keyboard(key);
                if !held_inputs.contains(&src) {
                    held_inputs.push(src);
                }
            }
            MacroStep::KeyUp(key) => {
                let src = InputSource::Keyboard(key);
                held_inputs.retain(|&h| h != src);
                synth.synthesize(SyntheticOutput::Key {
                    key,
                    state: KeyState::Up,
                })?;
            }
            MacroStep::MouseDown(button) => {
                synth.synthesize(SyntheticOutput::MouseButton {
                    button,
                    state: KeyState::Down,
                })?;
                let src = InputSource::MouseButton(button);
                if !held_inputs.contains(&src) {
                    held_inputs.push(src);
                }
            }
            MacroStep::MouseUp(button) => {
                let src = InputSource::MouseButton(button);
                held_inputs.retain(|&h| h != src);
                synth.synthesize(SyntheticOutput::MouseButton {
                    button,
                    state: KeyState::Up,
                })?;
            }
            MacroStep::Wait(ms) => {
                if ms > 0 {
                    match advance_and_wait(shared, run_gen, *schedule_cursor, ms) {
                        Some(next) => *schedule_cursor = next,
                        None => return Ok(false),
                    }
                }
            }
            MacroStep::LoopStart | MacroStep::LoopEnd => {}
        }
    }
    Ok(true)
}

fn release_held_inputs(
    held_inputs: &mut Vec<InputSource>,
    synth: &mut dyn InputSynthesizer,
) -> Result<(), PlatformError> {
    let mut first_err: Option<PlatformError> = None;

    while let Some(src) = held_inputs.pop() {
        let output = match src {
            InputSource::Keyboard(key) => SyntheticOutput::Key {
                key,
                state: KeyState::Up,
            },
            InputSource::MouseButton(button) => SyntheticOutput::MouseButton {
                button,
                state: KeyState::Up,
            },
        };
        if let Err(err) = synth.synthesize(output) {
            if first_err.is_none() {
                first_err = Some(err);
            }
        }
    }

    match first_err {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::keys::{KeyboardKey, MouseButton};
    use crate::domain::macros::{Clip, ClipKind, LoopRegion, MacroTrigger};

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

    fn wait_until_idle(player: &MacroPlayer, timeout_ms: u64) {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        while player.is_playing() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn run_once_executes_multitrack_clips_in_order() {
        let sink = SharedMockSynth::default();
        let player = MacroPlayer::new(Box::new(sink.clone()));

        let mac = MacroDefinition::new(
            MacroId(1),
            "Once",
            Some(MacroTrigger {
                source: InputSource::Keyboard(KeyboardKey::F2),
                mode: MacroPlaybackMode::RunOnce,
            }),
            vec![
                Clip {
                    id: 1,
                    kind: ClipKind::Key {
                        key: Some(KeyboardKey::KeyE),
                    },
                    track: 0,
                    start_ms: 0,
                    dur_ms: 15,
                },
                Clip {
                    id: 2,
                    kind: ClipKind::Mouse {
                        button: MouseButton::Left,
                    },
                    track: 1,
                    start_ms: 5,
                    dur_ms: 15,
                },
            ],
            4,
            None,
        );

        player.start_macro(mac);
        wait_until_idle(&player, 500);
        assert!(!player.is_playing());

        assert_eq!(
            sink.snapshot(),
            vec![
                SyntheticOutput::Key {
                    key: KeyboardKey::KeyE,
                    state: KeyState::Down
                },
                SyntheticOutput::MouseButton {
                    button: MouseButton::Left,
                    state: KeyState::Down
                },
                SyntheticOutput::Key {
                    key: KeyboardKey::KeyE,
                    state: KeyState::Up
                },
                SyntheticOutput::MouseButton {
                    button: MouseButton::Left,
                    state: KeyState::Up
                },
            ]
        );
    }

    #[test]
    fn hold_loop_cancellation_mid_wait_releases_all_held_keys_immediately() {
        let sink = SharedMockSynth::default();
        let player = MacroPlayer::new(Box::new(sink.clone()));

        let mac = MacroDefinition::new(
            MacroId(2),
            "Long Hold",
            Some(MacroTrigger {
                source: InputSource::MouseButton(MouseButton::XButton1),
                mode: MacroPlaybackMode::HoldLoop,
            }),
            vec![
                Clip {
                    id: 1,
                    kind: ClipKind::Key {
                        key: Some(KeyboardKey::KeyW),
                    },
                    track: 0,
                    start_ms: 0,
                    dur_ms: 5_000,
                },
                Clip {
                    id: 2,
                    kind: ClipKind::Mouse {
                        button: MouseButton::Right,
                    },
                    track: 1,
                    start_ms: 0,
                    dur_ms: 5_000,
                },
            ],
            4,
            None,
        );

        player.start_macro(mac);
        // Wait until both Down events have been emitted
        let deadline = Instant::now() + Duration::from_millis(300);
        while sink.snapshot().len() < 2 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(sink.snapshot().len(), 2);

        let t_cancel = Instant::now();
        player.stop_macro(MacroId(2));
        wait_until_idle(&player, 300);
        let cancel_latency = t_cancel.elapsed();

        assert!(
            cancel_latency < Duration::from_millis(200),
            "Cancellation should interrupt 5s wait immediately, took {cancel_latency:?}"
        );
        // Reverse-order guaranteed release: MouseRight Up, then KeyW Up
        assert_eq!(
            sink.snapshot(),
            vec![
                SyntheticOutput::Key {
                    key: KeyboardKey::KeyW,
                    state: KeyState::Down
                },
                SyntheticOutput::MouseButton {
                    button: MouseButton::Right,
                    state: KeyState::Down
                },
                SyntheticOutput::MouseButton {
                    button: MouseButton::Right,
                    state: KeyState::Up
                },
                SyntheticOutput::Key {
                    key: KeyboardKey::KeyW,
                    state: KeyState::Up
                },
            ]
        );
    }

    #[test]
    fn repeat_count_with_loop_region_runs_intro_once_and_repeats_loop_body() {
        let sink = SharedMockSynth::default();
        let player = MacroPlayer::new(Box::new(sink.clone()));

        let mac = MacroDefinition::new(
            MacroId(3),
            "Intro+Loop",
            Some(MacroTrigger {
                source: InputSource::Keyboard(KeyboardKey::F3),
                mode: MacroPlaybackMode::RepeatCount(2),
            }),
            vec![
                Clip {
                    id: 1,
                    kind: ClipKind::Key {
                        key: Some(KeyboardKey::KeyA),
                    },
                    track: 0,
                    start_ms: 0,
                    dur_ms: 5,
                },
                Clip {
                    id: 2,
                    kind: ClipKind::Key {
                        key: Some(KeyboardKey::KeyB),
                    },
                    track: 0,
                    start_ms: 10,
                    dur_ms: 5,
                },
            ],
            4,
            Some(LoopRegion {
                start_ms: 10,
                end_ms: 15,
            }),
        );

        player.start_macro(mac);
        wait_until_idle(&player, 500);
        assert!(!player.is_playing());

        assert_eq!(
            sink.snapshot(),
            vec![
                // Intro once: A down/up
                SyntheticOutput::Key {
                    key: KeyboardKey::KeyA,
                    state: KeyState::Down
                },
                SyntheticOutput::Key {
                    key: KeyboardKey::KeyA,
                    state: KeyState::Up
                },
                // Loop body twice: B down/up x2
                SyntheticOutput::Key {
                    key: KeyboardKey::KeyB,
                    state: KeyState::Down
                },
                SyntheticOutput::Key {
                    key: KeyboardKey::KeyB,
                    state: KeyState::Up
                },
                SyntheticOutput::Key {
                    key: KeyboardKey::KeyB,
                    state: KeyState::Down
                },
                SyntheticOutput::Key {
                    key: KeyboardKey::KeyB,
                    state: KeyState::Up
                },
            ]
        );
    }
}
