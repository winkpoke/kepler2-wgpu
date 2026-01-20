use super::dicom_helper::get_value;
use crate::define_dicom_struct;
use anyhow::{anyhow, Result};
use dicom_object::{FileDicomObject, InMemDicomObject};

// Use the macro to define the StudySet struct
define_dicom_struct!(StudySet, {
    (study_id, String, "(0020,0010) StudyID", false),           // StudyID is required
    (uid, String, "(0020,000D) StudyInstanceUID", false),     // StudyInstanceUID is required
    (patient_id, String, "(0010,0020) PatientID", false),           // PatientID is required
    (date, String, "(0008,0020) StudyDate", false),       // StudyDate is required
    (description, String, "(0008,1030) StudyDescription", true) // StudyDescription is optional
});

impl StudySet {
    // Function to parse the DICOM file and generate the StudySet structure
    pub fn from_bytes(dicom_data: &[u8]) -> Result<StudySet> {
        // Parse the DICOM file into a `FileDicomObject`
        let dicom_obj: FileDicomObject<InMemDicomObject> =
            FileDicomObject::from_reader(dicom_data)?;

        // Retrieve required fields using `get_value`
        let id =
            get_value::<String>(&dicom_obj, "StudyID").ok_or_else(|| anyhow!("Missing StudyID"))?;
        let uid = get_value::<String>(&dicom_obj, "StudyInstanceUID")
            .ok_or_else(|| anyhow!("Missing StudyInstanceUID"))?;
        let patient_id = get_value::<String>(&dicom_obj, "PatientID")
            .ok_or_else(|| anyhow!("Missing PatientID"))?;
        let date = get_value::<String>(&dicom_obj, "StudyDate")
            .ok_or_else(|| anyhow!("Missing StudyDate"))?;

        // Optional fields
        let description = get_value::<String>(&dicom_obj, "StudyDescription");

        // Return the populated struct
        Ok(StudySet {
            study_id: id,
            uid,
            patient_id,
            date,
            description,
        })
    }
}
