use serde::{Deserialize, Serialize};

// --- Protocol Constants ---
const ETX: u8 = 0x03;
const NAK: u8 = 0x15;
const ACK: u8 = 0x06;

// --- Data Structures ---

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LogColor {
    Info,   // White
    Rx,     // Green (Routine receive)
    Alert,  // Red (Critical status PR/XR/EL)
    Tx,     // Blue (Send)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemedyEvent {
    Connected(String),
    Disconnected,
    Error(String),
    Log { text: String, color: LogColor },
    DataUpdate(String, String), // Key, Value
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub kv: String,
    pub ma: String,
    pub ms: String,
    pub mx: String,
    pub status: String,
    pub anode_heat: String,
    pub housing_heat: String,
    pub post_mas: String,
    pub post_time: String,
    pub workstation: String,
    pub focus: String,
    pub technique: String,
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            kv: "N/A".to_string(),
            ma: "N/A".to_string(),
            ms: "N/A".to_string(),
            mx: "N/A".to_string(),
            status: "N/A".to_string(),
            anode_heat: "N/A".to_string(),
            housing_heat: "N/A".to_string(),
            post_mas: "N/A".to_string(),
            post_time: "N/A".to_string(),
            workstation: "N/A".to_string(),
            focus: "N/A".to_string(),
            technique: "N/A".to_string(),
        }
    }
}

// --- Helper Functions ---

fn calculate_checksum(data: &[u8]) -> u8 {
    let mut sum: usize = 0;
    for &b in data {
        sum += b as usize;
    }
    (sum & 0xFF) as u8
}

pub fn build_packet(cmd: &str) -> Vec<u8> {
    let mut packet = cmd.as_bytes().to_vec();
    packet.push(ETX);
    let checksum = calculate_checksum(&packet);
    packet.push(checksum);
    packet
}

// --- Protocol Handler ---

pub struct RemedyProtocol {
    buffer: Vec<u8>,
    is_standby: bool,
    pub system_state: SystemState,
}

impl RemedyProtocol {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            is_standby: true,
            system_state: SystemState::default(),
        }
    }

    pub fn process_input(&mut self, input: &[u8]) -> (Vec<RemedyEvent>, Vec<Vec<u8>>) {
        let mut events = Vec::new();
        let mut replies = Vec::new();

        self.buffer.extend_from_slice(input);

        loop {
            if self.buffer.is_empty() {
                break;
            }

            // 1. Handle control characters
            if self.buffer[0] == NAK || self.buffer[0] == ACK {
                self.buffer.remove(0);
                continue;
            }

            // 2. DataBag (0x01) - Skip logic
            if self.buffer[0] == 0x01 {
                if self.buffer.len() < 6 {
                    break;
                } // Wait for more data
                let len = ((self.buffer[2] as usize) << 8) | (self.buffer[3] as usize);
                let total_len = 2 + 2 + len + 1 + 1;
                if self.buffer.len() < total_len {
                    break;
                }
                self.buffer.drain(0..total_len);
                continue;
            }

            // 3. ASCII Protocol
            if let Some(etx_idx) = self.buffer.iter().position(|&x| x == ETX) {
                if self.buffer.len() <= etx_idx + 1 {
                    break;
                } // Wait for checksum

                let frame = self.buffer[0..=etx_idx + 1].to_vec();
                let checksum_calc = calculate_checksum(&frame[0..frame.len() - 1]);
                let checksum_recv = frame[frame.len() - 1];

                if checksum_calc == checksum_recv {
                    // Parse success
                    let msg_bytes = &frame[0..etx_idx]; // Remove ETX and checksum
                    let msg_str = String::from_utf8_lossy(msg_bytes).to_string();

                    // --- Core Business Logic ---
                    let mut reply_needed = false;
                    let mut reset_needed = false;

                    // Hand Switch PR
                    if msg_str.starts_with("PR") {
                        reply_needed = true;
                        let status = &msg_str[2..];
                        if status == "1" || status == "2" {
                            self.is_standby = false;
                            let log_text = if status == "1" {
                                format!(">>> [Step 1] Hand Switch Pressed (PR1) - Waiting for Ready...")
                            } else {
                                format!(">>> [Status] Anode Start Complete (PR2) - Please Press Step 2!")
                            };
                            events.push(RemedyEvent::Log {
                                text: log_text,
                                color: if status == "1" {
                                    LogColor::Rx
                                } else {
                                    LogColor::Alert
                                },
                            });
                        } else if status == "0" {
                            self.is_standby = true;
                            events.push(RemedyEvent::Log {
                                text: format!("<<< [Reset] Back to Standby (PR0)"),
                                color: LogColor::Rx,
                            });
                        }
                    }
                    // Exposure XR
                    else if msg_str.starts_with("XR") {
                        reply_needed = true;
                        self.is_standby = false;
                        let status = &msg_str[2..];
                        if status == "0" {
                            self.is_standby = true;
                            events.push(RemedyEvent::Log {
                                text: format!("<<< [End] Exposure Complete (XR0)"),
                                color: LogColor::Rx,
                            });
                        } else if status == "2" {
                            events.push(RemedyEvent::Log {
                                text: format!("!!! [Step 2] Exposing (XR2) !!!"),
                                color: LogColor::Alert,
                            });
                        } else {
                            events.push(RemedyEvent::Log {
                                text: format!("!!! [Step 2] Exposure Maintaining (XR1) ..."),
                                color: LogColor::Alert,
                            });
                        }
                    }
                    // Error EL
                    else if msg_str.starts_with("EL") {
                        reply_needed = true;
                        reset_needed = true;
                        self.is_standby = true;
                        events.push(RemedyEvent::Log {
                            text: format!("!!! Interlock Error {} - Auto Reset", msg_str),
                            color: LogColor::Alert,
                        });
                    }
                    // Warning ER/EW
                    else if msg_str.starts_with("ER") || msg_str.starts_with("EW") {
                        reply_needed = true;
                        events.push(RemedyEvent::Log {
                            text: format!("!!! Error Warning {}", msg_str),
                            color: LogColor::Alert,
                        });
                    }
                    // Others (ST etc.)
                    else {
                        // Only log non-ST or ST with content
                        if !msg_str.starts_with("ST") || msg_str.len() > 2 {
                            events.push(RemedyEvent::Log {
                                text: format!("RX <<< {}", msg_str),
                                color: LogColor::Rx,
                            });
                        }

                        // Parse data for UI update
                        if msg_str.len() > 2 {
                            let prefix = &msg_str[0..2];
                            let val = &msg_str[2..];
                            events.push(RemedyEvent::DataUpdate(prefix.to_string(), val.to_string()));
                            self.update_data(prefix, val);
                        }
                    }

                    // Auto Reply (Ack)
                    if reply_needed {
                        replies.push(frame.clone());
                    }
                    if reset_needed {
                        replies.push(build_packet("RE"));
                    }

                    // Remove processed frame from buffer
                    self.buffer.drain(0..=etx_idx + 1);
                } else {
                    // Checksum error, remove first byte and retry
                    self.buffer.remove(0);
                }
            } else {
                // No ETX, wait for more data
                break;
            }
        }

        (events, replies)
    }

    fn update_data(&mut self, key: &str, val: &str) {
        let s = &mut self.system_state;
        match key {
            "KV" => s.kv = val.to_string(),
            "MA" | "MS" | "MX" | "AP" | "AT" => {
                // These values are 1/10, need conversion
                if let Ok(num) = val.parse::<f32>() {
                    let formatted = format!("{:.1}", num / 10.0);
                    match key {
                        "MA" => s.ma = formatted,
                        "MS" => s.ms = formatted,
                        "MX" => s.mx = formatted,
                        "AP" => s.post_mas = formatted,
                        "AT" => s.post_time = formatted,
                        _ => {}
                    }
                }
            }
            "HE" => s.anode_heat = format!("{}%", val),
            "HH" => s.housing_heat = format!("{}%", val),
            "WS" => s.workstation = format!("Station {}", val),
            "FO" => s.focus = match val {
                "0" => "Small".to_string(),
                "1" => "Large".to_string(),
                "2" => "Micro".to_string(),
                _ => val.to_string(),
            },
            "ET" => s.technique = match val {
                "0" => "mA/Time".to_string(),
                "1" => "mAs".to_string(),
                "2" => "AEC".to_string(),
                _ => val.to_string(),
            },
            "ST" => s.status = match val {
                "001" => "Init".to_string(),
                "002" => "Standby".to_string(),
                "003" => "Prep".to_string(),
                "004" => "Ready".to_string(),
                "005" => "Exp".to_string(),
                "008" => "Error".to_string(),
                _ => val.to_string(),
            },
            _ => {}
        }
    }
}

// --- WASM Interface ---

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct RemedyWasm {
    protocol: RemedyProtocol,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl RemedyWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            protocol: RemedyProtocol::new(),
        }
    }

    /// Process incoming bytes from Serial Port (passed as Uint8Array from JS)
    pub fn process_input(&mut self, input: &[u8]) -> JsValue {
        let (events, replies) = self.protocol.process_input(input);
        
        let result = serde_wasm_bindgen::to_value(&ProcessResult {
            events,
            replies,
        }).unwrap();
        
        result
    }

    /// Build a command packet to send to Serial Port
    pub fn build_command(&self, cmd: &str) -> Vec<u8> {
        build_packet(cmd)
    }

    /// Get current system state
    pub fn get_state(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.protocol.system_state).unwrap()
    }
}

#[derive(Serialize)]
struct ProcessResult {
    events: Vec<RemedyEvent>,
    replies: Vec<Vec<u8>>,
}
