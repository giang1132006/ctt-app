# Roadmap (`ctt-app`)

## Current Development Stage
**Stage 0 — Project Scaffold & AI Agent Operating System Initialization**

Only the initial Rust `eframe`/`egui` desktop window shell exists today. Core application domains (macros, key mapping, configuration persistence, and OS input integration) have not yet been implemented.

---

## Completed Milestones
- **M0.1 — Repository Initialization**: Git repository created with [README.md](README.md).
- **M0.2 — Rust GUI Scaffold**: Created [.gitignore](.gitignore), [Cargo.toml](Cargo.toml) (`eframe = "0.31"`, `egui = "0.31"`), [src/main.rs](src/main.rs), and [src/app.rs](src/app.rs) starter UI. *(Pending first local `cargo check`/`cargo run` verification in user's shell.)*
- **M0.3 — AI Agent Operating System**: Established [AGENTS.md](AGENTS.md), modular rules in `.agents/rules/`, 10 specialized skills in `.agents/skills/`, and core architectural/context documentation.

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
