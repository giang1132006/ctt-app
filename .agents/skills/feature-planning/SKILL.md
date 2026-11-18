---
name: feature-planning
description: >-
  Transforms feature requests for ctt-app into actionable implementation plans
  with requirements analysis, affected modules, acceptance criteria, OS-input
  safety checks, and ordered tasks. Use this skill before implementing new
  features or medium-to-large enhancements.
---

# Feature Planning

## Purpose
Convert feature requests into structured, low-risk Rust implementation plans aligned with `ctt-app`'s architecture and OS input safety guarantees.

## Activation Criteria
Activate this skill when:
- A user requests a new capability, subsystem, UI view, or multi-file change.
- Requirements span multiple layers (e.g., `ui` + `domain` + `engine`/`platform` + `storage`).

## Workflow
1. **Review Context & Rules**:
   - Read [AGENTS.md](../../../AGENTS.md), [Architecture Rules](../../rules/architecture-rules.md), and applicable domain rules ([Macro System Rules](../../rules/macro-system-rules.md), [Key Mapping Rules](../../rules/key-mapping-rules.md), [UI/UX Rules](../../rules/ui-ux-rules.md)).
2. **Clarify Requirements & Ambiguities**:
   - Identify functional scope, edge cases, persistence impacts, and undecided decisions (`DECISIONS.md`).
   - Flag ambiguous requirements to the user before committing to irreversible assumptions.
3. **Identify Affected Modules & Interfaces**:
   - List exact files to create or update across `domain`, `engine`/`platform`, `storage`, and `ui`.
   - Define trait boundaries or message/channel types needed between UI and engine threads.
4. **Assess Safety & Concurrency Risks**:
   - Evaluate stuck-key hazards, synthetic input recursion, blocking calls on the UI or OS hook thread, and config migration compatibility.
5. **Break Down into Incremental Tasks**:
   - Sequence work so pure domain models and unit tests land before OS hook wiring and UI panels.
   - Define verifiable acceptance criteria and test cases for each step.

## Constraints
- Do not skip domain modeling and test planning by jumping straight into UI or raw Windows API calls.
- Explain any proposed new Cargo crate dependency and record significant trade-offs for [DECISIONS.md](../../../DECISIONS.md).
- Avoid unnecessary planning overhead for trivial one-line fixes.

## Expected Output
A structured plan containing:
1. **Objective** & **Acceptance Criteria**.
2. **Proposed Approach** & **Crate/Interface Impacts**.
3. **Affected Files & Modules**.
4. **Ordered Implementation Steps**.
5. **Safety, Concurrency & Regression Risks**.
6. **Testing Strategy** (`cargo test` unit/integration tests + manual OS smoke checks if applicable).
