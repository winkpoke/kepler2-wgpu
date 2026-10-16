//! Rejection-path tests for `Mesh::import_obj_bytes`.
//!
//! Four rejection categories requested for the needle OBJ-driven rendering
//! refactor, plus positive controls that keep the happy path honest:
//!
//! 1. Bad files      — garbage bytes, empty payload, header-only (no geometry)
//! 2. Missing normals— OBJ without `vn` lines, or with fewer `vn` than `v`
//! 3. Multi-model    — more than one `o` object in a single document
//! 4. Oversized      — payload exceeding `Mesh::MAX_OBJ_BYTES`
//!
//! These tests are pure byte-level and require no GPU device.

use kepler_wgpu::mesh::Mesh;

/// Minimal valid OBJ: one quad (two triangles) in the XY plane, per-vertex
/// normals, single `o` object. Lives inside [0,1]^2 so the UV-space (kind 0)
/// extent check passes.
fn valid_obj() -> String {
    [
        "o test_needle",
        "v 0 0 0",
        "v 1 0 0",
        "v 1 1 0",
        "v 0 1 0",
        "vn 0 0 1",
        "vn 0 0 1",
        "vn 0 0 1",
        "vn 0 0 1",
        "f 1//1 2//2 3//3",
        "f 1//1 3//3 4//4",
    ]
    .join("\n")
}

// ---------------------------------------------------------------------------
// Positive controls (happy path still works after the strict validation)
// ---------------------------------------------------------------------------

#[test]
fn test_valid_single_object_imports() {
    let meshes = Mesh::import_obj_bytes(valid_obj().as_bytes(), 0).expect("valid OBJ must import");
    assert_eq!(meshes.len(), 1);
    let mesh = &meshes[0];
    assert_eq!(mesh.label_name, "test_needle");
    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]); // triangulated quad
    // Normals are passed through verbatim (no silent recompute).
    for v in &mesh.vertices {
        assert_eq!(v.normal, [0.0, 0.0, 1.0]);
    }
}

// ---------------------------------------------------------------------------
// Category 1: bad files
// ---------------------------------------------------------------------------

#[test]
fn test_garbage_bytes_rejected() {
    let garbage = b"this is definitely not an obj file \x01\x02\xff\xfe".as_slice();
    assert!(
        Mesh::import_obj_bytes(garbage, 0).is_err(),
        "garbage payload must not parse into a mesh"
    );
}

#[test]
fn test_empty_payload_rejected() {
    assert!(Mesh::import_obj_bytes(b"", 0).is_err());
}

#[test]
fn test_object_without_geometry_rejected() {
    // Syntactically fine, but the single `o` object carries no vertices.
    let obj = "o empty_thing\n";
    let err = Mesh::import_obj_bytes(obj.as_bytes(), 0).expect_err("must reject empty object");
    assert!(
        err.to_string().contains("no geometry"),
        "unexpected error: {}",
        err
    );
}

#[test]
fn test_vertices_without_faces_rejected() {
    // Vertices with no faces and no normals: caught by the textual v/vn gate
    // before parsing even starts.
    let obj = "o vonly\nv 0 0 0\nv 1 0 0\nv 1 1 0\n";
    let err = Mesh::import_obj_bytes(obj.as_bytes(), 0).expect_err("must reject");
    let msg = err.to_string();
    assert!(
        msg.contains("3 vertex positions but 0 vertex normals"),
        "unexpected error: {}",
        msg
    );
}

// ---------------------------------------------------------------------------
// Category 2: missing normals
// ---------------------------------------------------------------------------

#[test]
fn test_missing_normals_rejected() {
    // Same quad, but all `vn` lines stripped.
    let obj = [
        "o no_normals",
        "v 0 0 0",
        "v 1 0 0",
        "v 1 1 0",
        "v 0 1 0",
        "f 1//1 2//2 3//3",
        "f 1//1 3//3 4//4",
    ]
    .join("\n");
    let err = Mesh::import_obj_bytes(obj.as_bytes(), 0).expect_err("must reject missing normals");
    let msg = err.to_string();
    assert!(
        msg.contains("4 vertex positions but 0 vertex normals"),
        "unexpected error: {}",
        msg
    );
}

#[test]
fn test_partial_normals_rejected() {
    // 4 vertices but only 2 normals, and every face still references an
    // in-bounds normal slot: tobj parses fine, so the count-mismatch gate in
    // import_obj_bytes must be the one to reject (not silently padded).
    let obj = [
        "o partial_normals",
        "v 0 0 0",
        "v 1 0 0",
        "v 1 1 0",
        "v 0 1 0",
        "vn 0 0 1",
        "vn 0 0 1",
        "f 1//1 2//2 3//1",
        "f 1//1 3//1 4//2",
    ]
    .join("\n");
    let err = Mesh::import_obj_bytes(obj.as_bytes(), 0).expect_err("must reject partial normals");
    let msg = err.to_string();
    assert!(
        msg.contains("4 vertex positions but 2 vertex normals"),
        "unexpected error: {}",
        msg
    );
}

#[test]
fn test_normals_index_out_of_bounds_rejected() {
    // Faces reference normal slots that do not exist (4 vn slots used, 2
    // written): the OBJ parser itself rejects before the count check.
    let obj = [
        "o bad_normal_idx",
        "v 0 0 0",
        "v 1 0 0",
        "v 1 1 0",
        "v 0 1 0",
        "vn 0 0 1",
        "vn 0 0 1",
        "f 1//1 2//2 3//3",
        "f 1//1 3//3 4//4",
    ]
    .join("\n");
    let err = Mesh::import_obj_bytes(obj.as_bytes(), 0).expect_err("must reject");
    assert!(
        err.to_string().to_lowercase().contains("normal"),
        "unexpected error: {}",
        err
    );
}

// ---------------------------------------------------------------------------
// Category 3: multi-model documents
// ---------------------------------------------------------------------------

#[test]
fn test_multiple_objects_rejected() {
    let obj = [
        "o first",
        "v 0 0 0",
        "v 1 0 0",
        "v 1 1 0",
        "vn 0 0 1",
        "vn 0 0 1",
        "vn 0 0 1",
        "f 1//1 2//2 3//3",
        "o second",
        "v 0 0 0",
        "v 1 0 0",
        "v 1 1 0",
        "vn 0 0 1",
        "vn 0 0 1",
        "vn 0 0 1",
        "f 1//1 2//2 3//3",
    ]
    .join("\n");
    let err = Mesh::import_obj_bytes(obj.as_bytes(), 0).expect_err("must reject multi-object file");
    let msg = err.to_string();
    assert!(
        msg.contains("2 objects"),
        "error should report the object count: {}",
        msg
    );
}

#[test]
fn test_combined_export_of_two_meshes_no_longer_reimports() {
    // meshes_to_obj() emits one combined multi-object document; combined
    // export remains available, but re-importing it is now a hard error by
    // design (one object per file, see mesh_to_obj_single). This locks the
    // contract so a future accidental relaxation is a conscious decision.
    let combined = [
        "o a",
        "v 0 0 0",
        "v 1 0 0",
        "v 1 1 0",
        "vn 0 0 1",
        "vn 0 0 1",
        "vn 0 0 1",
        "f 1//1 2//2 3//3",
        "o b",
        "v 0 0 0",
        "v 1 0 0",
        "v 1 1 0",
        "vn 0 0 1",
        "vn 0 0 1",
        "vn 0 0 1",
        "f 1//1 2//2 3//3",
    ]
    .join("\n");
    assert!(Mesh::import_obj_bytes(combined.as_bytes(), 0).is_err());
}

// ---------------------------------------------------------------------------
// Category 4: oversized payloads
// ---------------------------------------------------------------------------

#[test]
fn test_oversized_payload_rejected_before_parse() {
    // 1 byte over the limit; content is irrelevant because the size gate runs
    // before any parsing. 64 MiB allocation is fine for a test process.
    let oversized = vec![b'v' as u8; Mesh::MAX_OBJ_BYTES + 1];
    let err =
        Mesh::import_obj_bytes(&oversized, 0).expect_err("payload over the limit must be rejected");
    let msg = err.to_string();
    assert!(msg.contains("too large"), "unexpected error: {}", msg);
}

#[test]
fn test_payload_at_limit_is_parsed_not_size_rejected() {
    // Exactly at the limit must NOT trip the size gate (boundary check). The
    // payload content is a valid OBJ padded with comment lines to the limit.
    let base = valid_obj();
    let padding_needed = Mesh::MAX_OBJ_BYTES - base.len();
    // Comment lines: each "# \n" is 3 bytes; build the padding in bulk.
    let pad_line = "# padding padding padding\n"; // 26 bytes
    let mut obj = String::with_capacity(Mesh::MAX_OBJ_BYTES);
    let full_lines = padding_needed / pad_line.len();
    let remainder = padding_needed % pad_line.len();
    for _ in 0..full_lines {
        obj.push_str(pad_line);
    }
    if remainder > 0 {
        obj.push_str(&format!("# {}\n", "#".repeat(remainder.saturating_sub(3))));
    }
    obj.push_str(&base);
    assert_eq!(obj.len(), Mesh::MAX_OBJ_BYTES);
    // Must get past the size gate; parse itself should succeed.
    let meshes = Mesh::import_obj_bytes(obj.as_bytes(), 0)
        .expect("payload exactly at the limit must import");
    assert_eq!(meshes.len(), 1);
}

// ---------------------------------------------------------------------------
// Limits are exported and sane
// ---------------------------------------------------------------------------

#[test]
fn test_import_limits_constants() {
    assert_eq!(Mesh::MAX_OBJ_BYTES, 64 * 1024 * 1024);
    assert_eq!(Mesh::MAX_OBJ_MODELS, 1);
}
