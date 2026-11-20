# Development Guide (`ctt-app`)

## 1. Prerequisites
- **Rust Toolchain**: Stable Rust (`1.81+` recommended for `eframe 0.31` / `egui 0.31` compatibility) and `cargo` installed via `rustup`.
- **Platform Build Tools (Windows)**: Visual Studio Build Tools (C++ workload / Windows SDK) for the MSVC Rust target (`x86_64-pc-windows-msvc`).
- **Git**: Version control (`origin`: `https://github.com/giang1132006/ctt-app.git`).

Verify your local toolchain:

```powershell
rustc --version
cargo --version
```

---

## 2. Repository Layout (Current)

```text
ctt-app/
├── .agents/               # AI Agent rules (.agents/rules/) and skills (.agents/skills/)
├── ui/                    # Slint declarative UI views (app.slint, components/*.slint)
├── src/
│   ├── main.rs            # Application entry point & Slint callback wiring
│   ├── lib.rs             # Library root exporting domain, engine, platform, storage, app
│   ├── app.rs             # CttApp UI coordinator
│   ├── domain/            # Pure Rust key/mouse identifiers, rules, validation, profiles
│   ├── engine/            # RemapEngine state machine & command snapshots
│   ├── platform/          # OS traits, MockInputSynthesizer, and Windows keymap/packet scaffolding
│   └── storage/           # Atomic profile persistence and schema validation
├── tests/                 # Integration tests for domain validation and RemapEngine
├── build.rs               # Compiles ui/app.slint via slint-build
├── .gitignore             # Ignores /target
├── AGENTS.md              # Root AI Agent instructions & workflow
├── ARCHITECTURE.md        # Current & target layered architecture
├── Cargo.toml             # Package manifest (slint 1.19, slint-build 1.19)
├── DECISIONS.md           # Architectural Decision Records (ADRs)
├── DEVELOPMENT_GUIDE.md   # This guide
├── PROJECT_CONTEXT.md     # Scope, stack, constraints, and status
├── README.md              # Project title and brief description
└── ROADMAP.md             # Milestones, status, and technical risks
```

*Note: `Cargo.lock` and `target/` will be generated on first `cargo check` / `cargo build`.*

---

## 3. Development, Build & Verification Commands

No custom build scripts (`build.rs`, `Makefile`, `justfile`) are currently configured; standard Cargo commands apply:

```powershell
# Type-check and compile dependencies without producing final binary
cargo check

# Build and launch the desktop GUI application (dev profile)
cargo run

# Run automated unit and integration tests (none implemented yet)
cargo test

# Run linter checks
cargo clippy --all-targets

# Format code / verify formatting
cargo fmt
cargo fmt --check

# Build optimized release binary (hides console window on Windows)
cargo build --release

# Build & package portable Windows release folder + ZIP archive (dist\ctt-app-windows-x64.zip)
.\build_release.bat
```

*(Note: Packaging/installer commands are **TBD** pending release tooling decisions.)*

---

## 4. Coding Conventions
- **Edition**: Rust `2021`.
- **Naming**: `snake_case` (modules, functions, variables), `PascalCase` (types, traits, enums), `SCREAMING_SNAKE_CASE` (constants).
- **Strong Typing**: Prefer explicit enums/newtypes (`ScanCode`, `VirtualKey`, `MacroId`, `Duration`) over raw integers or untyped strings.
- **Error Handling**: Propagate typed errors via `Result`; do not use `.unwrap()` or `.expect()` in production runtime, UI rendering, or OS hook callbacks.
- **Architecture Discipline**: Keep UI (`egui`), domain logic, OS input hooks/synthesis, and disk persistence in separate modules per [ARCHITECTURE.md](ARCHITECTURE.md) and [Architecture Rules](.agents/rules/architecture-rules.md).

---

## 5. Testing & OS Input Safety Guidelines
- Automated tests (`cargo test`) must **never** synthesize real keyboard or mouse events into the host operating system. Always inject mock/fake implementations behind platform traits.
- Before manually testing live Windows input hooks or macro playback:
  1. Ensure the **Emergency Stop** hotkey is wired and tested first.
  2. Test with harmless keys (e.g., typing into Notepad) rather than system shortcuts or active IDE windows.
  3. Verify all virtual held keys release cleanly when stopping a macro or closing the app.

---

## 6. Working with AI Agents
Follow the 6-phase workflow (`Understand -> Analyze -> Plan -> Implement -> Verify -> Report`) and domain guardrails defined in [AGENTS.md](AGENTS.md).
