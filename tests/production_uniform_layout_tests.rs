//! Compare actual Rust offsets with Naga's layout of the production WGSL structs.
use kepler_wgpu::rendering::view::{
    mesh::mesh::VolumeUniforms, mip::MipUniforms, mpr::mpr_view_wgpu_impl::UniformsFrag,
    NeedleUniform, ObliquePlaneUniform,
};
use std::mem::{offset_of, size_of};

fn check(shader: &str, name: &str, size: usize, fields: &[(&str, usize)]) {
    let source = format!(
        "{}\n{}\n{}",
        include_str!("../src/rendering/shaders/common.wgsl"),
        include_str!("../src/rendering/shaders/volume_decode.wgsl"),
        shader
    );
    let module = naga::front::wgsl::parse_str(&source).expect("production WGSL parses");
    let ty = module
        .types
        .iter()
        .find(|(_, ty)| ty.name.as_deref() == Some(name))
        .unwrap()
        .1;
    let naga::TypeInner::Struct { members, span } = &ty.inner else {
        panic!("not a struct")
    };
    assert_eq!(*span as usize, size, "{name} size");
    assert_eq!(members.len(), fields.len(), "{name} fields");
    for member in members {
        let field = member.name.as_deref().unwrap();
        let expected = fields
            .iter()
            .find(|(n, _)| *n == field)
            .expect("every shader field has a Rust field");
        assert_eq!(member.offset as usize, expected.1, "{name}.{field}");
    }
}

#[test]
fn mpr_uniformsfrag_matches_production_wgsl() {
    check(
        include_str!("../src/rendering/shaders/mpr.wgsl"),
        "UniformsFrag",
        size_of::<UniformsFrag>(),
        &[
            ("window", offset_of!(UniformsFrag, window_width)),
            ("level", offset_of!(UniformsFrag, window_level)),
            ("slice", offset_of!(UniformsFrag, slice)),
            ("is_packed_rg8", offset_of!(UniformsFrag, is_packed_rg8)),
            ("bias", offset_of!(UniformsFrag, bias)),
            ("ball_enabled", offset_of!(UniformsFrag, ball_enabled)),
            ("seg_jet", offset_of!(UniformsFrag, seg_jet)),
            ("aliasing", offset_of!(UniformsFrag, aliasing)),
            ("mat", offset_of!(UniformsFrag, mat)),
            ("needle_count", offset_of!(UniformsFrag, needle_count)),
            ("needle_enabled", offset_of!(UniformsFrag, needle_enabled)),
            ("seg_enabled", offset_of!(UniformsFrag, seg_enabled)),
            ("seg_alpha", offset_of!(UniformsFrag, seg_alpha)),
            ("needles", offset_of!(UniformsFrag, needles)),
            ("label_colors", offset_of!(UniformsFrag, label_colors)),
            (
                "label_visibility",
                offset_of!(UniformsFrag, label_visibility),
            ),
            ("balls", offset_of!(UniformsFrag, balls)),
        ],
    );
}

#[test]
fn mip_mipuniforms_matches_production_wgsl() {
    check(
        include_str!("../src/rendering/shaders/mip.wgsl"),
        "MipUniforms",
        size_of::<MipUniforms>(),
        &[
            ("ray_step_size", offset_of!(MipUniforms, ray_step_size)),
            ("max_steps", offset_of!(MipUniforms, max_steps)),
            ("is_packed_rg8", offset_of!(MipUniforms, is_packed_rg8)),
            ("bias", offset_of!(MipUniforms, bias)),
            ("window", offset_of!(MipUniforms, window)),
            ("level", offset_of!(MipUniforms, level)),
            ("pan_x", offset_of!(MipUniforms, pan_x)),
            ("pan_y", offset_of!(MipUniforms, pan_y)),
            ("scale", offset_of!(MipUniforms, scale)),
            ("mode", offset_of!(MipUniforms, mip_mode)),
            ("lower_threshold", offset_of!(MipUniforms, lower_threshold)),
            ("upper_threshold", offset_of!(MipUniforms, upper_threshold)),
            ("needle_enabled", offset_of!(MipUniforms, needle_enabled)),
            ("needle_count", offset_of!(MipUniforms, needle_count)),
            ("_pad", offset_of!(MipUniforms, _pad)),
            ("ball_enabled", offset_of!(MipUniforms, ball_enabled)),
            ("balls", offset_of!(MipUniforms, balls)),
            ("needles", offset_of!(MipUniforms, needles)),
            ("rotation", offset_of!(MipUniforms, rotation)),
        ],
    );
}

#[test]
fn volume_volumeuniforms_matches_production_wgsl() {
    check(
        include_str!("../src/rendering/shaders/volume.wgsl"),
        "VolumeUniforms",
        size_of::<VolumeUniforms>(),
        &[
            ("ray_step_size", offset_of!(VolumeUniforms, ray_step_size)),
            ("max_steps", offset_of!(VolumeUniforms, max_steps)),
            ("is_packed_rg8", offset_of!(VolumeUniforms, is_packed_rg8)),
            ("bias", offset_of!(VolumeUniforms, bias)),
            ("roi_min", offset_of!(VolumeUniforms, roi_min)),
            ("window", offset_of!(VolumeUniforms, window)),
            ("roi_max", offset_of!(VolumeUniforms, roi_max)),
            ("level", offset_of!(VolumeUniforms, level)),
            ("vol_dims", offset_of!(VolumeUniforms, vol_dims)),
            ("opacity", offset_of!(VolumeUniforms, opacity)),
            ("view_proj", offset_of!(VolumeUniforms, view_proj)),
            ("inv_view_proj", offset_of!(VolumeUniforms, inv_view_proj)),
            (
                "camera_position",
                offset_of!(VolumeUniforms, camera_position),
            ),
            ("ball_enabled", offset_of!(VolumeUniforms, ball_enabled)),
            ("balls", offset_of!(VolumeUniforms, balls)),
            ("volume_scale", offset_of!(VolumeUniforms, volume_scale)),
            (
                "_volume_scale_pad",
                offset_of!(VolumeUniforms, _volume_scale_pad),
            ),
            ("light_dir", offset_of!(VolumeUniforms, light_dir)),
            ("aspect_ratio", offset_of!(VolumeUniforms, aspect_ratio)),
            ("needle_count", offset_of!(VolumeUniforms, needle_count)),
            ("needle_enabled", offset_of!(VolumeUniforms, needle_enabled)),
            ("needle_index", offset_of!(VolumeUniforms, needle_index)),
            (
                "plane_rotation_angle",
                offset_of!(VolumeUniforms, plane_rotation_angle),
            ),
            ("oblique_planes", offset_of!(VolumeUniforms, oblique_planes)),
            ("needles", offset_of!(VolumeUniforms, needles)),
        ],
    );
}

#[test]
fn volume_needleuniform_matches_production_wgsl() {
    check(
        include_str!("../src/rendering/shaders/volume.wgsl"),
        "NeedleUniform",
        size_of::<NeedleUniform>(),
        &[
            ("entry", offset_of!(NeedleUniform, entry)),
            ("radius", offset_of!(NeedleUniform, radius)),
            ("tip", offset_of!(NeedleUniform, tip)),
            ("id", offset_of!(NeedleUniform, id)),
            ("color", offset_of!(NeedleUniform, color)),
        ],
    );
}

#[test]
fn volume_obliqueplaneuniform_matches_production_wgsl() {
    check(
        include_str!("../src/rendering/shaders/volume.wgsl"),
        "ObliquePlane",
        size_of::<ObliquePlaneUniform>(),
        &[
            ("center", offset_of!(ObliquePlaneUniform, center)),
            ("visible", offset_of!(ObliquePlaneUniform, visible)),
            ("normal", offset_of!(ObliquePlaneUniform, normal)),
            ("plane_alpha", offset_of!(ObliquePlaneUniform, plane_alpha)),
        ],
    );
}
