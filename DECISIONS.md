# Architectural Decision Records (`DECISIONS.md`)

This file logs key architectural and technology decisions for `ctt-app`, including options considered, rationale, trade-offs, and open decisions marked `TBD`.

---

## ADR-001: Rust (`2021`) + Slint (`1.19`) for Desktop UI
- **Status**: Accepted (Supersedes previous `eframe`/`egui` baseline per user migration requirement)
- **Context**: `ctt-app` is a gaming-style peripheral key binding and macro utility that requires a responsive, high-performance native desktop GUI with clean separation between visual markup and OS background threads.
- **Alternatives Considered**: `eframe`/`egui` (immediate-mode), Tauri v2 (Rust + WebView/JS), `iced`.
- **Decision**: Migrate to `slint = "1.19"` declarative UI with `slint-build = "1.19"` via `ui/app.slint` and `build.rs`.
- **Consequences & Trade-Offs**:
  - *Pros*: Declarative component markup in `.slint`, strong decoupling between UI views and Rust domain state/engine, reactive property binding, first-class desktop rendering backends.
  - *Cons*: Requires build-time code generation via `slint-build` in `build.rs`.

---

## ADR-002: Multi-Threaded Separation Between `egui` UI, Domain Logic, and OS Input Engine
- **Status**: Accepted (Target Architecture Baseline)
- **Context**: OS-level keyboard/mouse hooks require microsecond response times, while macro playback requires accurate sleep/delay loops and immediate cancellation, and `egui` requires an unblocked UI render loop.
- **Alternatives Considered**: Single-threaded polling inside `egui::App::update()` vs. multi-threaded engine behind trait abstractions.
- **Decision**: Separate `ui`, pure `domain`, background `engine`, `platform` (OS hooks/synthesis), and `storage` layers, communicating across threads via bounded channels, lightweight snapshots, and atomic cancellation tokens.
- **Consequences & Trade-Offs**:
  - *Pros*: Prevents hook timeouts and UI freezes; keeps all domain validation, macro logic, and mapping conflict checks testable under `cargo test` without OS hooks or GUI windows.
  - *Cons*: Slightly more upfront boilerplate for channels/traits and explicit synchronization of active pressed-key state.

---

## ADR-003: OS-Level Input Hook and Event Synthesis Backend
- **Status**: **TBD** (Needs decision before Milestone 3)
- **Context**: `ctt-app` requires reliable low-level interception (including suppression of remapped physical keys), scan code accuracy, mouse movement/clicks, and synthetic event tagging to prevent feedback loops.
- **Alternatives Under Consideration**:
  1. Direct Win32 FFI via `windows` or `windows-sys` (`SetWindowsHookExW`, `SendInput` with `dwExtraInfo` marker).
  2. Higher-level cross-platform crates (`rdev` for listen/grab + `enigo` for input simulation).
- **Open Questions**: Is `ctt-app` Windows-only or cross-platform? Do higher-level crates expose sufficient control over `dwExtraInfo` loop tagging, extended scan codes, and Windows low-level hook suppression?

---

## ADR-004: Configuration, Profile, and Macro Serialization Format
- **Status**: **TBD** (Needs decision before Milestone 2)
- **Context**: Users need persistent application settings, mapping profiles, and importable/exportable macro files with forward-compatible schema versioning.
- **Alternatives Under Consideration**:
  1. `serde` + `serde_json` (ubiquitous, easy import/export interoperability).
  2. `serde` + `toml` (human-editable app config) paired with JSON for macro import/export.
  3. `serde` + `ron` (Rust-native enum ergonomics, less familiar to end users).
- **Open Questions**: Should app config use a separate format (e.g., TOML) from exported macro/profile bundles (e.g., JSON), and where should default data live on Windows (`%APPDATA%\ctt-app` via `directories` crate vs. portable mode)?

---

## ADR-005: Structured Logging and Diagnostics Strategy
- **Status**: **TBD** (Needs decision before Milestone 2)
- **Context**: Debugging OS hook failures, malformed imports, and macro state transitions requires structured diagnostics without logging sensitive user keystrokes during normal use.
- **Alternatives Under Consideration**:
  1. `tracing` + `tracing-subscriber` (structured fields, rolling file logs, optional in-app diagnostics buffer).
  2. `log` + `env_logger` / `simplelog` (lighter dependency footprint).
- **Open Questions**: Do users need an in-app "Diagnostics / Activity Log" panel and optional rolling log files for bug reports?

---

## ADR-006: Background Operation, System Tray, and Elevation Policy
- **Status**: **TBD** (Needs decision before Milestone 6)
- **Context**: Gaming peripheral tools typically minimize to the system tray while active and sometimes face Windows UIPI limits when interacting with elevated games/apps.
- **Alternatives Under Consideration**:
  - System tray (`tray-icon`) vs. standard taskbar window only.
  - On-demand elevation notice vs. optional run-as-admin startup preference.
- **Open Questions**: Should closing the main window minimize to the system tray when mapping is active, or exit immediately?
