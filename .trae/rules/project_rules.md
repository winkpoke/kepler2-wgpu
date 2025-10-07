1. The project is a Rust-based WGPU medical imaging framework for CT reconstruction, MPR (multi-planar reconstruction), MIP (maximum intensity projection), and 3D visualization.
2. The project must build for both native and WebAssembly (wasm) targets.
3. The wasm build command is: wasm-pack build -t web -- --features mesh
   The native build and test commands are:
        cargo build --features mesh
        cargo test --features mesh
4. Do not use npx, live-server, or any node-based tools for wasm testing. The wasm output will be manually tested in the browser.
6. All documentation resides under the doc/ folder. Each feature or change must include a short explanation in this folder.
7. The mesh feature must always be enabled for builds and tests.
8. Development must be incremental and minimal. Add features step by step, starting with the minimal viable product.
9. Default logging level is INFO.
10. Use DEBUG level logs for development and debugging.
11. Use TRACE level logs only for high-frequency or detailed performance diagnostics (e.g., within render loops).
12. TRACE logging must be gated by a feature flag named trace-logging.
13. When trace logging is enabled, consider sampling (e.g., log every Nth frame) to prevent log flooding.
14. Make logging configurable through the environment variable RUST_LOG (for native builds).
15. In wasm builds, route logs to the browser console using console_log.
16. Heavy trace logging must never be enabled in release builds.
17. GPU operations must remain efficient and non-blocking. Avoid excessive CPU-GPU synchronization in render loops.
18. Use cfg(feature = "trace-logging") to conditionally include expensive diagnostics.
19. Update doc/CHANGELOG.md for each user-visible change.
20. In the medical imaging context, ensure accuracy, numerical stability, and performance for CT reconstruction and visualization.
21. Manage GPU memory efficiently and minimize texture/data copies.
22. The codebase must remain buildable, testable, and inspectable at all times.
23. Do not use perspective projection in medical imaging for accuracy reasons unless asked.