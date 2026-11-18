---
name: macro-development
description: >-
  Designs and implements macro recording, typed keyboard and mouse event
  modeling, monotonic timing/delay scheduling, playback state machines,
  cancellation, stuck-key release guarantees, and macro import/export
  serialization. Use this skill when building or modifying macro features.
---

# Macro Development

## Purpose
Implement reliable, deterministic, and safe macro recording, editing, persistence, and playback features in `ctt-app`.

## Activation Criteria
Activate this skill when:
- Creating or editing macro models, step editors, or macro serialization schemas.
- Implementing macro recording from captured input events.
- Building or debugging the macro playback loop, timing scheduler, or cancellation logic.

## Workflow
1. **Align with Macro Rules**:
   - Read [Macro System Rules](../../rules/macro-system-rules.md) and [Architecture Rules](../../rules/architecture-rules.md).
2. **Model Pure Domain Types First**:
   - Define strongly typed `Macro`, `MacroStep`, `MacroTrigger`, `PlaybackMode`, and `MacroValidationError` types in the domain layer.
   - Include explicit `schema_version` and bounded timing types (`Duration` / `u64` ms).
3. **Implement Validation & Safety Invariants**:
   - Check empty sequences, minimum loop delay floors, step count limits, and unbalanced `Down`/`Up` pairs.
4. **Design Recording & Playback State Machine**:
   - Enforce mutually exclusive states (`Idle`, `Recording`, `Playing`, `Stopping`, `Error`).
   - Convert monotonic `Instant` deltas cleanly during recording; ignore control UI clicks targeting `Stop`.
   - Maintain an active pressed-key/button tracker during playback and guarantee matching `Up` synthesis on normal finish, error, or cancellation.
   - Make delays interruptible in short slices (or via `Condvar::wait_timeout`) so Emergency Stop responds immediately.
5. **Implement Serialization & Unit Tests**:
   - Add versioned import/export tests, validation edge-case tests, and mock-synthesizer playback tests checking stuck-key release on mid-macro cancellation.

## Constraints
- Never run macro playback delays on the `egui` UI thread.
- Never allow unbounded zero-delay `ToggleLoop` / `HoldToLoop` execution without a minimum yield/delay floor.
- Tag synthesized playback events so they do not re-trigger macro recording or key-mapping hooks.

## Expected Output
- Clean domain models, validation functions, engine playback/recording handlers, and unit/integration tests verifying timing, cancellation, stuck-key release, and serialization round-trips.
