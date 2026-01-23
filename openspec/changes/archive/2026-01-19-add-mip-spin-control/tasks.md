## 1. Implementation
- [x] Define MIP rotation state and setter interface
- [x] Extend MIP uniforms with rotation parameters
- [x] Implement robust ray-box intersection for rotated rays
- [x] Apply rotation consistently in MIP shader sampling
- [x] Route rotation control through AppView and user events
- [x] Add uniform layout and rotation smoke tests

## 2. Validation
- [x] Run native tests (`cargo test`)
- [x] Run native build (`cargo build`)
- [x] Build wasm package (`wasm-pack build -t web`)
- [x] Manually verify rotation in browser (project manual workflow)
