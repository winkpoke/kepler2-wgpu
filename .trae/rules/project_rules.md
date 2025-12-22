1. Always chat in English.
3. My system is Windows but the code shall work in Mac and Linux.
4. The project is a Rust-based WGPU medical imaging framework for CT reconstruction, MPR (multi-planar reconstruction), MIP (maximum intensity projection), and 3D visualization.
5. The project must build for both native and WebAssembly (wasm) targets.
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
22. The codebase must remain buildable, testable, and inspectable at all times.
23. Time stamp format shall be YYYY-MM-DDTHH-MM-SS which takes the local time zone into account. The current local time zone is Beijing. Use "Get-Date -Format "yyyy-MM-ddTHH-mm-ss" to check the system time to make sure the time is correctly handled.