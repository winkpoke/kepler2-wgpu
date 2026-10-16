//! Synthetic NIfTI fixtures test the public importer, not a parallel decoder.
use std::io::Write;

use flate2::{write::GzEncoder, Compression};
use glam::{Mat4, Vec3};
use kepler_wgpu::data::medical_imaging::{
    import_nifti_bytes, NiftiImportOptions, NiftiParser, NiftiReferenceSpace,
};

fn short(b: &mut [u8], offset: usize, n: i16, le: bool) {
    b[offset..offset + 2].copy_from_slice(&if le { n.to_le_bytes() } else { n.to_be_bytes() });
}
fn float(b: &mut [u8], offset: usize, n: f32, le: bool) {
    b[offset..offset + 4].copy_from_slice(&if le { n.to_le_bytes() } else { n.to_be_bytes() });
}
fn fixture(le: bool, datatype: i16, bitpix: i16, payload: &[u8]) -> Vec<u8> {
    let mut b = vec![0; 352];
    b[..4].copy_from_slice(&if le {
        348_i32.to_le_bytes()
    } else {
        348_i32.to_be_bytes()
    });
    short(&mut b, 40, 3, le);
    for axis in 1..=7 {
        short(&mut b, 40 + axis * 2, if axis == 1 { 2 } else { 1 }, le);
    }
    short(&mut b, 70, datatype, le);
    short(&mut b, 72, bitpix, le);
    for offset in [80, 84, 88] {
        float(&mut b, offset, 1.0, le);
    }
    float(&mut b, 108, 352.0, le);
    b[123] = 2;
    b[344..348].copy_from_slice(b"n+1\0");
    b.extend_from_slice(payload);
    b
}
fn sform(b: &mut [u8], code: i16, matrix: Mat4) {
    short(b, 254, code, true);
    for row in 0..3 {
        for col in 0..4 {
            float(b, 280 + row * 16 + col * 4, matrix.col(col)[row], true);
        }
    }
}
fn gzip(b: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(b).unwrap();
    encoder.finish().unwrap()
}
fn import(b: &[u8]) -> kepler_wgpu::data::medical_imaging::ImportedNifti {
    import_nifti_bytes(b, NiftiImportOptions::default()).unwrap()
}

#[test]
fn decodes_each_supported_scalar_type_in_both_byte_orders() {
    for le in [true, false] {
        let words16 = |values: [i16; 2]| {
            values
                .into_iter()
                .flat_map(|v| if le { v.to_le_bytes() } else { v.to_be_bytes() })
                .collect::<Vec<_>>()
        };
        let words32 = |values: [i32; 2]| {
            values
                .into_iter()
                .flat_map(|v| if le { v.to_le_bytes() } else { v.to_be_bytes() })
                .collect::<Vec<_>>()
        };
        let floats32 = |values: [f32; 2]| {
            values
                .into_iter()
                .flat_map(|v| if le { v.to_le_bytes() } else { v.to_be_bytes() })
                .collect::<Vec<_>>()
        };
        let floats64 = |values: [f64; 2]| {
            values
                .into_iter()
                .flat_map(|v| if le { v.to_le_bytes() } else { v.to_be_bytes() })
                .collect::<Vec<_>>()
        };
        for (dtype, bits, payload, expected) in [
            (2, 8, vec![0, 255], [0, 255]),
            (256, 8, vec![128, 127], [-128, 127]),
            (4, 16, words16([-30000, 1200]), [-30000, 1200]),
            (512, 16, words16([0, -1]), [0, 32767]),
            (8, 32, words32([-50000, 1234]), [-32768, 1234]),
            (16, 32, floats32([-2000.5, 1.5]), [-2001, 2]),
            (64, 64, floats64([-2000.5, 1.5]), [-2001, 2]),
        ] {
            let volume = import(&fixture(le, dtype, bits, &payload));
            assert_eq!(
                volume.volume.voxel_data(),
                expected,
                "datatype {dtype}, little endian {le}"
            );
        }
    }
}

#[test]
fn scaling_rounding_and_clipping_are_independent_of_ct_floor() {
    let mut b = fixture(
        true,
        4,
        16,
        &[-1000_i16, 30000]
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    float(&mut b, 112, 2.0, true);
    float(&mut b, 116, -0.5, true);
    let imported = import(&b);
    assert_eq!(imported.volume.voxel_data(), [-2001, 32767]);
    assert_eq!(imported.diagnostics.rounded_voxels, 2);
    assert_eq!(imported.diagnostics.clipped_voxels, 1);
    float(&mut b, 112, 0.0, true);
    float(&mut b, 116, f32::NAN, true);
    assert_eq!(import(&b).volume.voxel_data(), [-1000, 30000]);
}

#[test]
fn rejects_nonfinite_scaling_and_voxels() {
    let mut b = fixture(
        true,
        16,
        32,
        &[f32::NAN, 0.0]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    assert!(import_nifti_bytes(&b, Default::default()).is_err());
    b[352..356].copy_from_slice(&0.0_f32.to_le_bytes());
    for (slope, intercept) in [(f32::NAN, 0.0), (1.0, f32::INFINITY)] {
        float(&mut b, 112, slope, true);
        float(&mut b, 116, intercept, true);
        assert!(import_nifti_bytes(&b, Default::default()).is_err());
    }
}

#[test]
fn accepts_singleton_extra_dimensions_but_never_selects_first_timepoint() {
    let mut b = fixture(true, 2, 8, &[1, 2]);
    for rank in 2..=7 {
        short(&mut b, 40, rank, true);
        let imported = import(&b);
        assert_eq!(imported.volume.dimensions(), (2, 1, 1));
        assert_eq!(imported.is_2d, rank == 2);
    }
    for axis in 4..=7 {
        short(&mut b, 40 + axis * 2, 2, true);
        assert!(import_nifti_bytes(&b, Default::default()).is_err());
        short(&mut b, 40 + axis * 2, 1, true);
    }
}

#[test]
fn normalizes_small_offsets_and_accepts_unaligned_offsets() {
    for offset in [-10.0, 0.0, 348.0, 351.0, 352.0] {
        let mut b = fixture(true, 2, 8, &[1, 2]);
        float(&mut b, 108, offset, true);
        assert_eq!(import(&b).volume.voxel_data(), [1, 2]);
    }
    let mut b = fixture(true, 2, 8, &[1, 2]);
    b.insert(352, 0);
    float(&mut b, 108, 353.75, true);
    assert_eq!(import(&b).volume.voxel_data(), [1, 2]);
    for offset in [f32::NAN, f32::INFINITY, f32::MAX] {
        float(&mut b, 108, offset, true);
        assert!(import_nifti_bytes(&b, Default::default()).is_err());
    }
}

#[test]
fn rejects_truncation_at_every_byte_and_invalid_boundary_fields() {
    let b = fixture(true, 4, 16, &[0; 4]);
    for cut in 0..b.len() {
        assert!(
            import_nifti_bytes(&b[..cut], Default::default()).is_err(),
            "cut {cut}"
        );
        assert!(
            NiftiParser::parse_bytes(&b[..cut]).is_err(),
            "legacy cut {cut}"
        );
    }
    for (offset, value) in [(40, 0), (40, 8), (42, 0), (42, -1), (72, 32), (70, 32)] {
        let mut invalid = b.clone();
        short(&mut invalid, offset, value, true);
        assert!(import_nifti_bytes(&invalid, Default::default()).is_err());
    }
    let mut pair = b.clone();
    pair[344..348].copy_from_slice(b"ni1\0");
    assert!(import_nifti_bytes(&pair, Default::default()).is_err());
}

#[test]
fn gzip_and_all_three_limits_are_enforced_at_the_boundary() {
    let b = fixture(true, 2, 8, &[1, 2]);
    let gz = gzip(&b);
    assert_eq!(import(&gz).volume.voxel_data(), [1, 2]);
    for options in [
        NiftiImportOptions {
            max_input_bytes: gz.len() - 1,
            ..Default::default()
        },
        NiftiImportOptions {
            max_decompressed_bytes: b.len() - 1,
            ..Default::default()
        },
        NiftiImportOptions {
            max_voxel_bytes: 3,
            ..Default::default()
        },
    ] {
        assert!(import_nifti_bytes(&gz, options).is_err());
    }
    assert!(import_nifti_bytes(
        &gz,
        NiftiImportOptions {
            max_input_bytes: gz.len(),
            max_decompressed_bytes: b.len(),
            max_voxel_bytes: 4
        }
    )
    .is_ok());
    assert!(import_nifti_bytes(&gz[..gz.len() - 4], Default::default()).is_err());
    let bomb = gzip(&vec![0; 100_000]);
    assert!(import_nifti_bytes(
        &bomb,
        NiftiImportOptions {
            max_decompressed_bytes: 1000,
            ..Default::default()
        }
    )
    .is_err());
}

#[test]
fn sform_scales_once_converts_ras_to_lps_and_preserves_provenance() {
    let mut b = fixture(true, 2, 8, &[1, 2]);
    let ras = Mat4::from_translation(Vec3::new(10.0, 20.0, 30.0))
        * Mat4::from_rotation_z(0.3)
        * Mat4::from_scale(Vec3::new(2.0, -3.0, 4.0));
    sform(&mut b, 1, ras);
    // Deliberately disagreeing pixdim must not scale an sform a second time.
    float(&mut b, 80, 17.0, true);
    let imported = import(&b);
    let expected = Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0)) * ras;
    assert!(imported.volume.base().matrix.abs_diff_eq(expected, 1e-5));
    assert_eq!(
        imported.geometry.reference_space,
        NiftiReferenceSpace::Scanner
    );
    assert_eq!(imported.geometry.transforms.sform, Some(ras));
    let metadata = NiftiParser::parse_metadata_only(&b).unwrap();
    let p = Vec3::new(0.7, 0.2, 0.4);
    let world = expected.transform_point3(p);
    assert!(Vec3::from_array(metadata.voxel_to_world(p.to_array())).abs_diff_eq(world, 1e-5));
    assert!(
        Vec3::from_array(metadata.try_world_to_voxel(world.to_array()).unwrap())
            .abs_diff_eq(p, 1e-5)
    );
    assert!(imported
        .geometry
        .world_to_voxel
        .transform_point3(world)
        .abs_diff_eq(p, 1e-5));
}

#[test]
fn qform_handles_rotation_qfac_reflection_and_scanner_preference() {
    let mut b = fixture(true, 2, 8, &[1, 2]);
    short(&mut b, 252, 1, true);
    float(&mut b, 76, -1.0, true);
    float(&mut b, 264, std::f32::consts::FRAC_1_SQRT_2, true);
    float(&mut b, 268, 10.0, true);
    float(&mut b, 272, 20.0, true);
    float(&mut b, 276, 30.0, true);
    float(&mut b, 80, 2.0, true);
    let q = import(&b);
    assert!(q
        .volume
        .base()
        .matrix
        .transform_point3(Vec3::new(1.0, 0.0, 1.0))
        .abs_diff_eq(Vec3::new(-10.0, -22.0, 29.0), 1e-4));
    sform(&mut b, 4, Mat4::IDENTITY);
    let selected = import(&b);
    assert!(selected
        .volume
        .base()
        .matrix
        .abs_diff_eq(q.volume.base().matrix, 1e-5));
    assert_eq!(selected.geometry.transforms.sform_code, 4);
    assert_eq!(selected.geometry.transforms.qform_code, 1);
    short(&mut b, 254, 1, true);
    assert!(import_nifti_bytes(&b, Default::default()).is_err());
}

#[test]
fn atlas_space_and_unknown_units_are_never_promoted_to_scanner_geometry() {
    let mut b = fixture(true, 2, 8, &[1, 2]);
    sform(&mut b, 4, Mat4::IDENTITY);
    let imported = import(&b);
    assert_eq!(
        imported.geometry.reference_space,
        NiftiReferenceSpace::Mni152
    );
    for (units, factor) in [(1, 1000.0), (2, 1.0), (3, 0.001)] {
        b[123] = units;
        assert!((import(&b).volume.voxel_spacing().0 - factor).abs() < 1e-6);
    }
    b[123] = 0;
    let unknown = import(&b);
    assert!(!unknown.geometry.units_known);
    assert!(!unknown.diagnostics.warnings.is_empty());
    short(&mut b, 254, 0, true);
    assert_eq!(
        import(&b).geometry.reference_space,
        NiftiReferenceSpace::Unknown
    );
}

#[test]
fn shear_has_a_true_metadata_inverse_but_is_rejected_for_rendering() {
    let mut b = fixture(true, 2, 8, &[1, 2]);
    let mut affine = Mat4::IDENTITY;
    affine.y_axis.x = 0.4;
    sform(&mut b, 1, affine);
    let metadata = NiftiParser::parse_metadata_only(&b).unwrap();
    let voxel = [0.2, 0.7, 0.3];
    let world = metadata.voxel_to_world(voxel);
    assert!(
        Vec3::from_array(metadata.try_world_to_voxel(world).unwrap())
            .abs_diff_eq(Vec3::from_array(voxel), 1e-5)
    );
    assert!(import_nifti_bytes(&b, Default::default())
        .unwrap_err()
        .to_string()
        .contains("shear"));
    affine.x_axis = Vec3::ZERO.extend(0.0);
    sform(&mut b, 1, affine);
    assert!(NiftiParser::parse_metadata_only(&b).is_err());
    for scale in [
        Vec3::new(f32::INFINITY, 1.0, 1.0),
        Vec3::new(1e30, 1e-30, 1.0),
    ] {
        sform(&mut b, 1, Mat4::from_scale(scale));
        assert!(import_nifti_bytes(&b, Default::default()).is_err());
    }
    let mut singular = metadata;
    singular.orientation = [[0.0; 3]; 3];
    assert!(singular.try_world_to_voxel([0.0; 3]).is_err());
    assert!(singular.world_to_voxel([0.0; 3]).iter().all(|v| v.is_nan()));
}

#[test]
fn native_path_and_bytes_produce_identical_volumes() {
    use kepler_wgpu::data::medical_imaging::open_nifti;
    let bytes = gzip(&fixture(true, 256, 8, &[128, 127]));
    let path = std::env::temp_dir().join(format!("kepler-import-{}.nii.gz", std::process::id()));
    std::fs::write(&path, &bytes).unwrap();
    let opened = open_nifti(&path, Default::default());
    std::fs::remove_file(path).unwrap();
    let opened = opened.unwrap();
    assert_eq!(
        opened.volume.voxel_data(),
        import(&bytes).volume.voxel_data()
    );
    assert_eq!(
        opened.volume.base().matrix,
        import(&bytes).volume.base().matrix
    );
}

#[test]
fn rectangular_volume_voxel_access_uses_x_fastest_layout() {
    let mut b = fixture(true, 2, 8, &[0, 1, 2, 3, 4, 5]);
    short(&mut b, 42, 3, true);
    short(&mut b, 44, 2, true);
    let imported = import(&b);
    assert_eq!(imported.volume.get_voxel(2, 1, 0), Some(5));
    assert_eq!(imported.volume.get_voxel(3, 0, 0), None);
}

#[test]
fn big_endian_qform_geometry_matches_little_endian() {
    let mut le = fixture(true, 2, 8, &[1, 2]);
    let mut be = fixture(false, 2, 8, &[1, 2]);
    for (bytes, little) in [(&mut le, true), (&mut be, false)] {
        short(bytes, 252, 1, little);
        float(bytes, 76, -1.0, little);
        float(bytes, 264, 0.5, little);
        float(bytes, 268, 123.0, little);
        float(bytes, 272, -55.0, little);
        float(bytes, 276, 37.0, little);
    }
    assert!(import(&le)
        .volume
        .base()
        .matrix
        .abs_diff_eq(import(&be).volume.base().matrix, 1e-6));
}

#[test]
fn metaimage_embedded_and_detached_paths_keep_their_existing_contract() {
    use kepler_wgpu::data::medical_imaging::MetaimageParser;
    let common = "ObjectType = Image\nNDims = 3\nBinaryData = True\nBinaryDataByteOrderMSB = False\nCompressedData = False\nTransformMatrix = 0 1 0 -1 0 0 0 0 1\nOffset = 10 20 30\nElementSpacing = 0.5 0.75 2\nDimSize = 2 2 1\nElementType = MET_SHORT\n";
    let payload: Vec<_> = [-100_i16, 0, 100, 200]
        .into_iter()
        .flat_map(i16::to_le_bytes)
        .collect();
    let mut embedded = format!("{common}ElementDataFile = LOCAL\n").into_bytes();
    embedded.extend_from_slice(&payload);
    let detached = format!("{common}ElementDataFile = image.raw\n");
    for parsed in [
        MetaimageParser::parse_bytes(&embedded, None),
        MetaimageParser::parse_bytes(detached.as_bytes(), Some(&payload)),
    ] {
        let parsed = parsed.unwrap();
        assert_eq!(parsed.metadata.dimensions, [2, 2, 1]);
        assert_eq!(parsed.metadata.spacing, [0.5, 0.75, 2.0]);
        assert_eq!(parsed.pixel_data.as_bytes(), payload);
        let voxel = [0.25, 0.75, 0.0];
        let world = parsed.metadata.voxel_to_world(voxel);
        assert!(
            Vec3::from_array(parsed.metadata.try_world_to_voxel(world).unwrap())
                .abs_diff_eq(Vec3::from_array(voxel), 1e-5)
        );
    }
}

#[test]
fn concatenated_gzip_members_share_one_validated_payload() {
    let b = fixture(true, 2, 8, &[1, 2]);
    let mut compressed = gzip(&b[..352]);
    compressed.extend(gzip(&b[352..]));
    assert_eq!(import(&compressed).volume.voxel_data(), [1, 2]);
}
