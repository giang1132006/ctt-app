# `ctt-app` — AI Agent Operating System

## 1. Project Identity & Current State
`ctt-app` is a Rust (`edition = "2021"`) desktop application for keyboard and mouse automation, macro playback/recording, and gaming-style key remapping (`eframe = "0.31"`, `egui = "0.31"`).

**Current Repository Baseline (Do Not Fabricate Features):**
- Implemented: Only the initial `eframe`/`egui` window shell in [src/main.rs](src/main.rs) and placeholder UI (`Search:` input and `Increment` counter) in [src/app.rs](src/app.rs), configured in [Cargo.toml](Cargo.toml).
- Not yet implemented: Macro system, key mapping engine, OS-level input hooks/synthesis, profile persistence, logging, and automated tests.

Always read [PROJECT_CONTEXT.md](PROJECT_CONTEXT.md), [ARCHITECTURE.md](ARCHITECTURE.md), and [DECISIONS.md](DECISIONS.md) before designing or implementing non-trivial changes.

---

## 2. Core Operating Principles
1. **Inspect Before Acting**: Read the affected modules and relevant rules/skills before writing code. Never invent requirements or assume undecided technology choices.
2. **Respect Architecture Boundaries**: Keep UI (`egui`), domain logic (macros, key mapping, config), OS input hooks/synthesis, and persistence cleanly separated.
3. **Preserve User Work & Interfaces**: Never delete existing functionality, silently change public interfaces, refactor unrelated code, or discard uncommitted Git changes.
4. **Justify Dependencies**: Do not add crates to `Cargo.toml` without explaining the rationale, trade-offs, and recording major choices in [DECISIONS.md](DECISIONS.md).
5. **Verify Before Claiming Done**: Never claim a feature or fix is complete without compiling/testing (or explicitly reporting when verification could not run).

---

## 3. Agent Working Workflow (6 Phases)

For trivial edits (e.g., typo fixes, single-line doc updates), use a lightweight pass. For medium or large features, execute all six phases:

### Phase 1: Understand
- Read relevant rules (`.agents/rules/*.md`), matching skills (`.agents/skills/*/SKILL.md`), and affected source/doc files.
- Identify acceptance criteria, edge cases, and OS-input constraints.

### Phase 2: Analyze
- Clarify root problem or feature scope and identify impacted modules.
- Evaluate architectural options, crate constraints, and safety risks.
- Ask the user when critical requirements or architectural choices remain ambiguous.

### Phase 3: Plan
Before writing code for medium/large tasks, define:
1. **Objective**
2. **Proposed Approach**
3. **Files & Modules Affected**
4. **Implementation Steps**
5. **Potential Risks** (especially OS hooks, input loops, stuck keys)
6. **Testing Strategy**

### Phase 4: Implement
- Execute the plan incrementally in small, cohesive steps.
- Follow Rust 2021 idioms, explicit domain types, and modular separation.
- Avoid unrelated changes or duplicate logic.

### Phase 5: Verify
- Run `cargo check`, `cargo test`, `cargo clippy --all-targets`, and `cargo fmt --check` (when shell execution is available).
- Inspect the Git diff for unintended changes or boundary violations.
- Verify error paths, cleanup logic, and regressions.

### Phase 6: Report
Provide a concise summary covering:
- What was implemented and files modified.
- Key technical decisions and assumptions.
- Tests/checks executed and observed results (or explicit gaps if unrun).
- Known limitations, remaining issues, and recommended next steps.

---

## 4. OS-Level Input Safety & Reliability Mandates

Because `ctt-app` intercepts and synthesizes keyboard and mouse input at the operating system level, **safety takes precedence over convenience**:

- **Explicit User Control**: Never record input or activate remappings/macros automatically without clear, explicit user action.
- **Emergency Stop (Panic Hotkey)**: Always provide a dedicated, non-remappable emergency stop mechanism that immediately cancels all running macros, releases all virtual held keys/buttons, and disables active remappings.
- **Visible Active State**: Clearly indicate in the UI whenever macro recording, macro playback, or key mapping is active.
- **Synthetic Input Loop Prevention**: Every synthesized key or mouse event emitted by `ctt-app` must be tagged/filtered so low-level hooks ignore self-generated events and cannot enter infinite feedback loops.
- **Stuck-Key Prevention & Hook Cleanup**: Track every synthesized `Down` event and guarantee matching `Up` events on completion, cancellation, panic, or exit. Always unhook OS hooks cleanly via RAII (`Drop`) and shutdown paths.
- **No Stealth or Security Bypass**: Never implement hidden keylogging, credential capture, background surveillance without UI visibility, or circumvention of OS security controls (e.g., UAC/UIPI bypass).

---

## 5. Rules, Skills & Documentation Index

### Modular Rules (`.agents/rules/`)
- [Global Project Rules](.agents/rules/global-rules.md) — Coding standards, change management, development discipline, and communication.
- [Architecture Rules](.agents/rules/architecture-rules.md) — Module boundaries, dependency direction, threading, persistence, and testing boundaries.
- [Macro System Rules](.agents/rules/macro-system-rules.md) — Macro data models, timing, lifecycle, cancellation, stuck-key guarantees, and serialization.
- [Key Mapping Rules](.agents/rules/key-mapping-rules.md) — Scan codes vs. virtual keys, conflict/cycle detection, profiles, and OS hook integration.
- [UI/UX Rules](.agents/rules/ui-ux-rules.md) — `egui` panel organization, state separation, key capture UX, and immediate-mode performance.

### Specialized Skills (`.agents/skills/`)
- `project-analysis` — [SKILL.md](.agents/skills/project-analysis/SKILL.md)
- `feature-planning` — [SKILL.md](.agents/skills/feature-planning/SKILL.md)
- `software-architecture` — [SKILL.md](.agents/skills/software-architecture/SKILL.md)
- `macro-development` — [SKILL.md](.agents/skills/macro-development/SKILL.md)
- `key-mapping-development` — [SKILL.md](.agents/skills/key-mapping-development/SKILL.md)
- `ui-development` — [SKILL.md](.agents/skills/ui-development/SKILL.md)
- `testing-and-debugging` — [SKILL.md](.agents/skills/testing-and-debugging/SKILL.md)
- `code-review` — [SKILL.md](.agents/skills/code-review/SKILL.md)
- `documentation` — [SKILL.md](.agents/skills/documentation/SKILL.md)
- `release-and-build-management` — [SKILL.md](.agents/skills/release-and-build-management/SKILL.md)

### Project Documentation
- [PROJECT_CONTEXT.md](PROJECT_CONTEXT.md) — Scope, platforms, stack, constraints, and status.
- [ARCHITECTURE.md](ARCHITECTURE.md) — Implemented vs. target architecture and data flows.
- [DEVELOPMENT_GUIDE.md](DEVELOPMENT_GUIDE.md) — Toolchain setup, Cargo commands, and conventions.
- [ROADMAP.md](ROADMAP.md) — Milestones, stage status, and technical risks.
- [DECISIONS.md](DECISIONS.md) — Architectural Decision Records (ADRs) and open choices.
