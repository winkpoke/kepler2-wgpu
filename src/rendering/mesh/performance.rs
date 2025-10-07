/// Performance monitoring and automatic quality adjustment for mesh rendering
use std::time::Duration;
use crate::core::timing::{Instant, DurationExt};

/// Function-level comment: Performance targets and thresholds for automatic quality adjustment
#[derive(Debug, Clone)]
pub struct PerformanceTargets {
    /// Target frame time in milliseconds (16.67ms for 60fps)
    pub target_frame_time_ms: f32,
    /// Maximum acceptable frame time before quality reduction (20ms for 50fps)
    pub max_frame_time_ms: f32,
    /// Minimum frame time to consider for quality increase (13.33ms for 75fps)
    pub min_frame_time_ms: f32,
    /// Number of consecutive frames above threshold before quality reduction
    pub quality_reduction_threshold: u32,
    /// Number of consecutive frames below threshold before quality increase
    pub quality_increase_threshold: u32,
}

impl Default for PerformanceTargets {
    fn default() -> Self {
        Self {
            target_frame_time_ms: 16.67,  // 60fps
            max_frame_time_ms: 20.0,      // 50fps
            min_frame_time_ms: 13.33,     // 75fps
            quality_reduction_threshold: 5,
            quality_increase_threshold: 60, // Wait longer before increasing quality
        }
    }
}

/// Function-level comment: Quality levels for automatic adjustment
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QualityLevel {
    /// Minimum quality - wireframe only
    Minimal = 0,
    /// Low quality - reduced geometry, simple shading
    Low = 1,
    /// Medium quality - normal geometry, basic lighting
    Medium = 2,
    /// High quality - full geometry, advanced lighting
    High = 3,
    /// Maximum quality - all features enabled
    Maximum = 4,
}

impl Default for QualityLevel {
    fn default() -> Self {
        QualityLevel::Medium
    }
}

impl QualityLevel {
    /// Function-level comment: Get the next lower quality level
    pub fn decrease(self) -> Self {
        match self {
            QualityLevel::Maximum => QualityLevel::High,
            QualityLevel::High => QualityLevel::Medium,
            QualityLevel::Medium => QualityLevel::Low,
            QualityLevel::Low => QualityLevel::Minimal,
            QualityLevel::Minimal => QualityLevel::Minimal,
        }
    }

    /// Function-level comment: Get the next higher quality level
    pub fn increase(self) -> Self {
        match self {
            QualityLevel::Minimal => QualityLevel::Low,
            QualityLevel::Low => QualityLevel::Medium,
            QualityLevel::Medium => QualityLevel::High,
            QualityLevel::High => QualityLevel::Maximum,
            QualityLevel::Maximum => QualityLevel::Maximum,
        }
    }

    /// Function-level comment: Get quality settings for this level
    pub fn get_settings(&self) -> QualitySettings {
        match self {
            QualityLevel::Minimal => QualitySettings {
                wireframe_mode: true,
                lighting_enabled: false,
                shadow_quality: ShadowQuality::Disabled,
                mesh_lod_bias: 2.0,
                texture_quality: TextureQuality::Quarter,
                msaa_samples: 1,
            },
            QualityLevel::Low => QualitySettings {
                wireframe_mode: false,
                lighting_enabled: true,
                shadow_quality: ShadowQuality::Disabled,
                mesh_lod_bias: 1.5,
                texture_quality: TextureQuality::Half,
                msaa_samples: 1,
            },
            QualityLevel::Medium => QualitySettings {
                wireframe_mode: false,
                lighting_enabled: true,
                shadow_quality: ShadowQuality::Low,
                mesh_lod_bias: 1.0,
                texture_quality: TextureQuality::Full,
                msaa_samples: 1,
            },
            QualityLevel::High => QualitySettings {
                wireframe_mode: false,
                lighting_enabled: true,
                shadow_quality: ShadowQuality::Medium,
                mesh_lod_bias: 0.5,
                texture_quality: TextureQuality::Full,
                msaa_samples: 2,
            },
            QualityLevel::Maximum => QualitySettings {
                wireframe_mode: false,
                lighting_enabled: true,
                shadow_quality: ShadowQuality::High,
                mesh_lod_bias: 0.0,
                texture_quality: TextureQuality::Full,
                msaa_samples: 4,
            },
        }
    }
}

/// Function-level comment: Quality settings derived from quality level
#[derive(Debug, Clone)]
pub struct QualitySettings {
    pub wireframe_mode: bool,
    pub lighting_enabled: bool,
    pub shadow_quality: ShadowQuality,
    pub mesh_lod_bias: f32,
    pub texture_quality: TextureQuality,
    pub msaa_samples: u32,
}

/// Function-level comment: Shadow quality levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShadowQuality {
    Disabled,
    Low,    // 512x512
    Medium, // 1024x1024
    High,   // 2048x2048
}

/// Function-level comment: Texture quality levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextureQuality {
    Quarter, // 1/4 resolution
    Half,    // 1/2 resolution
    Full,    // Full resolution
}

/// Function-level comment: Frame timing tracker for performance monitoring
#[derive(Debug)]
pub struct FrameTimer {
    frame_start: Instant,
    frame_times: Vec<f32>,
    max_samples: usize,
    current_index: usize,
}

impl FrameTimer {
    /// Function-level comment: Create a new frame timer with specified sample count
    pub fn new(max_samples: usize) -> Self {
        Self {
            frame_start: Instant::now(),
            frame_times: Vec::with_capacity(max_samples),
            max_samples,
            current_index: 0,
        }
    }

    /// Function-level comment: Start timing a new frame
    pub fn start_frame(&mut self) {
        self.frame_start = Instant::now();
    }

    /// Function-level comment: End frame timing and record the duration
    pub fn end_frame(&mut self) -> f32 {
        let frame_time = self.frame_start.elapsed().as_millis_f32();
        
        if self.frame_times.len() < self.max_samples {
            self.frame_times.push(frame_time);
        } else {
            self.frame_times[self.current_index] = frame_time;
            self.current_index = (self.current_index + 1) % self.max_samples;
        }
        
        frame_time
    }

    /// Function-level comment: Get average frame time over recent samples
    pub fn get_average_frame_time(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
    }

    /// Function-level comment: Get the 95th percentile frame time (for detecting spikes)
    pub fn get_percentile_frame_time(&self, percentile: f32) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        
        let mut sorted_times = self.frame_times.clone();
        sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let index = ((sorted_times.len() as f32 * percentile / 100.0) as usize)
            .min(sorted_times.len() - 1);
        sorted_times[index]
    }
}

/// Function-level comment: Automatic quality adjustment controller
#[derive(Debug)]
pub struct QualityController {
    current_quality: QualityLevel,
    targets: PerformanceTargets,
    frame_timer: FrameTimer,
    consecutive_slow_frames: u32,
    consecutive_fast_frames: u32,
    last_adjustment_time: Instant,
    adjustment_cooldown: Duration,
}

impl QualityController {
    /// Function-level comment: Create a new quality controller with default settings
    pub fn new() -> Self {
        Self {
            current_quality: QualityLevel::default(),
            targets: PerformanceTargets::default(),
            frame_timer: FrameTimer::new(120), // Track last 2 seconds at 60fps
            consecutive_slow_frames: 0,
            consecutive_fast_frames: 0,
            last_adjustment_time: Instant::now(),
            adjustment_cooldown: Duration::from_secs(2), // Wait 2 seconds between adjustments
        }
    }

    /// Function-level comment: Create a quality controller with custom targets
    pub fn with_targets(targets: PerformanceTargets) -> Self {
        Self {
            targets,
            ..Self::new()
        }
    }

    /// Function-level comment: Start timing a new frame
    pub fn start_frame(&mut self) {
        self.frame_timer.start_frame();
    }

    /// Function-level comment: End frame timing and potentially adjust quality
    pub fn end_frame(&mut self) -> Option<QualityLevel> {
        let frame_time = self.frame_timer.end_frame();
        
        // Check if we're in cooldown period
        if self.last_adjustment_time.elapsed() < self.adjustment_cooldown {
            return None;
        }

        // Determine if frame time is acceptable
        if frame_time > self.targets.max_frame_time_ms {
            self.consecutive_slow_frames += 1;
            self.consecutive_fast_frames = 0;
            
            // Reduce quality if we've had too many slow frames
            if self.consecutive_slow_frames >= self.targets.quality_reduction_threshold {
                let new_quality = self.current_quality.decrease();
                if new_quality != self.current_quality {
                    log::warn!("Reducing quality from {:?} to {:?} due to slow frames ({}ms avg)", 
                        self.current_quality, new_quality, self.frame_timer.get_average_frame_time());
                    self.current_quality = new_quality;
                    self.consecutive_slow_frames = 0;
                    self.last_adjustment_time = Instant::now();
                    return Some(new_quality);
                }
            }
        } else if frame_time < self.targets.min_frame_time_ms {
            self.consecutive_fast_frames += 1;
            self.consecutive_slow_frames = 0;
            
            // Increase quality if we've had many fast frames
            if self.consecutive_fast_frames >= self.targets.quality_increase_threshold {
                let new_quality = self.current_quality.increase();
                if new_quality != self.current_quality {
                    log::info!("Increasing quality from {:?} to {:?} due to fast frames ({}ms avg)", 
                        self.current_quality, new_quality, self.frame_timer.get_average_frame_time());
                    self.current_quality = new_quality;
                    self.consecutive_fast_frames = 0;
                    self.last_adjustment_time = Instant::now();
                    return Some(new_quality);
                }
            }
        } else {
            // Frame time is acceptable, reset counters
            self.consecutive_slow_frames = 0;
            self.consecutive_fast_frames = 0;
        }

        None
    }

    /// Function-level comment: Get current quality level
    pub fn get_quality_level(&self) -> QualityLevel {
        self.current_quality
    }

    /// Function-level comment: Get current quality settings
    pub fn get_quality_settings(&self) -> QualitySettings {
        self.current_quality.get_settings()
    }

    /// Function-level comment: Manually set quality level (disables automatic adjustment temporarily)
    pub fn set_quality_level(&mut self, quality: QualityLevel) {
        if quality != self.current_quality {
            log::info!("Manually setting quality level to {:?}", quality);
            self.current_quality = quality;
            self.consecutive_slow_frames = 0;
            self.consecutive_fast_frames = 0;
            self.last_adjustment_time = Instant::now();
        }
    }

    /// Function-level comment: Get performance statistics
    pub fn get_performance_stats(&self) -> PerformanceStats {
        PerformanceStats {
            current_quality: self.current_quality,
            average_frame_time_ms: self.frame_timer.get_average_frame_time(),
            p95_frame_time_ms: self.frame_timer.get_percentile_frame_time(95.0),
            consecutive_slow_frames: self.consecutive_slow_frames,
            consecutive_fast_frames: self.consecutive_fast_frames,
            target_frame_time_ms: self.targets.target_frame_time_ms,
        }
    }
}

impl Default for QualityController {
    fn default() -> Self {
        Self::new()
    }
}

/// Function-level comment: Performance statistics for monitoring and debugging
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub current_quality: QualityLevel,
    pub average_frame_time_ms: f32,
    pub p95_frame_time_ms: f32,
    pub consecutive_slow_frames: u32,
    pub consecutive_fast_frames: u32,
    pub target_frame_time_ms: f32,
}

impl PerformanceStats {
    /// Function-level comment: Check if performance is currently meeting targets
    pub fn is_meeting_targets(&self) -> bool {
        self.average_frame_time_ms <= self.target_frame_time_ms * 1.2 // 20% tolerance
    }

    /// Function-level comment: Get current FPS estimate
    pub fn get_fps(&self) -> f32 {
        if self.average_frame_time_ms > 0.0 {
            1000.0 / self.average_frame_time_ms
        } else {
            0.0
        }
    }
}