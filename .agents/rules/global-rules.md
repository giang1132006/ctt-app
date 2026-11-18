# Global Project Rules

These rules apply unconditionally across the entire `ctt-app` repository.

## 1. Project Understanding
- Inspect existing modules, `Cargo.toml`, and documentation (`PROJECT_CONTEXT.md`, `ARCHITECTURE.md`, `DECISIONS.md`) before proposing or writing code.
- Respect established patterns and module boundaries; avoid speculative or sweeping architectural rewrites.
- Never invent product requirements or claim unimplemented features already exist. Mark unresolved decisions as `TBD` and ask the user for clarification when behavior is ambiguous.

## 2. Coding Standards (Rust 2021)
- **Naming**: Use `snake_case` for modules, files, functions, methods, and variables; `PascalCase` for structs, enums, traits, and type aliases; `SCREAMING_SNAKE_CASE` for constants and statics.
- **Modularity**: Keep functions focused and small with shallow nesting and early returns. Avoid duplicating logic across modules—extract shared helpers or trait abstractions where appropriate.
- **Explicit Types**: Use strong domain types (newtypes and enums such as `ScanCode`, `VirtualKey`, `MacroId`, `Duration`) rather than passing raw `u32`/`u64`/`String` primitives across boundaries.
- **Error Handling**:
  - Return `Result<T, E>` with descriptive domain error enums for all fallible operations (I/O, parsing, validation, OS hook registration, input injection).
  - Never use `.unwrap()` or `.expect()` in application runtime paths, UI frame updates, or OS input callbacks. (`unwrap()` is acceptable only in unit tests.)
- **Dependencies**: Prefer standard library primitives and already-adopted crates (`eframe`/`egui`) before introducing new crates. Keep dependency trees lean and audit OS/Windows compatibility before adding crates.

## 3. Architecture Rules (Summary)
- Maintain strict separation between:
  1. **Presentation (`ui`)**: `egui` rendering and transient view state.
  2. **Domain (`domain`)**: Pure Rust business rules, models, conflict checks, and state machines.
  3. **Engine & Platform (`engine` / `platform`)**: Background execution threads, OS hooks, and input synthesis.
  4. **Persistence (`storage`)**: Serialization, schema versioning, and file I/O.
- Keep module dependencies acyclic and pointing inward toward domain types.
- See [Architecture Rules](architecture-rules.md) for detailed module boundaries and concurrency constraints.

## 4. Development Rules
- Analyze requirements and risks before writing code.
- Break medium and large features into small, verifiable increments.
- Write unit tests for pure domain logic (validation, state transitions, conflict detection, serialization) alongside implementation.
- Run `cargo check`, `cargo test`, and `cargo clippy` after changes when toolchain execution is available.
- Record significant structural, dependency, or protocol choices as ADRs in [DECISIONS.md](../../DECISIONS.md).

## 5. Change Management
- Do not delete or disable existing functionality without explicit justification.
- Do not silently break public module/trait interfaces without updating all callers and documenting why.
- Do not perform unrelated formatting or refactoring mixed into feature or bugfix changes.
- Do not modify generated lockfiles (`Cargo.lock`) or build outputs manually; let Cargo manage them when dependencies intentionally change.
- Never add a new dependency to `Cargo.toml` without explaining why existing tools/crates are insufficient.
- Preserve existing uncommitted Git work and user edits at all times.

## 6. Communication
- Explain technical decisions and trade-offs concisely.
- State assumptions, platform constraints, and safety risks explicitly.
- Ask clear, targeted questions when requirements are underspecified.
- Only report builds, tests, or behaviors as verified if directly observed after the final edit.
