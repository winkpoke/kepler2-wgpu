# Build & Development Commands

**Last Updated**: 2025-01-15

## Native Development

### Building

```bash
# Build optimized binary
cargo build --release

# Debug build
cargo build

# Check without building
cargo check
```

### Running

```bash
# Run native application
cargo run

# Run with specific binary
cargo run --bin kepler

# Run with debug logging
RUST_LOG=info cargo run

# Run with trace logging (including wgpu modules)
RUST_LOG=debug,trace cargo run
```

## WebAssembly Development

### Prerequisites

```bash
# Install wasm-pack if not installed
cargo install wasm-pack

# Ensure wasm32 target is installed
rustup target add wasm32-unknown-unknown
```

### Building

```bash
# Build for web (generates pkg/ with .wasm and .js bindings)
wasm-pack build --target web

# Build in release mode
wasm-pack build --release --target web
```

### Serving

```bash
# Serve static files (opens http://localhost:8080)
npx live-server ./static

# Alternative: any static file server
python -m http.server 8080

# Alternative: using Python 2
python -m SimpleHTTPServer 8080
```

## Testing

### Running Tests

```bash
# Run all tests (unit + integration)
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_patient_creation

# Run tests in release mode
cargo test --release

# Run ignored tests (integration tests requiring real data)
cargo test -- --ignored

# Run only tests in a specific file
cargo test --test dicom_tests
```

### Test Organization

```
tests/
├── dicom_tests.rs              # DICOM parsing and validation
├── medical_imaging_tests.rs     # MHA/MHD format tests
├── mesh_integration_tests.rs    # 3D mesh rendering tests
├── mpr_view_validation_tests.rs # MPR view correctness tests
├── performance_tests.rs         # Performance benchmarks
└── error_handling_tests.rs     # Error handling edge cases
```

## Environment Variables

| Variable | Purpose | Default |
|-----------|---------|---------|
| `RUST_LOG` | Logging level (error, warn, info, debug, trace) | `info` if unset |
| `KEPLER_WGPU_BACKEND` | GPU backend selection (dx12, vulkan, metal, gl, primary) | `PRIMARY` |
| `KEPLER_WGPU_VALIDATION` | Enable WGPU validation layers | `disabled` |

### Examples

```bash
# Set log level to debug
RUST_LOG=debug cargo run

# Force Vulkan backend
KEPLER_WGPU_BACKEND=vulkan cargo run

# Enable WGPU validation
KEPLER_WGPU_VALIDATION=true cargo run

# Combine settings
RUST_LOG=info KEPLER_WGPU_BACKEND=vulkan KEPLER_WGPU_VALIDATION=true cargo run
```

## Code Quality

### Formatting

```bash
# Format all code
cargo fmt

# Check formatting without modifying
cargo fmt -- --check
```

### Linting

```bash
# Run clippy lints
cargo clippy

# Run clippy with warnings as errors (strict)
cargo clippy -- -D warnings
```

### Type Checking

```bash
# Type check without building
cargo check

# Check all targets
cargo check --all-targets
```

## Common Issues

### Surface errors on startup

```bash
# Check backend selection
KEPLER_WGPU_BACKEND=vulkan cargo run

# Enable validation layers
KEPLER_WGPU_VALIDATION=true cargo run
```

### WASM build fails

```bash
# Ensure wasm32 target is installed
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack
```

### Tests hang on async operations

Use `pollster` to block on async in tests (already configured in Cargo.toml):
```bash
cargo test
```

### Logging not showing in WASM

Check browser console (F12). Logs are routed to `web_sys::console`.

## Performance Profiling

### Build Performance

```bash
# Profile build time
cargo build --timings
```

### Runtime Performance

```rust
use crate::core::Instant;

let start = Instant::now();
// ... do work ...
let elapsed = start.elapsed();
info!("Operation took {:.2} ms", elapsed.as_millis_f32());
```

## Release Checklist

Before deploying:

```bash
# Format code
cargo fmt

# Check for warnings
cargo clippy

# Run all tests
cargo test

# Build release binary
cargo build --release

# Build WASM package
wasm-pack build --release --target web
```

## CI/CD Integration

If setting up CI:

```yaml
# Example GitHub Actions workflow
- name: Install wasm-pack
  run: cargo install wasm-pack

- name: Run tests
  run: cargo test

- name: Build WASM
  run: wasm-pack build --target web

- name: Check formatting
  run: cargo fmt -- --check

- name: Run clippy
  run: cargo clippy -- -D warnings
```

## Related Documentation

- **Quick Reference**: `QUICK_REFERENCE.md` - Common commands at a glance
- **Conventions**: `CONVENTIONS.md` - Coding standards and patterns
- **Architecture**: `ARCHITECTURE.md` - Project structure
- **PR Guidelines**: `PR_GUIDELINES.md` - Contribution workflow
