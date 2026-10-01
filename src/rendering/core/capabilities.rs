//! Device-local volume capabilities and browser limit negotiation.

use crate::core::error::KeplerError;

/// Rendering capabilities negotiated for one specific device.
#[derive(Debug, Clone, Copy)]
pub struct GpuCapabilities {
    backend: wgpu::Backend,
    r16float_filterable: bool,
    rg8_filterable: bool,
    max_3d_texture_dimension: u32,
}

impl GpuCapabilities {
    /// Builds the capability snapshot from the selected adapter and created device.
    pub fn from_device(
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
    ) -> Result<Self, KeplerError> {
        let enabled_features = device.features();
        Self::from_formats(
            adapter.get_info().backend,
            device.limits().max_texture_dimension_3d,
            device_format_features(
                wgpu::TextureFormat::R16Float,
                adapter.get_texture_format_features(wgpu::TextureFormat::R16Float),
                enabled_features,
            ),
            device_format_features(
                wgpu::TextureFormat::Rg8Unorm,
                adapter.get_texture_format_features(wgpu::TextureFormat::Rg8Unorm),
                enabled_features,
            ),
        )
    }

    /// Builds the capability snapshot from format features legal on the created device.
    pub fn from_formats(
        backend: wgpu::Backend,
        max_3d_texture_dimension: u32,
        r16float: wgpu::TextureFormatFeatures,
        rg8: wgpu::TextureFormatFeatures,
    ) -> Result<Self, KeplerError> {
        let capabilities = Self {
            backend,
            r16float_filterable: usable_volume_format(r16float),
            rg8_filterable: usable_volume_format(rg8),
            max_3d_texture_dimension,
        };
        if max_3d_texture_dimension == 0 {
            return Err(KeplerError::Graphics(format!(
                "Backend {backend:?} reports an unusable 3D texture limit of 0"
            )));
        }
        if !capabilities.r16float_filterable && !capabilities.rg8_filterable {
            return Err(KeplerError::Graphics(format!(
                "Backend {backend:?} supports neither filtered R16Float nor filtered Rg8Unorm volume textures"
            )));
        }
        Ok(capabilities)
    }

    pub fn backend(&self) -> wgpu::Backend {
        self.backend
    }

    pub fn r16float_filterable(&self) -> bool {
        self.r16float_filterable
    }

    pub fn rg8_filterable(&self) -> bool {
        self.rg8_filterable
    }

    pub fn max_3d_texture_dimension(&self) -> u32 {
        self.max_3d_texture_dimension
    }

    pub fn default_float_encoding(&self) -> bool {
        self.encoding_for_preference(true) == crate::data::volume_encoding::VolumeEncoding::R16Float
    }

    pub fn volume_encoding(&self) -> crate::data::volume_encoding::VolumeEncoding {
        self.encoding_for_preference(true)
    }

    /// Preserves a requested encoding when supported and selects the only usable one otherwise.
    pub fn encoding_for_preference(
        &self,
        prefer_float: bool,
    ) -> crate::data::volume_encoding::VolumeEncoding {
        use crate::data::volume_encoding::VolumeEncoding;
        if prefer_float && self.r16float_filterable || !self.rg8_filterable {
            VolumeEncoding::R16Float
        } else {
            VolumeEncoding::PackedI16Rg8
        }
    }

    pub fn supports_encoding(
        &self,
        encoding: crate::data::volume_encoding::VolumeEncoding,
    ) -> bool {
        match encoding {
            crate::data::volume_encoding::VolumeEncoding::R16Float => self.r16float_filterable,
            crate::data::volume_encoding::VolumeEncoding::PackedI16Rg8 => self.rg8_filterable,
        }
    }
}

/// Whether a format can support the volume texture's sampled, uploaded, filtered use.
fn usable_volume_format(features: wgpu::TextureFormatFeatures) -> bool {
    features
        .allowed_usages
        .contains(wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST)
        && features
            .flags
            .contains(wgpu::TextureFormatFeatureFlags::FILTERABLE)
}

/// Restricts adapter-reported features to those legal under the selected device features.
///
/// wgpu reports adapter format support, which can include adapter-specific extensions.
/// When the device did not request those extensions, only the WebGPU-guaranteed subset is
/// legal. An enabled adapter-specific format feature permits using the adapter's report.
pub fn device_format_features(
    format: wgpu::TextureFormat,
    adapter_features: wgpu::TextureFormatFeatures,
    enabled_device_features: wgpu::Features,
) -> wgpu::TextureFormatFeatures {
    if enabled_device_features.contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES) {
        return adapter_features;
    }

    let guaranteed = format.guaranteed_format_features(enabled_device_features);
    wgpu::TextureFormatFeatures {
        allowed_usages: adapter_features.allowed_usages & guaranteed.allowed_usages,
        flags: adapter_features.flags & guaranteed.flags,
    }
}

/// New aggregate/varying limits must not raise unused requirements relative to wgpu 23.
/// Keep existing per-stage requirements and request no more than the new defaults.
fn restrict_new_limits(mut requested: wgpu::Limits, supported: &wgpu::Limits) -> wgpu::Limits {
    requested.max_bind_groups_plus_vertex_buffers = requested
        .max_bind_groups_plus_vertex_buffers
        .min(supported.max_bind_groups_plus_vertex_buffers);
    requested.max_buffers_and_acceleration_structures_per_shader_stage = requested
        .max_buffers_and_acceleration_structures_per_shader_stage
        .min(supported.max_buffers_and_acceleration_structures_per_shader_stage);
    requested.max_inter_stage_shader_variables = requested
        .max_inter_stage_shader_variables
        .min(supported.max_inter_stage_shader_variables);
    requested
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn native_limits(supported: &wgpu::Limits) -> wgpu::Limits {
    restrict_new_limits(wgpu::Limits::default(), supported)
}

/// Browser uses WebGL2 downlevel limits except for the adapter's supported 3D texture limit.
pub fn browser_limits(supported: &wgpu::Limits) -> Result<wgpu::Limits, KeplerError> {
    let requested = restrict_new_limits(
        wgpu::Limits {
            max_texture_dimension_3d: supported.max_texture_dimension_3d,
            ..wgpu::Limits::downlevel_webgl2_defaults()
        },
        supported,
    );
    if !requested.check_limits(supported) {
        return Err(KeplerError::Graphics(format!(
            "Browser adapter cannot satisfy Kepler's downlevel limits (requested 3D texture dimension {}, supported {})",
            requested.max_texture_dimension_3d, supported.max_texture_dimension_3d
        )));
    }
    Ok(requested)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::volume_encoding::VolumeEncoding;

    fn usable() -> wgpu::TextureFormatFeatures {
        wgpu::TextureFormatFeatures {
            allowed_usages: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            flags: wgpu::TextureFormatFeatureFlags::FILTERABLE,
        }
    }

    fn caps(
        r16: wgpu::TextureFormatFeatures,
        rg8: wgpu::TextureFormatFeatures,
    ) -> Result<GpuCapabilities, KeplerError> {
        GpuCapabilities::from_formats(wgpu::Backend::Gl, 256, r16, rg8)
    }

    #[test]
    fn prefers_float_on_any_backend_and_reports_device_limit() {
        let caps = caps(usable(), usable()).unwrap();
        assert!(caps.default_float_encoding());
        assert_eq!(caps.volume_encoding(), VolumeEncoding::R16Float);
        assert_eq!(caps.max_3d_texture_dimension(), 256);
        assert_eq!(caps.backend(), wgpu::Backend::Gl);
    }

    #[test]
    fn respects_supported_encoding_preferences_during_device_replacement() {
        let float_only = caps(
            usable(),
            wgpu::TextureFormatFeatures {
                allowed_usages: wgpu::TextureUsages::empty(),
                flags: wgpu::TextureFormatFeatureFlags::empty(),
            },
        )
        .unwrap();
        assert_eq!(
            float_only.encoding_for_preference(false),
            VolumeEncoding::R16Float
        );

        let packed_only = caps(
            wgpu::TextureFormatFeatures {
                allowed_usages: wgpu::TextureUsages::empty(),
                flags: wgpu::TextureFormatFeatureFlags::empty(),
            },
            usable(),
        )
        .unwrap();
        assert_eq!(
            packed_only.encoding_for_preference(true),
            VolumeEncoding::PackedI16Rg8
        );
    }

    #[test]
    fn rejects_each_missing_float_requirement_and_falls_back_to_rg8() {
        for requirement in [
            wgpu::TextureUsages::COPY_DST,
            wgpu::TextureUsages::TEXTURE_BINDING,
        ] {
            let mut format = usable();
            format.allowed_usages.remove(requirement);
            assert_eq!(
                caps(format, usable()).unwrap().volume_encoding(),
                VolumeEncoding::PackedI16Rg8
            );
        }
        let mut format = usable();
        format
            .flags
            .remove(wgpu::TextureFormatFeatureFlags::FILTERABLE);
        let caps = caps(format, usable()).unwrap();
        assert!(!caps.r16float_filterable());
        assert_eq!(caps.volume_encoding(), VolumeEncoding::PackedI16Rg8);
    }

    #[test]
    fn rejects_no_usable_volume_format() {
        let mut format = usable();
        format.allowed_usages = wgpu::TextureUsages::empty();
        let error = caps(format, format).unwrap_err().to_string();
        assert!(error.contains("volume"));
        assert!(error.contains("Gl"));
    }

    #[test]
    fn adapter_extras_are_only_legal_when_enabled_on_device() {
        let adapter = wgpu::TextureFormat::R32Float
            .guaranteed_format_features(wgpu::Features::FLOAT32_FILTERABLE);
        let restricted = device_format_features(
            wgpu::TextureFormat::R32Float,
            adapter,
            wgpu::Features::empty(),
        );
        assert!(!restricted
            .flags
            .contains(wgpu::TextureFormatFeatureFlags::FILTERABLE));
        let enabled = device_format_features(
            wgpu::TextureFormat::R32Float,
            adapter,
            wgpu::Features::FLOAT32_FILTERABLE,
        );
        assert!(enabled
            .flags
            .contains(wgpu::TextureFormatFeatureFlags::FILTERABLE));
        let specific = device_format_features(
            wgpu::TextureFormat::R32Float,
            adapter,
            wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
        );
        assert!(specific
            .flags
            .contains(wgpu::TextureFormatFeatureFlags::FILTERABLE));
    }

    #[test]
    fn adapter_restrictions_are_not_overridden_by_webgpu_guarantees() {
        let mut adapter = usable();
        adapter.flags = wgpu::TextureFormatFeatureFlags::empty();
        assert!(!device_format_features(
            wgpu::TextureFormat::R16Float,
            adapter,
            wgpu::Features::empty()
        )
        .flags
        .contains(wgpu::TextureFormatFeatureFlags::FILTERABLE));
    }

    #[test]
    fn new_unused_default_limits_do_not_reject_formerly_usable_adapters() {
        let supported = wgpu::Limits {
            max_bind_groups_plus_vertex_buffers: 5,
            max_buffers_and_acceleration_structures_per_shader_stage: 12,
            max_inter_stage_shader_variables: 8,
            ..wgpu::Limits::default()
        };
        let browser = browser_limits(&supported).unwrap();
        assert_eq!(browser.max_bind_groups_plus_vertex_buffers, 5);
        assert_eq!(
            browser.max_buffers_and_acceleration_structures_per_shader_stage,
            12
        );
        assert_eq!(browser.max_inter_stage_shader_variables, 8);
        let native = native_limits(&supported);
        assert!(native.check_limits(&supported));
        assert_eq!(native.max_immediate_size, 0);
        assert_eq!(
            native.max_bind_groups,
            wgpu::Limits::default().max_bind_groups
        );
    }

    #[test]
    fn browser_limits_accept_lower_3d_limits_without_changing_other_requirements() {
        for dimension in [256, 512, 2048] {
            let supported = wgpu::Limits {
                max_texture_dimension_3d: dimension,
                ..wgpu::Limits::default()
            };
            let requested = browser_limits(&supported).unwrap();
            assert_eq!(requested.max_texture_dimension_3d, dimension);
            assert_eq!(
                requested.max_bind_groups,
                wgpu::Limits::downlevel_webgl2_defaults().max_bind_groups
            );
            assert!(requested.check_limits(&supported));
        }
    }

    #[test]
    fn incompatible_browser_limits_return_an_error() {
        let supported = wgpu::Limits {
            max_bind_groups: 0,
            ..wgpu::Limits::default()
        };
        assert!(browser_limits(&supported).is_err());
    }
}
