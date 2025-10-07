use anyhow::{Result, anyhow};
use dicom_object::{FileDicomObject, InMemDicomObject};
use crate::define_dicom_struct;
use super::dicom_helper::get_value;


// Use the macro to define the Patient struct
define_dicom_struct!(Patient, {
    (patient_id, String, "(0010,0020) PatientID", false),           // PatientID is required
    (name, String, "(0010,0010) PatientName", false),       // PatientName is required
    (birthdate, String, "(0010,0030) PatientBirthDate", true),  // PatientBirthDate is optional
    (sex, String, "(0010,0040) PatientSex", true)              // Sex is optional
});

// Native version for reading DICOM from a file directly (e.g., from the file system)
impl Patient {
    // Function to parse the DICOM file and generate the Patient structure
    pub fn from_bytes(dicom_data: &[u8]) -> Result<Patient> {
        // Parse the DICOM file into a `FileDicomObject`
        let dicom_obj: FileDicomObject<InMemDicomObject> =
            FileDicomObject::from_reader(dicom_data)?;

        // Retrieve required fields using `get_value`
        let id = get_value::<String>(&dicom_obj, "PatientID")
            .ok_or_else(|| anyhow!("Missing PatientID"))?;
        let name = get_value::<String>(&dicom_obj, "PatientName")
            .ok_or_else(|| anyhow!("Missing PatientName"))?;

        // Optional fields
        let birthdate = get_value::<String>(&dicom_obj, "PatientBirthDate");
        let sex = get_value::<String>(&dicom_obj, "PatientSex");

        // Return the populated struct
        Ok(Patient {
            patient_id: id,
            name,
            birthdate,
            sex,
        })
    }
}
