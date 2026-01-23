//! Patient and study/series relationship fixtures

use kepler_wgpu::data::dicom::{ImageSeries, Patient, StudySet};

/// Create test patient with configurable ID
#[allow(dead_code)]
pub fn create_test_patient(id: &str) -> Vec<u8> {
    vec![]
}

/// Create patient with empty ID (invalid)
#[allow(dead_code)]
pub fn create_invalid_patient() -> Vec<u8> {
    vec![]
}

/// Create test study with configurable parameters
#[allow(dead_code)]
pub fn create_test_study(study_id: &str, patient_id: &str) -> Vec<u8> {
    vec![]
}

/// Create test image series
#[allow(dead_code)]
pub fn create_test_image_series(series_uid: &str, study_uid: &str) -> Vec<u8> {
    vec![]
}
