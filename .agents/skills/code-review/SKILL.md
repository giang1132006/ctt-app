---
name: code-review
description: >-
  Audits code changes in ctt-app for logical correctness, architectural boundary
  compliance, OS input safety (stuck keys, hook latency, loop prevention),
  Rust idioms, error handling, performance, and test coverage. Use this skill
  before marking an implementation task complete or when reviewing diffs.
---

# Code Review

## Purpose
Perform a rigorous engineering review of changes in `ctt-app` before completion to catch correctness, architectural, safety, and maintainability defects early.

## Activation Criteria
Activate this skill when:
- Completing Phase 5 (Verify) of any implementation task.
- Reviewing a module refactor, new dependency, or OS input subsystem.

## Workflow
1. **Inspect Rules & Diff Scope**:
   - Check changed files against [Global Project Rules](../../rules/global-rules.md), [Architecture Rules](../../rules/architecture-rules.md), and relevant domain rules.
   - Verify no unrelated files, generated artifacts, or user changes were altered accidentally.
2. **Audit Architecture & Dependencies**:
   - Confirm `domain` stays free of `egui`/`eframe` and OS-specific imports.
   - Confirm no blocking calls, disk I/O, or OS hooks run inside `egui` rendering or hook callbacks.
3. **Audit OS-Input Safety**:
   - Check synthetic event tagging (loop prevention).
   - Check pressed-key tracking and matching `Up` release on error, cancellation, or profile switch.
   - Check RAII (`Drop`) hook cleanup and emergency stop responsiveness.
4. **Audit Rust Quality & Maintainability**:
   - Flag `.unwrap()` / `.expect()` in non-test runtime paths, silent error swallowing (`let _ =`), duplicated logic, weak primitive typing, and missing edge-case validation.
   - Verify unit test coverage for modified behavior and clean `cargo clippy` / `cargo test` runs.

## Constraints
- Report concrete findings backed by exact file paths and symbol names.
- Never approve or ignore unverified safety hazards in OS hook/synthesis code.

## Expected Output
A structured review summary categorizing findings by severity:
- **Critical / Blocking** (safety hazards, input loops, stuck keys, boundary violations, panics).
- **Important** (missing tests, weak error handling, performance risks).
- **Minor / Polish** (naming, duplication, documentation gaps).
- **Verification Status** (commands run and observed outcomes).
