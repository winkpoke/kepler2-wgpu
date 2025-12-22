# Proposal: Semantic Volume Encoding Refactor

## Problem
The codebase currently relies on scattered checks (`if is_packed_rg8`) and magic numbers (`1100.0` bias) to interpret volume texture data. This logic is duplicated across:
- `AppModel` (encoding/packing)
- `ViewFactory` (texture creation)
- `MipView` (shader uniforms)
- `MprView` (shader uniforms)
- `App` (window/level defaults)

Critically, this logic is often inferred from `wgpu::TextureFormat`, which is a storage detail, not a semantic contract. A concrete bug also exists where `WindowLevel` clamps the bias to `±1024`, preventing the required `1100.0` offset from working correctly in all contexts.

## Solution
Introduce a semantic `VolumeEncoding` enum that acts as the single source of truth for how to interpret volumetric data.

### 1. Semantic Encoding Enum
Define `VolumeEncoding` in `src/data` to decouple it from rendering implementation details.

```rust
pub enum VolumeEncoding {
    HuPackedRg8 { offset: f32 },
    HuFloat,
}
```

### 2. Centralized Logic in `RenderContent`
`RenderContent` will own the `VolumeEncoding` and provide a helper method to derive shader parameters.

```rust
impl RenderContent {
    pub fn decode_parameters(&self) -> VolumeDecodeParameters { ... }
}
```

### 3. Updated Data Flow
- **AppModel**: Returns `VolumeEncoding` alongside raw bytes.
- **RenderContent**: Stores the encoding.
- **Views**: Query `RenderContent` for decode parameters instead of implementing custom logic.

## Impact
- **Refactoring**: Affects `AppModel`, `RenderContent`, `ViewFactory`, `MipView`, `MprView`, and `App`.
- **Bug Fix**: Increases `WindowLevel` bias limits to `±2048`.
- **Safety**: Removes magic numbers and ensures encoding/decoding logic is always in sync.
