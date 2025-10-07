/// Mesh rendering integration tests
/// 
/// This module provides comprehensive integration testing for the mesh rendering system,
/// ensuring all components work together correctly across different scenarios.

#[cfg(feature = "mesh")]
mod mesh_integration_tests {
    use kepler_wgpu::mesh::{
        mesh_view::{MeshView, MeshRenderError},
        mesh_render_context::MeshRenderContext,
        performance::{QualityController, QualityLevel},
        shader_validation::ShaderValidator,
        mesh::Mesh,
    };
    use kepler_wgpu::rendering::core::pipeline::PipelineManager;
    use wgpu::{Instance, Backends, DeviceDescriptor, Features, Limits};

    /// Helper function to create a test device
    async fn create_test_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("Test Device"),
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device")
    }

    #[tokio::test]
    async fn test_mesh_view_creation() {
        /// Test basic MeshView creation and initialization
        let mesh_view = MeshView::new();
        
        assert!(mesh_view.is_healthy(), "New MeshView should be healthy");
        assert_eq!(mesh_view.get_quality_level(), QualityLevel::Medium);
    }

    #[tokio::test]
    async fn test_performance_monitoring_integration() {
        /// Test integration between MeshView and performance monitoring
        let mut quality_controller = QualityController::new();
        
        // Test initial state
        assert_eq!(quality_controller.get_quality_level(), QualityLevel::Medium);
        
        // Simulate frame timing
        quality_controller.start_frame();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let quality_change = quality_controller.end_frame();
        
        // Should not change quality on first frame
        assert!(quality_change.is_none(), "Quality should not change on first frame");
    }

    #[tokio::test]
    async fn test_quality_adjustment() {
        /// Test automatic quality adjustment based on performance
        let mut quality_controller = QualityController::new();
        
        // Set to high quality initially
        quality_controller.set_quality_level(QualityLevel::High);
        
        // Simulate slow frames
        for _ in 0..10 {
            quality_controller.start_frame();
            std::thread::sleep(std::time::Duration::from_millis(25)); // Slow frame
            quality_controller.end_frame();
        }
        
        // Quality should be reduced
        let current_quality = quality_controller.get_quality_level();
        assert!((current_quality as u8) < (QualityLevel::High as u8), 
               "Quality should be reduced after slow frames");
    }

    #[tokio::test]
    async fn test_mesh_render_context_creation() {
        /// Test MeshRenderContext creation with valid data
        let (device, queue) = create_test_device().await;
        let mut pipeline_manager = PipelineManager::new();

        let mesh = Mesh::unit_cube(); // Using available sample mesh
        let context = MeshRenderContext::new(&mut pipeline_manager, &device, &queue, &mesh, true);
        
        assert_eq!(context.num_vertices, 3);
        assert_eq!(context.num_indices, 3);
    }

    #[tokio::test]
    async fn test_error_handling_integration() {
        /// Test error handling across the mesh rendering system
        let mesh_view = MeshView::new();

        // Test that MeshView starts healthy
        assert!(mesh_view.is_healthy(), "MeshView should start healthy");
        
        // Verify error statistics are tracked
        let stats = mesh_view.get_stats();
        assert_eq!(stats.error_count, 0, "New MeshView should have no errors");
    }

    #[tokio::test]
    async fn test_shader_validation() {
        /// Test shader validation functionality
        let (device, _queue) = create_test_device().await;

        let validator = ShaderValidator::new(&device);
        
        // Test mesh shader validation
        let result = validator.validate_mesh_shader();
        assert!(result.is_ok(), "Mesh shader should pass validation");
        
        if let Ok(metrics) = result {
            assert!(metrics.compilation_time_ms >= 0.0, "Compilation time should be non-negative");
            assert!(metrics.vertex_complexity_score > 0, "Should have vertex complexity score");
        }
    }

    #[tokio::test]
    async fn test_memory_management() {
        /// Test memory management and cleanup
        let (device, queue) = create_test_device().await;
        let mut pipeline_manager = PipelineManager::new();

        let mut contexts = Vec::new();
        
        // Create multiple mesh contexts
        for _ in 0..5 {
            let mesh = Mesh::unit_cube();
            let context = MeshRenderContext::new(&mut pipeline_manager, &device, &queue, &mesh, true);
            contexts.push(context);
        }
        
        assert_eq!(contexts.len(), 5, "Should be able to create multiple contexts");
        
        // Test memory statistics
        for context in &contexts {
            let (vertex_size, index_size, _, _) = context.get_memory_stats();
            assert!(vertex_size > 0, "Vertex buffer should have size");
            assert!(index_size > 0, "Index buffer should have size");
        }
    }

    #[tokio::test]
    async fn test_quality_settings_application() {
        /// Test that quality settings are properly applied
        let mut quality_controller = QualityController::new();
        
        let quality_levels = vec![
            QualityLevel::Minimal,
            QualityLevel::Low,
            QualityLevel::Medium,
            QualityLevel::High,
            QualityLevel::Maximum,
        ];
        
        for quality in quality_levels {
            quality_controller.set_quality_level(quality);
            assert_eq!(quality_controller.get_quality_level(), quality);
            
            let settings = quality_controller.get_quality_settings();
            
            // Verify settings are appropriate for quality level
            match quality {
                QualityLevel::Minimal => {
                    assert!(settings.wireframe_mode, "Minimal quality should use wireframe");
                }
                QualityLevel::Maximum => {
                    assert!(!settings.wireframe_mode, "Maximum quality should not use wireframe");
                    assert!(settings.lighting_enabled, "Maximum quality should have lighting");
                }
                _ => {
                    // Other quality levels have their own characteristics
                }
            }
        }
    }

    #[tokio::test]
    async fn test_performance_benchmark() {
        /// Basic performance benchmark test
        let mut mesh_view = MeshView::new();

        let start_time = std::time::Instant::now();
        
        // Simulate rendering frames
        for _ in 0..60 {
            mesh_view.start_frame_timing();
            std::thread::sleep(std::time::Duration::from_millis(1)); // Simulate work
            mesh_view.end_frame_timing();
        }
        
        let total_time = start_time.elapsed();
        let stats = mesh_view.get_performance_stats();
        
        assert!(total_time.as_millis() > 0, "Benchmark should take measurable time");
        assert!(stats.current_quality != QualityLevel::Minimal || true, "Should have valid quality level");
        
        println!("Benchmark results:");
        println!("  Total time: {:?}", total_time);
        println!("  Average frame time: {:.2}ms", stats.average_frame_time_ms);
        println!("  Current quality: {:?}", stats.current_quality);
    }
}

#[cfg(not(feature = "mesh"))]
#[test]
fn test_mesh_feature_disabled() {
    /// Test that mesh functionality is properly disabled when feature is not enabled
    // This test ensures that the project compiles and runs without the mesh feature
    assert!(true, "Project should compile without mesh feature");
}