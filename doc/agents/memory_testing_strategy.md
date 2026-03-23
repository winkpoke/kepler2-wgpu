# Memory Testing Strategy

## Overview

Memory leak detection ensures long-running medical imaging applications don't accumulate memory, which could lead to crashes or degraded performance during extended use cases (e.g., viewing multiple patient scans).

## Chosen Strategy: Valgrind Integration in CI

### Rationale

1. **Valgrind** is the industry-standard memory debugging tool for native Rust code
2. **CI Integration** ensures every PR is checked for leaks
3. **Rust's ownership model** already prevents most leaks (no reference cycles in safe Rust)
4. **Native-only approach** avoids complexity of custom allocators
5. **Established tooling** with good error messages and suppression files

## CI Integration

### GitHub Actions Configuration

**File**: `.github/workflows/memory-tests.yml`

```yaml
name: Memory Tests
on: [push, pull_request]

jobs:
  memory-leak-detection:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Install Valgrind
        run: |
          sudo apt-get update
          sudo apt-get install -y valgrind

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true

      - name: Build release binary
        run: cargo build --release --bin kepler

      - name: Run DICOM parsing memory test
        run: |
          valgrind \
            --leak-check=full \
            --show-leak-kinds=all \
            --error-exitcode=1 \
            --suppressions=.github/valgrind-suppressions.txt \
            ./target/release/kepler test-dicom-memory \
            || echo "Memory leak detected in DICOM parsing"

      - name: Run volume loading memory test
        run: |
          valgrind \
            --leak-check=full \
            --show-leak-kinds=all \
            --error-exitcode=1 \
            --suppressions=.github/valgrind-suppressions.txt \
            ./target/release/kepler test-volume-memory \
            || echo "Memory leak detected in volume loading"

      - name: Run texture management memory test
        run: |
          valgrind \
            --leak-check=full \
            --show-leak-kinds=all \
            --error-exitcode=1 \
            --suppressions=.github/valgrind-suppressions.txt \
            ./target/release/kepler test-texture-memory \
            || echo "Memory leak detected in texture management"
```

### Valgrind Suppression File

**File**: `.github/valgrind-suppressions.txt`

```valgrind
# Suppress known benign leaks from external libraries

# wgpu-native (WebGPU implementation)
{
    wgpu_native_leak
    Memcheck:Leak
    fun:wgpu*.*
}
```

## Memory Test Implementation

### Memory Leak Tests

**File**: `tests/memory_leak_tests.rs`

```rust
#[cfg(test)]
mod memory_leak_tests {
    use crate::data::dicom::CTImage;
    use crate::data::ct_volume::CTVolume;
    use crate::rendering::Texture;

    /// Test repeated DICOM parsing and cleanup
    #[test]
    fn test_repeated_dicom_parsing_memory_cleanup() {
        let dicom_data = create_test_dicom_data();

        // Load 100 DICOM files sequentially
        for i in 0..100 {
            let _ct_image = CTImage::from_bytes(&dicom_data).unwrap();

            // Each iteration should clean up previous allocations
            // Valgrind will detect any memory growth
        }
        // Valgrind: expect 0 bytes lost, 0 bytes indirect lost
    }

    /// Test repeated volume load/unload cycles
    #[test]
    fn test_volume_load_unload_memory_cleanup() {
        for i in 0..100 {
            let volume = CTVolume::create_test_512x512x100();

            // Drop volume
            drop(volume);

            // Memory should be freed
        }
        // Valgrind: expect 0 bytes lost
    }

    /// Test texture memory cleanup
    #[test]
    fn test_texture_memory_cleanup() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use crate::rendering::core::graphics::GraphicsState;

            let graphics = GraphicsState::new().unwrap();

            for i in 0..100 {
                let _texture = Texture::create(&graphics, 512, 512);
                drop(_texture);
            }
        }
        // Valgrind: expect 0 bytes lost (excluding GPU driver allocations)
    }

    /// Test DICOM repository memory growth
    #[test]
    fn test_dicom_repo_memory_growth() {
        use crate::data::dicom::DicomRepo;

        let mut repo = DicomRepo::new();

        // Add 1000 patients
        for i in 0..1000 {
            repo.add_patient(create_test_patient(&format!("patient_{}", i)));
        }

        // Drop repository
        drop(repo);

        // Valgrind: all patient data should be freed
    }
}
```

### Memory Leak Thresholds

**Criteria**:
- **Definite leak**: > 0 bytes lost (FAIL)
- **Indirect leak**: > 0 bytes indirect lost (FAIL)
- **Still reachable**: Allowed only for long-lived singletons (e.g., GPU device)
- **Suppressed**: External library leaks (wgpu, web-sys)

**Expected Valgrind Output**:
```
==12345== HEAP SUMMARY:
==12345==     in use at exit: 0 bytes in 0 blocks
==12345==   total heap usage: 1,234,567 allocs, 1,234,567 frees, 123.45 MB allocated
==12345==
==12345== All heap blocks were freed -- no leaks are possible
```

## Performance Benchmarks for Memory

### Memory Usage Benchmark

**File**: `benches/memory_usage.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crate::data::ct_volume::CTVolume;

fn benchmark_volume_memory_usage(c: &mut Criterion) {
    c.bench_function("load_volume_512x512x100", |b| {
        b.iter(|| {
            let volume = CTVolume::create_test_512x512x100();
            black_box(&volume);
            drop(volume);
        });
    });
}

criterion_group!(benches, benchmark_volume_memory_usage);
criterion_main!(benches);
```

### Memory Growth Detection

Run valgrind with `--track-origins=yes` to detect uninitialised memory:

```bash
valgrind \
  --leak-check=full \
  --track-origins=yes \
  --error-exitcode=1 \
  ./target/release/kepler test-memory
```

## Alternative: Custom Allocator (Rejected)

### Why Not Custom Allocator?

1. **Complexity**: Requires wrapping all allocation/deallocation
2. **Performance**: Adds runtime overhead to every allocation
3. **Limited benefit**: Rust's ownership already prevents most leaks
4. **False positives**: Rust's aggressive deallocation causes noise
5. **Tooling gap**: No well-maintained Rust allocator tracking libraries

### If We Had Used Custom Allocator

```rust
// NOT USED - only for reference
use std::alloc::{GlobalAlloc, Layout, System};

struct MemoryTrackingAllocator;

unsafe impl GlobalAlloc for MemoryTrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        MEMORY_TRACKER.record_allocation(ptr, layout.size());
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        MEMORY_TRACKER.record_deallocation(ptr, layout.size());
        System.dealloc(ptr, layout);
    }
}

#[global_allocator]
static GLOBAL: MemoryTrackingAllocator = MemoryTrackingAllocator;
```

## Memory Test Requirements

### Test Cases

1. **Repeated Volume Load/Unload**: 100 iterations, 0 bytes leaked
2. **Texture Memory Cleanup**: 100 iterations, 0 bytes leaked
3. **DICOM Repository Growth**: 1000 patients, 0 bytes leaked after drop
4. **Large Volume Allocation**: 1024x1024x1000, graceful failure if OOM
5. **Concurrent Memory Operations**: 10 threads, 0 bytes leaked

### Coverage Requirements

- **Memory management code**: ≥ 70% coverage
- **Drop implementations**: 100% coverage (critical for memory cleanup)
- **Clone implementations**: 100% coverage (to detect accidental deep copies)

### CI Gates

- **Valgrind failure**: Block PR (exit code 1)
- **Memory growth**: Fail if > 10 KB growth after cleanup
- **Still reachable**: Allow only for known singletons (GPU device, app state)

## Memory Leak Detection in WASM

### Browser DevTools

WASM memory cannot be tracked with valgrind. Use browser DevTools instead:

```javascript
// In browser console
performance.memory.usedJSHeapSize;

// Monitor memory before and after operations
const before = performance.memory.usedJSHeapSize;
loadVolume();
const after = performance.memory.usedJSHeapSize;
console.log(`Memory growth: ${after - before} bytes`);
```

### WASM Memory Tests

```rust
#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod wasm_memory_tests {
    #[test]
    fn test_wasm_memory_growth() {
        use wasm_bindgen::JsCast;

        let performance = web_sys::window()
            .unwrap()
            .performance()
            .unwrap();

        let before = performance.memory().unwrap().used_js_heap_size() as usize;

        // Load volume
        let volume = create_test_volume_512x512x100();

        let after = performance.memory().unwrap().used_js_heap_size() as usize;

        // Allow 10% overhead
        let growth = (after - before) as f64;
        let expected = std::mem::size_of_val(&volume.data) as f64;
        assert!(growth < expected * 1.1, "Memory growth: {} vs expected {}", growth, expected);
    }
}
```

## Known Limitations

1. **GPU Memory**: Valgrind cannot detect GPU memory leaks (WebGPU driver allocations)
   - **Mitigation**: Test texture cleanup with explicit destroy calls
   - **Mitigation**: Monitor GPU memory with vendor tools (nvidia-smi, AMD GPU Tools)

2. **External Libraries**: Valgrind cannot inspect inside closed-source libraries
   - **Mitigation**: Use suppression file for known benign leaks
   - **Mitigation**: Test library behavior with standalone benchmarks

3. **CI Timeout**: Valgrind slows execution ~10-50x
   - **Mitigation**: Run memory tests only on release builds
   - **Mitigation**: Run memory tests in separate job (not on every push)

## References

- Valgrind User Manual: https://valgrind.org/docs/manual/manual.html
- Rust Memory Safety: https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html
- Memory Profiling with Valgrind: https://valgrind.org/docs/manual/massif.html
- Criterion Benchmarks: https://bheisler.github.io/criterion.rs/book/
