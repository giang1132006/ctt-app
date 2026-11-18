---
name: release-and-build-management
description: >-
  Guides Cargo release profile configuration, versioning, Windows binary
  metadata and subsystem setup, runtime dependency verification, packaging,
  and pre-release safety checklists for ctt-app. Use this skill when preparing
  release builds, configuring packaging, or validating deployment readiness.
---

# Release and Build Management

## Purpose
Ensure `ctt-app` builds cleanly, reproducibly, and safely for end-user distribution on target desktop platforms (Windows primary).

## Activation Criteria
Activate this skill when:
- Preparing a tagged release or distribution binary.
- Configuring `[profile.release]`, Windows application icons/manifests, or packaging scripts.
- Auditing runtime dependencies, binary size, or Windows SmartScreen/UAC/UIPI behavior.

## Workflow
1. **Inspect Build & Manifest Configuration**:
   - Read [Cargo.toml](../../../Cargo.toml), [src/main.rs](../../../src/main.rs) (verify `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`), and [DEVELOPMENT_GUIDE.md](../../../DEVELOPMENT_GUIDE.md).
2. **Validate Versioning & Changelog**:
   - Confirm SemVer bump in `Cargo.toml` matches planned release scope and update [ROADMAP.md](../../../ROADMAP.md).
3. **Run Pre-Release Verification Suite**:
   - Execute `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `cargo build --release` (when shell execution is available).
4. **Audit Packaging & Runtime Requirements**:
   - Verify no unexpected debug-only assets, hardcoded local paths, or missing Windows runtime DLLs/manifests.
   - Document any Windows UAC/UIPI elevation behavior (e.g., interacting with elevated target windows requires matching elevation or clear user notice).
5. **Run Manual Input-Safety Release Checklist**:
   - Confirm Emergency Stop hotkey works under release build.
   - Confirm clean hook teardown on normal exit, Alt+F4, and tray exit.
   - Confirm corrupted or missing user config files fall back gracefully without crashing on launch.

## Constraints
- Never publish or tag a release without running `cargo test` and `cargo build --release`.
- Never embed secrets, telemetry keys, or hidden background behavior in release builds.

## Expected Output
- Build/packaging readiness report, updated release checklist status, exact commands executed, and any remaining platform or signing blockers.
