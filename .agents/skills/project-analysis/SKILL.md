---
name: project-analysis
description: >-
  Evaluates the ctt-app repository structure, architectural boundaries, crate
  dependencies, technical debt, and missing components. Use this skill when
  inspecting the codebase, onboarding to a task, auditing project status, or
  producing implementation recommendations.
---

# Project Analysis

## Purpose
Establish an accurate, evidence-based understanding of `ctt-app`'s current codebase, module boundaries, dependencies, and gaps before planning or writing code.

## Activation Criteria
Activate this skill when:
- Starting work on a new milestone, audit, or unfamiliar subsystem.
- Assessing what is actually implemented versus planned or `TBD`.
- Identifying architectural drift, circular dependencies, or technical debt.

## Workflow
1. **Inspect Core Manifests & Docs**:
   - Read [Cargo.toml](../../../Cargo.toml), [PROJECT_CONTEXT.md](../../../PROJECT_CONTEXT.md), [ARCHITECTURE.md](../../../ARCHITECTURE.md), [ROADMAP.md](../../../ROADMAP.md), and [DECISIONS.md](../../../DECISIONS.md).
2. **Map Source Structure**:
   - Locate modules under `src/` and any tests under `tests/`.
   - Identify entry points (`src/main.rs`), UI state (`src/app.rs`), domain types, platform backends, and storage layers.
3. **Evaluate Boundaries & Dependencies**:
   - Check whether UI, domain logic, OS input hooks/synthesis, and persistence obey the unidirectional rules in [Architecture Rules](../../rules/architecture-rules.md).
   - Inspect `Cargo.toml` dependencies for version alignment, Windows compatibility, and unnecessary crates.
4. **Assess Safety & Test Coverage**:
   - Verify presence of emergency stop logic, synthetic input tagging, RAII hook cleanup, and unit/integration test coverage.
5. **Synthesize Gaps & Recommendations**:
   - Classify components into **Implemented**, **Partially Implemented**, **Missing**, and **Undecided (`TBD`)**.

## Constraints
- Never assume a planned module or feature exists without reading the file on disk.
- Do not modify application source code during pure analysis tasks.
- Clearly separate observed facts from architectural recommendations.

## Expected Output
- **Repository Snapshot**: Files inspected, current tech stack, and entry points.
- **Boundary & Dependency Assessment**: Compliance status and any coupling risks.
- **Gap Analysis**: Implemented vs. missing components across Macro, Key Mapping, Configuration, Platform, and UI layers.
- **Prioritized Next Steps**: Concrete, ordered engineering recommendations.
