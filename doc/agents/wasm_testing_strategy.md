# WASM Testing Strategy

## Overview

Kepler2-WGPU supports both native and WebAssembly (WASM) targets. WASM testing requires distinct approaches due to browser environment constraints, lack of tokio, and DOM interaction requirements.

## WASM Unit Testing

### Target-Specific Tests

Use `#[cfg(target_arch = "wasm32")]` guards for WASM-specific test paths:

```rust
#[cfg(test)]
mod wasm_tests {
    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_mha_parsing_without_tokio() {
        // Test MHA parsing without tokio async operations
    }
}
```

### WASM-Specific Unit Tests

1. **MHA Parsing Without Tokio**
   - Test MHA header parsing (synchronous I/O)
   - Test MHA data offset calculation
   - Test MHA endianness detection
   - Test MHA pixel type validation
   - Test MHA dimension validation
   - **File**: `tests/wasm_unit_tests.rs`

2. **MHD Parsing Without Tokio**
   - Test MHD header parsing (synchronous)
   - Test MHD external file resolution
   - Test MHD transform matrix parsing
   - Test MHD anatomical orientation parsing
   - **File**: `tests/wasm_unit_tests.rs`

3. **WASM-Bindgen Bridge Error Handling**
   - Test JavaScript → Rust error propagation
   - Test Rust → JavaScript error propagation
   - Test invalid parameter handling
   - Test type conversion errors (String → &str, u32 → usize)
   - **File**: `tests/wasm_unit_tests.rs`

4. **Browser-Specific Error Paths**
   - Test WebGL2 availability check
   - Test WebGPU availability check
   - Test canvas element not found
   - Test canvas sizing errors
   - Test memory allocation failures
   - **File**: `tests/wasm_unit_tests.rs`

5. **Memory Limits in Browser**
   - Test large volume allocation limits
   - Test WebGL texture size limits
   - Test GPU buffer size limits
   - Test out-of-memory graceful degradation
   - **File**: `tests/wasm_unit_tests.rs`

### WASM Unit Test Execution

```bash
# Build WASM tests
wasm-pack test --firefox --headless
wasm-pack test --chrome --headless

# Run WASM unit tests from npm
npm test
```

## Browser Integration Testing

### Tool Selection: Playwright

**Rationale**:
- Cross-browser support (Chrome, Firefox, Safari)
- Headless execution in CI
- WebGPU/WebGL2 support in browser contexts
- Better CI integration than Puppeteer

### Integration Test Suite

#### File: `tests/wasm_integration_tests.rs`

1. **Browser DOM Interaction**
   - Test canvas element access (`wasm-example` ID)
   - Test canvas sizing (CSS-controlled)
   - Test canvas context creation (WebGPU/WebGL2)
   - Test event listener registration
   - Test canvas destruction

2. **WASM-Specific File I/O**
   - Test File API integration (user uploads)
   - Test IndexedDB for persistent storage
   - Test File System Access API (when available)
   - Test drag-and-drop file upload
   - Test large file handling (> 500MB)

3. **Browser Rendering Pipeline**
   - Test WebGPU device initialization
   - Test shader compilation in browser
   - Test texture upload from WASM memory
   - Test rendering loop requestAnimationFrame
   - Test performance frame rate (> 30 FPS)

4. **Cross-Browser Compatibility**
   - Test Chrome WebGPU support
   - Test Firefox WebGPU support (when available)
   - Test Safari WebGL2 fallback
   - Test browser-specific behavior differences
   - Test feature detection and graceful degradation

### Integration Test Execution

```bash
# Install Playwright
npx playwright install chromium firefox webkit

# Run integration tests
npm run test:integration
```

### CI Integration

```yaml
# .github/workflows/wasm-tests.yml
name: WASM Tests
on: [push, pull_request]
jobs:
  wasm-unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
      - uses: jetli/wasm-pack-action@v0.3.0
      - run: wasm-pack test --firefox --headless

  wasm-integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
      - run: npm install
      - run: npx playwright install chromium
      - run: npm run test:integration
```

## WASM Test Coverage Targets

- **WASM-specific code paths**: ≥ 50% coverage
- **Browser interaction code**: ≥ 40% coverage
- **Cross-browser tests**: Test on Chrome, Firefox, Safari (or latest two browsers)

## Known WASM Limitations

1. **No Tokio**: Cannot test async code that requires tokio runtime
   - **Workaround**: Use synchronous equivalents where possible
   - **Workaround**: Mock async behavior in unit tests

2. **GPU Testing in Browser**: Limited GPU access in headless CI
   - **Workaround**: Use WebGL2/WebGPU software rendering
   - **Workaround**: Mock GPU operations in unit tests

3. **File System**: No native file system access
   - **Workaround**: Use File API or IndexedDB for file I/O tests
   - **Workaround**: Use in-memory mock files for unit tests

4. **Time-based Tests**: Timing-dependent tests unreliable in browser
   - **Workaround**: Use relaxed timing thresholds
   - **Workaround**: Use mock clocks or manual event triggering

## WASM Testing Workflow

### Development
1. Write unit tests with `#[cfg(target_arch = "wasm32")]` guards
2. Test locally: `wasm-pack test --firefox --headless`
3. Write integration tests with Playwright
4. Test locally: `npm run test:integration`

### Continuous Integration
1. Run WASM unit tests on every push/PR
2. Run integration tests on every push/PR
3. Fail build if WASM tests fail
4. Coverage measured with `cargo llvm-cov` (native only - see limitations)

### Release
1. Run full WASM test suite before release
2. Test on all supported browsers (Chrome, Firefox, Safari)
3. Verify rendering performance (≥ 30 FPS)
4. Verify memory usage (< 500MB typical)

## References

- Wasm-Pack Testing: https://rustwasm.github.io/wasm-pack/book/cargo-toml-configuration.html
- Playwright: https://playwright.dev/
- WebGPU in WASM: https://github.com/gfx-rs/wgpu
- Testing WASM with Rust: https://rustwasm.github.io/docs/wasm-pack/tests/
