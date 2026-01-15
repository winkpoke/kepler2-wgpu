use crate::data::medical_imaging::metadata::image_info::{PatientPosition, create_patient_position};
use anyhow::{Result, anyhow};
use log::{info, warn, debug};

/// Enhanced patient position parser for DICOM CT images
/// Supports various patient positioning formats and provides robust parsing
pub struct PatientPositionParser;

impl PatientPositionParser {
    /// Parse patient position from DICOM PatientPosition tag (0018,5100)
    /// 
    /// # Arguments
    /// * `position_str` - The raw PatientPosition string from DICOM
    /// 
    /// # Returns
    /// * `Result<PatientPosition>` - Parsed and validated patient position
    /// 
    /// # Function-level comment
    /// This function handles various formats of patient position strings including
    /// standard DICOM positions (HFS, HFP, etc.) and anatomical orientations (RAI, LPS, etc.)
    pub fn parse_position(position_str: Option<String>) -> Result<PatientPosition> {
        match position_str {
            Some(pos_str) => {
                let trimmed = pos_str.trim().to_uppercase();
                
                if trimmed.is_empty() {
                    warn!("Empty PatientPosition string, defaulting to HFS");
                    return Ok(PatientPosition::HFS);
                }

                debug!("Parsing PatientPosition: '{}'", trimmed);
                
                // Try direct position parsing first
                let position = create_patient_position(&trimmed);
                
                // Log the parsed result
                match position {
                    PatientPosition::Unknown => {
                        warn!("Unknown PatientPosition '{}', attempting fallback parsing", trimmed);
                        Self::parse_fallback_position(&trimmed)
                    }
                    _ => {
                        info!("Successfully parsed PatientPosition: {} -> {:?}", trimmed, position);
                        Ok(position)
                    }
                }
            }
            None => {
                warn!("Missing PatientPosition tag, defaulting to HFS");
                Ok(PatientPosition::HFS)
            }
        }
    }

    /// Fallback parsing for non-standard position formats
    /// 
    /// # Function-level comment
    /// Attempts to parse position strings that don't match standard formats
    /// by analyzing common patterns and abbreviations
    fn parse_fallback_position(position_str: &str) -> Result<PatientPosition> {
        // Handle common variations and abbreviations
        let normalized = position_str.replace(['-', '_', ' '], "");
        
        match normalized.as_str() {
            // Head first variations
            "HEADFIRSTSUPINE" | "HEADFIRST" | "HF" => Ok(PatientPosition::HFS),
            "HEADFIRSTPRONE" => Ok(PatientPosition::HFP),
            "HEADFIRSTRIGHT" | "HEADFIRSTDECUBITUSRIGHT" => Ok(PatientPosition::HFDR),
            "HEADFIRSTLEFT" | "HEADFIRSTDECUBITUSLEFT" => Ok(PatientPosition::HFDL),
            
            // Feet first variations
            "FEETFIRSTSUPINE" | "FEETFIRST" | "FF" => Ok(PatientPosition::FFS),
            "FEETFIRSTPRONE" => Ok(PatientPosition::FFP),
            "FEETFIRSTRIGHT" | "FEETFIRSTDECUBITUSRIGHT" => Ok(PatientPosition::FFDR),
            "FEETFIRSTLEFT" | "FEETFIRSTDECUBITUSLEFT" => Ok(PatientPosition::FFDL),
            
            // Supine/Prone only
            "SUPINE" | "SUP" => Ok(PatientPosition::HFS), // Default to head first
            "PRONE" | "PR" => Ok(PatientPosition::HFP),   // Default to head first
            
            // If still unknown, default to HFS with warning
            _ => {
                warn!("Could not parse PatientPosition '{}', defaulting to HFS", position_str);
                Ok(PatientPosition::HFS)
            }
        }
    }

    /// Validate patient position consistency with image orientation
    /// 
    /// # Arguments
    /// * `position` - The parsed patient position
    /// * `image_orientation` - Optional image orientation patient values
    /// 
    /// # Function-level comment
    /// Validates that the patient position is consistent with the image orientation
    /// and provides warnings for potential inconsistencies
    pub fn validate_position_consistency(
        position: &PatientPosition,
        image_orientation: Option<(f32, f32, f32, f32, f32, f32)>
    ) -> Result<()> {
        if let Some(orientation) = image_orientation {
            let (row_x, row_y, row_z, col_x, col_y, col_z) = orientation;
            
            // Calculate slice direction (cross product of row and column directions)
            let slice_x = row_y * col_z - row_z * col_y;
            let slice_y = row_z * col_x - row_x * col_z;
            let slice_z = row_x * col_y - row_y * col_x;
            
            debug!("Image orientation - Row: ({:.3}, {:.3}, {:.3}), Col: ({:.3}, {:.3}, {:.3}), Slice: ({:.3}, {:.3}, {:.3})",
                   row_x, row_y, row_z, col_x, col_y, col_z, slice_x, slice_y, slice_z);
            
            // Validate consistency based on expected orientations for each position
            match position {
                PatientPosition::HFS => {
                    // Head First Supine: expect slice direction pointing superior-inferior
                    if slice_z.abs() < 0.5 {
                        warn!("PatientPosition HFS but slice direction doesn't align with S-I axis");
                    }
                }
                PatientPosition::HFP => {
                    // Head First Prone: expect slice direction pointing superior-inferior
                    if slice_z.abs() < 0.5 {
                        warn!("PatientPosition HFP but slice direction doesn't align with S-I axis");
                    }
                }
                PatientPosition::FFS | PatientPosition::FFP => {
                    // Feet First: expect slice direction pointing inferior-superior
                    if slice_z.abs() < 0.5 {
                        warn!("PatientPosition feet-first but slice direction doesn't align with I-S axis");
                    }
                }
                PatientPosition::HFDR | PatientPosition::HFDL | 
                PatientPosition::FFDR | PatientPosition::FFDL => {
                    // Decubitus positions: expect slice direction in lateral plane
                    if slice_x.abs() < 0.5 && slice_y.abs() < 0.5 {
                        warn!("PatientPosition decubitus but slice direction doesn't align with lateral axis");
                    }
                }
                PatientPosition::Unknown => {
                    warn!("Unknown patient position, cannot validate orientation consistency");
                }
            }
        }
        
        Ok(())
    }

    /// Get expected coordinate system transformation for a given patient position
    /// 
    /// # Arguments
    /// * `position` - The patient position
    /// 
    /// # Returns
    /// * `(bool, bool, bool)` - Flags indicating if X, Y, Z axes should be flipped
    /// 
    /// # Function-level comment
    /// Returns coordinate system transformation flags based on patient position
    /// to ensure consistent anatomical orientation in the reconstructed volume
    pub fn get_coordinate_transform(position: &PatientPosition) -> (bool, bool, bool) {
        match position {
            PatientPosition::HFS => (false, false, false), // Standard orientation
            PatientPosition::HFP => (true, false, false),  // Flip X for prone
            PatientPosition::FFS => (false, true, true),   // Flip Y and Z for feet first
            PatientPosition::FFP => (true, true, true),    // Flip all for feet first prone
            PatientPosition::HFDR => (false, true, false), // Flip Y for right decubitus
            PatientPosition::HFDL => (false, false, false), // No flip for left decubitus
            PatientPosition::FFDR => (false, true, true),  // Flip Y and Z for feet first right
            PatientPosition::FFDL => (false, false, true), // Flip Z for feet first left
            PatientPosition::Unknown => {
                warn!("Unknown patient position, using default coordinate transform");
                (false, false, false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_position_parsing() {
        assert!(matches!(
            PatientPositionParser::parse_position(Some("HFS".to_string())).unwrap(),
            PatientPosition::HFS
        ));
        
        assert!(matches!(
            PatientPositionParser::parse_position(Some("HFP".to_string())).unwrap(),
            PatientPosition::HFP
        ));
        
        assert!(matches!(
            PatientPositionParser::parse_position(Some("FFS".to_string())).unwrap(),
            PatientPosition::FFS
        ));
    }

    #[test]
    fn test_fallback_position_parsing() {
        assert!(matches!(
            PatientPositionParser::parse_position(Some("HEAD FIRST SUPINE".to_string())).unwrap(),
            PatientPosition::HFS
        ));
        
        assert!(matches!(
            PatientPositionParser::parse_position(Some("SUPINE".to_string())).unwrap(),
            PatientPosition::HFS
        ));
    }

    #[test]
    fn test_missing_position() {
        assert!(matches!(
            PatientPositionParser::parse_position(None).unwrap(),
            PatientPosition::HFS
        ));
    }

    #[test]
    fn test_coordinate_transform() {
        let (flip_x, flip_y, flip_z) = PatientPositionParser::get_coordinate_transform(&PatientPosition::HFS);
        assert!(!flip_x && !flip_y && !flip_z);
        
        let (flip_x, flip_y, flip_z) = PatientPositionParser::get_coordinate_transform(&PatientPosition::HFP);
        assert!(flip_x && !flip_y && !flip_z);
    }
}