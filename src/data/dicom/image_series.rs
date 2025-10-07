use anyhow::{Result, anyhow};
use dicom_object::{FileDicomObject, InMemDicomObject};
use crate::define_dicom_struct;
use super::dicom_helper::get_value;


// Use the macro to define the ImageSeries struct
define_dicom_struct!(ImageSeries, {
    (uid, String, "(0020,000E) SeriesInstanceUID", false),  // SeriesID is required
    (study_uid, String, "(0020,000D) StudyInstanceUID", false),     // StudyInstanceUID is required
    (modality, String, "(0008,0060) Modality", false),     // Modality is required
    (description, String, "(0008,103E) SeriesDescription", true) // SeriesDescription is optional
});

impl ImageSeries {
    // Function to parse the DICOM file and generate the ImageSeries structure
    pub fn from_bytes(dicom_data: &[u8]) -> Result<ImageSeries> {
        // Parse the DICOM file into a `FileDicomObject`
        let dicom_obj: FileDicomObject<InMemDicomObject> =
            FileDicomObject::from_reader(dicom_data)?;

        // Retrieve required fields using `get_value`
        let series_uid = get_value::<String>(&dicom_obj, "SeriesInstanceUID")
            .ok_or_else(|| anyhow!("Missing SeriesInstanceUID"))?;
        let studyset_uid = get_value::<String>(&dicom_obj, "StudyInstanceUID")
            .ok_or_else(|| anyhow!("Missing StudyInstanceUID"))?;
        let modality = get_value::<String>(&dicom_obj, "Modality")
            .ok_or_else(|| anyhow!("Missing Modality"))?;

        // Ensure the modality is "CT"
        if modality != "CT" {
            return Err(anyhow!("Expected CT image, but got {} image", modality).into());
        }

        // Optional fields
        let description = get_value::<String>(&dicom_obj, "SeriesDescription");

        // Return the populated struct
        Ok(ImageSeries {
            uid: series_uid,
            study_uid: studyset_uid,
            modality,
            description,
        })
    }
}