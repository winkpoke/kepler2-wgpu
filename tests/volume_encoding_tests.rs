use kepler_wgpu::application::app_model::AppModel;
use kepler_wgpu::core::coord::Base;
use kepler_wgpu::data::ct_volume::CTVolume;
use kepler_wgpu::{pack_i16_rg8, unpack_rg8_i16, VolumeEncoding};

fn model_with_voxels(voxels: Vec<i16>, use_float: bool) -> AppModel {
    let mut model = AppModel::new(use_float);
    model
        .load_volume(CTVolume::new(
            (voxels.len(), 1, 1),
            (1.0, 1.0, 1.0),
            voxels,
            Base::default(),
        ))
        .unwrap();
    model
}

#[test]
fn packed_upload_preserves_values_below_old_ct_bound() {
    let model = model_with_voxels(vec![-1200], false);
    let (bytes, encoding) = model.get_volume_render_data().unwrap();
    assert_eq!(encoding, VolumeEncoding::PackedI16Rg8);
    let decoded = f32::from(u16::from_le_bytes([bytes[0], bytes[1]])) - encoding.decode_bias();
    assert_eq!(decoded, -1200.0);
}

#[test]
fn packed_i16_rg8_roundtrips_entire_domain() {
    for value in i16::MIN..=i16::MAX {
        assert_eq!(unpack_rg8_i16(pack_i16_rg8(value)), value);
    }
}

#[test]
fn important_ct_values_have_canonical_little_endian_encoding() {
    for (value, encoded) in [
        (i16::MIN, 0u16),
        (-2000, 30768),
        (-1200, 31568),
        (-1100, 31668),
        (-1024, 31744),
        (-1000, 31768),
        (0, 32768),
        (1000, 33768),
        (3000, 35768),
        (i16::MAX, 65535),
    ] {
        assert_eq!(pack_i16_rg8(value), encoded.to_le_bytes());
        assert_eq!(unpack_rg8_i16(encoded.to_le_bytes()), value);
    }
}

#[test]
fn app_model_upload_roundtrips_entire_domain_without_overflow() {
    let voxels: Vec<i16> = (i16::MIN..=i16::MAX).collect();
    let model = model_with_voxels(voxels.clone(), false);
    let (bytes, encoding) = model.get_volume_render_data().unwrap();
    assert_eq!(bytes, encoding.encode_volume(&voxels));
    assert_eq!(encoding.texture_format(), wgpu::TextureFormat::Rg8Unorm);
    assert_eq!(bytes.len(), voxels.len() * 2);
    for (&expected, bytes) in voxels.iter().zip(bytes.chunks_exact(2)) {
        assert_eq!(unpack_rg8_i16([bytes[0], bytes[1]]), expected);
    }
}

#[test]
fn half_float_upload_keeps_existing_rounding_semantics() {
    let voxels = vec![i16::MIN, -1200, -1024, 0, 1000, 3000, 32767];
    let model = model_with_voxels(voxels.clone(), true);
    let (bytes, encoding) = model.get_volume_render_data().unwrap();
    assert_eq!(encoding, VolumeEncoding::R16Float);
    assert_eq!(encoding.texture_format(), wgpu::TextureFormat::R16Float);
    assert_eq!(encoding.decode_bias(), 0.0);
    for (&value, bytes) in voxels.iter().zip(bytes.chunks_exact(2)) {
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            half::f16::from_f32(f32::from(value)).to_bits()
        );
    }
    assert_eq!(half::f16::from_f32(32767.0).to_f32(), 32768.0);
}

#[test]
fn encodings_handle_empty_volumes() {
    for encoding in [VolumeEncoding::PackedI16Rg8, VolumeEncoding::R16Float] {
        assert!(encoding.encode_volume(&[]).is_empty());
    }
}

#[test]
fn missing_volume_returns_an_error() {
    assert!(AppModel::new(false).get_volume_render_data().is_err());
}

#[test]
fn packed_channel_interpolation_preserves_signed_values_and_byte_carries() {
    let encoding = VolumeEncoding::PackedI16Rg8;
    for (a, b) in [
        (-1, 0),
        (-1200, -1000),
        (-32513, -32512),
        (i16::MIN, i16::MAX),
    ] {
        let a_bytes = pack_i16_rg8(a);
        let b_bytes = pack_i16_rg8(b);
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let lo = f32::from(a_bytes[0]) * (1.0 - t) + f32::from(b_bytes[0]) * t;
            let hi = f32::from(a_bytes[1]) * (1.0 - t) + f32::from(b_bytes[1]) * t;
            let decoded = lo + hi * 256.0 - encoding.decode_bias();
            let expected = f32::from(a) * (1.0 - t) + f32::from(b) * t;
            assert_eq!(decoded, expected);
        }
    }
}

// Native Naga currently disables fragment derivative-uniformity requirements,
// so successful native pipeline creation alone cannot catch WebGPU's rejection.
// Check the production shader structure: MPR samples before divergent bounds,
// while MIP helpers take footprint-based LOD computed before per-fragment control.
// Fixed LOD is not equivalent: minification and magnification filters differ.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn production_volume_shaders_preserve_derivatives_outside_divergent_control_flow() {
    use kepler_wgpu::rendering::shaders::volume_shader_source;
    use wgpu::naga::{Expression, SampleLevel, Statement};

    fn assert_uniform_prefix(
        function: &wgpu::naga::Function,
        expression: wgpu::naga::Handle<Expression>,
    ) {
        let sample = function.body.iter().position(|statement| {
            matches!(statement, Statement::Emit(range) if range.clone().any(|h| h == expression))
        }).expect("Derivatives/sampling must be unconditional in the entry-point body");
        let control = function
            .body
            .iter()
            .position(|statement| {
                matches!(
                    statement,
                    Statement::If { .. }
                        | Statement::Switch { .. }
                        | Statement::Loop { .. }
                        | Statement::Return { .. }
                )
            })
            .unwrap();
        assert!(
            sample < control,
            "Derivatives must precede per-fragment bounds/returns"
        );
    }

    for (name, body) in [
        (
            "MPR",
            include_str!("../src/rendering/shaders/shader_tex.wgsl"),
        ),
        ("MIP", include_str!("../src/rendering/shaders/mip.wgsl")),
    ] {
        let source = volume_shader_source(body);
        let module = wgpu::naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|error| panic!("{name}: {}", error.emit_to_string(&source)));
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|error| panic!("{name}: {error:?}"));
        let mut samples = 0;
        let mut derivatives = 0;
        for function in module
            .functions
            .iter()
            .map(|(_, f)| f)
            .chain(module.entry_points.iter().map(|e| &e.function))
        {
            for (handle, expression) in function.expressions.iter() {
                if let Expression::ImageSample { level, .. } = expression {
                    samples += 1;
                    if name == "MPR" {
                        assert!(
                            matches!(level, SampleLevel::Auto),
                            "MPR must retain derivative-based filter selection"
                        );
                        assert_uniform_prefix(function, handle);
                    } else {
                        let SampleLevel::Exact(argument) = level else {
                            panic!("MIP rays need footprint-based explicit LOD, not implicit derivatives");
                        };
                        assert!(
                            matches!(
                                function.expressions[*argument],
                                Expression::FunctionArgument(_)
                            ),
                            "MIP LOD must come from the ray footprint, not fixed LOD zero"
                        );
                    }
                }
                if name == "MIP" && matches!(expression, Expression::Derivative { .. }) {
                    derivatives += 1;
                    assert_uniform_prefix(function, handle);
                }
            }
        }
        if name == "MIP" {
            assert_eq!(
                derivatives, 2,
                "Both ray footprint derivatives must be checked"
            );
        }
        assert!(
            samples > 0,
            "{name}: production volume sampling must be checked"
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gpu_packed_upload_roundtrips_entire_domain_with_shared_wgsl_decoder() {
    use kepler_wgpu::rendering::shaders::volume_shader_source;
    use kepler_wgpu::rendering::view::render_content::RenderContent;
    use wgpu::util::DeviceExt;

    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        force_fallback_adapter: false,
        power_preference: wgpu::PowerPreference::LowPower,
        apply_limit_buckets: false,
    }))
    .expect("Volume roundtrip GPU test requires an adapter; it must not silently skip");
    log::info!("Volume roundtrip adapter: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();

    // Validate both production shader modules, not just the compute harness.
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    for body in [
        include_str!("../src/rendering/shaders/shader_tex.wgsl"),
        include_str!("../src/rendering/shaders/mip.wgsl"),
    ] {
        let _shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Production volume shader validation"),
            source: wgpu::ShaderSource::Wgsl(volume_shader_source(body).into()),
        });
    }
    assert!(pollster::block_on(scope.pop()).is_none());
    let compute_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);

    let voxels: Vec<i16> = (i16::MIN..=i16::MAX).collect();
    let encoding = VolumeEncoding::PackedI16Rg8;
    let content = RenderContent::from_bytes(
        &device,
        &queue,
        &encoding.encode_volume(&voxels),
        "Roundtrip volume",
        256,
        256,
        1,
        encoding,
    )
    .unwrap();
    assert_eq!(content.texture_format, encoding.texture_format());
    let decode = content.decode_parameters();
    let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Volume decode parameters"),
        contents: bytemuck::cast_slice(&[decode.is_packed_flag as f32, decode.bias, 0.0, 0.0]),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let output_size = ((voxels.len() + 2) * std::mem::size_of::<f32>()) as u64;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Decoded voxels"),
        size: output_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Voxel readback"),
        size: output_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shared volume decoder roundtrip"),
        source: wgpu::ShaderSource::Wgsl(
            volume_shader_source(
                r#"
            @group(0) @binding(0) var volume: texture_3d<f32>;
            @group(0) @binding(1) var<uniform> params: vec4<f32>;
            @group(0) @binding(2) var<storage, read_write> output: array<f32>;
            @group(0) @binding(3) var linear_sampler: sampler;
            @compute @workgroup_size(64)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                let index = id.x;
                if (index < 65536u) {
                    let coords = vec3<i32>(i32(index % 256u), i32(index / 256u), 0);
                    let sampled = textureLoad(volume, coords, 0);
                    output[index] = decode_volume_sample(sampled, params.x, params.y);
                } else if (index < 65538u) {
                    // Midpoints across zero and a low-byte carry, respectively.
                    let coords = select(
                        vec3<f32>(0.5 / 256.0, 128.0 / 256.0, 0.5),
                        vec3<f32>(128.0 / 256.0, 0.5 / 256.0, 0.5),
                        index == 65537u
                    );
                    let sampled = textureSampleLevel(volume, linear_sampler, coords, 0.0);
                    output[index] = decode_volume_sample(sampled, params.x, params.y);
                }
            }
        "#,
            )
            .into(),
        ),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Volume decoder roundtrip"),
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Roundtrip resources"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&content.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: output.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(&linear_sampler),
            },
        ],
    });
    let mut commands = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = commands.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(1025, 1, 1);
    }
    commands.copy_buffer_to_buffer(&output, 0, &readback, 0, output_size);
    queue.submit(Some(commands.finish()));
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("Roundtrip GPU polling failed");
    assert!(pollster::block_on(compute_scope.pop()).is_none());
    rx.recv().unwrap().unwrap();
    let mapped = readback
        .slice(..)
        .get_mapped_range()
        .expect("Roundtrip mapped range unavailable");
    let decoded: &[f32] = bytemuck::cast_slice(&mapped);
    for (&expected, &actual) in voxels.iter().zip(decoded) {
        // RG8 storage is exact, but normalized sampling has small f32 error.
        // Recovering the discrete integer must still be exact for every voxel.
        assert_eq!(
            actual.round() as i32,
            i32::from(expected),
            "GPU decode failed for {expected}"
        );
        assert!((actual - f32::from(expected)).abs() < 0.01);
    }
    for (&actual, expected) in decoded[voxels.len()..].iter().zip([-128.0, -32640.5]) {
        assert!(
            (actual - expected).abs() < 0.01,
            "GPU interpolation: {actual} != {expected}"
        );
    }
    drop(mapped);
    readback.unmap();
}
