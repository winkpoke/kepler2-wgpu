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

# AI Agent Instructions for Kepler2-WGPU

This is the main entry point for AI assistants. See `doc/agents/` for detailed guidance.

## Quick Links

### Core Documentation
- **Architecture**: `doc/agents/ARCHITECTURE.md` - Module structure and dependencies
- **Conventions**: `doc/agents/CONVENTIONS.md` - Coding standards and patterns
- **Build & Test**: `doc/agents/BUILD.md` - Development workflow
- **Test Strategy**: `doc/agents/test_strategy_comprehensive.md` - Medical software testing coverage and strategy
- **Test Implementation Plan**: `doc/agents/test_implementation_plan.md` - Concrete test plan with file names, test counts, and effort estimates
- **Rendering**: `doc/agents/RENDERING.md` - GPU and rendering patterns
- **Common Pitfalls**: `doc/agents/PITFALLS.md` - Anti-patterns to avoid
- **PR Guidelines**: `doc/agents/PR_GUIDELINES.md` - Contribution workflow
- **OpenSpec**: `doc/agents/OPENSPEC.md` - Spec-driven development

### Quick Reference
- **One-page cheat sheet**: `doc/agents/QUICK_REFERENCE.md` - Common commands at a glance
- **Full documentation**: `doc/agents/README.md` - Complete navigation guide
- **Test Coverage**: `doc/agents/test_strategy_comprehensive.md` - Medical software testing strategy
- **Test Implementation**: `doc/agents/test_implementation_plan.md` - Concrete test plan with actionable tasks

## TL;DR

### Quick Commands
```bash
# Native
cargo run                                    # Run application
cargo build --release                         # Build release
RUST_LOG=info cargo run                       # Run with logging
cargo test                                   # Run tests

# WASM
wasm-pack build --target web                  # Build for web
npx live-server ./static                      # Serve static files

# Code Quality
cargo fmt                                    # Format code
cargo clippy                                 # Lint
cargo check                                   # Type check
```

### Essential Patterns
- Never use `tokio` in WASM modules
- Use `KeplerResult<T>` for error handling
- Use crate-level `log` macros (info/warn/error/debug)
- Coordinate systems: World, Screen, Voxel, Base (use `glam` for math)
- Recreate pipelines when surface format changes

### Key Locations
- Entry points: `src/main.rs` (native), `src/lib.rs` (WASM, library)
- Error types: `src/core/error.rs`
- CT volumes: `src/data/ct_volume.rs`
- App state: `src/application/app.rs`
- GPU init: `src/rendering/core/graphics.rs`

## Getting Started

### New to Kepler2-WGPU?
1. Read `doc/agents/QUICK_REFERENCE.md` for quick overview
2. Read `doc/agents/ARCHITECTURE.md` to understand project structure
3. Read `doc/agents/CONVENTIONS.md` for coding standards

### Ready to code?
- **Bug fix**: Check `doc/agents/PITFALLS.md` for common issues
- **New feature**: See `doc/agents/OPENSPEC.md` for proposal workflow
- **GPU work**: See `doc/agents/RENDERING.md` for rendering patterns

### Need more detail?
See `doc/agents/README.md` for complete documentation guide and workflows.

### Working on Test Coverage?
- **Test Strategy**: `doc/agents/test_strategy_comprehensive.md` - Overall strategy and medical safety requirements
- **Implementation Plan**: `doc/agents/test_implementation_plan.md` - Concrete tasks with file names, test counts, and effort estimates

---

**Last Updated**: 2026-01-18
**Maintained By**: Sisyphus (AI Agent)
