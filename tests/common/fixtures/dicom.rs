use kepler_wgpu::data::dicom::{CTImage, ImageSeries, Patient, StudySet};
use kepler_wgpu::data::medical_imaging::metadata::PatientPosition;

#[derive(Debug, Clone)]
pub struct DicomFixtureBuilder {
    patient_id: String,
    patient_name: String,
    study_id: String,
    study_uid: String,
    series_uid: String,
    modality: String,
    image_count: usize,
    dimensions: (u16, u16),
    pixel_spacing: Option<(f32, f32)>,
    slice_thickness: Option<f32>,
    spacing_between_slices: Option<f32>,
    pixel_representation: u16,
    patient_position: Option<PatientPosition>,
    rescale_slope: Option<f32>,
    rescale_intercept: Option<f32>,
}

impl Default for DicomFixtureBuilder {
    fn default() -> Self {
        Self {
            patient_id: "TEST_PATIENT_001".to_string(),
            patient_name: "Test^Patient".to_string(),
            study_id: "STUDY_001".to_string(),
            study_uid: "1.2.840.113619.2.55.3.604598.2.1213376976.999.123456789".to_string(),
            series_uid: "1.2.840.113619.2.55.3.604598.2.1213376976.999.123456790".to_string(),
            modality: "CT".to_string(),
            image_count: 10,
            dimensions: (512, 512),
            pixel_spacing: Some((1.0, 1.0)),
            slice_thickness: Some(2.0),
            spacing_between_slices: Some(2.0),
            pixel_representation: 1,
            patient_position: Some(PatientPosition::HFS),
            rescale_slope: Some(1.0),
            rescale_intercept: Some(-1024.0),
        }
    }
}

impl DicomFixtureBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn patient_id(mut self, id: impl Into<String>) -> Self {
        self.patient_id = id.into();
        self
    }

    pub fn patient_name(mut self, name: impl Into<String>) -> Self {
        self.patient_name = name.into();
        self
    }

    pub fn study_id(mut self, id: impl Into<String>) -> Self {
        self.study_id = id.into();
        self
    }

    pub fn study_uid(mut self, uid: impl Into<String>) -> Self {
        self.study_uid = uid.into();
        self
    }

    pub fn series_uid(mut self, uid: impl Into<String>) -> Self {
        self.series_uid = uid.into();
        self
    }

    pub fn modality(mut self, modality: impl Into<String>) -> Self {
        self.modality = modality.into();
        self
    }

    pub fn image_count(mut self, count: usize) -> Self {
        self.image_count = count;
        self
    }

    pub fn dimensions(mut self, dimensions: (u16, u16)) -> Self {
        self.dimensions = dimensions;
        self
    }

    pub fn pixel_spacing(mut self, spacing: (f32, f32)) -> Self {
        self.pixel_spacing = Some(spacing);
        self
    }

    pub fn no_pixel_spacing(mut self) -> Self {
        self.pixel_spacing = None;
        self
    }

    pub fn slice_thickness(mut self, thickness: f32) -> Self {
        self.slice_thickness = Some(thickness);
        self
    }

    pub fn no_slice_thickness(mut self) -> Self {
        self.slice_thickness = None;
        self
    }

    pub fn spacing_between_slices(mut self, spacing: f32) -> Self {
        self.spacing_between_slices = Some(spacing);
        self
    }

    pub fn no_spacing_between_slices(mut self) -> Self {
        self.spacing_between_slices = None;
        self
    }

    pub fn pixel_representation(mut self, representation: u16) -> Self {
        self.pixel_representation = representation;
        self
    }

    pub fn patient_position(mut self, position: PatientPosition) -> Self {
        self.patient_position = Some(position);
        self
    }

    pub fn no_patient_position(mut self) -> Self {
        self.patient_position = None;
        self
    }

    pub fn rescale_slope(mut self, slope: f32) -> Self {
        self.rescale_slope = Some(slope);
        self
    }

    pub fn no_rescale_slope(mut self) -> Self {
        self.rescale_slope = None;
        self
    }

    pub fn rescale_intercept(mut self, intercept: f32) -> Self {
        self.rescale_intercept = Some(intercept);
        self
    }

    pub fn no_rescale_intercept(mut self) -> Self {
        self.rescale_intercept = None;
        self
    }

    pub fn build_patient(&self) -> Patient {
        Patient::new(
            self.patient_id.clone(),
            self.patient_name.clone(),
            Some("19850101".to_string()),
            Some("M".to_string()),
        )
    }

    pub fn build_study(&self) -> StudySet {
        StudySet::new(
            self.study_id.clone(),
            self.study_uid.clone(),
            self.patient_id.clone(),
            "20240101".to_string(),
            Some("Test Study".to_string()),
        )
    }

    pub fn build_series(&self) -> ImageSeries {
        ImageSeries::new(
            self.series_uid.clone(),
            self.study_uid.clone(),
            self.modality.clone(),
            Some("Test Series".to_string()),
        )
    }

    pub fn build_images(&self) -> Vec<CTImage> {
        let mut images = Vec::with_capacity(self.image_count);
        let (rows, columns) = self.dimensions;

        for i in 0..self.image_count {
            let z_position = (i as f32) * self.spacing_between_slices.unwrap_or(1.0);

            let pixel_data = self.generate_synthetic_pixel_data(rows, columns, i);

            images.push(CTImage::new(
                format!("{}.{}", self.series_uid, i),
                self.series_uid.clone(),
                rows,
                columns,
                self.pixel_spacing,
                self.slice_thickness,
                self.spacing_between_slices,
                Some((0.0, 0.0, z_position)),
                Some((1.0, 0.0, 0.0, 0.0, 1.0, 0.0)),
                self.patient_position.clone().map(|p| p.to_string()),
                self.rescale_slope,
                self.rescale_intercept,
                Some(40.0),
                Some(400.0),
                self.pixel_representation,
                pixel_data,
            ));
        }

        images
    }

    pub fn build_complete_fixture(&self) -> (Patient, StudySet, ImageSeries, Vec<CTImage>) {
        (
            self.build_patient(),
            self.build_study(),
            self.build_series(),
            self.build_images(),
        )
    }

    fn generate_synthetic_pixel_data(
        &self,
        rows: u16,
        columns: u16,
        slice_index: usize,
    ) -> Vec<u8> {
        let num_pixels = rows as usize * columns as usize;
        let mut pixel_data = Vec::with_capacity(num_pixels * 2);

        for y in 0..rows {
            for x in 0..columns {
                let base_value = (slice_index as i16) * 100;
                let x_value = (x as i16) / 10;
                let y_value = (y as i16) / 10;
                let value = base_value + x_value + y_value;

                let bytes = value.to_le_bytes();
                pixel_data.push(bytes[0]);
                pixel_data.push(bytes[1]);
            }
        }

        pixel_data
    }
}

pub fn create_minimal_fixture() -> (Patient, StudySet, ImageSeries, Vec<CTImage>) {
    DicomFixtureBuilder::new()
        .image_count(5)
        .dimensions((64, 64))
        .build_complete_fixture()
}

pub fn create_standard_ct_volume_fixture() -> (Patient, StudySet, ImageSeries, Vec<CTImage>) {
    DicomFixtureBuilder::new()
        .image_count(10)
        .dimensions((512, 512))
        .pixel_spacing((1.0, 1.0))
        .slice_thickness(2.0)
        .spacing_between_slices(2.0)
        .build_complete_fixture()
}

pub fn create_missing_optional_fields_fixture() -> (Patient, StudySet, ImageSeries, Vec<CTImage>) {
    DicomFixtureBuilder::new()
        .image_count(3)
        .dimensions((128, 128))
        .no_pixel_spacing()
        .no_slice_thickness()
        .no_spacing_between_slices()
        .no_patient_position()
        .no_rescale_slope()
        .no_rescale_intercept()
        .build_complete_fixture()
}

pub fn create_invalid_modality_fixture() -> (Patient, StudySet, ImageSeries, Vec<CTImage>) {
    DicomFixtureBuilder::new()
        .image_count(3)
        .modality("INVALID")
        .build_complete_fixture()
}

pub fn create_unsigned_pixel_fixture() -> (Patient, StudySet, ImageSeries, Vec<CTImage>) {
    DicomFixtureBuilder::new()
        .image_count(5)
        .pixel_representation(0)
        .build_complete_fixture()
}

pub fn create_rescaled_fixture() -> (Patient, StudySet, ImageSeries, Vec<CTImage>) {
    DicomFixtureBuilder::new()
        .image_count(5)
        .rescale_slope(2.0)
        .rescale_intercept(-1000.0)
        .build_complete_fixture()
}
