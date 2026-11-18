---
name: ui-development
description: >-
  Designs and implements modular eframe/egui user interface panels, widgets,
  key-capture modals, active-automation status indicators, and non-blocking
  UI-to-engine state synchronization. Use this skill when building or modifying
  UI views and interactions in ctt-app.
---

# UI Development

## Purpose
Build clean, responsive, modular `eframe`/`egui` (`0.31`) views in `ctt-app` without coupling presentation components to domain validation, file I/O, or OS input APIs.

## Activation Criteria
Activate this skill when:
- Adding or updating `egui` panels, dialogs, editors, navigation, or status bars.
- Implementing interactive key/mouse capture flows or profile/macro management forms.
- Synchronizing engine status snapshots or user commands with [src/app.rs](../../../src/app.rs).

## Workflow
1. **Review UI/UX & Architecture Rules**:
   - Read [UI/UX Rules](../../rules/ui-ux-rules.md), [Architecture Rules](../../rules/architecture-rules.md), and [src/app.rs](../../../src/app.rs).
2. **Separate View State from Domain & Engine State**:
   - Keep transient UI controls (search filters, active tab, modal visibility, edit buffers) inside UI structs.
   - Send validated user intentions to the engine/domain via explicit actions/commands; read lightweight snapshots back for rendering.
3. **Structure Modular Panels**:
   - Extract each major screen/panel (top status/control bar, sidebar navigation, macro editor, key mapper, settings) into focused modules under `src/ui/` instead of bloating `CttApp::update`.
4. **Implement Safe Input Capture & Visibility**:
   - Render explicit capture overlays (`"Press any key or mouse button (Esc to cancel)"`) when binding keys.
   - Keep global automation status (active profile, recording/playing state, Emergency Stop button/hotkey) persistently visible.
   - Surface inline validation warnings and errors clearly.
5. **Audit Immediate-Mode Performance**:
   - Avoid disk reads, heavy sorting, or blocking locks inside `update()`.
   - Verify theme compatibility (light/dark via `global_theme_preference_buttons` already in `src/app.rs`).

## Constraints
- Never call OS hook APIs, `thread::sleep`, or synchronous disk writes inside `eframe::App::update()`.
- Never bury business validation or serialization logic inside UI widget callbacks.

## Expected Output
- Modular `egui` view modules/components, responsive non-blocking command/snapshot integration, clear error/status states, and verified `cargo check` compilation.
