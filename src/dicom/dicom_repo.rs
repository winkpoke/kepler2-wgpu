use super::ct_image::CTImage;
use super::image_series::ImageSeries;
use super::patient::Patient;
use super::studyset::StudySet;
use crate::coord::{Base, Matrix4x4};
use crate::ct_volume::{CTVolume, CTVolumeGenerator};
use anyhow::{anyhow, Result};
use std::cmp::Ordering;
use std::collections::HashMap;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct DicomRepo {
    patients: HashMap<String, Patient>, // Map of patient ID to Patient
    study_sets: HashMap<String, StudySet>, // Map of study ID to StudySet
    image_series: HashMap<String, ImageSeries>, // Map of series ID to ImageSeries
    ct_images: HashMap<String, CTImage>, // Map of image ID to CTImage
}

impl DicomRepo {
    // Constructor
    pub fn new() -> Self {
        DicomRepo {
            patients: HashMap::new(),
            study_sets: HashMap::new(),
            image_series: HashMap::new(),
            ct_images: HashMap::new(),
        }
    }

    // Add or update a patient
    pub fn add_patient(&mut self, patient: Patient) {
        self.patients.insert(patient.patient_id.clone(), patient);
    }

    // Add or update a study
    pub fn add_study(&mut self, study: StudySet) {
        self.study_sets.insert(study.uid.clone(), study);
    }

    // Add or update an image series
    pub fn add_image_series(&mut self, series: ImageSeries) {
        self.image_series.insert(series.uid.clone(), series);
    }

    // Add or update a CT image
    pub fn add_ct_image(&mut self, image: CTImage) {
        self.ct_images.insert(image.uid.clone(), image);
    }

    pub fn to_string(&self) -> String {
        let mut result = String::new();

        // Iterate over patients
        for patient in self.patients.values() {
            result.push_str(&format!("Patient: {}\n", patient.name));
            result.push_str(&format!("  ID: {}\n", patient.patient_id));
            result.push_str(&format!("  Birthdate: {:?}\n", patient.birthdate));
            result.push_str(&format!("  Sex: {:?}\n", patient.sex));

            // Find study sets for the patient
            for study_set in self
                .study_sets
                .values()
                .filter(|s| s.patient_id == patient.patient_id)
            {
                result.push_str(&format!("  StudySet: {}\n", study_set.uid));
                result.push_str(&format!("    Date: {}\n", study_set.date));
                result.push_str(&format!("    Description: {:?}\n", study_set.description));

                // Find image series for the study set
                for image_series in self
                    .image_series
                    .values()
                    .filter(|is| is.study_uid == study_set.uid)
                {
                    result.push_str(&format!("    ImageSeries: {}\n", image_series.uid));
                    result.push_str(&format!("      Modality: {}\n", image_series.modality));
                    result.push_str(&format!(
                        "      Description: {:?}\n",
                        image_series.description
                    ));

                    // Find CT images for the image series
                    for ct_image in self
                        .ct_images
                        .values()
                        .filter(|img| img.series_uid == image_series.uid)
                    {
                        result.push_str(&format!("      CTImage: {}\n", ct_image.uid));
                        result.push_str(&format!("        Rows: {}\n", ct_image.rows));
                        result.push_str(&format!("        Columns: {}\n", ct_image.columns));
                        result.push_str(&format!(
                            "        PixelSpacing: {:?}\n",
                            ct_image.pixel_spacing
                        ));
                    }
                }
            }
        }
        result
    }

    // Common function to generate CTVolume (shared code)
    fn generate_ct_volume_common(&self, image_series_id: &str) -> Result<CTVolume, String> {
        // Retrieve the ImageSeries by ID
        let series = self
            .image_series
            .get(image_series_id)
            .ok_or_else(|| format!("ImageSeries with ID '{}' not found", image_series_id))?;

        // Collect all CTImages belonging to the ImageSeries
        let mut ct_images: Vec<&CTImage> = self
            .ct_images
            .values()
            .filter(|img| img.series_uid == series.uid)
            .collect();

        if ct_images.is_empty() {
            return Err(format!(
                "No CTImages found for ImageSeries with ID '{}'",
                image_series_id
            ));
        }

        // Sort CTImages by their z-position (third component of ImagePositionPatient)
        ct_images.sort_by(|a, b| {
            let z_a = a.image_position_patient.map(|pos| pos.2).unwrap_or(0.0);
            let z_b = b.image_position_patient.map(|pos| pos.2).unwrap_or(0.0);
            z_a.partial_cmp(&z_b).unwrap_or(Ordering::Equal)
        });

        // Validate consistency of rows, columns, and retrieve metadata from the first image
        let rows = ct_images[0].rows;
        let columns = ct_images[0].columns;
        let pixel_spacing = ct_images[0]
            .pixel_spacing
            .ok_or_else(|| "PixelSpacing is missing in the first CTImage".to_string())?;
        let spacing_between_slices = ct_images[0].spacing_between_slices.unwrap_or_else(|| {
            // If spacing_between_slices is missing, calculate it from ImagePositionPatient and ImageOrientationPatient
            if ct_images.len() > 1 {
                let pos_a = ct_images[0]
                    .image_position_patient
                    .unwrap_or((0.0, 0.0, 0.0));
                let pos_b = ct_images[1]
                    .image_position_patient
                    .unwrap_or((0.0, 0.0, 1.0));

                // Calculate the difference in the z-components (3rd component of ImagePositionPatient)
                let z_diff = (pos_b.2 - pos_a.2).abs();

                if z_diff > 0.0 {
                    z_diff
                } else {
                    1.0 // Fallback value if the difference is zero or invalid
                }
            } else {
                1.0 // Fallback value if there is only one slice (or insufficient data)
            }
        });

        // Ensure all images have consistent dimensions
        if !ct_images
            .iter()
            .all(|img| img.rows == rows && img.columns == columns)
        {
            return Err(format!(
                "Inconsistent image dimensions in ImageSeries '{}'",
                series.uid
            ));
        }

        let voxel_spacing = (pixel_spacing.0, pixel_spacing.1, spacing_between_slices);

        // Pre-allocate the vector with enough capacity to hold all voxel data
        let total_voxels = rows as usize * columns as usize * ct_images.len();
        let mut voxel_data: Vec<i16> = Vec::with_capacity(total_voxels);

        // Collect voxel data from each CTImage sequentially and apply rescale slope + intercept
        for img in &ct_images {
            let raw_data = img.get_pixel_data().map_err(|e| e.to_string())?;

            let slope = img.rescale_slope.unwrap_or(1.0);
            let intercept = img.rescale_intercept.unwrap_or(0.0);

            // Only apply slope/intercept if they actually modify the values
            if (slope - 1.0).abs() > f32::EPSILON || (intercept.abs() > f32::EPSILON) {
                for &raw_val in &raw_data {
                    let hu = (raw_val as f32 * slope + intercept).round() as i16;
                    voxel_data.push(hu);
                }
            } else {
                voxel_data.extend(raw_data);
            }
        }

        // Extract ImageOrientationPatient and compute the Base matrix
        let image_orientation_patient = ct_images[0]
            .image_orientation_patient
            .ok_or_else(|| "ImageOrientationPatient is missing in the first CTImage".to_string())?;

        // Row and column direction vectors
        let row_direction = (
            image_orientation_patient.0,
            image_orientation_patient.1,
            image_orientation_patient.2,
        );
        let column_direction = (
            image_orientation_patient.3,
            image_orientation_patient.4,
            image_orientation_patient.5,
        );

        // Slice direction (cross product of row and column directions)
        let slice_direction = (
            row_direction.1 * column_direction.2 - row_direction.2 * column_direction.1,
            row_direction.2 * column_direction.0 - row_direction.0 * column_direction.2,
            row_direction.0 * column_direction.1 - row_direction.1 * column_direction.0,
        );

        // Image position patient (origin of the base matrix)
        let image_position_patient = ct_images[0]
        .image_position_patient
        .ok_or_else(|| "ImagePositionPatient is missing in the first CTImage".to_string())?;

        // Define the scaling matrix (voxel spacings)
        let scaling_matrix = Matrix4x4::from_array([
            voxel_spacing.0, 0.0, 0.0, 0.0,
            0.0, voxel_spacing.1, 0.0, 0.0,
            0.0, 0.0, voxel_spacing.2, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]);

        // Define the direction matrix (row, column, slice directions)
        let direction_matrix = Matrix4x4::from_array([
            row_direction.0, column_direction.0, slice_direction.0, 0.0,
            row_direction.1, column_direction.1, slice_direction.1, 0.0,
            row_direction.2, column_direction.2, slice_direction.2, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]);

        // Define the translation matrix (image position)
        let translation_matrix = Matrix4x4::from_array([
            1.0, 0.0, 0.0, image_position_patient.0,
            0.0, 1.0, 0.0, image_position_patient.1,
            0.0, 0.0, 1.0, image_position_patient.2,
            0.0, 0.0, 0.0, 1.0,
        ]);

        // Multiply the scaling, direction, and translation matrices
        let base_matrix = direction_matrix
            .multiply(&translation_matrix)
            .multiply(&scaling_matrix);

        // Return the constructed CTVolume
        Ok(CTVolume {
            dimensions: (rows as usize, columns as usize, ct_images.len()),
            voxel_spacing,
            voxel_data,
            base: Base {
                label: series.uid.clone(),
                matrix: base_matrix,
            },
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl CTVolumeGenerator for DicomRepo {
    // Non-WASM synchronous implementation
    fn generate_ct_volume(&self, image_series_id: &str) -> Result<CTVolume, anyhow::Error> {
        // Call the common function and handle errors
        self.generate_ct_volume_common(image_series_id)
            .map_err(|e| anyhow::anyhow!(e))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl DicomRepo {
    pub fn get_all_patients(&self) -> Vec<&Patient> {
        self.patients.values().collect()
    }

    // Query patients
    pub fn get_patient(&self, patient_id: &str) -> Option<&Patient> {
        self.patients.get(patient_id)
    }

    // Query studies by patient
    pub fn get_studies_by_patient(&self, patient_id: &str) -> Vec<&StudySet> {
        self.study_sets
            .values()
            .filter(|s| s.patient_id == patient_id)
            .collect()
    }

    // Query series by study
    pub fn get_series_by_study(&self, study_id: &str) -> Vec<&ImageSeries> {
        self.image_series
            .values()
            .filter(|s| s.study_uid == study_id)
            .collect()
    }

    // Query images by series
    pub fn get_images_by_series(&self, series_id: &str) -> Vec<&CTImage> {
        self.ct_images
            .values()
            .filter(|img| img.series_uid == series_id)
            .collect()
    }
}

//------------------------------ WASM Code -------------------------------------

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg(target_arch = "wasm32")]
impl DicomRepo {
    // console.log(JSON.parse(patientsJson)); // Display the patient data
    pub fn get_all_patients(&self) -> Result<String, String> {
        // Collect all patients into a vector
        let patients: Vec<&Patient> = self.patients.values().collect();

        // Serialize to JSON
        serde_json::to_string(&patients).map_err(|err| err.to_string())
    }

    // Query a specific patient and return it as JSON
    pub fn get_patient(&self, patient_id: &str) -> Result<String, String> {
        self.patients
            .get(patient_id)
            .map(|patient| serde_json::to_string(patient).unwrap()) // Serialize patient to JSON
            .ok_or_else(|| format!("Patient with id {} not found", patient_id))
    }

    // Query studies by patient and return them as JSON
    pub fn get_studies_by_patient(&self, patient_id: &str) -> Result<String, String> {
        let studies: Vec<&StudySet> = self
            .study_sets
            .values()
            .filter(|study| study.patient_id == patient_id)
            .collect();

        serde_json::to_string(&studies).map_err(|err| err.to_string()) // Serialize studies to JSON
    }

    // Query series by study and return them as JSON
    pub fn get_series_by_study(&self, study_id: &str) -> Result<String, String> {
        let series: Vec<&ImageSeries> = self
            .image_series
            .values()
            .filter(|series| series.study_uid == study_id)
            .collect();

        serde_json::to_string(&series).map_err(|err| err.to_string()) // Serialize series to JSON
    }

    // Query images by series and return them as JSON
    pub fn get_images_by_series(&self, series_id: &str) -> Result<String, String> {
        let images: Vec<CTImage> = self
            .ct_images
            .values()
            .filter(|image| image.series_uid == series_id)
            .map(|image| {
                let mut cloned_image = image.clone(); // Clone the CTImage
                cloned_image.pixel_data.clear(); // Clear the pixel_data field
                cloned_image
            })
            .collect();

        serde_json::to_string(&images).map_err(|err| err.to_string()) // Serialize images to JSON
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg(target_arch = "wasm32")]
impl DicomRepo {
    // WASM-specific async implementation
    pub fn generate_ct_volume(&self, image_series_id: &str) -> Result<CTVolume, JsValue> {
        // Call the common function and handle errors
        self.generate_ct_volume_common(image_series_id)
            .map_err(|e| JsValue::from_str(&e))
    }
}
