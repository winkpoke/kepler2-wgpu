//! Native-only compute smoke test: `Vec<f32>` → GPU storage buffers → WGSL
//! compute → readback → `Vec<f32>`.
//!
//! This validates the `Axum → ServerState → GpuState → wgpu 30 → compute → GPU →
//! CPU` path **without** touching `CTVolume` / `DICOM` / `MPR` / volume rendering.
//! It is intentionally the smallest possible GPU workload.

use std::sync::mpsc;

use anyhow::{bail, Result};

use crate::gpu::native::GpuContext;

const SHADER: &str = include_str!("./shader.wgsl");

/// Pass-through compute used by the volume upload smoke test.
///
/// `wgpu` 30 / WGSL has no `i16` storage type, so the voxel bytes are bound as
/// `array<u32>`; a pass-through copy proves the upload -> compute -> readback
/// path without any semantic transform (and without promoting i16 -> i32 on the
/// CPU, which would copy the whole buffer).
const VOLUME_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read> inp: array<u32>;
@group(0) @binding(1) var<storage, read_write> outp: array<u32>;

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i < arrayLength(&inp)) {
        outp[i] = inp[i];
    }
}
"#;

/// Multiplies every element of `input` by `2.0` using a GPU compute shader.
pub async fn run_double(ctx: &GpuContext, input: &[f32]) -> Result<Vec<f32>> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let device = ctx.device();
    let queue = ctx.queue();

    let input_bytes = bytemuck::cast_slice::<f32, u8>(input);
    let input_size = input_bytes.len() as u64;
    let output_size = (input.len() * std::mem::size_of::<f32>()) as u64;

    // Source buffer: STORAGE (read in shader) + COPY_DST (uploaded from CPU).
    let input_buf = device.create_buffer(&wgpu30::BufferDescriptor {
        label: Some("gpu-test-input"),
        size: input_size,
        usage: wgpu30::BufferUsages::STORAGE | wgpu30::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&input_buf, 0, input_bytes);

    // Output buffer: STORAGE (written in shader) + COPY_SRC (for readback copy).
    let output_buf = device.create_buffer(&wgpu30::BufferDescriptor {
        label: Some("gpu-test-output"),
        size: output_size,
        usage: wgpu30::BufferUsages::STORAGE | wgpu30::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let shader = device.create_shader_module(wgpu30::ShaderModuleDescriptor {
        label: Some("gpu-test-shader"),
        source: wgpu30::ShaderSource::Wgsl(SHADER.into()),
    });

    let pipeline = device.create_compute_pipeline(&wgpu30::ComputePipelineDescriptor {
        label: Some("gpu-test-pipeline"),
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: wgpu30::PipelineCompilationOptions::default(),
        cache: None,
    });

    let bind_group_layout = pipeline.get_bind_group_layout(0);
    let bind_group = device.create_bind_group(&wgpu30::BindGroupDescriptor {
        label: Some("gpu-test-bind-group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu30::BindGroupEntry {
                binding: 0,
                resource: input_buf.as_entire_binding(),
            },
            wgpu30::BindGroupEntry {
                binding: 1,
                resource: output_buf.as_entire_binding(),
            },
        ],
    });

    // Dispatch the compute pass.
    let mut encoder = device.create_command_encoder(&wgpu30::CommandEncoderDescriptor {
        label: Some("gpu-test-encoder"),
    });
    {
        let mut cpass = encoder.begin_compute_pass(&wgpu30::ComputePassDescriptor {
            label: Some("gpu-test-pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        let workgroups = input.len().div_ceil(64).max(1) as u32;
        cpass.dispatch_workgroups(workgroups, 1, 1);
    }
    queue.submit(Some(encoder.finish()));

    // Read back through a separate MAP_READ buffer (output buffer is STORAGE|COPY_SRC,
    // which cannot be mapped directly). Avoids any needless clones of the data.
    let read_buf = device.create_buffer(&wgpu30::BufferDescriptor {
        label: Some("gpu-test-readback"),
        size: output_size,
        usage: wgpu30::BufferUsages::COPY_DST | wgpu30::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut copy_encoder = device.create_command_encoder(&wgpu30::CommandEncoderDescriptor {
        label: Some("gpu-test-copy"),
    });
    copy_encoder.copy_buffer_to_buffer(&output_buf, 0, &read_buf, 0, output_size);
    queue.submit(Some(copy_encoder.finish()));

    // Mapping is async: the callback fires when `device.poll` runs on this thread.
    let (tx, rx) = mpsc::channel();
    read_buf.map_async(wgpu30::MapMode::Read, .., move |res| {
        let _ = tx.send(res);
    });
    device
        .poll(wgpu30::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .map_err(|e| anyhow::anyhow!("device poll failed: {e:?}"))?;

    match rx.recv() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => bail!("failed to map GPU readback buffer: {e:?}"),
        Err(_) => bail!("GPU readback channel closed unexpectedly"),
    }

    let view = read_buf
        .get_mapped_range(..)
        .map_err(|e| anyhow::anyhow!("get_mapped_range failed: {e:?}"))?;
    let result: Vec<f32> = bytemuck::cast_slice(&view).to_vec();
    drop(view);
    read_buf.unmap();

    Ok(result)
}

/// Result of a volume upload smoke test.
pub struct VolumeUploadResult {
    pub voxel_count: usize,
    pub sample: Vec<i16>,
}

/// Owning handle to the GPU buffers created for a volume upload.
///
/// Returned by the synchronous [`prepare_volume_upload`] so callers can release any
/// borrowed voxel data (and any lock guard) *before* the async compute/readback
/// stage. This keeps the caller's future `Send` — e.g. an Axum handler can hold a
/// `MutexGuard` only during the sync upload and drop it before `.await`ing.
pub struct VolumeUploadBuffers {
    input_buf: wgpu30::Buffer,
    output_buf: wgpu30::Buffer,
    padded: u64,
    byte_len: usize,
    sample_bytes: u64,
}

/// Synchronously uploads voxel bytes to the GPU and returns an owning handle.
///
/// Takes a borrowed `&[i16]` — the caller's `Vec<i16>` is never cloned. The bytes
/// are uploaded through a zero-copy `cast_slice` view into a storage buffer. This
/// function is synchronous so it is safe to call while holding a lock guard; drop
/// the guard before the async [`run_volume_compute`] stage.
pub fn prepare_volume_upload(ctx: &GpuContext, voxels: &[i16]) -> Result<VolumeUploadBuffers> {
    if voxels.is_empty() {
        bail!("cannot upload an empty volume");
    }

    let device = ctx.device();
    let queue = ctx.queue();

    // Zero-copy view of the i16 slice as raw bytes. No allocation, no clone.
    let bytes = bytemuck::cast_slice::<i16, u8>(voxels);
    // WGSL storage arrays are `array<u32>`, so round the byte length up to a
    // multiple of 4. The trailing padding bytes stay zero (wgpu zero-inits).
    let padded = ((bytes.len() + 3) & !3) as u64;
    let sample_bytes = std::cmp::min(bytes.len(), 32) as u64;

    let input_buf = device.create_buffer(&wgpu30::BufferDescriptor {
        label: Some("gpu-volume-input"),
        size: padded,
        usage: wgpu30::BufferUsages::STORAGE | wgpu30::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&input_buf, 0, bytes);

    let output_buf = device.create_buffer(&wgpu30::BufferDescriptor {
        label: Some("gpu-volume-output"),
        size: padded,
        usage: wgpu30::BufferUsages::STORAGE | wgpu30::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    Ok(VolumeUploadBuffers { input_buf, output_buf, padded, byte_len: bytes.len(), sample_bytes })
}

/// Async compute + readback stage for a prepared volume upload.
///
/// Runs the voxel bytes through a pass-through compute shader and reads back only a
/// bounded prefix (`min(len, 16)` voxels), so this stays cheap even for a full CT
/// volume. Returns the voxel count plus that sample.
pub async fn run_volume_compute(ctx: &GpuContext, bufs: VolumeUploadBuffers) -> Result<VolumeUploadResult> {
    let device = ctx.device();
    let queue = ctx.queue();
    let VolumeUploadBuffers { input_buf, output_buf, padded, byte_len, sample_bytes } = bufs;
    let voxel_count = byte_len / 2;

    let shader = device.create_shader_module(wgpu30::ShaderModuleDescriptor {
        label: Some("gpu-volume-shader"),
        source: wgpu30::ShaderSource::Wgsl(VOLUME_SHADER.into()),
    });

    let pipeline = device.create_compute_pipeline(&wgpu30::ComputePipelineDescriptor {
        label: Some("gpu-volume-pipeline"),
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: wgpu30::PipelineCompilationOptions::default(),
        cache: None,
    });

    let bind_group = device.create_bind_group(&wgpu30::BindGroupDescriptor {
        label: Some("gpu-volume-bind-group"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu30::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
            wgpu30::BindGroupEntry { binding: 1, resource: output_buf.as_entire_binding() },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu30::CommandEncoderDescriptor {
        label: Some("gpu-volume-encoder"),
    });
    {
        let mut cpass = encoder.begin_compute_pass(&wgpu30::ComputePassDescriptor {
            label: Some("gpu-volume-pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        let workgroups = (padded as usize / 4).div_ceil(64).max(1) as u32;
        cpass.dispatch_workgroups(workgroups, 1, 1);
    }
    queue.submit(Some(encoder.finish()));

    let read_buf = device.create_buffer(&wgpu30::BufferDescriptor {
        label: Some("gpu-volume-readback"),
        size: sample_bytes,
        usage: wgpu30::BufferUsages::COPY_DST | wgpu30::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut copy_encoder = device.create_command_encoder(&wgpu30::CommandEncoderDescriptor {
        label: Some("gpu-volume-copy"),
    });
    copy_encoder.copy_buffer_to_buffer(&output_buf, 0, &read_buf, 0, sample_bytes);
    queue.submit(Some(copy_encoder.finish()));

    let (tx, rx) = std::sync::mpsc::channel();
    read_buf.map_async(wgpu30::MapMode::Read, .., move |res| {
        let _ = tx.send(res);
    });
    device
        .poll(wgpu30::PollType::Wait { submission_index: None, timeout: None })
        .map_err(|e| anyhow::anyhow!("device poll failed: {e:?}"))?;

    match rx.recv() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => bail!("failed to map GPU readback buffer: {e:?}"),
        Err(_) => bail!("GPU readback channel closed unexpectedly"),
    }

    let view = read_buf
        .get_mapped_range(..)
        .map_err(|e| anyhow::anyhow!("get_mapped_range failed: {e:?}"))?;
    let read_i16: &[i16] = bytemuck::cast_slice(&view);
    let sample_len = std::cmp::min(voxel_count, 16);
    let sample: Vec<i16> = read_i16[..sample_len].to_vec();
    drop(view);
    read_buf.unmap();

    Ok(VolumeUploadResult { voxel_count, sample })
}

/// Convenience single-call API: upload + compute + readback.
///
/// Equivalent to [`prepare_volume_upload`] followed by [`run_volume_compute`]. Used by
/// callers that already own the voxel slice (e.g. tests); for an Axum handler that
/// only borrows the data behind a lock, prefer the two-step split so the lock guard
/// is not held across an `.await` (keeps the handler future `Send`).
pub async fn run_volume_upload(ctx: &GpuContext, voxels: &[i16]) -> Result<VolumeUploadResult> {
    let bufs = prepare_volume_upload(ctx, voxels)?;
    run_volume_compute(ctx, bufs).await
}

#[cfg(test)]
mod tests {
    use super::run_volume_upload;
    use crate::gpu::native::GpuContext;
    use tokio::test;

    // Requires a physical GPU with a wgpu 30 native backend. Run with:
    //   cargo test --lib -- --ignored gpu_volume_upload_roundtrip
    #[test]
    #[ignore = "requires a physical GPU (wgpu 30 native backend)"]
    async fn gpu_volume_upload_roundtrip() {
        let ctx = GpuContext::new().await.expect("a GPU must be available");
        let voxels: Vec<i16> = vec![-1024, 0, 32767, 1, 2, 3, 4, 5, 6, 7];
        let res = run_volume_upload(&ctx, &voxels).await.expect("upload");
        assert_eq!(res.voxel_count, voxels.len());
        assert_eq!(res.sample, voxels[..res.sample.len()].to_vec());
    }
}
