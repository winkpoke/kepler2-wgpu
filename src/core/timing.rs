/// Cross-platform timing utilities that work on both native and WASM targets
/// 
/// This module provides timing functionality that gracefully handles the lack of
/// std::time::Instant on WASM platforms by providing no-op implementations.

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant as StdInstant};

#[cfg(target_arch = "wasm32")]
use std::time::Duration;

/// A cross-platform instant that works on both native and WASM
#[derive(Debug, Clone, Copy)]
pub struct Instant {
    #[cfg(not(target_arch = "wasm32"))]
    inner: StdInstant,
    #[cfg(target_arch = "wasm32")]
    _phantom: (),
}

impl Instant {
    /// Creates a new `Instant` representing the current time
    /// 
    /// On WASM, this is a no-op and returns a dummy instant
    pub fn now() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {
                inner: StdInstant::now(),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self { _phantom: () }
        }
    }

    /// Returns the elapsed time since this instant
    /// 
    /// On WASM, this always returns zero duration
    pub fn elapsed(&self) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.elapsed()
        }
        #[cfg(target_arch = "wasm32")]
        {
            Duration::from_secs(0)
        }
    }

    /// Returns the duration since another instant
    /// 
    /// On WASM, this always returns zero duration
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.duration_since(earlier.inner)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = earlier; // Suppress unused variable warning
            Duration::from_secs(0)
        }
    }
}

/// Extension trait for Duration to provide convenient timing utilities
pub trait DurationExt {
    /// Convert duration to milliseconds as f64
    fn as_millis_f64(&self) -> f64;
    /// Convert duration to milliseconds as f32
    fn as_millis_f32(&self) -> f32;
    /// Convert duration to seconds as f64
    fn as_secs_f64(&self) -> f64;
    /// Convert duration to seconds as f32
    fn as_secs_f32(&self) -> f32;
}

impl DurationExt for Duration {
    fn as_millis_f64(&self) -> f64 {
        self.as_secs_f64() * 1000.0
    }
    
    fn as_millis_f32(&self) -> f32 {
        self.as_secs_f32() * 1000.0
    }
    
    fn as_secs_f64(&self) -> f64 {
        self.as_secs() as f64 + self.subsec_nanos() as f64 / 1_000_000_000.0
    }
    
    fn as_secs_f32(&self) -> f32 {
        self.as_secs() as f32 + self.subsec_nanos() as f32 / 1_000_000_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instant_creation() {
        let instant = Instant::now();
        let _elapsed = instant.elapsed();
        // Should not panic on any platform
    }

    #[test]
    fn test_duration_conversion() {
        let duration = Duration::from_millis(1500);
        assert_eq!(duration.as_millis_f64(), 1500.0);
    }
}