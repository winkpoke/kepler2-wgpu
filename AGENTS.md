<!-- OPENSPEC:START -->
# OpenSpec Instructions

These instructions are for AI assistants working in this project.

Always open `@/openspec/AGENTS.md` when the request:
- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:
- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

# Repository Guidelines

## Project Structure & Module Organization

`src/` contains the main source code organized into four subsystems:
- `application/` - UI orchestration, event handling, and app lifecycle
- `core/` - Utilities, error types, and mathematical helpers
- `data/` - DICOM parsing, CT volume generation, and file I/O
- `rendering/` - WebGPU pipelines, views, mesh processing

`tests/` holds unit and integration tests. `static/` contains web assets for WASM builds. `pkg/` stores pre-built JS/WASM artifacts.

## Build, Test, and Development Commands

### Native Development
```sh
cargo build --release    # Build optimized native binary
cargo run               # Run native application
cargo test              # Run all tests
```

### WebAssembly Development
```sh
wasm-pack build -t web        # Build for web
npx live-server ./static # Serve web interface
```

### Testing
```sh
cargo test              # Run all unit and integration tests
cargo test --release     # Run tests in release mode
```

## Coding Style & Naming Conventions

- Use `rustfmt` for code formatting
- 4-space indentation
- Follow Rust naming conventions: `PascalCase` for types, `snake_case` for functions and variables
- Platform-specific code uses `cfg(target_arch = "wasm32")` attributes
- Cross-platform async patterns: `async_lock::Mutex` for WASM, `tokio::sync::Mutex` for native

## Testing Guidelines

- Test files in `tests/` with `*_tests.rs` naming pattern
- Use `#[cfg(not(target_arch = "wasm32"))]` for native-only tests
- Integration tests should verify cross-platform behavior
- Run `cargo test` before submitting changes

## Commit & Pull Request Guidelines

### Commit Message Format
Follow the pattern: `type(scope): description`
Examples: `feat(rendering): add view factory pattern`, `fix(mesh): resolve WASM panic`

Types: `feat`, `fix`, `refactor`, `docs`, `style`, `test`, `chore`

### Pull Request Requirements
- Keep changes minimal and well-scoped
- Update relevant re-exports in `src/lib.rs` when changing public API
- Include tests for new functionality
- Ensure both native and WASM builds pass
- Reference related issues in PR description

### Platform Considerations
- Never import `tokio` in WASM-targeted modules
- Preserve platform-gated dependencies from `Cargo.toml`
- Test GPU pipeline changes across both targets
- DOM canvas element must maintain id `wasm-example` for web compatibility
