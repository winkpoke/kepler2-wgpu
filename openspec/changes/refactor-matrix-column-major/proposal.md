# Refactor Matrix4x4 to Column-Major Layout

## Why
The current `Matrix4x4` implementation uses a row-major memory layout. This creates friction with WGPU and modern graphics standards, which favor column-major layouts. Additionally, the current struct lacks `#[repr(C)]`, which makes memory layout undefined and unsafe for direct buffer copying.

## What Changes
- **Memory Layout**: Transition `Matrix4x4` internal storage to **Column-Major**.
- **Safety**: Add `#[repr(C)]` to `Matrix4x4` to guarantee predictable memory layout.
- **Field Renaming**: Rename `data` to `columns` to force compilation errors on existing row-major access patterns.
- **API Clarification**:
    - Rename/Alias `from_array` to `from_rows` (performs transpose).
    - Add `from_cols` (direct storage).
- **Migration**: Update tests and call sites to use explicit constructors instead of struct literals.

## Impact
- **Safety**: Guaranteed memory layout for FFI/WGPU.
- **Performance**: Potential for direct copy to WGPU buffers without runtime transposition.
- **Code Health**: Explicit API prevents accidental layout mismatches.
- **Breaking Changes**: 
    - `Matrix4x4` field `data` is renamed.
    - `from_array` behavior is clarified (conceptually unchanged but explicit).
    - Struct literal initialization will break.
