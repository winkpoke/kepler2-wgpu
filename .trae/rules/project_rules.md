1. Always chat in English.
3. My system is Windows but the code shall work in Mac and Linux.
4. The project is a Rust-based WGPU medical imaging framework for CT reconstruction, MPR (multi-planar reconstruction), MIP (maximum intensity projection), and 3D visualization.
5. The project must build for both native and WebAssembly (wasm) targets.
6. The wasm build command is: wasm-pack build -t web 
7. Do not use npx, live-server, or any node-based tools for wasm testing. The wasm output will be manually tested in the browser.
8. All documentation resides under the doc/ folder and its subfolders. Each feature or change must include a short explanation in this folder.
9. Development must be incremental and minimal. Add features step by step, starting with the minimal viable product.
10. Default logging level is INFO.
11. Use DEBUG level logs for development and debugging.
12. Use TRACE level logs only for high-frequency or detailed performance diagnostics (e.g., within render loops).
13. TRACE logging must be gated by a feature flag named trace-logging.
14. When trace logging is enabled, consider sampling (e.g., log every Nth frame) to prevent log flooding.
15. Make logging configurable through the environment variable RUST_LOG (for native builds).
16. In wasm builds, route logs to the browser console using console_log.
17. Heavy trace logging must never be enabled in release builds.
18. GPU operations must remain efficient and non-blocking. Avoid excessive CPU-GPU synchronization in render loops.
19. Use cfg(feature = "trace-logging") to conditionally include expensive diagnostics.
20. Update doc/CHANGELOG.md for each user-visible change.
21. In the medical imaging context, ensure accuracy, numerical stability, and performance for CT reconstruction and visualization.
22. Manage GPU memory efficiently and minimize texture/data copies.
23. The codebase must remain buildable, testable, and inspectable at all times.
24. Do not use perspective projection in medical imaging for accuracy reasons unless asked.
25. Time stamp format shall be YYYY-MM-DDTHH-MM-SS which takes the local time zone into account. The current local time zone is Beijing. Use "Get-Date -Format "yyyy-MM-ddTHH-mm-ss" to check the system time to make sure the time is correctly handled.
26. Add `no_run` directives to all code examples in the documentation to indicate they should not be executed during documentation generation. This ensures examples are displayed for reference purposes only while preventing accidental execution during build processes.
27. Alway keep in mind the Open Spec rules as following:
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
