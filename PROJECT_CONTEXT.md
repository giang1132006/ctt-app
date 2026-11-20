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
- **OS Input Hook & Synthesis Backend**: **TBD** (see [DECISIONS.md](DECISIONS.md)).
- **Serialization / Persistence Format**: **TBD** (see [DECISIONS.md](DECISIONS.md)).
- **Logging / Telemetry Crate**: **TBD** (see [DECISIONS.md](DECISIONS.md)).
- **System Tray / Hotkey Integration**: **TBD**.

## 6. Current Implementation Status
- **Implemented**:
  - Cargo package scaffold (`ctt-app` `v0.1.0`) in [Cargo.toml](Cargo.toml), [src/lib.rs](src/lib.rs), and [`.gitignore`](.gitignore).
  - Slint UI shell (`AppWindow`), overview page ([ui/components/home.slint](ui/components/home.slint)), interactive Multi-Track NLE Create Macro Studio ([ui/components/macros_view.slint](ui/components/macros_view.slint)) + `import.md` sequence modal ([ui/components/macro_io_modal.slint](ui/components/macro_io_modal.slint)), interactive Key Binding view ([ui/components/binding_view.slint](ui/components/binding_view.slint)), and persistent safety status bar ([ui/components/status_bar.slint](ui/components/status_bar.slint)) compiled via [build.rs](build.rs).
  - Coordinator `CttApp` in [src/app.rs](src/app.rs), NLE timeline state machine `MacroEditorState` in [src/macro_editor.rs](src/macro_editor.rs), and entry point in [src/main.rs](src/main.rs).
  - Pure `domain` layer (`src/domain/*`): exhaustive `KeyboardKey` (`123` standard keys + `OemScanCode`), `MouseButton`, `MouseWheel`, `MappingTarget`, `CompiledMappingTable` (`O(1)` lookup), `Profile`, conflict/swap/chain/lockout validator, plus Multi-Track NLE macro timeline models/compiler (`src/domain/macros.rs`) and `import.md` parser/exporter (`src/domain/macro_io.rs`).
  - `engine` layer (`src/engine/*`): `RemapEngine` state machine handling key/combo/mouse press, hold, auto-repeat suppression, profile-switch release, synthetic loop filtering, and emergency stop cleanup.
  - `platform` abstraction (`src/platform/*`): `InputSynthesizer`/`InputInterceptor` traits, `MockInputSynthesizer`, Win32 keymap round-trip table, and `CTT_SYNTHETIC_TAG` packet scaffolding.
  - `storage` layer (`src/storage/*`): schema-versioned `ProfileStore` with atomic file writes and validation on load (persisting both key-binding rules and multi-track macros).
  - Unit tests (`src/**`) and integration tests (`tests/key_mapping_domain_tests.rs`, `tests/remap_engine_tests.rs`).
- **Not Yet Implemented**:
  - Live Windows OS hook/synthesis FFI wiring (`SetWindowsHookExW` / `SendInput` — **TBD** under `ADR-003`) and background macro playback worker thread dispatching compiled `MacroStep` sequences to OS synthesis.
  - Application config persistence wiring at startup (`AppConfig` disk load/save).

## 7. Important Constraints & Known Limitations
- **OS Hook Latency & Stability**: Windows low-level keyboard/mouse hooks (`WH_KEYBOARD_LL` / `WH_MOUSE_LL`, if chosen) have strict timeout limits and must never block.
- **Windows UIPI / Elevation**: Standard-privilege desktop apps cannot inject input into elevated (Administrator) windows unless elevated or signed with UIAccess; exact privilege strategy is **TBD**.
- **Safety Requirements**: Emergency stop, synthetic event loop isolation, and guaranteed release of pressed virtual keys on interruption are mandatory before shipping any input injection code.
- **Build Verification Gap**: Initial scaffold files have been inspected on disk, while `cargo check` / `cargo test` have not yet been executed in this repository environment.
