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
  - Cargo package scaffold (`ctt-app` `v0.1.0`) in [Cargo.toml](Cargo.toml) and [`.gitignore`](.gitignore).
  - Slint UI window (`900x620`, min `480x360`, title `"CTT App"`) in [ui/app.slint](ui/app.slint) compiled via [build.rs](build.rs) and bootstrapped in [src/main.rs](src/main.rs).
  - Starter `CttApp` state struct in [src/app.rs](src/app.rs) with top bar (`Quit`), `"Status: Ready"` bar, `"Search:"` field, and `"Increment"` counter demo.
- **Not Yet Implemented**:
  - Domain models (`macros`, `mapping`, `config`).
  - OS input hooks, event synthesis, and background execution engine.
  - Persistent storage for profiles, macros, and settings.
  - Automated unit or integration tests (`tests/` or `#[cfg(test)]` modules).
  - `Cargo.lock` (dependencies have not yet been resolved/built in this repository).

## 7. Important Constraints & Known Limitations
- **OS Hook Latency & Stability**: Windows low-level keyboard/mouse hooks (`WH_KEYBOARD_LL` / `WH_MOUSE_LL`, if chosen) have strict timeout limits and must never block.
- **Windows UIPI / Elevation**: Standard-privilege desktop apps cannot inject input into elevated (Administrator) windows unless elevated or signed with UIAccess; exact privilege strategy is **TBD**.
- **Safety Requirements**: Emergency stop, synthetic event loop isolation, and guaranteed release of pressed virtual keys on interruption are mandatory before shipping any input injection code.
- **Build Verification Gap**: Initial scaffold files have been inspected on disk, while `cargo check` / `cargo test` have not yet been executed in this repository environment.
