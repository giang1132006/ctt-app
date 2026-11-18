# Macro System Rules

These rules govern the design, execution, persistence, and safety of the Macro System in `ctt-app`.

## 1. Macro Data Structures
- Every macro model must define:
  - Stable unique identifier (`MacroId`) and human-readable `name`.
  - Schema `version` for forward-compatible persistence and import/export.
  - Optional trigger configuration (hotkey/button + activation mode).
  - Playback mode (`RunOnce`, `RepeatCount(u32)`, `ToggleLoop`, `HoldToLoop`).
  - Ordered sequence of `MacroStep` items (`Vec<MacroStep>`).
- Keep macro definitions pure data structures inside the domain layer, free of UI widgets or OS handles.

## 2. Keyboard & Mouse Event Representation
- Represent input actions with strongly typed enums—never untyped magic numbers:
  - Keyboard: `KeyDown(KeyId)`, `KeyUp(KeyId)` (and optional composite convenience helper that expands cleanly into down + delay + up).
  - Mouse buttons: `MouseDown(MouseButton)`, `MouseUp(MouseButton)`.
  - Mouse motion & wheel: `MouseMoveRelative { dx: i32, dy: i32 }`, `MouseMoveAbsolute { x: i32, y: i32 }`, `MouseScroll { delta_x: i32, delta_y: i32 }`.
  - Timing: `Delay(Duration)` (explicit millisecond or microsecond representation).
- Preserve distinction between physical scan codes and virtual key codes where applicable so game/application input behaves predictably.

## 3. Event Sequencing & Stuck-Key Prevention
- **Down/Up Balance Tracking**: Track every `KeyDown` and `MouseDown` emitted by the playback engine in an active pressed-input set.
- **Guaranteed Release**: Whenever a macro finishes, encounters an error, is paused/stopped, or hits the emergency stop hotkey, the engine must immediately emit matching `KeyUp` / `MouseUp` events for all tracked pressed inputs before transitioning to `Idle`.
- Validate macro sequences on save/import and warn the user if a macro leaves keys pressed at the end of a single iteration.

## 4. Timing & Delays
- Store durations explicitly (`std::time::Duration` or serialized `u64` milliseconds/microseconds with clear field names like `delay_ms`).
- Enforce a configurable minimum delay floor (or post-iteration yield) on `ToggleLoop` / `HoldToLoop` / high `RepeatCount` macros so tight zero-delay loops cannot freeze the OS input queue or starve CPU cores.
- Use monotonic clocks (`std::time::Instant`) when recording event timestamps and scheduling playback timing—never wall-clock (`SystemTime`) timestamps.

## 5. Recording & Playback Lifecycle
- Model execution with an explicit state machine: `Idle`, `Recording`, `Playing`, `Paused` (optional), `Stopping`, `Error`.
- Enforce mutual exclusion: recording cannot start while playback is active, and playback cannot run while recording is active.
- During recording, ignore input events targeting `ctt-app`'s own "Stop Recording" / "Emergency Stop" controls so control clicks are not captured into the macro.

## 6. Cancellation & Interruption
- All playback execution must accept a thread-safe cancellation signal (e.g., `Arc<AtomicBool>` or channel/condvar signal).
- Check cancellation before executing each step and break long `Delay` intervals into small interruptible slices (or wait on a `Condvar` with timeout) so emergency stop interrupts playback within a few milliseconds rather than waiting for a multi-second delay to expire.

## 7. Serialization, Import/Export & Validation
- Include a `schema_version` and application identifier in exported macro files.
- Validate imported macros before activation:
  - Reject malformed schemas, negative/overflowing durations, unknown step types, or oversized step counts exceeding safety limits.
  - Report structured validation errors to the UI without crashing or partially mutating existing macros.
