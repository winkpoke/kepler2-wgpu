1. The project is a Rust project.
2. The project is a WGPU project.
3. The code shall be compiled with native and wasm target. The wasm shall be compiled with "wasm-pack build -t web"
4. The project documents are all under the folder "doc". Please refer to the documentations in the folder.
5. Do not use npx or live serve to test the wasm build since I will manually test the wasm build in the browser for now.
6. Compilation and test shall turn on feature mesh.
7. Please log information with debug level when necessary in order to help debug the project.
8. Implement trace-level logging to capture detailed information when logging frequency becomes excessive, particularly within the render loop. This will help monitor performance without overwhelming the log output. Ensure the trace logging is only activated when necessary to maintain system efficiency.
9. As a foundamential rule, anything add to the code base shall minimal and step-by-step. This shall produce a minimal viable product first and add more features incrementally.