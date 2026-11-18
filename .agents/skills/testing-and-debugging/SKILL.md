---
name: testing-and-debugging
description: >-
  Guides root-cause bug analysis, unit and integration test creation, trait-mock
  testing of input engines without live OS hooks, edge-case validation, and
  regression checks. Use this skill when writing tests, investigating bugs, or
  verifying ctt-app implementations.
---

# Testing and Debugging

## Purpose
Ensure functional correctness, concurrency safety, and regression protection across `ctt-app` through deterministic automated testing and systematic debugging.

## Activation Criteria
Activate this skill when:
- Investigating a bug, panic, stuck key, timing drift, or UI freeze.
- Writing unit or integration tests for new or modified modules.
- Validating edge cases in macro playback, key remapping, or config persistence.

## Workflow
1. **Reproduce & Isolate**:
   - Read [Architecture Rules](../../rules/architecture-rules.md) and the failing module(s).
   - Reproduce the bug at the lowest possible layer (prefer a failing deterministic unit test in `domain` or `engine` over manual GUI clicking).
2. **Analyze Root Cause**:
   - Inspect state transitions, error handling, channel synchronization, key `Down`/`Up` pairing, and serialization edge cases.
   - Do not patch symptoms blindly; identify why the invariant broke.
3. **Use Trait Fakes for OS Boundaries**:
   - Test macro playback, cancellation, stuck-key release, and remap loop prevention against an in-memory mock `InputSynthesizer` / `InputEvent` stream so `cargo test` runs safely without injecting real keystrokes into the developer's OS.
4. **Cover Edge Cases**:
   - Test empty macros, mid-delay emergency stop cancellation, unbalanced `KeyDown` cleanup, duplicate/cyclic key mappings, corrupted config JSON/TOML, and rapid profile switches while a key is held.
5. **Verify Suite & Regressions**:
   - Run `cargo test`, `cargo check`, and `cargo clippy --all-targets`.

## Constraints
- Never inject real OS keyboard/mouse events inside automated `cargo test` suites.
- Never delete or weaken assertions to make a failing test pass.
- Report exact test scopes and whether OS-level hook behavior still requires manual smoke testing.

## Expected Output
- Root-cause diagnosis (for bugs), deterministic unit/integration tests covering edge cases, passing `cargo test` output (or explicit verification status), and regression assessment.
