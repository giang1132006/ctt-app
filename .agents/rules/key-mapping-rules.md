# Key Mapping System Rules

These rules govern key remapping, key binding, conflict handling, and profile management in `ctt-app`.

## 1. Physical vs. Logical Key Identification
- Explicitly separate:
  - **Physical Key (`ScanCode`)**: Hardware position/scan code (essential for layout-independent gaming binds and extended keys like Right Ctrl/Numpad Enter).
  - **Logical Key (`VirtualKey`)**: OS-mapped key identity (e.g., `KeyA`, `Enter`, `F13`, media keys).
  - **Mouse Input (`MouseButton` / `WheelDirection`)**: Left, Right, Middle, XButton1/Side1, XButton2/Side2, WheelUp/WheelDown.
- Use typed enums/structs with stable string serialization identifiers so saved profiles remain readable and portable across versions.

## 2. Mapping Configuration
- Represent each mapping rule explicitly with:
  - `id`, `enabled: bool`, `source: InputTrigger`, `target: MappingAction`, and optional per-rule options (e.g., pass-through vs. suppress original input).
- Supported target actions may include:
  - Single key remap (`Key`).
  - Chord/combo (`Modifiers + Key`).
  - Mouse button or wheel action.
  - Macro trigger binding (`MacroId`).
  - Input suppression / disable (`Disabled`).
- Keep mapping evaluation fast (`O(1)` lookup table / hash map compiled when a profile activates) because the OS hook thread evaluates rules on every input event.

## 3. Conflict & Cycle Detection
- Validate mapping tables before saving or activating a profile:
  - **Duplicate Source Conflicts**: Detect two enabled rules with the same `source` input within the same active profile/layer.
  - **Chained/Cyclic Rules**: Detect or explicitly disallow ambiguous cycles (`A -> B` and `B -> A`) and unbounded chains.
  - **Critical Input Lockout Guard**: Warn or block configurations that disable/remap the Emergency Stop hotkey, or disable primary mouse click (`LeftClick`) without an active alternative or easy UI recovery.

## 4. Loop Prevention & Self-Trigger Isolation
- When an OS hook intercepts physical key `A` and suppresses it to inject logical key `B`, the injected `B` event **must be tagged/bypassed** so it does not re-enter the remapping hook and trigger a `B -> C` or `B -> A` feedback loop.
- Maintain pressed-key pairing: if physical `A` is pressed while mapped to `B`, track the active mapping so releasing physical `A` emits `B` Up—even if the user switches profiles or disables mapping while `A` is still held down.

## 5. Profile Management & Lifecycle
- Support multiple named profiles with:
  - `id`, `name`, `schema_version`, `enabled`, `mappings`, and optional per-profile metadata (e.g., target application rules if added later).
- Enforce a single active profile (or explicitly prioritized layer stack if layered profiles are introduced later).
- Switching or disabling a profile must:
  1. Release any currently held remapped outputs tied to active physical keys.
  2. Atomically swap the engine's compiled active mapping table.
  3. Update UI status immediately.

## 6. OS-Level Integration & Persistence
- Keep profile schema and conflict/validation logic in `domain`; keep OS hook interception and event injection in `platform`.
- Persist profiles atomically via `storage` and validate on load before activating any mappings at startup.
- Provide a safe startup option (or clear indicator + emergency stop) if a user accidentally binds critical keys on launch.
