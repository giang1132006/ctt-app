# UI/UX Rules

These rules govern the desktop user interface built with `eframe` and `egui` (`0.31`) in `ctt-app`.

## 1. Component & Panel Structure
- Keep [src/app.rs](../../src/app.rs) as the top-level application coordinator (`CttApp`) and split views into focused UI modules (e.g., `src/ui/top_bar.rs`, `src/ui/sidebar.rs`, `src/ui/macros_view.rs`, `src/ui/mapping_view.rs`, `src/ui/settings_view.rs`, `src/ui/key_capture.rs`) as features are added.
- Do not place business logic, macro validation, conflict detection, file I/O, or OS hook calls inside `egui` rendering functions.

## 2. UI State vs. Domain State
- Maintain a clear split between:
  - **Transient UI State**: Active tab/route, search filter text, selected item ID, modal open flags, inline form drafts, key-capture listening state.
  - **Domain & Engine State**: Saved profiles, macro definitions, active engine status (`Idle`, `Recording`, `Playing`, `MappingActive`).
- Edit form drafts safely and apply validated changes explicitly (or via validated commands) instead of mutating live engine tables mid-keystroke.

## 3. User Interaction & Key Capture UX
- **Explicit Capture Mode**: Binding a trigger key, target key, or recording a macro must enter an unmistakable capture state (e.g., modal/banner: `"Press any key or mouse button (Esc to cancel)"`).
- Ensure captured keys during binding do not accidentally fire UI navigation shortcuts or trigger active remappings while selecting a key.
- Provide clear controls for creating, duplicating, reordering, enabling/disabling, importing, exporting, and deleting macros and mappings with confirmation for destructive deletes.

## 4. Active Automation Visibility & Safety Controls
- Always keep automation state visible in the persistent top or bottom status bar:
  - Whether global key mapping is **Enabled / Paused**.
  - Active profile name.
  - Macro engine state (**Idle / Recording / Playing** + macro name).
  - Configured **Emergency Stop** hotkey and a clickable **Stop All** button.

## 5. Feedback, Validation & Error States
- Display clear, inline, non-blocking feedback for:
  - Mapping conflicts, cyclic rules, and lockout warnings.
  - Invalid macro delays or unbalanced `KeyDown`/`KeyUp` steps.
  - Persistence errors (e.g., failed profile save/import) and OS permission/hook errors.
- Avoid silent failures (`let _ = ...`) for user-triggered actions.

## 6. Accessibility & Visual Consistency
- Keep readable typography, consistent spacing/padding constants, clear contrast in both light and dark themes (note `global_theme_preference_buttons` is already wired in [src/app.rs](../../src/app.rs)), and keyboard-navigable controls.
- Use unambiguous labels and tooltips for technical concepts (e.g., Scan Code vs. Virtual Key, Hold vs. Toggle loop).

## 7. Immediate-Mode (`egui`) Performance
- `eframe::App::update()` runs every rendered frame—never perform disk I/O, heavy cloning of full macro logs, or blocking waits inside `update()`.
- Keep UI responsive by using bounded engine-to-UI state snapshots and triggering `ctx.request_repaint()` from the background engine only when status changes.
