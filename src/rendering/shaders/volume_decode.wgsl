// Shared storage decoder for MPR and MIP. The offset is supplied by VolumeEncoding.
// Do not round the channels: the affine decoder must preserve hardware interpolation.
fn decode_volume_sample(sampled: vec4<f32>, is_packed_rg8: f32, storage_bias: f32) -> f32 {
    if (is_packed_rg8 > 0.5) {
        return (sampled.r + sampled.g * 256.0) * 255.0 - storage_bias;
    }
    return sampled.r;
}
