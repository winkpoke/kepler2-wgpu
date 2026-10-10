//! Shader files for the rendering system
//!
//! This module contains all WGSL and GLSL shader files used by the rendering pipeline.
//! The shaders are embedded as string constants for easy access by the rendering system.

/// Shared RG8/half-float storage decoder used by every volume shader.
pub const VOLUME_DECODE_WGSL: &str = include_str!("volume_decode.wgsl");

/// Prepends the canonical volume decoder to a shader body.
pub fn volume_shader_source(body: &str) -> String {
    format!("{VOLUME_DECODE_WGSL}\n{body}")
}
