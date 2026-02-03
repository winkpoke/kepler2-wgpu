#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    LatchingError(u16),    // 锁定错误，包含错误代码
    NonLatchingError(u16),  // 非锁定错误，包含错误代码
    Warning(u16),          // 警告，包含警告代码
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorError {
    pub error_type: ErrorType,
    pub description: String,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GeneratorPhase {
    Initialization,
    Ready,
    ErrorPhase(Vec<GeneratorError>),
    Operation,
    Shutdown,
}

pub struct ErrorValidator {
    current_phase: GeneratorPhase,
    errors: Vec<GeneratorError>,
    warnings: Vec<GeneratorError>,
}

impl ErrorValidator {
    pub fn new() -> Self {
        Self {
            current_phase: GeneratorPhase::Initialization,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    // 报告错误
    pub fn report_error(&mut self, error_type: ErrorType, description: String) {
        let error = GeneratorError {
            error_type: error_type.clone(),
            description,
            timestamp: std::time::SystemTime::now(),
        };

        match error_type {
            ErrorType::LatchingError(_) => {
                self.errors.push(error.clone());
                self.enter_error_phase();
            }
            ErrorType::NonLatchingError(_) => {
                self.errors.push(error);
                // 非锁定错误不改变阶段
            }
            ErrorType::Warning(_) => {
                self.warnings.push(error);
            }
        }
    }

    fn enter_error_phase(&mut self) {
        if !matches!(self.current_phase, GeneratorPhase::ErrorPhase(_)) {
            self.current_phase = GeneratorPhase::ErrorPhase(self.errors.clone());
        }
    }

    pub fn clear_non_latching_errors(&mut self) {
        self.errors.retain(|error| {
            if let ErrorType::NonLatchingError(_) = error.error_type {
                false
            } else {
                true
            }
        });
    }

    pub fn get_error_status(&self) -> Vec<String> {
        let mut status = Vec::new();
        
        for error in &self.errors {
            let error_code = match &error.error_type {
                ErrorType::LatchingError(code) => format!("EL{:03}", code),
                ErrorType::NonLatchingError(code) => format!("ER{:03}", code),
                ErrorType::Warning(code) => format!("EW{:03}", code),
            };
            status.push(format!("{}: {}", error_code, error.description));
        }
        
        status
    }
}

/// Lookup error message by error code
pub fn get_error_message(code: u16) -> &'static str {
    match code {
        3 => "Generator CPU EEPROM Data Checksum Error",
        4 => "Generator CPU Real Time Clock error",
        5 => "Main Contactor error",
        6 => "Rotor Fault",
        7 => "Filament Fault",
        9 => "Beam_Cathode Fault",
        10 => "Beam_Anode Fault",
        11 => "Beam_INVA Fault",
        12 => "Beam_INVB Fault",
        13 => "Beam_KV Fault",
        14 => "Beam_IR Fault",
        15 => "Beam KV too low.",
        16 => "Bean kv unbalance",
        17 => "Inverter is too hot",
        18 => "Preparation Time-out Error",
        20 => "No KV during exposure",
        21 => "mA during exposure too high",
        22 => "mA during exposure too low",
        23 => "Manually Terminated Exposure",
        24 => "AEC Back-up Timer - Exposure Terminated",
        25 => "AEC MAS Exceeded - Exposure Terminated",
        27 => "Anode Heat Limit",
        28 => "Thermal Switch Interlock Error",
        29 => "Door Interlock Error",
        31 => "Bucky 1 Not Contact Error",
        33 => "Bucky 2 Not Contact Error",
        34 => "Prep Input active during Initialization Phase",
        35 => "X-ray Input active during Initialization Phase",
        36 => "Communication Error Console",
        37 => "+12VDC Error",
        38 => "-12VDC Error",
        43 => "High Voltage Error - KV detected in non x-ray state",
        44 => "Invalid Communication Message",
        45 => "Communication Message Not Supported",
        46 => "Communication Message Not Allowed",
        48 => "Current reception is not enabled",
        49 => "AEC channel is not enable in current reception",
        51 => "AEC Feedback Error (No Feedback Signal Detected)",
        52 => "High Small Focus Filament Current Error in Standby",
        53 => "High Large Focus Filament Current Error in Standby",
        54 => "AEC Reference out of range",
        55 => "No Fields Selected in AEC mode",
        56 => "No Tube Programmed",
        57 => "AEC Stop signal in wrong state",
        60 => "High KV Error",
        61 => "Low KV Error",
        71 => "Boost filament current error",
        72 => "Preheat filament current error",
        73 => "Film screen is invalid",
        74 => "DC BUS voltage is too higher or too low",
        75 => "Tube count data corrupt",
        82 => "INV1 Error",
        83 => "INV2 Error",
        84 => "INV3 Error",
        100 => "Calibration Error - Maximum mA Exceeded",
        101 => "Calibration Error - Calibration Data Table Exceeded",
        102 => "Calibration Error - Maximum Filament Current Exceeded",
        103 => "Calibration Error - Manually Terminated",
        104 => "Calibration Error - No mA",
        105 => "Calibration Error - Minimum mA not calibrated",
        106 => "Generator Limit, Selected Parameter Not calibrated",
        107 => "pre-charge relay fault",
        108 => "large filament set parameter is more than max. filament current",
        109 => "small filament set parameter is more than max. filament current.",
        200 => "Anode Warning Level Exceeded",
        202 => "Generator KW Limit",
        203 => "Generator KV Limit",
        204 => "Generator MA Limit",
        205 => "Generator MS Limit",
        206 => "Generator MAS Limit",
        207 => "Tube KW Limit",
        208 => "Tube KV Limit",
        209 => "Tube MA Limit",
        210 => "Tube MAS Limit",
        212 => "Generator AEC Density Limit",
        213 => "Invalid Communication Parameter",
        214 => "Housing Heat Warning2",
        _ => "Unknown Error",
    }
}
