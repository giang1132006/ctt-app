---
name: software-architecture
description: >-
  Guides architectural design decisions for ctt-app, including module
  boundaries, Rust trait interfaces, threading and channel topologies between
  egui and the input engine, crate evaluation, and ADR documentation. Use this
  skill when designing new subsystems, choosing dependencies, or refactoring
  module boundaries.
---

# Software Architecture

## Purpose
Maintain clean modularity, thread safety, testability, and long-term maintainability across `ctt-app`'s Rust codebase.

## Activation Criteria
Activate this skill when:
- Designing a new subsystem (e.g., input backend, playback engine, persistence layer).
- Evaluating third-party Cargo crates (`windows-sys`, `rdev`, `enigo`, `serde`, `tracing`, etc.).
- Resolving cross-module coupling, threading bottlenecks, or circular dependencies.

## Workflow
1. **Inspect Architectural Baseline**:
   - Read [ARCHITECTURE.md](../../../ARCHITECTURE.md), [DECISIONS.md](../../../DECISIONS.md), and [Architecture Rules](../../rules/architecture-rules.md).
2. **Define Responsibilities & Boundaries**:
   - Keep pure domain state machines and validation decoupled from `eframe`/`egui` and OS APIs.
   - Define minimal, idiomatic Rust traits at the `platform` and `storage` boundaries so domain/engine logic can run against fakes in unit tests.
3. **Design Concurrency & Data Flow**:
   - Specify thread ownership (`UI thread`, `OS hook thread`, `Playback worker thread`).
   - Choose bounded channels, atomic cancellation tokens, and lightweight UI snapshot updates that never block `eframe::App::update()`.
4. **Evaluate Alternatives & Trade-Offs**:
   - Compare candidate designs and crates across latency, Windows UIPI/admin behavior, MSRV, unsafe code surface, and maintenance health.
5. **Document Decisions**:
   - Draft or update an ADR entry in [DECISIONS.md](../../../DECISIONS.md) and update [ARCHITECTURE.md](../../../ARCHITECTURE.md) if module boundaries change.

## Constraints
- Never let `domain` modules depend on `egui`, `eframe`, or OS hook crates.
- Never introduce `unsafe` FFI code without encapsulating it inside `platform` with clear safety invariants and RAII cleanup.
- Do not over-engineer patterns (e.g., heavyweight plugin systems or async runtimes) unless justified by concrete requirements.

## Expected Output
- **Subsystem Design**: Module layout, ownership boundaries, and trait/message signatures.
- **Threading & Safety Model**: Data flow, synchronization primitives, and shutdown guarantees.
- **Trade-Off Analysis & ADR Draft**: Alternatives considered, rationale, and consequences ready for [DECISIONS.md](../../../DECISIONS.md).
