// Uniform buffer for transformation matrices and camera data
struct CameraUniforms {
    view_matrix: mat4x4<f32>,
    projection_matrix: mat4x4<f32>,
    view_projection_matrix: mat4x4<f32>,
    camera_position: vec3<f32>,
    _padding: f32,
};

// Material properties for PBR rendering
struct MaterialProperties {
    albedo: vec3<f32>,
    metallic: f32,
    roughness: f32,
    ao: f32,
    emission: vec3<f32>,
    _padding: f32,
};

// Individual light source definition with proper 16-byte alignment
struct Light {
    position: vec4<f32>, // Changed to vec4 for better alignment (w component unused)
    color: vec4<f32>, // Changed to vec4 for better alignment (w = intensity)
    direction: vec4<f32>, // Changed to vec4 for better alignment (w = light_type)
    params: vec4<f32>, // range, inner_cone_angle, outer_cone_angle, padding
};

// Enhanced lighting system with multiple light sources
struct LightingUniforms {
    lights: array<Light, 8>, // Support up to 8 lights
    num_lights: u32,
    ambient_color: vec3<f32>,
    ambient_strength: f32,
};

// Uniform buffer for model transformation
struct ModelUniforms {
    model_matrix: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
};

// Bind group 0: Camera uniforms
@group(0) @binding(0) var<uniform> camera: CameraUniforms;

// Bind group 1: Lighting uniforms  
@group(1) @binding(0) var<uniform> lighting: LightingUniforms;

// Bind group 2: Model uniforms
@group(2) @binding(0) var<uniform> model: ModelUniforms;

// Bind group 3: Material uniforms
@group(3) @binding(0) var<uniform> material: MaterialProperties;

// Vertex shader output structure
struct VSOut {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

/// Vertex shader with proper MVP transformation and normal calculation
/// Transforms vertices from model space to clip space and prepares data for lighting
@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>
) -> VSOut {
    // Transform position to world space
    let world_position = (model.model_matrix * vec4<f32>(position, 1.0)).xyz;
    
    // Transform position to clip space using view-projection matrix
    let clip_position = camera.view_projection_matrix * vec4<f32>(world_position, 1.0);
    
    // Transform normal to world space (using normal matrix to handle non-uniform scaling)
    let world_normal = normalize((model.normal_matrix * vec4<f32>(normal, 0.0)).xyz);
    
    return VSOut(
        clip_position,
        world_position,
        world_normal,
        uv
    );
}

/// PBR utility functions for realistic lighting calculations

/// Distribution function (GGX/Trowbridge-Reitz)
fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let denom = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (3.14159265 * denom * denom);
}

/// Geometry function (Smith's method)
fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    
    let ggx1 = n_dot_v / (n_dot_v * (1.0 - k) + k);
    let ggx2 = n_dot_l / (n_dot_l * (1.0 - k) + k);
    
    return ggx1 * ggx2;
}

/// Fresnel function (Schlick approximation)
fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

/// Calculate lighting contribution from a single light source
fn calculate_light_contribution(
    light: Light,
    world_pos: vec3<f32>,
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    albedo: vec3<f32>,
    metallic: f32,
    roughness: f32,
    f0: vec3<f32>
) -> vec3<f32> {
    var light_dir: vec3<f32>;
    var attenuation = 1.0;
    
    // Calculate light direction and attenuation based on light type
    let light_type = u32(light.direction.w); // light_type stored in direction.w
    if (light_type == 0u) { // Directional light
        light_dir = normalize(-light.direction.xyz);
    } else if (light_type == 1u) { // Point light
        light_dir = normalize(light.position.xyz - world_pos);
        let distance = length(light.position.xyz - world_pos);
        attenuation = 1.0 / (1.0 + 0.09 * distance + 0.032 * distance * distance);
        attenuation = min(attenuation, 1.0 / (distance * distance / (light.params.x * light.params.x))); // range stored in params.x
    } else if (light_type == 2u) { // Spot light
        light_dir = normalize(light.position.xyz - world_pos);
        let distance = length(light.position.xyz - world_pos);
        let theta = dot(light_dir, normalize(-light.direction.xyz));
        let epsilon = cos(light.params.y) - cos(light.params.z); // inner_cone_angle in params.y, outer_cone_angle in params.z
        let intensity = clamp((theta - cos(light.params.z)) / epsilon, 0.0, 1.0);
        attenuation = intensity / (1.0 + 0.09 * distance + 0.032 * distance * distance);
    }
    
    let half_dir = normalize(view_dir + light_dir);
    
    // Calculate angles
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let n_dot_v = max(dot(normal, view_dir), 0.0);
    let n_dot_h = max(dot(normal, half_dir), 0.0);
    let v_dot_h = max(dot(view_dir, half_dir), 0.0);
    
    // Early exit if light doesn't contribute
    if (n_dot_l <= 0.0) {
        return vec3<f32>(0.0);
    }
    
    // Calculate BRDF components
    let ndf = distribution_ggx(n_dot_h, roughness);
    let g = geometry_smith(n_dot_v, n_dot_l, roughness);
    let f = fresnel_schlick(v_dot_h, f0);
    
    // Calculate specular and diffuse components
    let numerator = ndf * g * f;
    let denominator = 4.0 * n_dot_v * n_dot_l + 0.0001; // Prevent division by zero
    let specular = numerator / denominator;
    
    let ks = f;
    let kd = (vec3<f32>(1.0) - ks) * (1.0 - metallic);
    
    let radiance = light.color.xyz * light.color.w * attenuation; // color.xyz = RGB, color.w = intensity
    
    return (kd * albedo / 3.14159265 + specular) * radiance * n_dot_l;
}

/// Advanced PBR fragment shader with multiple light sources
/// Implements physically based rendering with proper BRDF calculations
@fragment
fn fs_main(input: VSOut) -> @location(0) vec4<f32> {
    // Material properties
    let albedo = material.albedo;
    let metallic = material.metallic;
    let roughness = max(material.roughness, 0.04); // Prevent division by zero
    let ao = material.ao;
    
    // Normalize interpolated normal
    let normal = normalize(input.world_normal);
    
    // Calculate view direction
    let view_dir = normalize(camera.camera_position - input.world_position);
    
    // Calculate base reflectance (F0)
    let f0 = mix(vec3<f32>(0.04), albedo, metallic);
    
    // Accumulate lighting from all light sources
    var lo = vec3<f32>(0.0);
    for (var i = 0u; i < lighting.num_lights && i < 8u; i = i + 1u) {
        lo = lo + calculate_light_contribution(
            lighting.lights[i],
            input.world_position,
            normal,
            view_dir,
            albedo,
            metallic,
            roughness,
            f0
        );
    }
    
    // Add ambient lighting
    let ambient = lighting.ambient_color * lighting.ambient_strength * albedo * ao;
    let final_color = ambient + lo + material.emission;
    
    // Simple tone mapping (Reinhard)
    let mapped_color = final_color / (final_color + vec3<f32>(1.0));
    
    // Gamma correction
    let gamma_corrected = pow(mapped_color, vec3<f32>(1.0 / 2.2));
    
    return vec4<f32>(gamma_corrected, 1.0);
}