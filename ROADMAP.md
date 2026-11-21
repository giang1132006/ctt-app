# Roadmap (`ctt-app`)

## Current Development Stage
**Stage 2 — Live Windows OS Hook/Synthesis Backend, Background Macro Playback Engine & Automatic Profile Persistence**

The 4-layer architecture (`domain`, `engine`, `platform`, `storage`, `ui`) is now fully wired end-to-end on Windows: live `WH_KEYBOARD_LL` + `WH_MOUSE_LL` interception, `SendInput` synthesis with `CTT_SYNTHETIC_TAG` loop isolation, background multi-track `MacroPlayer` execution (`HoldLoop`, `ToggleLoop`, `RunOnce`, `RepeatCount`, `LoopRegion`), global `Ctrl + Shift + F12` Emergency Stop, and automatic atomic profile persistence.

---

## Completed Milestones
- **M0.1 — Repository Initialization**: Git repository created with [README.md](README.md).
- **M0.2 — Rust + Slint GUI Scaffold**: Created [.gitignore](.gitignore), [Cargo.toml](Cargo.toml) (`slint = "1.19"`, `slint-build = "1.19"`), [build.rs](build.rs), [ui/app.slint](ui/app.slint), [src/main.rs](src/main.rs), and [src/app.rs](src/app.rs).
- **M0.3 — AI Agent Operating System**: Established [AGENTS.md](AGENTS.md), modular rules in `.agents/rules/`, specialized skills in `.agents/skills/`, and core architectural/context documentation.
- **M1 — Key Binding Domain, Remap Engine, Atomic Storage & UI Integration**:
  - Implemented `src/domain/` (`keys.rs`, `mapping.rs`, `validation.rs`, `profile.rs`, `config.rs`, `macros.rs`) supporting all 123 standard keyboard keys + `OemScanCode` mapped to any keyboard key, combo, mouse button, mouse wheel, or disabled state.
  - Implemented `src/engine/` (`remapper.rs`, `command.rs`) with `O(1)` dispatch, `ActiveHolds` tracking, mouse hold anti-spam, profile-switch release, loop prevention, and emergency stop cleanup.
  - Implemented `src/platform/` traits, `MockInputSynthesizer`, Win32 keymap/scancode mapping table, and `src/storage/` atomic `ProfileStore`.
  - Built Slint `BindingView` (`ui/components/binding_view.slint`) and `StatusBar` (`ui/components/status_bar.slint`), plus unit & integration test suites.

---

## In-Progress Tasks
- None active (AI Agent Operating System initialization just completed).

---

## Planned Milestones

### Milestone 1: Core Architecture Decisions & Domain Models
- Finalize open architectural decisions in [DECISIONS.md](DECISIONS.md) (OS input backend, serialization format/crate, logging crate, storage directory convention).
- Implement `domain` module: key/mouse identifiers (`ScanCode`, `VirtualKey`, `MouseButton`), macro steps/modes, mapping rules, profiles, validation, and conflict/cycle detection.
- Establish comprehensive `cargo test` unit test suite for all domain invariants.

### Milestone 2: Persistence & Application Configuration
- Implement `storage` module with versioned schemas, atomic file writes, and safe fallback on corrupted files.
- Persist user preferences (theme, emergency stop hotkey, active profile, macro library).

### Milestone 3: Platform Input Abstraction & Safety Harness
- Define `platform` input interception and synthesis traits plus in-memory mock implementations for automated engine tests.
- Implement Windows input backend (`#[cfg(target_os = "windows")]`) featuring:
  - Low-latency non-blocking hook forwarding.
  - Synthetic-event tagging to prevent remap/macro feedback loops.
  - Always-active Emergency Stop panic hotkey and RAII hook cleanup.

### Milestone 4: Key Mapping Engine & Profile UI
- Implement compiled key-mapping dispatch, active pressed-key tracking, and clean profile switching.
- Build modular `egui` views for Key Mapping rules, Profile Management, Key Capture modal, and persistent Active Status bar.

### Milestone 5: Macro Recording, Editor & Playback Engine
- Implement background macro playback engine with monotonic timing, interruptible delays, loop safety floors, and guaranteed virtual `KeyUp`/`MouseUp` release on stop/cancel.
- Implement macro recording (excluding app stop controls) and modular `egui` Macro Editor + import/export flows.

### Milestone 6: Desktop Polish & Release Readiness
- Evaluate system tray minimization / background running behavior (**TBD**).
- Configure release profiles, Windows icon/metadata, packaging, and safety release checklists.

---

## Known Technical Risks
1. **Windows Hook Timeout (`LowLevelHooksTimeout`)**: Blocking inside `WH_KEYBOARD_LL` or `WH_MOUSE_LL` causes OS-wide input stutter and silent hook eviction by Windows.
2. **Synthetic Input Recursion**: Failing to tag/ignore events injected by `ctt-app` can cause infinite key loops (`A -> B -> A`).
3. **Stuck Modifier / Virtual Keys**: Canceling a macro or switching profiles mid-hold without tracking and emitting matching `Up` events leaves keys logically stuck down.
4. **UAC / UIPI Restrictions**: Non-elevated processes on Windows cannot inject input into elevated (Administrator) applications.
5. **Dual-Layer Key Identity Bugs**: Confusing hardware scan codes with virtual key codes causes extended keys, Numpad keys, or non-US keyboard layouts to behave inconsistently.
