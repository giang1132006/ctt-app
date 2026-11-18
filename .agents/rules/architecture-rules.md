# Architecture Rules

These rules define the technical architecture boundaries for `ctt-app` based on its Rust (`2021`) + `eframe`/`egui` (`0.31`) stack. Note: modules beyond `src/main.rs` and `src/app.rs` represent the agreed target structure as the codebase grows.

## 1. Module Boundaries
Organize code into four distinct layers:
- **`ui` (`src/app.rs`, future `src/ui/*`)**: Window setup, `eframe::App::update`, panels, widgets, transient view state (filters, modals, key-capture prompts). Contains zero OS hook calls and zero direct file I/O.
- **`domain` (future `src/domain/*` or `src/macros/*`, `src/mapping/*`, `src/config/*`)**: Pure Rust types and logic—macro sequences, key-mapping rules, profile models, validation, conflict/cycle detection, and playback state machines. Must be completely independent of `egui`/`eframe` and OS-specific crates.
- **`engine` & `platform` (future `src/engine/*`, `src/platform/*`)**: Background automation runtime (macro player, recorder, remapper) and OS-specific backends implementing platform traits (low-level hooks, event synthesis, monotonic timing).
- **`storage` (future `src/storage/*`)**: Profile, macro, and settings persistence, schema migrations, import/export parsing, and atomic disk writes.

## 2. Dependency Direction
- Allowed flow: `ui` -> `domain`, `ui` -> `engine` (via commands/snapshots), `engine`/`platform` -> `domain`, `storage` -> `domain`.
- **Forbidden**:
  - `domain` importing `egui`, `eframe`, `windows`, `winapi`, or filesystem persistence details.
  - `ui` directly invoking OS hook APIs or performing synchronous disk writes inside `update()`.
  - Circular dependencies between `macros`, `mapping`, and `config` submodules.

## 3. Data Flow & Threading Model
- **Main UI Thread (`eframe`)**: Must stay responsive at frame rate. Never execute `thread::sleep`, blocking mutex waits, or synchronous macro delays on the UI thread.
- **OS Input Hook Thread**: Must process incoming OS events in microseconds (match rule or forward event to a bounded channel and return immediately). Blocking in a Windows low-level hook causes system-wide input lag and silent hook removal by the OS.
- **Engine / Playback Worker Thread(s)**: Own active macro execution, timing loops, and state transitions. Receive commands from the UI via bounded channels (e.g., `std::sync::mpsc::SyncSender` or equivalent) and publish lightweight immutable state snapshots or events back to the UI, calling `egui::Context::request_repaint()` when UI updates are needed.

## 4. Event Handling & OS Integration
- Isolate OS-specific hook/synthesis code behind traits in `platform` (e.g., `InputInterceptor`, `InputSynthesizer`) with `#[cfg(target_os = "windows")]` backends and fake/mock implementations for tests.
- Tag all events synthesized by `ctt-app` (e.g., via platform-specific extra info or internal suppression tracking) so hooks distinguish physical user input from synthetic automation output.
- Enforce RAII (`Drop`) cleanup for all registered hooks and ensure active virtual keys/buttons are released on stop, panic, or shutdown.

## 5. Configuration Persistence
- Include an explicit `schema_version` field in all saved configuration, profile, and macro files.
- Perform writes atomically (write to a temporary file in the target directory, flush, then rename) so crashes or power loss cannot corrupt user profiles.
- Validate loaded configurations against domain invariants before activating them; surface clear recovery errors on malformed files instead of panicking.

## 6. Logging & Error Handling
- Define boundary-specific typed errors (e.g., `MacroError`, `MappingError`, `StorageError`, `PlatformError`) using structured variants.
- Never allow Rust panics to unwind across FFI/OS hook boundaries (`std::panic::catch_unwind` at FFI callbacks where applicable, logging and failing safe).
- Never log raw keystrokes during normal key-mapping operation outside explicit user-initiated diagnostics or active macro recording debug views.

## 7. Testing Boundaries
- **Unit Tests**: All `domain` logic (macro step validation, delay bounds, key-mapping conflict/cycle detection, profile switching rules, serialization round-trips) must run deterministically under `cargo test` with no GUI window and no OS hooks.
- **Integration Tests**: Test engine playback cancellation, stuck-key cleanup, and loop prevention using an in-memory mock `InputSynthesizer` and simulated input stream.
- **Manual / Hardware Smoke Tests**: Platform-specific Windows hook/synthesis backends require documented manual verification checklists for UAC/UIPI, multi-monitor coordinates, and emergency stop responsiveness.
