use super::dicom_helper::get_value;
use crate::define_dicom_struct;
use anyhow::{anyhow, Result, Context};
use dicom_object::{FileDicomObject, InMemDicomObject};
use std::borrow::Cow;

define_dicom_struct!(CTImage, {
    (uid, String, "(0008,0018) SOPInstanceUID", false),              // Unique identifier for the image
    (series_uid, String, "(0020,000E) SeriesInstanceUID", false),  // SeriesID is required
    (rows, u16, "(0028,0010) Rows", false),                         // Rows (Mandatory)
    (columns, u16, "(0028,0011) Columns", false),                    // Columns (Mandatory)
    (pixel_spacing, (f32, f32), "(0028,0030) PixelSpacing", true),   // PixelSpacing (Optional)
    (slice_thickness, f32, "(0018,0050) SliceThickness", true),      // SliceThickness (Optional)
    (spacing_between_slices, f32, "(0018,0088) SpacingBetweenSlices", true), // SpacingBetweenSlices (Optional)
    (image_position_patient, (f32, f32, f32), "(0020,0032) ImagePositionPatient", true), // ImagePositionPatient (Optional)
    (image_orientation_patient, (f32, f32, f32, f32, f32, f32), "(0020,0037) ImageOrientationPatient", true), // ImageOrientationPatient (Optional)
    (rescale_slope, f32, "(0028,1053) RescaleSlope", true),          // RescaleSlope (Optional)
    (rescale_intercept, f32, "(0028,1052) RescaleIntercept", true),  // RescaleIntercept (Optional)
    (window_center, f32, "(0028,1050) WindowCenter", true),          // WindowCenter (Optional)
    (window_width, f32, "(0028,1051) WindowWidth", true),            // WindowWidth (Optional)
    (pixel_representation, u16, "(0028,0103) PixelRepresentation", false), // Pixel Representation (Mandatory, but important for interpretation)
    (pixel_data, Vec<u8>, "(7FE0,0010) PixelData", false)            // PixelData (Mandatory)
});

impl CTImage {
    // Function to parse the DICOM file and generate the CTImage structure
    pub fn from_bytes(dicom_data: &[u8]) -> Result<CTImage> {
        // Parse the DICOM file into a `FileDicomObject`
        let obj: FileDicomObject<InMemDicomObject> = FileDicomObject::from_reader(dicom_data)?;

        // Populate fields based on DICOM tags
        Ok(CTImage {
            uid: get_value::<String>(&obj, "SOPInstanceUID")
                .ok_or_else(|| anyhow!("Missing SOPInstanceUID"))?,
            series_uid: get_value::<String>(&obj, "SeriesInstanceUID")
                .ok_or_else(|| anyhow!("Missing SeriesInstanceUID"))?,
            rows: get_value::<u16>(&obj, "Rows").ok_or_else(|| anyhow!("Missing Rows"))?,
            columns: get_value::<u16>(&obj, "Columns").ok_or_else(|| anyhow!("Missing Columns"))?,
            pixel_spacing: {
                let spacing = get_value::<String>(&obj, "PixelSpacing");
                spacing.and_then(|v| {
                    let vals: Vec<f32> = v
                        .split('\\')
                        .filter_map(|s| s.parse::<f32>().ok())
                        .collect();
                    if vals.len() == 2 {
                        Some((vals[0], vals[1]))
                    } else {
                        None
                    }
                })
            },
            slice_thickness: get_value::<f32>(&obj, "SliceThickness"),
            spacing_between_slices: get_value::<f32>(&obj, "SpacingBetweenSlices"),
            image_position_patient: {
                let pos = get_value::<String>(&obj, "ImagePositionPatient");
                pos.and_then(|v| {
                    let vals: Vec<f32> = v
                        .split('\\')
                        .filter_map(|s| s.parse::<f32>().ok())
                        .collect();
                    if vals.len() == 3 {
                        Some((vals[0], vals[1], vals[2]))
                    } else {
                        None
                    }
                })
            },
            image_orientation_patient: {
                let orientation = get_value::<String>(&obj, "ImageOrientationPatient");
                orientation.and_then(|v| {
                    let vals: Vec<f32> = v
                        .split('\\')
                        .filter_map(|s| s.parse::<f32>().ok())
                        .collect();
                    if vals.len() == 6 {
                        Some((vals[0], vals[1], vals[2], vals[3], vals[4], vals[5]))
                    } else {
                        None
                    }
                })
            },
            rescale_slope: get_value::<f32>(&obj, "RescaleSlope"),
            rescale_intercept: get_value::<f32>(&obj, "RescaleIntercept"),
            window_center: get_value::<f32>(&obj, "WindowCenter"),
            window_width: get_value::<f32>(&obj, "WindowWidth"),
            pixel_representation: get_value::<u16>(&obj, "PixelRepresentation")
                .ok_or_else(|| anyhow!("Missing PixelRepresentation"))?,
            pixel_data: obj.element_by_name("PixelData")?.to_bytes()?.to_vec(), // Pixel data is mandatory
        })
    }

    pub fn get_pixel_data(&self) -> Result<Vec<i16>> {
        let pixel_data = &self.pixel_data; // Original pixel data as Vec<u8>
        let pixel_representation = self.pixel_representation;
        let rescale_slope = self.rescale_slope.unwrap_or(1.0); // Default to 1.0 if not provided
        let rescale_intercept = self.rescale_intercept.unwrap_or(0.0); // Default to 0.0 if not provided

        // Define a small epsilon for float comparison
        const EPSILON: f32 = 1e-6;

        // Case 1: No rescaling and pixel data is signed (pixel_representation == 1)
        if (rescale_slope - 1.0).abs() < EPSILON && (rescale_intercept - 0.0).abs() < EPSILON && pixel_representation == 1 {
            // No transformation is needed, return the data as borrowed
            let data: Vec<i16> = pixel_data
                .chunks_exact(2)
                .map(|chunk| {
                    if chunk.len() != 2 {
                        anyhow::bail!("Invalid pixel data chunk size: expected 2 bytes, found {}", chunk.len());
                    }
                    Ok(i16::from_ne_bytes([chunk[0], chunk[1]])) // Example for 16-bit signed
                })
                .collect::<Result<_>>()
                .context("Failed to process pixel data from DICOM file")?;

            return Ok(data); 
        }

        // Case 2: Transformation required (rescale or pixel_representation is different)
        let transformed_data: Vec<i16> = pixel_data
            .chunks_exact(2)
            .map(|chunk| {
                if chunk.len() != 2 {
                    anyhow::bail!("Invalid pixel data chunk size: expected 2 bytes, found {}", chunk.len());
                }
                let raw_value = match pixel_representation {
                    0 => u16::from_ne_bytes([chunk[0], chunk[1]]) as i16, // Unsigned to signed conversion
                    1 => i16::from_ne_bytes([chunk[0], chunk[1]]),         // Already signed
                    _ => anyhow::bail!("Unsupported pixel representation: {}", pixel_representation),
                };
                // Apply rescale slope and intercept
                let transformed_value = (raw_value as f32) * rescale_slope + rescale_intercept;
                Ok(transformed_value.round() as i16)
            })
            .collect::<Result<_>>()
            .context("Failed to transform pixel data with rescale slope and intercept")?;

        Ok(transformed_data)
    }
}
