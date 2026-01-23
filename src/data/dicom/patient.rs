use super::dicom_helper::get_value;
use crate::define_dicom_struct;
use anyhow::{anyhow, Result};
use dicom_object::{FileDicomObject, InMemDicomObject};

// Use the macro to define the Patient struct
define_dicom_struct!(Patient, {
    (patient_id, String, "(0010,0020) PatientID", false),           // PatientID is required
    (name, String, "(0010,0010) PatientName", false),       // PatientName is required
    (birthdate, String, "(0010,0030) PatientBirthDate", true),  // PatientBirthDate is optional
    (sex, String, "(0010,0040) PatientSex", true),              // Sex is optional
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
        let patient = Patient {
            patient_id: id,
            name,
            birthdate,
            sex,
        };
        // patient.validate()?; // Skip strict validation to allow display of imperfect data
        Ok(patient)
    }

    /// Validates the patient data against DICOM standards
    pub fn validate(&self) -> Result<()> {
        // Validate PatientID
        if self.patient_id.is_empty() {
            return Err(anyhow!("PatientID cannot be empty"));
        }
        if self.patient_id.len() > 64 {
            return Err(anyhow!("PatientID exceeds 64 characters"));
        }

        // Validate name characters
        if self.name.is_empty() {
            return Err(anyhow!("PatientName cannot be empty"));
        }
        if self.name.contains(['$', '@', '#']) {
            return Err(anyhow!("Invalid character in PatientName"));
        }

        // Validate component length (DICOM PN VR limit is 64 chars per component)
        for component in self.name.split('^') {
            if component.len() > 64 {
                return Err(anyhow!("PatientName component exceeds 64 characters"));
            }
        }

        // Validate birthdate format
        if let Some(date) = &self.birthdate {
            if date.len() != 8 || !date.chars().all(char::is_numeric) {
                return Err(anyhow!("Invalid PatientBirthDate format (expected YYYYMMDD)"));
            }
        }

        // Validate sex
        if let Some(sex) = &self.sex {
            if !["M", "F", "O", ""].contains(&sex.as_str()) {
                return Err(anyhow!("Invalid PatientSex (expected M, F, or O)"));
            }
        }

        Ok(())
    }
}
