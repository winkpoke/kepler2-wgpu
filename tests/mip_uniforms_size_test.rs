use kepler_wgpu::rendering::mip::MipUniforms;
use std::mem::offset_of;

#[test]
fn test_mip_uniforms_size() {
    let size = std::mem::size_of::<MipUniforms>();
    println!("MipUniforms size: {} bytes", size);
    
    // Print field offsets for debugging
    println!("camera_pos offset: {}", offset_of!(MipUniforms, camera_pos));
    println!("view_matrix offset: {}", offset_of!(MipUniforms, view_matrix));
    
    // Check if size matches what WGPU expects
    println!("Expected size: 176 bytes, Actual size: {} bytes", size);
}