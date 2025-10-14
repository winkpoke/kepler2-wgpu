use kepler_wgpu::rendering::mip::MipUniforms;
use std::mem::offset_of;

#[test]
fn test_mip_uniforms_size() {
    let size = std::mem::size_of::<MipUniforms>();
    println!("MipUniforms size: {} bytes", size);
       
    // Check if size matches what WGPU expects
    println!("Expected size: 176 bytes, Actual size: {} bytes", size);
}