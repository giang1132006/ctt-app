---
name: key-mapping-development
description: >-
  Designs and implements physical-to-logical key remapping models, scan code
  and virtual key identifiers, conflict and cycle detection, profile switching,
  and OS hook integration. Use this skill when working on key remapping,
  mouse button binds, or mapping profiles.
---

# Key Mapping Development

## Purpose
Deliver low-latency, conflict-free, and safe keyboard/mouse remapping and profile management in `ctt-app`.

## Activation Criteria
Activate this skill when:
- Designing or updating key/mouse identifiers (`ScanCode`, `VirtualKey`, `MouseButton`).
- Implementing mapping rules, profile switching, conflict/cycle detection, or OS hook interception.
- Debugging sticky modifiers, wrong key codes, or recursive remap triggers.

## Workflow
1. **Align with Domain & Safety Rules**:
   - Read [Key Mapping Rules](../../rules/key-mapping-rules.md) and [AGENTS.md](../../../AGENTS.md) Section 4.
2. **Separate Key Identification Layers**:
   - Distinguish hardware scan codes (including extended key flags) from OS virtual keys and mouse buttons.
   - Use stable, readable serialization names for cross-version profile storage.
3. **Implement Validation & Compiled Lookup**:
   - Detect duplicate source keys, cyclic mappings (`A -> B -> A`), and lockout hazards (remapping Emergency Stop or disabling primary mouse click without safeguards).
   - Compile active profile rules into a fast lookup map before pushing to the input hook thread.
4. **Handle Pressed-Key State & Loop Prevention**:
   - Track which physical keys triggered which remapped outputs so key release always matches the original target even if profiles switch mid-press.
   - Mark synthesized output events so the input hook skips self-injected events.
5. **Verify Profile Lifecycle & Cleanup**:
   - Release held virtual keys when disabling mapping, switching profiles, or exiting.
   - Add unit tests for conflict detection, cycle detection, and state-preserving profile switches.

## Constraints
- Never perform heavy allocations, disk I/O, or blocking locks inside the OS low-level input hook callback.
- Never allow a profile to remap or suppress the Emergency Stop hotkey.
- Keep OS-specific scan code translation isolated inside `platform`, not leaked into `domain` or `ui`.

## Expected Output
- Pure domain mapping/profile models with validation, fast compiled lookup logic, clean OS-hook integration interfaces, and deterministic unit tests.
