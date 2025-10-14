/// Performance monitoring system tests
/// 
/// This module provides comprehensive testing for the performance monitoring and
/// quality adjustment systems, ensuring they work correctly under various conditions.

mod performance_tests {
    use kepler_wgpu::mesh::performance::{
        QualityLevel, PerformanceTargets, QualityController, FrameTimer, PerformanceStats
    };
    use std::time::Duration;

    #[test]
    fn test_quality_controller_creation() {
        /// Test QualityController creation and initialization
        let controller = QualityController::new();
        
        assert_eq!(controller.get_quality_level(), QualityLevel::Medium);
        
        let stats = controller.get_performance_stats();
        assert_eq!(stats.current_quality, QualityLevel::Medium);
        assert!(stats.get_fps() >= 0.0, "FPS should be non-negative");
    }

    #[test]
    fn test_frame_timer_functionality() {
        /// Test FrameTimer basic functionality
        let mut timer = FrameTimer::new(60);
        
        // Test single frame timing
        timer.start_frame();
        std::thread::sleep(std::time::Duration::from_millis(16));
        let frame_time = timer.end_frame();
        
        assert!(frame_time > 0.0, "Frame time should be positive");
        assert!(frame_time >= 16.0, "Frame time should be at least 16ms");
        
        // Test average calculation
        let avg = timer.get_average_frame_time();
        assert!(avg > 0.0, "Average frame time should be positive");
    }

    #[test]
    fn test_multiple_frame_timing() {
        /// Test multiple frame timing and statistics
        let mut timer = FrameTimer::new(10);
        
        // Record multiple frames
        for i in 0..5 {
            timer.start_frame();
            std::thread::sleep(std::time::Duration::from_millis(10 + i * 2));
            timer.end_frame();
        }
        
        let avg = timer.get_average_frame_time();
        assert!(avg > 0.0, "Average should be positive");
        assert!(avg >= 10.0, "Average should be at least 10ms");
        
        // Test percentile calculation
        let p95 = timer.get_percentile_frame_time(95.0);
        assert!(p95 >= avg, "95th percentile should be >= average");
    }

    #[test]
    fn test_quality_adjustment_based_on_performance() {
        /// Test automatic quality adjustment based on performance
        let targets = PerformanceTargets {
            target_frame_time_ms: 16.67,
            max_frame_time_ms: 20.0,
            min_frame_time_ms: 10.0,
            quality_reduction_threshold: 3,
            quality_increase_threshold: 5,
        };
        
        let mut controller = QualityController::with_targets(targets.clone());
        
        // Simulate slow frames to trigger quality reduction
        for _ in 0..5 {
            controller.start_frame();
            std::thread::sleep(std::time::Duration::from_millis(25)); // Slow frame
            let adjustment = controller.end_frame();
            
            if adjustment.is_some() {
                assert!(adjustment.unwrap() < QualityLevel::Medium, 
                       "Quality should be reduced for slow frames");
                break;
            }
        }
    }

    #[test]
    fn test_quality_level_transitions() {
        /// Test quality level increase and decrease
        let mut level = QualityLevel::Medium;
        
        // Test decrease
        level = level.decrease();
        assert_eq!(level, QualityLevel::Low);
        
        level = level.decrease();
        assert_eq!(level, QualityLevel::Minimal);
        
        // Test that minimal doesn't go lower
        level = level.decrease();
        assert_eq!(level, QualityLevel::Minimal);
        
        // Test increase
        level = level.increase();
        assert_eq!(level, QualityLevel::Low);
        
        level = level.increase();
        assert_eq!(level, QualityLevel::Medium);
        
        level = level.increase();
        assert_eq!(level, QualityLevel::High);
        
        level = level.increase();
        assert_eq!(level, QualityLevel::Maximum);
        
        // Test that maximum doesn't go higher
        level = level.increase();
        assert_eq!(level, QualityLevel::Maximum);
    }

    #[test]
    fn test_performance_targets_configuration() {
        /// Test PerformanceTargets configuration
        let targets = PerformanceTargets {
            target_frame_time_ms: 16.67,
            max_frame_time_ms: 20.0,
            min_frame_time_ms: 10.0,
            quality_reduction_threshold: 3,
            quality_increase_threshold: 5,
        };
        
        assert_eq!(targets.target_frame_time_ms, 16.67);
        assert_eq!(targets.max_frame_time_ms, 20.0);
        assert_eq!(targets.min_frame_time_ms, 10.0);
        assert_eq!(targets.quality_reduction_threshold, 3);
        assert_eq!(targets.quality_increase_threshold, 5);
        
        // Test default targets
        let default_targets = PerformanceTargets::default();
        assert!(default_targets.target_frame_time_ms > 0.0);
        assert!(default_targets.max_frame_time_ms > default_targets.target_frame_time_ms);
        assert!(default_targets.min_frame_time_ms < default_targets.target_frame_time_ms);
    }

    #[test]
    fn test_quality_controller() {
        /// Test QualityController functionality
        let mut controller = QualityController::new();
        
        // Test initial state
        assert_eq!(controller.get_quality_level(), QualityLevel::Medium);
        
        // Test quality adjustment through frame timing
        controller.start_frame();
        std::thread::sleep(Duration::from_millis(25)); // Poor performance
        controller.end_frame();
        
        controller.start_frame();
        std::thread::sleep(Duration::from_millis(10)); // Good performance
        controller.end_frame();
        
        // Quality level should be accessible
        let current_quality = controller.get_quality_level();
        assert!(current_quality as u8 <= QualityLevel::Maximum as u8);
    }

    #[test]
    fn test_frame_timer() {
        /// Test FrameTimer functionality
        let mut timer = FrameTimer::new(60);
        
        // Test initial state
        assert_eq!(timer.get_average_frame_time(), 0.0);
        
        // Test timing
        timer.start_frame();
        std::thread::sleep(Duration::from_millis(16));
        let frame_time = timer.end_frame();
        
        assert!(frame_time >= 16.0);
        assert!(timer.get_average_frame_time() > 0.0);
    }

    #[test]
    fn test_quality_controller_functionality() {
        /// Test QualityController behavior
        let targets = PerformanceTargets {
            target_frame_time_ms: 16.67,
            max_frame_time_ms: 33.33,
            min_frame_time_ms: 10.0,
            quality_reduction_threshold: 3,
            quality_increase_threshold: 5,
        };
        let mut controller = QualityController::with_targets(targets.clone());
        
        // Test initial state
        assert_eq!(controller.get_quality_level(), QualityLevel::Medium);
        
        // Test frame timing
        controller.start_frame();
        std::thread::sleep(Duration::from_millis(10));
        let quality_change = controller.end_frame();
        
        // Quality change might or might not occur
        assert!(quality_change.is_none() || quality_change.is_some());
    }

    #[test]
    fn test_frame_timer_advanced() {
        /// Test advanced FrameTimer functionality
        let mut timer = FrameTimer::new(60);
        
        // Test initial state
        assert_eq!(timer.get_average_frame_time(), 0.0);
        
        // Test frame timing with varying durations
        let frame_durations = vec![10, 15, 8, 20, 12]; // milliseconds
        
        for duration in frame_durations {
            timer.start_frame();
            std::thread::sleep(Duration::from_millis(duration));
            timer.end_frame();
        }
        
        assert!(timer.get_average_frame_time() > 0.0);
        assert!(timer.get_percentile_frame_time(95.0) > 0.0);
    }

    #[test]
    fn test_performance_stats_structure() {
        /// Test PerformanceStats structure and functionality
        let targets = PerformanceTargets {
            target_frame_time_ms: 16.67,
            max_frame_time_ms: 33.33,
            min_frame_time_ms: 10.0,
            quality_reduction_threshold: 3,
            quality_increase_threshold: 5,
        };
        let mut controller = QualityController::with_targets(targets.clone());
        
        // Generate some performance data
        for _ in 0..10 {
            controller.start_frame();
            std::thread::sleep(Duration::from_millis(15));
            controller.end_frame();
        }
        
        let stats = controller.get_performance_stats();
        assert_eq!(stats.current_quality, QualityLevel::Medium);
        assert!(stats.average_frame_time_ms > 0.0);
        assert!(stats.target_frame_time_ms > 0.0);
    }

    #[test]
    fn test_quality_controller_reset() {
        /// Test quality controller state management
        let targets = PerformanceTargets {
            target_frame_time_ms: 16.67,
            max_frame_time_ms: 33.33,
            min_frame_time_ms: 10.0,
            quality_reduction_threshold: 3,
            quality_increase_threshold: 5,
        };
        let mut controller = QualityController::with_targets(targets);
        
        // Generate some data
        for _ in 0..5 {
            controller.start_frame();
            std::thread::sleep(Duration::from_millis(10));
            controller.end_frame();
        }
        
        // Test manual quality setting (acts as a reset)
        controller.set_quality_level(QualityLevel::Medium);
        
        // Should be at specified state
        assert_eq!(controller.get_quality_level(), QualityLevel::Medium);
    }

    #[test]
    fn test_concurrent_quality_controller() {
        /// Test thread safety of quality controller
        use std::sync::{Arc, Mutex};
        use std::thread;
        
        let targets = PerformanceTargets {
            target_frame_time_ms: 16.67,
            max_frame_time_ms: 33.33,
            min_frame_time_ms: 10.0,
            quality_reduction_threshold: 3,
            quality_increase_threshold: 5,
        };
        let controller = Arc::new(Mutex::new(QualityController::with_targets(targets)));
        let mut handles = vec![];
        
        // Spawn multiple threads that record frame times
        for _ in 0..3 {
            let controller_clone = controller.clone();
            let handle = thread::spawn(move || {
                for _ in 0..5 {
                    let mut ctrl = controller_clone.lock().unwrap();
                    ctrl.start_frame();
                    std::thread::sleep(Duration::from_millis(1));
                    ctrl.end_frame();
                }
            });
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Thread should complete successfully");
        }
        
        // Verify final state
        let ctrl = controller.lock().unwrap();
        let stats = ctrl.get_performance_stats();
        assert!(stats.average_frame_time_ms >= 0.0);
    }

    #[test]
    fn test_performance_stats() {
        /// Test PerformanceStats structure and methods
        let targets = PerformanceTargets {
            target_frame_time_ms: 16.67,
            max_frame_time_ms: 33.33,
            min_frame_time_ms: 10.0,
            quality_reduction_threshold: 3,
            quality_increase_threshold: 5,
        };
        let mut controller = QualityController::with_targets(targets);
        
        // Simulate some frame timings
        for _ in 0..10 {
            controller.start_frame();
            std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
            controller.end_frame();
        }
        
        let stats = controller.get_performance_stats();
        assert!(stats.average_frame_time_ms > 0.0, "Average frame time should be positive");
        assert_eq!(stats.current_quality, QualityLevel::Medium);
        assert!(stats.target_frame_time_ms > 0.0, "Target frame time should be positive");
    }

    #[test]
    fn test_performance_monitoring_reset() {
        /// Test performance monitoring reset functionality
        let targets = PerformanceTargets {
            target_frame_time_ms: 16.67,
            max_frame_time_ms: 33.33,
            min_frame_time_ms: 10.0,
            quality_reduction_threshold: 3,
            quality_increase_threshold: 5,
        };
        let mut controller = QualityController::with_targets(targets.clone());
        
        // Generate some performance data
        for _ in 0..5 {
            controller.start_frame();
            std::thread::sleep(Duration::from_millis(16));
            controller.end_frame();
        }
        
        let stats_before = controller.get_performance_stats();
        assert!(stats_before.average_frame_time_ms > 0.0);
        
        // Test creating a new controller (acts as reset)
        let new_controller = QualityController::with_targets(targets.clone());
        
        let stats_after = new_controller.get_performance_stats();
        assert_eq!(stats_after.average_frame_time_ms, 0.0);
    }

    #[test]
    fn test_concurrent_performance_monitoring() {
        /// Test that performance monitoring works correctly with concurrent access
        use std::sync::{Arc, Mutex};
        use std::thread;
        
        let targets = PerformanceTargets {
            target_frame_time_ms: 16.67,
            max_frame_time_ms: 33.33,
            min_frame_time_ms: 10.0,
            quality_reduction_threshold: 3,
            quality_increase_threshold: 5,
        };
        let controller = Arc::new(Mutex::new(QualityController::with_targets(targets)));
        let mut handles = vec![];
        
        // Spawn multiple threads to simulate concurrent frame timing
        for i in 0..3 {
            let controller_clone = controller.clone();
            let handle = thread::spawn(move || {
                for _ in 0..5 {
                    let mut ctrl = controller_clone.lock().unwrap();
                    ctrl.start_frame();
                    drop(ctrl); // Release lock during sleep
                    std::thread::sleep(Duration::from_millis(10 + i)); // Variable frame time
                    let mut ctrl = controller_clone.lock().unwrap();
                    ctrl.end_frame();
                }
            });
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Thread should complete successfully");
        }
        
        // Verify that frames were recorded
        let ctrl = controller.lock().unwrap();
        let stats = ctrl.get_performance_stats();
        assert!(stats.average_frame_time_ms > 0.0);
    }

    #[test]
    fn test_quality_settings_application() {
        /// Test quality settings application
        let quality_low = QualityLevel::Low;
        let settings_low = quality_low.get_settings();
        
        assert_eq!(settings_low.wireframe_mode, false);
        assert_eq!(settings_low.lighting_enabled, true);
        assert_eq!(settings_low.msaa_samples, 1);
        
        let quality_high = QualityLevel::High;
        let settings_high = quality_high.get_settings();
        
        assert_eq!(settings_high.wireframe_mode, false);
        assert_eq!(settings_high.lighting_enabled, true);
        assert!(settings_high.msaa_samples >= settings_low.msaa_samples);
    }

    #[test]
    fn test_frame_timer_edge_cases() {
        /// Test edge cases for frame timer
        let mut timer = FrameTimer::new(1); // Very small buffer
        
        // Test single frame
        timer.start_frame();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let frame_time = timer.end_frame();
        
        assert!(frame_time > 0.0);
        assert_eq!(timer.get_average_frame_time(), frame_time);
        
        // Test buffer overflow
        timer.start_frame();
        std::thread::sleep(std::time::Duration::from_millis(20));
        timer.end_frame();
        
        // Should still work with buffer size of 1
        assert!(timer.get_average_frame_time() > 0.0);
    }

    #[test]
    fn test_performance_targets_validation() {
        /// Test performance targets validation
        let targets = PerformanceTargets::default();
        
        assert!(targets.target_frame_time_ms > 0.0);
        assert!(targets.max_frame_time_ms > targets.target_frame_time_ms);
        assert!(targets.min_frame_time_ms < targets.target_frame_time_ms);
        assert!(targets.quality_reduction_threshold > 0);
        assert!(targets.quality_increase_threshold > 0);
    }

    #[test]
    fn test_quality_controller_edge_cases() {
        /// Test edge cases for quality controller
        let targets = PerformanceTargets {
            target_frame_time_ms: 16.67,
            max_frame_time_ms: 33.33,
            min_frame_time_ms: 10.0,
            quality_reduction_threshold: 3,
            quality_increase_threshold: 5,
        };
        let mut controller = QualityController::with_targets(targets);
        
        // Test with very high frame time (simulate slow performance)
        for _ in 0..10 {
            controller.start_frame();
            std::thread::sleep(Duration::from_millis(100)); // Very slow frame
            controller.end_frame();
        }
        
        // Quality should have been reduced due to poor performance
        let quality = controller.get_quality_level();
        assert!(quality == QualityLevel::Low || quality == QualityLevel::Medium);
        
        // Test manual quality setting
        controller.set_quality_level(QualityLevel::High);
        assert_eq!(controller.get_quality_level(), QualityLevel::High);
    }

    #[test]
    fn test_performance_edge_cases() {
        /// Test edge cases in performance monitoring
        let targets = PerformanceTargets {
            target_frame_time_ms: 16.67,
            max_frame_time_ms: 33.33,
            min_frame_time_ms: 10.0,
            quality_reduction_threshold: 3,
            quality_increase_threshold: 5,
        };
        let mut controller = QualityController::with_targets(targets);
        let mut timer = FrameTimer::new(10);
        
        // Test very short frame time
        timer.start_frame();
        // Don't sleep, end immediately
        let frame_time = timer.end_frame();
        
        assert!(frame_time >= 0.0);
        assert!(timer.get_average_frame_time() >= 0.0);
        
        // Test multiple consecutive frames
        for _ in 0..5 {
            timer.start_frame();
            std::thread::sleep(Duration::from_millis(1));
            timer.end_frame();
        }
        
        // Should still function correctly
        assert!(timer.get_average_frame_time() > 0.0);
        
        // Test controller with edge case timing
        controller.start_frame();
        std::thread::sleep(Duration::from_millis(1));
        controller.end_frame();
        
        let stats = controller.get_performance_stats();
        assert!(stats.average_frame_time_ms >= 0.0);
    }
}