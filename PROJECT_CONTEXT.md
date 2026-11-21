# Project Context (`ctt-app`)

## 1. Overview
`ctt-app` is a desktop application built with Rust for keyboard and mouse automation, macro recording/playback, and gaming-style peripheral key binding (*"Ứng dụng bind phím giống driver chuột phím gaming"*, per [README.md](README.md)).

## 2. Main Objectives
- Provide a lightweight, responsive native desktop utility for configuring custom key mappings and automated input macros without vendor-locked peripheral bloatware.
- Maintain strict operational safety around OS-level input hooks and event synthesis (explicit user activation, emergency stop, hook cleanup, no hidden monitoring).

## 3. Functional Scope (Initial Direction)
The planned functional scope covers three core domains (all currently **unimplemented** beyond the GUI shell):

1. **Macro System**
   - Record and play back keyboard and mouse sequences.
   - Create, edit, save, and organize macros.
   - Configure delays, playback modes (`RunOnce`, repeats, toggle/hold loops), and sequences.
   - Support reliable playback controls and macro import/export.
2. **Key Mapping System**
   - Remap physical keyboard keys and mouse inputs to other keys, combos, or macro triggers.
   - Manage named mapping profiles with switching and enable/disable toggles.
   - Detect conflicting or cyclic mappings and persist profile configurations.
3. **Application Configuration**
   - Manage application preferences, startup/safety settings (e.g., emergency stop hotkey), and persisted user profiles.
   - Provide structured error handling and diagnostics/logging.

## 4. Target Platforms
- **Primary Target**: Windows desktop (evidenced by `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` in [src/main.rs](src/main.rs) and the Windows workspace environment).
- **Cross-Platform Support (macOS / Linux)**: **TBD** — not yet decided or scoped.

## 5. Current Technology Stack
- **Language**: Rust (`edition = "2021"`, [Cargo.toml](Cargo.toml)).
- **Build & Package Manager**: Cargo.
- **Desktop GUI Framework**: `slint = "1.19"` with `slint-build = "1.19"` (declarative native UI in `ui/app.slint`).
- **OS Input Hook & Synthesis Backend**: Direct zero-dependency Win32 FFI (`user32`, `kernel32`, `winmm` in `src/platform/windows/*` per `ADR-003`).
- **Serialization / Persistence Format**: Custom schema-versioned UTF-8 `.ctt` profile format + `app_state.cfg` with atomic writes (`src/storage/profile_store.rs`).
- **Logging / Telemetry Crate**: **TBD** (see [DECISIONS.md](DECISIONS.md)).
- **System Tray / Hotkey Integration**: Global Emergency Stop (`Ctrl + Shift + F12`) active via `WH_KEYBOARD_LL`; System Tray remains **TBD**.

## 6. Current Implementation Status
- **Implemented**:
  - Cargo package scaffold (`ctt-app` `v0.1.0`) in [Cargo.toml](Cargo.toml), [src/lib.rs](src/lib.rs), and [`.gitignore`](.gitignore).
  - Slint UI shell (`AppWindow`), overview page ([ui/components/home.slint](ui/components/home.slint)), interactive Multi-Track NLE Create Macro Studio ([ui/components/macros_view.slint](ui/components/macros_view.slint)) + `import.md` sequence modal ([ui/components/macro_io_modal.slint](ui/components/macro_io_modal.slint)), interactive Key Binding view ([ui/components/binding_view.slint](ui/components/binding_view.slint)), and persistent safety status bar ([ui/components/status_bar.slint](ui/components/status_bar.slint)) compiled via [build.rs](build.rs).
  - Coordinator `CttApp` in [src/app.rs](src/app.rs), NLE timeline state machine `MacroEditorState` in [src/macro_editor.rs](src/macro_editor.rs), and entry point in [src/main.rs](src/main.rs) wired to live Windows hooks/synthesis (`CttApp::new_live`) and automatic profile persistence.
  - Pure `domain` layer (`src/domain/*`): exhaustive `KeyboardKey` (`123` standard keys + `OemScanCode`), `MouseButton`, `MouseWheel`, `MappingTarget`, `CompiledMappingTable` (`O(1)` lookup), `Profile`, conflict/swap/chain/lockout validator, plus Multi-Track NLE macro timeline models/compiler (`src/domain/macros.rs`) and `import.md` parser/exporter (`src/domain/macro_io.rs`).
  - `engine` layer (`src/engine/*`): `RemapEngine` state machine, background `MacroPlayer` worker thread (`HoldLoop`, `ToggleLoop`, `RunOnce`, `RepeatCount`, `LoopRegion`, monotonic timing, sub-ms `Condvar` cancellation, guaranteed stuck-key release), and unified `InputDispatcher` + `LiveAutomationBackend` (`src/engine/runtime.rs`).
  - `platform` layer (`src/platform/*`): `InputSynthesizer`/`InputInterceptor` traits, `MockInputSynthesizer`, bidirectional Win32 keymap table, and live Windows FFI backend (`src/platform/windows/ffi.rs`, `hook.rs` installing `WH_KEYBOARD_LL` + `WH_MOUSE_LL`, `synth.rs` executing `SendInput` stamped with `CTT_SYNTHETIC_TAG`, RAII hook cleanup, `HighResTimerGuard`).
  - `storage` layer (`src/storage/*`): schema-versioned `ProfileStore` with atomic file writes, directory loading/saving/deletion, active profile restoration (`app_state.cfg`), and validation on load (persisting both key-binding rules and multi-track macros).
  - Unit tests (`src/**`) and integration tests (`tests/key_mapping_domain_tests.rs`, `tests/remap_engine_tests.rs`, `tests/automation_runtime_tests.rs`).
- **Not Yet Implemented**:
  - Configurable `AppConfig` persistence (e.g., custom Emergency Stop hotkey UI editor).
  - System tray minimization & structured rolling log files (`ADR-005`, `ADR-006`).

## 7. Important Constraints & Known Limitations
- **OS Hook Latency & Stability**: Windows low-level keyboard/mouse hooks (`WH_KEYBOARD_LL` / `WH_MOUSE_LL`) run on a dedicated message-pump thread, short-circuit `WM_MOUSEMOVE` and `CTT_SYNTHETIC_TAG` events without locking, and guard FFI callbacks with `std::panic::catch_unwind`.
- **Windows UIPI / Elevation**: Standard-privilege desktop apps cannot inject input into elevated (Administrator) windows unless elevated; `SendInput` UIPI failures are surfaced cleanly in the status bar with an Administrator hint.
- **Safety Requirements**: Emergency stop (`Ctrl + Shift + F12` + UI `Stop All`), synthetic event loop isolation (`CTT_SYNTHETIC_TAG`), capture/recording mutual exclusion, and guaranteed reverse-order release of pressed virtual keys/buttons on interruption or shutdown are enforced.
- **Verification**: Automated unit and integration tests run safely against mock synthesizers under `cargo test`; live Windows hook and `SendInput` behavior against target games/apps requires manual smoke testing.
