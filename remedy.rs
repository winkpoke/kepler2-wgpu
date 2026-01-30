use eframe::egui;
use serialport::SerialPort;
use std::io::{self, Read, Write};
use std::sync::{atomic::AtomicBool, Arc};
use std::thread;
use std::time::{Duration, Instant};

// --- 协议常量 ---
const ETX: u8 = 0x03;
const NAK: u8 = 0x15;
const ACK: u8 = 0x06;

// --- 数据结构 ---

/// 发送到串口线程的命令
#[derive(Debug, Clone)]
enum SerialCommand {
    Connect(String, String), // TX Port, RX Port
    Disconnect,
    Send(String), // 发送 ASCII 指令
}

/// 从串口线程发回 GUI 的事件
#[derive(Debug, Clone)]
enum GuiEvent {
    Connected(String),
    Disconnected,
    Error(String),
    Log { text: String, color: LogColor },
    DataUpdate(String, String), // Key, Value
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LogColor {
    Info,   // 白色
    Rx,     // 绿色 (常规接收)
    Alert,  // 红色 (关键状态 PR/XR/EL)
    Tx,     // 蓝色 (发送)
}

/// 状态监控数据
struct SystemState {
    kv: String,
    ma: String,
    ms: String,
    mx: String,
    status: String,
    anode_heat: String,
    housing_heat: String,
    post_mas: String,
    post_time: String,
    workstation: String,
    focus: String,
    technique: String,
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

// --- 辅助函数 ---

fn calculate_checksum(data: &[u8]) -> u8 {
    let mut sum: usize = 0;
    for &b in data {
        sum += b as usize;
    }
    (sum & 0xFF) as u8
}

fn build_packet(cmd: &str) -> Vec<u8> {
    let mut packet = cmd.as_bytes().to_vec();
    packet.push(ETX);
    let checksum = calculate_checksum(&packet);
    packet.push(checksum);
    packet
}

// --- 串口后台线程逻辑 ---

fn serial_thread_loop(
    cmd_receiver: crossbeam_channel::Receiver<SerialCommand>,
    event_sender: crossbeam_channel::Sender<GuiEvent>,
) {
    let mut port: Option<Box<dyn SerialPort>> = None;
    let mut buffer: Vec<u8> = Vec::new();
    let mut is_standby = true;
    let mut last_heartbeat = Instant::now();
    
    // 用于控制读取循环的 flag - 此处暂时未使用，预留
    let _running = Arc::new(AtomicBool::new(true));

    loop {
        // 1. 处理 GUI 发来的命令
        let cmd_opt = if port.is_none() {
             cmd_receiver.recv().ok() // 没连接时阻塞等待连接命令
        } else {
             cmd_receiver.try_recv().ok()
        };

        if let Some(cmd) = cmd_opt {
            match cmd {
                SerialCommand::Connect(tx_name, _rx_name) => {
                    // 目前简化逻辑，只开一个端口作为 TX/RX
                    match serialport::new(&tx_name, 19_200)
                        .timeout(Duration::from_millis(10))
                        .open()
                    {
                        Ok(p) => {
                            port = Some(p);
                            is_standby = true;
                            buffer.clear();
                            let _ = event_sender.send(GuiEvent::Connected(tx_name));
                            let _ = event_sender.send(GuiEvent::Log {
                                text: "--- 串口已连接，智能心跳已启动 ---".to_string(),
                                color: LogColor::Info,
                            });
                        }
                        Err(e) => {
                            let _ = event_sender.send(GuiEvent::Error(e.to_string()));
                        }
                    }
                }
                SerialCommand::Disconnect => {
                    port = None;
                    let _ = event_sender.send(GuiEvent::Disconnected);
                    let _ = event_sender.send(GuiEvent::Log {
                        text: "--- 连接已断开 ---".to_string(),
                        color: LogColor::Info,
                    });
                }
                SerialCommand::Send(text) => {
                    if let Some(p) = port.as_mut() {
                        let pkt = build_packet(&text);
                        if let Err(e) = p.write_all(&pkt) {
                            let _ = event_sender.send(GuiEvent::Error(format!("发送失败: {}", e)));
                        } else {
                            // 记录发送日志
                             let _ = event_sender.send(GuiEvent::Log {
                                text: format!("发送命令: {}", text),
                                color: LogColor::Tx,
                            });
                        }
                    }
                }
            }
        }

        // 2. 串口业务逻辑 (仅当连接时)
        if let Some(p) = port.as_mut() {
            // A. 心跳逻辑
            if is_standby && last_heartbeat.elapsed() > Duration::from_secs(1) {
                let pkt = build_packet("ST");
                if p.write_all(&pkt).is_ok() {
                    // 心跳包不打印日志，避免刷屏
                    last_heartbeat = Instant::now();
                }
            }

            // B. 读取数据
            let mut tmp_buf = [0u8; 1024];
            match p.read(&mut tmp_buf) {
                Ok(n) if n > 0 => {
                    buffer.extend_from_slice(&tmp_buf[0..n]);
                    
                    // 处理粘包和解析
                    loop {
                        if buffer.is_empty() { break; }

                        // 1. 处理控制字符
                        if buffer[0] == NAK || buffer[0] == ACK {
                            buffer.remove(0);
                            continue;
                        }
                        // 2. DataBag (0x01) 简单跳过
                        if buffer[0] == 0x01 {
                            if buffer.len() < 6 { break; } // 等待更多数据
                            let len = ((buffer[2] as usize) << 8) | (buffer[3] as usize);
                            let total_len = 2 + 2 + len + 1 + 1;
                            if buffer.len() < total_len { break; }
                            buffer.drain(0..total_len);
                            continue;
                        }

                        // 3. ASCII 协议
                        if let Some(etx_idx) = buffer.iter().position(|&x| x == ETX) {
                            if buffer.len() <= etx_idx + 1 { break; } // 等待 checksum

                            let frame = &buffer[0..=etx_idx + 1];
                            let checksum_calc = calculate_checksum(&frame[0..frame.len()-1]);
                            let checksum_recv = frame[frame.len()-1];

                            if checksum_calc == checksum_recv {
                                // 解析成功
                                let msg_bytes = &frame[0..etx_idx]; // 去掉 ETX 和 checksum
                                let msg_str = String::from_utf8_lossy(msg_bytes).to_string();
                                
                                // --- 核心业务逻辑处理 ---
                                let mut reply_needed = false;
                                let mut reset_needed = false;
                                
                                // 手闸 PR
                                if msg_str.starts_with("PR") {
                                    reply_needed = true;
                                    let status = &msg_str[2..];
                                    if status == "1" || status == "2" {
                                        is_standby = false;
                                        let log_text = if status == "1" { 
                                            format!(">>> [一档] 手闸按下 (PR1) - 等待 Ready...")
                                        } else { 
                                            format!(">>> [状态] 阳极启动完成 (PR2) - 请按二档曝光！")
                                        };
                                        let _ = event_sender.send(GuiEvent::Log { text: log_text, color: if status == "1" { LogColor::Rx } else { LogColor::Alert } });
                                    } else if status == "0" {
                                        is_standby = true;
                                        let _ = event_sender.send(GuiEvent::Log { text: format!("<<< [复位] 回到待机 (PR0)"), color: LogColor::Rx });
                                    }
                                }
                                // 曝光 XR
                                else if msg_str.starts_with("XR") {
                                    reply_needed = true;
                                    is_standby = false;
                                    let status = &msg_str[2..];
                                    if status == "0" {
                                        is_standby = true;
                                        let _ = event_sender.send(GuiEvent::Log { text: format!("<<< [结束] 曝光完成 (XR0)"), color: LogColor::Rx });
                                    } else if status == "2" {
                                        let _ = event_sender.send(GuiEvent::Log { text: format!("!!! [二档] 正在曝光 (XR2) !!!"), color: LogColor::Alert });
                                    } else {
                                        let _ = event_sender.send(GuiEvent::Log { text: format!("!!! [二档] 曝光维持 (XR1) ..."), color: LogColor::Alert });
                                    }
                                }
                                // 错误 EL
                                else if msg_str.starts_with("EL") {
                                    reply_needed = true;
                                    reset_needed = true;
                                    is_standby = true;
                                    let _ = event_sender.send(GuiEvent::Log { text: format!("!!! 收到闭锁错误 {} - 已自动复位", msg_str), color: LogColor::Alert });
                                }
                                // 警告 ER/EW
                                else if msg_str.starts_with("ER") || msg_str.starts_with("EW") {
                                    reply_needed = true;
                                    let _ = event_sender.send(GuiEvent::Log { text: format!("!!! 收到错误警告 {}", msg_str), color: LogColor::Alert });
                                }
                                // 其他 (ST等)
                                else {
                                    // 只有非 ST 或 有内容的 ST 才打印
                                    if !msg_str.starts_with("ST") || msg_str.len() > 2 {
                                        let _ = event_sender.send(GuiEvent::Log { text: format!("RX <<< {}", msg_str), color: LogColor::Rx });
                                    }
                                    
                                    // 解析数据用于更新 UI
                                    if msg_str.len() > 2 {
                                        let prefix = &msg_str[0..2];
                                        let val = &msg_str[2..];
                                        let _ = event_sender.send(GuiEvent::DataUpdate(prefix.to_string(), val.to_string()));
                                    }
                                }

                                // 自动回复 (Ack)
                                if reply_needed {
                                    let _ = p.write_all(frame); // 回发原包
                                }
                                if reset_needed {
                                    let _ = p.write_all(&build_packet("RE"));
                                }

                                // 从 buffer 移除已处理的帧
                                buffer.drain(0..=etx_idx + 1);
                            } else {
                                // 校验和错误，移除第一个字节重试
                                buffer.remove(0);
                            }
                        } else {
                            // 没有 ETX，等待更多数据
                            break;
                        }
                    }
                }
                // 【修复1】 处理读取 0 字节的情况（空读取）
                Ok(_) => {
                     // 没读到数据，什么都不做，继续
                }
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                    // 超时是正常的（非阻塞模式），继续循环
                }
                Err(e) => {
                    let _ = event_sender.send(GuiEvent::Error(e.to_string()));
                    port = None; // 断开连接
                    let _ = event_sender.send(GuiEvent::Disconnected);
                }
            }

            // 短暂休眠防止 CPU 占用过高
            thread::sleep(Duration::from_millis(5));
        } else {
            // 未连接状态，休眠久一点
            thread::sleep(Duration::from_millis(50));
        }
    }
}

// --- GUI 应用程序 ---

struct RemedyApp {
    // 通信通道
    tx_cmd: crossbeam_channel::Sender<SerialCommand>,
    rx_event: crossbeam_channel::Receiver<GuiEvent>,

    // UI 状态
    tx_port_name: String,
    rx_port_name: String,
    connected: bool,
    available_ports: Vec<String>,
    
    // 输入框状态
    kv_input: String,
    ma_input: String,
    ms_input: String,

    // 系统数据
    system_state: SystemState,

    // 日志
    logs: Vec<(String, LogColor)>,
}

impl RemedyApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        // 设置默认字体以支持中文
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "my_font".to_owned(),
            egui::FontData::from_static(include_bytes!("c:/windows/fonts/msyh.ttc")).tweak(
                egui::FontTweak {
                    scale: 1.2, 
                    ..Default::default()
                },
            ),
        );
        
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "my_font".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "my_font".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        let (tx_cmd, rx_cmd) = crossbeam_channel::unbounded();
        let (tx_event, rx_event) = crossbeam_channel::unbounded();

        // 启动后台线程
        thread::spawn(move || {
            serial_thread_loop(rx_cmd, tx_event);
        });

        Self {
            tx_cmd,
            rx_event,
            tx_port_name: "COM2".to_string(),
            rx_port_name: "".to_string(),
            connected: false,
            available_ports: vec![],
            kv_input: "".to_string(),
            ma_input: "".to_string(),
            ms_input: "".to_string(),
            system_state: SystemState::default(),
            logs: Vec::new(),
        }
    }

    fn refresh_ports(&mut self) {
        if let Ok(ports) = serialport::available_ports() {
            self.available_ports = ports.into_iter().map(|p| p.port_name).collect();
        }
    }

    fn send_ascii(&self, cmd: &str) {
        let _ = self.tx_cmd.send(SerialCommand::Send(cmd.to_string()));
    }

    fn update_data(&mut self, key: &str, val: &str) {
        let s = &mut self.system_state;
        match key {
            "KV" => s.kv = val.to_string(),
            "MA" | "MS" | "MX" | "AP" | "AT" => {
                // 这些值是 1/10，需要转换
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

impl eframe::App for RemedyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. 处理后台事件
        while let Ok(event) = self.rx_event.try_recv() {
            match event {
                GuiEvent::Connected(_) => self.connected = true,
                GuiEvent::Disconnected => self.connected = false,
                GuiEvent::Error(e) => {
                    self.logs.push((format!("错误: {}", e), LogColor::Alert));
                }
                GuiEvent::Log { text, color } => {
                    self.logs.push((text, color));
                    if self.logs.len() > 1000 {
                        self.logs.remove(0);
                    }
                }
                GuiEvent::DataUpdate(k, v) => self.update_data(&k, &v),
            }
        }

        // 2. 绘制 UI
        egui::CentralPanel::default().show(ctx, |ui| {
            // --- 顶部：连接设置 ---
            ui.horizontal(|ui| {
                ui.label("TX端口:");
                egui::ComboBox::from_id_source("tx_port")
                    .selected_text(&self.tx_port_name)
                    .show_ui(ui, |ui| {
                        for p in &self.available_ports {
                            ui.selectable_value(&mut self.tx_port_name, p.clone(), p);
                        }
                    });
                if ui.button("刷新").clicked() {
                    self.refresh_ports();
                }

                if self.connected {
                    if ui.button("断开连接").clicked() {
                        let _ = self.tx_cmd.send(SerialCommand::Disconnect);
                    }
                    ui.colored_label(egui::Color32::GREEN, "● 已连接");
                } else {
                    if ui.button("打开连接").clicked() {
                        let _ = self.tx_cmd.send(SerialCommand::Connect(
                            self.tx_port_name.clone(),
                            "".to_string(),
                        ));
                    }
                    ui.colored_label(egui::Color32::GRAY, "● 未连接");
                }
            });

            ui.separator();

            // --- 中间部分：左右分栏 ---
            ui.columns(2, |columns| {
                // 左栏：控制
                columns[0].vertical(|ui| {
                    ui.heading("参数控制");
                    
                    ui.horizontal(|ui| {
                        ui.label("KV: ");
                        ui.text_edit_singleline(&mut self.kv_input).rect.width();
                        if ui.button("设").clicked() {
                            if let Ok(num) = self.kv_input.parse::<i32>() {
                                self.send_ascii(&format!("KV{:0width$}", num, width = 3));
                            }
                        }
                        if ui.button("查").clicked() {
                            self.send_ascii("KV?");
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("MA(0.1): ");
                        ui.text_edit_singleline(&mut self.ma_input).rect.width();
                        if ui.button("设").clicked() {
                            if let Ok(num) = self.ma_input.parse::<i32>() {
                                self.send_ascii(&format!("MA{:0width$}", num, width = 5));
                            }
                        }
                        if ui.button("查").clicked() {
                            self.send_ascii("MA?");
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("MS(0.1): ");
                        ui.text_edit_singleline(&mut self.ms_input).rect.width();
                        if ui.button("设").clicked() {
                            if let Ok(num) = self.ms_input.parse::<i32>() {
                                self.send_ascii(&format!("MS{:0width$}", num, width = 5));
                            }
                        }
                        if ui.button("查").clicked() {
                            self.send_ascii("MS?");
                        }
                    });

                    ui.add_space(10.0);
                    ui.heading("常用指令");
                    if ui.button("复位 (RE)").clicked() { self.send_ascii("RE"); }
                    if ui.button("刷新系统 (RS)").clicked() { self.send_ascii("RS"); }
                    if ui.button("刷新拍片 (RR)").clicked() { self.send_ascii("RR"); }
                });

                // 右栏：状态显示 & 高级查询
                columns[1].vertical(|ui| {
                    ui.heading("系统状态监控");
                    egui::Grid::new("status_grid").striped(true).show(ui, |ui| {
                        ui.label("KV:"); ui.label(&self.system_state.kv); ui.label("MA:"); ui.label(&self.system_state.ma); ui.end_row();
                        ui.label("MS:"); ui.label(&self.system_state.ms); ui.label("mAs:"); ui.label(&self.system_state.mx); ui.end_row();
                        ui.label("Status:"); ui.label(&self.system_state.status); ui.label("Tech:"); ui.label(&self.system_state.technique); ui.end_row();
                        ui.label("Focus:"); ui.label(&self.system_state.focus); ui.label("Station:"); ui.label(&self.system_state.workstation); ui.end_row();
                        ui.label("Anode Heat:"); ui.label(&self.system_state.anode_heat); ui.label("Housing Heat:"); ui.label(&self.system_state.housing_heat); ui.end_row();
                        ui.label("Post mAs:"); ui.label(&self.system_state.post_mas); ui.label("Post Time:"); ui.label(&self.system_state.post_time); ui.end_row();
                    });

                    ui.add_space(10.0);
                    ui.heading("高级查询");
                    egui::Grid::new("query_grid").show(ui, |ui| {
                        if ui.button("MX查询").clicked() { self.send_ascii("MX?"); }
                        if ui.button("阳极热容量").clicked() { self.send_ascii("HE?"); }
                        if ui.button("管套热容量").clicked() { self.send_ascii("HH?"); }
                        if ui.button("曝光后mAs").clicked() { self.send_ascii("AP?"); }
                        ui.end_row();
                        if ui.button("曝光后时间").clicked() { self.send_ascii("AT?"); }
                        if ui.button("工作站").clicked() { self.send_ascii("WS?"); }
                        if ui.button("焦点").clicked() { self.send_ascii("FO?"); }
                        if ui.button("曝光技术").clicked() { self.send_ascii("ET?"); }
                        ui.end_row();
                    });
                });
            });

            ui.separator();

            // --- 底部：日志 ---
            ui.heading("实时交互日志");
            egui::ScrollArea::vertical()
                // 【修复2】移除了 .auto_scroll([false; 2])，该方法在新版已废弃
                .stick_to_bottom(true) 
                .show(ui, |ui| {
                    for (text, color) in &self.logs {
                        let text_color = match color {
                            LogColor::Info => egui::Color32::WHITE, 
                            LogColor::Rx => egui::Color32::GREEN,
                            LogColor::Alert => egui::Color32::RED,
                            LogColor::Tx => egui::Color32::LIGHT_BLUE,
                        };
                        
                        ui.colored_label(text_color, text);
                    }
                });
        });
        
        // 保持界面刷新以处理串口消息
        ctx.request_repaint();
    }
}

fn main() -> eframe::Result<()> {
    // 初始化日志
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([950.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "REMEDY Generator Rust Console",
        options,
        Box::new(|cc| {
            // 强制使用暗色主题，配合我们的日志颜色
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Box::new(RemedyApp::new(cc))
        }),
    )
}
