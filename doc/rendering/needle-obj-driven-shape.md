# Proposal: Replace Procedural Needle with OBJ-Driven Shape

Status: Draft (Proposal — no code changes yet)
Authors: Senior Developer
Date: 2026-07-16
Scope: `src/rendering/view/mesh/*`, `src/rendering/view/mip/*`, `src/rendering/view/mpr/*`,
`src/rendering/shaders/{mesh,mip,mpr}.wgsl`, `static/index.html`, `src/server/handlers.rs`

---

## 1. Background & Motivation

Today, every needle in the project is rendered as a **procedural cylinder**:

- `MeshUniforms.needles[i] = { entry, tip, radius, id, color }` ([mesh.rs:14-32](file:///c:/Users/admin/Documents/GIT/me/dev/kepler2-wgpu/src/rendering/view/mesh/mesh.rs#L14-L32))
- `mesh.wgsl::point_inside_needle` raymarches a finite cylinder along `entry→tip` ([mesh.wgsl:121-136](file:///c:/Users/admin/Documents/GIT/me/dev/kepler2-wgpu/src/rendering/shaders/mesh.wgsl#L121-L136))
- `mip.wgsl::point_inside_needle` does the same for MIP ([mip.wgsl:104-118](file:///c:/Users/admin/Documents/GIT/me/dev/kepler2-wgpu/src/rendering/shaders/mip.wgsl#L104-L118))
- `mpr.wgsl` projects the segment orthogonally to the slice plane with `distance_point_to_segment_3d` ([mpr.wgsl:179-194](file:///c:/Users/admin/Documents/GIT/me/dev/kepler2-wgpu/src/rendering/shaders/mpr.wgsl#L179-L194))

This is fast and deterministic, but the **shape is fixed**: any variation (bevelled tip, curved shaft, trocar pattern, asymmetric hub, multi-lumen cannula) is impossible to express. The team needs users to be able to import an OBJ of a real instrument (e.g. a specific biopsy needle model) and have that geometry drive the visual.

A high-level architecture for OBJ loading is already written down in
[obj-loading-and-rendering-architecture.md](file:///c:/Users/admin/Documents/GIT/me/dev/kepler2-wgpu/doc/rendering/obj-loading-and-rendering-architecture.md),
but **no parser/loader is implemented** in the tree yet (no `tobj` crate in `Cargo.toml`,
no `src/rendering/mesh/obj_*.rs`). This proposal adopts that design and extends it specifically
for the needle use case.

## 2. Goals & Non-Goals

### Goals

1. Replace the procedural cylinder with an **OBJ-loaded mesh** that still respects the existing
   `entry / tip / radius / id / color` API.
2. Keep all existing JS event handlers and WASM exports working — the change must be additive
   (a new event, e.g. `SetMeshNeedleShape`).
3. Render the imported needle correctly in **all three views**: Mesh (3D DVR), MIP, MPR.
4. Keep current clipping/plane modes (`inside`, `inside crop`, `plan`) functional for the
   mesh-shaped needle.
5. WASM-compatible: OBJ bytes cross the JS boundary as `Uint8Array`; no filesystem dependency.
6. Stay cross-platform (Windows, macOS, Linux native + `wasm32-unknown-unknown`).

### Non-Goals (this PR)

- MTL / texture support (deferred per architecture doc).
- Skeletal / hierarchical meshes.
- Curved (non-straight) needle paths — entry→tip stays a straight line; the mesh only
  replaces the **shape** around that line.
- Editing OBJ files in-app.

## 3. High-Level Architecture

```
                       ┌────────────────────┐
   JS  (index.html)    │  FileReader (.obj) │
   ──────────────────▶ │  Uint8Array bytes  │
                       └─────────┬──────────┘
                                 │ SetMeshNeedleShape(id, name, bytes)
                                 ▼
   ┌──────────────────────────────────────────────────────────┐
   │   src/server/handlers.rs (WASM)                          │
   │   parse_obj(bytes) → MeshData                            │
   │   mesh_data_to_mesh(data, NeedleConventions) → Mesh      │
   │   NeedleShapeRegistry::upsert(id, mesh, axis, length)    │
   └─────────┬────────────────────────────────────────────────┘
             │  (later: set_new_needle_mm keeps entry/tip/color)
             ▼
   ┌──────────────────────────────────────────────────────────┐
   │  MeshView / MipView / MprView                            │
   │  ┌──────────────────┐  ┌──────────────────┐  ┌─────────┐  │
   │  │ NeedleMeshContext│  │ MipNeedleProj    │  │ MprNee- │  │
   │  │  (rasterize OBJ) │  │  (project tri)   │  │ dleProj │  │
   │  └──────────────────┘  └──────────────────┘  └─────────┘  │
   └──────────────────────────────────────────────────────────┘
```

Key shift: the needle is no longer expressed in `NeedleUniform.{entry,tip,radius}` for shape.
The shape is a real **mesh**; `entry / tip / color` continue to drive **placement** and
**tint**, and a new per-needle model matrix is built from them.

## 4. Detailed Design

### 4.1 Canonical Needle Convention

Every imported OBJ is normalized at load time to a single convention so placement math is uniform:

| Property            | Convention                                       |
|---------------------|--------------------------------------------------|
| Up axis (shaft)     | +Z = tip direction                               |
| Base anchor         | (0, 0, 0)                                        |
| Tip                 | (0, 0, +1.0)                                     |
| Shaft radius        | 1.0 in object space (X/Y plane)                  |
| Unit scale          | 1.0 in object space                              |
| Handedness          | Right-handed, CCW winding                        |
| WGSL clip-space     | Matches the volume's UV [0,1]³ centered at .5    |

The conversion to a runtime instance is then:

```
S = diag(radius_mm_in_uv, radius_mm_in_uv, length_mm_in_uv)   // see §4.4
R = rotate_from_axis_to_z(actual_dir)                          // dir = tip - entry
T = entry_world_uv
M_model = T · R · S
```

We deliberately **do not** require the user to author OBJs in this exact convention — the
adapter handles arbitrary inputs (see §4.3).

### 4.2 OBJ Parser (per existing architecture doc)

Create the four files already specified in `obj-loading-and-rendering-architecture.md`:

```
src/rendering/mesh/obj_parser.rs      # streaming v/vt/vn/f parser
src/rendering/mesh/mesh_data.rs      # MeshData, Submesh, MaterialRef
src/rendering/mesh/mesh_adapter.rs   # MeshData → Mesh (project's vertex format)
src/rendering/mesh/obj_loader.rs     # public API, cache, normalization
```

Add `tobj = "0.1"` (or a hand-rolled parser) to `Cargo.toml`. Native and WASM both get
the same path because everything takes `&[u8]`, not `Path`.

### 4.3 Needle-Specific Normalization Adapter

A new function `normalize_for_needle(data: &MeshData) -> NormalizedNeedle`:

1. **Detect shaft direction**:
   - If the OBJ has a `g shaft` / named group, use it; otherwise, use PCA on vertex
     positions to find the principal axis.
   - Fall back: longest bounding-box axis.
2. **Re-orient**: rotate the whole mesh so the shaft axis aligns with +Z and the tip is
   the +Z extreme. Handle mirrored normals by checking winding parity.
3. **Normalize length**: rescale so tip_z = 1.0; record `original_length` for the model
   matrix.
4. **Normalize radius**: rescale X/Y so max radial distance from Z-axis = 1.0; record
   `original_radius` for the model matrix.
5. **Compute bounding AABB** in normalized space (used by MIP/MPR for fast pre-cull).

Output: `(vertices, indices, original_length, original_radius, aabb)`.

### 4.4 New GPU Buffer: Per-Needle Model Matrix

Extend the `MeshUniforms` / `MipUniforms` / `MprUniforms` needle array by **4 floats per needle**
(model matrix's scale+rotation block) **without breaking the existing 16-byte stride**:

```rust
struct NeedleUniform {
    entry : [f32; 3],     // anchor in volume-UV [0,1]
    radius: f32,          // shaft radius in volume-UV (X/Y extent)
    tip   : [f32; 3],
    id    : u32,
    color : [f32; 4],
    // NEW (4 floats, fits existing padding):
    dir   : [f32; 3],     // unit vector (tip - entry).normalized
    len   : f32,          // |tip - entry| in volume-UV
}
```

The fragment shader builds the model matrix inline:

```wgsl
fn needle_model(idx: u32) -> mat4x4<f32> {
    let n = u_vol.needles[idx];
    let axis = n.dir;                  // already normalized CPU-side
    // build orthonormal basis (right, up, axis)
    let up_ref = select(vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0), abs(axis.y) > 0.9);
    let right = normalize(cross(up_ref, axis));
    let up    = cross(axis, right);
    // R packs (right, up, axis) as columns
    let R = mat3x3(right, up, axis);
    let S = mat3x3(n.radius, n.radius, n.len);
    return mat4x4(
        (R * S).x, 0.0,
        (R * S).y, 0.0,
        (R * S).z, 0.0,
        n.entry.x, n.entry.y, n.entry.z, 1.0,
    );
}
```

`radius` and `len` retain their old semantic meaning: the **bounding cylinder** used by
clipping and the plane visualization. The shape drawn inside that cylinder is the imported mesh.

### 4.5 Mesh View (3D) — Switch from Raymarched Cylinder to Rasterized Mesh

The `mesh.wgsl` fullscreen-quad raymarcher cannot rasterize arbitrary triangles. We add a
**second render pass** after the DVR pass, dedicated to the needle mesh:

- New pipeline: `create_needle_mesh_pipeline(device, ...)` (depth = `CompareFunction::Less`).
- One `MultiMeshContext` per needle, or a single batched context (one slot per active needle).
  Re-use the pattern already proven by spine meshes ([basic_mesh_context.rs:221-348](file:///c:/Users/admin/Documents/GIT/me/dev/kepler2-wgpu/src/rendering/view/mesh/basic_mesh_context.rs#L221-L348)).
- Each frame, for each active needle, update MVP = `view_proj * needle_model(id)`.
- Vertex shader: apply the model matrix; pass UV-space position to fragment.
- Fragment shader: shade with the existing `compute_lighting` helper; tint by `NeedleUniform.color`.

**Behavior preservation** for the three modes:

| Mode    | Old behavior                              | New behavior                                              |
|---------|-------------------------------------------|------------------------------------------------------------|
| clear   | cylinder invisible                       | mesh invisible (no draw call)                              |
| inside  | cylinder shaded in fragment raymarcher    | mesh rasterized over DVR; **DVR keeps full raymarch**     |
| crop    | cylinder + DVR half-space clip            | mesh rasterized + DVR half-space clip (clip plane intact)  |
| plan    | cylinder + plane-quad overlay             | mesh + plane-quad overlay (unchanged)                      |

The DVR raymarcher keeps its `point_inside_needle` call but treats it as a **bounding-cylinder
test** only (cheap rejection for the clip modes). The actual visual comes from the rasterized
mesh, so the cylinder's procedural shading is no longer used.

For the **half-space clip** mode: today's code derives a normal from
`cross(needle.tip - needle.entry, rotate_vec_around_axis(+X, axis, plane_rotation_angle))`.
That math still works because we already store `dir` and `len` per needle.

### 4.6 MIP View — Replace Procedural Cylinder with Triangle Projection

MIP also uses a fullscreen-quad raymarcher. The cleanest swap is **analogous to Mesh view**:
add a second rasterization pass that draws the needle mesh using the same MVP.

- Re-use the `create_basic_mesh_pipeline_with_lighting` (already exists).
- Single `MultiMeshContext` in `MipView` mirroring the spine pattern in `MeshView`.
- For each needle with shape, the vertex shader transforms `position_uv` by `needle_model(id)`,
  the fragment shades with the needle's color and `compute_lighting`, depth-tested against the
  MIP pass.

The `point_inside_needle` cylinder test in `mip.wgsl` is kept only for **needle hit detection
on the intensity path** (sets `needle_color`), so the existing JS `getHitNeedle`-style logic
(if any) keeps working. The visible pixel will come from the rasterized mesh, not from
the MIP volume sample.

### 4.7 MPR View (2D Slice) — Project Triangles to Slice

`mpr.wgsl` is a fragment shader on the slice plane. Triangles cannot be rasterized there
directly. Two viable approaches:

**Option A (recommended, simpler): rasterize in 3D, sample into slice.**

- Add a small offscreen 3D pass that draws the needle mesh with depth.
- In the slice fragment shader, sample the depth (or a needle-id attachment) and overlay
  needle color where the slice's depth is behind the needle.
- This mirrors how some volume-clipping overlays already work in the project.

**Option B (alternative, shader-only): per-fragment triangle iteration.**

- Store the needle mesh in a structured buffer (`storage_buffer` in WGSL) — positions +
  per-needle model matrix.
- For each pixel, iterate up to N triangles (cap at e.g. 256 for cost), compute
  ray-triangle distance in volume space, take the closest hit.
- Pros: no extra pass, no texture sample. Cons: bounded by triangle count; potentially
  expensive at 512×512 per slice × 3 viewports.

**Recommendation**: start with **Option A** because (a) it re-uses the same `MultiMeshContext`
we already need for Mesh/MIP, (b) it keeps the MPR shader simple, and (c) it is the
architectural pattern the project already favours (see spine-orthogonal projection work).

Either way: the existing `distance_point_to_segment_3d` cylinder test in `mpr.wgsl` is
**removed** in favor of a depth-buffer sample (or replaced with a bounding-cylinder
pre-cull for Option B).

### 4.8 Application-Layer State Changes

`App::set_new_needle_mm` and friends stay unchanged. The new API is:

```rust
// In src/application/app.rs
pub fn set_needle_shape(&mut self, id: u32, name: String, obj_bytes: Vec<u8>) -> Result<(), String> {
    // 1. Parse OBJ (delegated to obj_loader)
    // 2. Normalize to needle convention
    // 3. Upsert into NeedleShapeRegistry (keyed by (id, hash of bytes))
    // 4. Re-publish needles[id] to MeshView / MipView / MprView so they rebuild
    //    their mesh contexts with the new shape
    // 5. Return Err on parse failure with a human-readable message
}
```

The new JS event is `SetMeshNeedleShape { index, id, name, bytes: Uint8Array }`.
WASM-binding: `pub fn set_needle_shape_wasm(id: u32, name: String, bytes: &[u8])`.

`NeedleShapeRegistry` is a `HashMap<u32, Arc<NeedleShape>>` stored in `AppModel`. It caches
parsed+normalized meshes so re-importing the same OBJ (or reloading a session) is free.

### 4.9 Front-End (static/index.html)

Add to the existing needle tab:

```html
<div class="needle-row">
  <span class="needle-label">Shape</span>
  <input type="file" id="needle-shape-obj" accept=".obj">
  <span id="needle-shape-name" class="needle-hint">procedural cylinder</span>
</div>
```

Wiring:

1. `FileReader.readAsArrayBuffer(file)` → `Uint8Array`.
2. `gl_canvas.set_needle_shape(id, file.name, bytes)` (new WASM export).
3. On success, show file name; on error, show the error message in red.

The existing controls (Mode, Length, Angle, Radius, Entry) remain — they now drive the
**placement transform** of the imported shape.

### 4.10 Server / HTTP Boundary

`src/server/handlers.rs` is **not** in scope: needle shapes are uploaded directly from the
browser to the WASM module, just like the existing volume upload flow. No new endpoint needed.

## 5. Migration & Compatibility

| Surface                          | Change                                       |
|----------------------------------|----------------------------------------------|
| `set_new_needle_mm`              | **Unchanged.** entry/tip/color still set here |
| `set_needle_position_mm`         | **Unchanged.** updates `tip` (and dir/len)   |
| `set_needle_radius`              | **Unchanged semantics** — now scales the X/Y extent of the imported mesh |
| `set_needle_enabled`             | **Unchanged.**                              |
| `set_needle_angle`               | **Unchanged.** still drives clip plane        |
| `MeshUniforms.needles` stride    | **Preserved** (added `dir[3]+len` inside the same 16-byte padding) |
| Shader uniform buffer layout     | **Identical size**; new fields are alias-free |
| Existing JS event handlers       | **No change** required                       |
| Default needle appearance        | **Backward compat:** if no shape is set for an id, fall back to the procedural cylinder (keeps current demo runs working) |

The procedural fallback means the migration is fully **opt-in** — every existing call site
keeps working until the user explicitly imports an OBJ.

## 6. File-by-File Change List (no code yet — just the surface)

| File                                                              | Change                                                                  |
|-------------------------------------------------------------------|--------------------------------------------------------------------------|
| `Cargo.toml`                                                      | Add `tobj = "0.1"` (or none if we hand-roll); bump nothing else          |
| `src/rendering/mesh/obj_parser.rs`                                | **New** — streaming OBJ parser                                          |
| `src/rendering/mesh/mesh_data.rs`                                 | **New** — `MeshData`, `Submesh`, `MeshMeta`                              |
| `src/rendering/mesh/mesh_adapter.rs`                              | **New** — generic `MeshData → Mesh`; needle-specific bits in §4.3      |
| `src/rendering/mesh/obj_loader.rs`                                | **New** — `pub fn load_needle_shape(bytes) -> Result<NeedleShape, …>`   |
| `src/rendering/view/mesh/mesh.rs`                                 | Add `dir[3] + len` to `NeedleUniform`; keep stride 16 bytes              |
| `src/rendering/view/mesh/mesh_view.rs`                            | Add `MultiMeshContext` for needle shapes; re-publish on shape change     |
| `src/rendering/view/mip/mod.rs`                                   | Add needle `MultiMeshContext`; same `NeedleUniform` shape                |
| `src/rendering/view/mpr/mpr_view.rs`                              | Add 3D needle pass that feeds a needle-id/depth attachment for the slice |
| `src/rendering/shaders/mesh.wgsl`                                 | Add `needle_model(id)` helper; keep cylinder test as bounding-cull only  |
| `src/rendering/shaders/mip.wgsl`                                  | Same as above                                                           |
| `src/rendering/shaders/mpr.wgsl`                                  | Replace `distance_point_to_segment_3d` with depth-attach sample          |
| `src/rendering/shaders/needle_mesh.wgsl`                          | **New** — small vert+frag dedicated to the rasterized needle             |
| `src/application/app_model.rs`                                    | Add `NeedleShapeRegistry` field; reset on volume change                  |
| `src/application/app.rs`                                          | Add `set_needle_shape` API + the existing setters keep working           |
| `src/application/render_app.rs`                                   | Add `Event::UserEvent(UserEvent::SetMeshNeedleShape(…))` handler         |
| `src/lib.rs` / wasm-bindgen exports                               | Expose `set_needle_shape_wasm`                                          |
| `static/index.html`                                               | Add `<input type="file">` in the needle tab + JS handler                 |
| `doc/CHANGELOG.md`                                                | Add an entry under today's timestamp                                     |
| `doc/rendering/obj-loading-and-rendering-architecture.md`         | Mark §2.1 / §2.2 / §2.3 as implemented (PR closes that loop)             |
| `doc/rendering/needle-obj-driven-shape.md`                        | **New** — this proposal, kept as the design record                       |

## 7. Risk & Open Questions

1. **Per-frame mesh rebuild on shape change**: shapes are uploaded once and cached, but the
   first import of a new shape still triggers a buffer re-create on the active device.
   Mitigated by the registry + reference counting.
2. **Mode `inside` with no DVR raymarcher contribution**: the cylinder raymarch was a
   fallback. We now rely on the rasterized mesh always being present. Need a guard for
   "shape not loaded yet" → render the procedural cylinder, or skip with a warning.
3. **MIP triangle overlap with intensity path**: when a needle mesh crosses a high-intensity
   region, the user might want the needle to win (surgical priority). Plan: `depth_write` on
   the needle pass, `depth_test = Less`, and the MIP pass keeps `LessEqual` to itself but
   outputs **after** the needle pass for the same color attachment. This mirrors the
   `CompareFunction::Always` pattern the spine already uses.
4. **OBJ file size**: a real biopsy instrument can be a few thousand triangles. 32 needles
   × 5k tris = 160k tris — well within wgpu's comfort zone. We'll log the triangle count at
   load time and warn above 50k per needle.
5. **OBJ units**: most modeling tools export in millimetres, but some in metres. The user
   sets Length/Angle/Radius in mm via the existing UI; the adapter's normalization lets the
   OBJ carry any unit. We should expose a "scale_hint" in the file picker (1.0 default) so
   users can correct it.

## 8. Acceptance Criteria (for the PR that follows this proposal)

- [ ] `cargo build` and `cargo build --target wasm32-unknown-unknown` succeed without new warnings.
- [ ] Importing `assets/needles/standard_biopsy_18g.obj` (test fixture, ~500 tris) makes
      the needle appear in Mesh / MIP / MPR views in the correct entry→tip pose.
- [ ] Existing `set_new_needle_mm` calls without a prior shape still draw the procedural
      cylinder (backward compatibility).
- [ ] `set_needle_angle` still controls the clip plane correctly with an imported shape.
- [ ] 32 needles × 1k tris each sustain ≥ 60 fps on the test machine (verify in the
      `mesh_integration_tests.rs` style).
- [ ] `cargo test` passes; new unit tests cover the parser, the needle-normalization adapter,
      and the bounding-cylinder clip math.
- [ ] `doc/CHANGELOG.md` updated; `doc/rendering/needle-obj-driven-shape.md` is this document.

## 9. Phased Roll-out

- **Phase 1 (this PR scope)**: parser + adapter + Mesh-view rasterization + procedural
  fallback. Defer MIP and MPR to Phase 2.
- **Phase 2**: MIP rasterization, MPR depth-attach projection.
- **Phase 3**: caching, MTL (color override from material), `scale_hint` UI.

This phasing keeps the diff small, lets us validate the OBJ pipeline with the simplest
view first, and matches the project's preference for "incremental and minimal" delivery
(see `project_rules.md` rule 9).
