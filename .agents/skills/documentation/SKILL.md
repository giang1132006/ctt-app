---
name: documentation
description: >-
  Maintains synchronization between ctt-app source code and project
  documentation (PROJECT_CONTEXT.md, ARCHITECTURE.md, DEVELOPMENT_GUIDE.md,
  ROADMAP.md, DECISIONS.md, and Rustdoc comments). Use this skill after
  architectural changes, new feature deliveries, or configuration schema updates.
---

# Documentation

## Purpose
Keep all project-level technical documentation, ADRs, and developer setup instructions accurate, honest, and synchronized with the actual `ctt-app` codebase.

## Activation Criteria
Activate this skill when:
- Adding or refactoring modules, public traits, or config schemas.
- Making or revising technology choices (crates, persistence formats, OS backends).
- Completing a milestone in [ROADMAP.md](../../../ROADMAP.md) or adding build/test tooling.

## Workflow
1. **Compare Docs Against Code**:
   - Inspect [Cargo.toml](../../../Cargo.toml), `src/`, and existing docs ([PROJECT_CONTEXT.md](../../../PROJECT_CONTEXT.md), [ARCHITECTURE.md](../../../ARCHITECTURE.md), [DEVELOPMENT_GUIDE.md](../../../DEVELOPMENT_GUIDE.md), [ROADMAP.md](../../../ROADMAP.md), [DECISIONS.md](../../../DECISIONS.md)).
2. **Update Accurate Status**:
   - Move completed items in `ROADMAP.md` and update current stack/module tables in `PROJECT_CONTEXT.md` and `ARCHITECTURE.md`.
   - Record new architectural or dependency choices in `DECISIONS.md` with context, alternatives, rationale, and trade-offs.
3. **Sync Public Interfaces & Setup**:
   - Update `DEVELOPMENT_GUIDE.md` when commands, prerequisites, or environment variables change.
   - Add concise Rustdoc (`///`) comments on new public traits, domain types, and safety invariants.
4. **Verify Cross-References**:
   - Ensure all relative markdown file links and symbol names match actual files on disk.

## Constraints
- Never document planned features as already implemented; mark undecided items explicitly as `TBD`.
- Preserve existing user comments and historical ADR entries (supersede ADRs rather than erasing history).

## Expected Output
- Updated documentation files with verified links, accurate implemented-vs-planned separation, and any newly recorded ADRs.
