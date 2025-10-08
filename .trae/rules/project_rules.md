1. The project is a Rust-based WGPU medical imaging framework for CT reconstruction, MPR (multi-planar reconstruction), MIP (maximum intensity projection), and 3D visualization.
2. The project must build for both native and WebAssembly (wasm) targets.
3. The wasm build command is: wasm-pack build -t web 
   The native build and test commands are:
        cargo build 
        cargo test 
4. Do not use npx, live-server, or any node-based tools for wasm testing. The wasm output will be manually tested in the browser.
5. All documentation resides under the doc/ folder. Each feature or change must include a short explanation in this folder.
6. Development must be incremental and minimal. Add features step by step, starting with the minimal viable product.
7. Default logging level is INFO.
8.  Use DEBUG level logs for development and debugging.
9.  Use TRACE level logs only for high-frequency or detailed performance diagnostics (e.g., within render loops).
10. TRACE logging must be gated by a feature flag named trace-logging.
11. When trace logging is enabled, consider sampling (e.g., log every Nth frame) to prevent log flooding.
12. Make logging configurable through the environment variable RUST_LOG (for native builds).
13. In wasm builds, route logs to the browser console using console_log.
14. Heavy trace logging must never be enabled in release builds.
15. GPU operations must remain efficient and non-blocking. Avoid excessive CPU-GPU synchronization in render loops.
16. Use cfg(feature = "trace-logging") to conditionally include expensive diagnostics.
17. Update doc/CHANGELOG.md for each user-visible change.
18. In the medical imaging context, ensure accuracy, numerical stability, and performance for CT reconstruction and visualization.
19. Manage GPU memory efficiently and minimize texture/data copies.
20. The codebase must remain buildable, testable, and inspectable at all times.
21. Do not use perspective projection in medical imaging for accuracy reasons unless asked.
22. When working with Rust matrices (which are row-major) and shader matrices (which require column-major format), ensure you transpose the matrices before uploading them to maintain data consistency and correct mathematical operations.