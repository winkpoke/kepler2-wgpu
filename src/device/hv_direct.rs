use serialport::SerialPort;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

const ETX: u8 = 0x03;
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);

pub struct HvDevice {
    port: Option<Box<dyn SerialPort>>,
    rx_buffer: Vec<u8>,
    last_heartbeat: Instant,
    is_standby: bool,
    pub event_queue: Vec<String>, // 存放解析好的 ASCII 消息
}

impl HvDevice {
    pub fn new() -> Self {
        Self {
            port: None,
            rx_buffer: Vec::new(),
            last_heartbeat: Instant::now(),
            is_standby: true,
            event_queue: Vec::new(),
        }
    }

    /// 1. 基础配置：打开串口并验证设备是否在线
    pub fn connect(&mut self, port_name: &str, baud: u32) -> Result<(), String> {
        let mut port = serialport::new(port_name, baud)
            .timeout(Duration::from_millis(500)) // 初始连接时给较长超时用于握手
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .data_bits(serialport::DataBits::Eight)
            .open()
            .map_err(|e| format!("无法打开本机串口 {}: {}", port_name, e))?;

        println!("--- 本机串口 {} 已打开，正在探测高压设备... ---", port_name);

        // 发送一个状态查询或心跳指令来验证对面是否有高压设备响应
        // 假设 "REQ" 或类似指令是高压会响应的，这里以发送 "REQ" 为例。
        // 你可以根据实际高压协议修改这个探测指令，比如发一个空命令或者状态查询命令
        let test_cmd = "REQ";
        let frame = Self::build_frame(test_cmd);
        
        if let Err(e) = port.write_all(&frame).and_then(|_| port.flush()) {
            return Err(format!("写入串口失败: {}", e));
        }

        // 等待响应
        let mut temp_buf = [0u8; 1024];
        match port.read(&mut temp_buf) {
            Ok(n) if n > 0 => {
                println!("--- 高压设备响应成功，串口 {} 连接验证通过 ---", port_name);
            }
            _ => {
                println!("!!! 警告: 串口 {} 已打开，但高压设备无响应 (可能未开机或未接线) !!!", port_name);
            }
        }

        // 恢复较短的超时时间用于后续的快速轮询
        if let Err(e) = port.set_timeout(Duration::from_millis(10)) {
            println!("警告：无法重置串口超时时间: {}", e);
        }

        self.port = Some(port);
        self.last_heartbeat = Instant::now();
        Ok(())
    }

    pub fn disconnect(&mut self) {
        if self.port.is_some() {
            println!("--- 高压串口断开连接 ---");
            self.port = None; // Drop the port, closing the connection
        }
    }

    /// 2. 协议核心：构建 ASCII 帧 (Payload + ETX + Checksum)
    fn build_frame(cmd: &str) -> Vec<u8> {
        let mut frame = cmd.as_bytes().to_vec();
        frame.push(ETX);
        let checksum = frame.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
        frame.push(checksum);
        frame
    }

    /// 3. 发送指令并打印 [ASCII] | HEX 日志
    pub fn send_command(&mut self, cmd: &str, is_auto: bool) -> Result<(), String> {
        if let Some(ref mut p) = self.port {
            let frame = Self::build_frame(cmd);
            p.write_all(&frame).map_err(|e| e.to_string())?;
            p.flush().map_err(|e| e.to_string())?;
            if !is_auto{
                self.log_io("TX", cmd, &frame);
            }
            Ok(())
        } else {
            Err("串口未连接".to_string())
        }
    }

    /// 4. 任务模块：主循环更新 (需在外部 Loop 中调用)
    pub fn update(&mut self) {
        // A. 读取数据
        self.read_to_buffer();
        // B. 解析并处理协议
        self.process_buffer();
        // C. 维持心跳
        self.maintain_heartbeat();
    }

    fn read_to_buffer(&mut self) {
        if let Some(ref mut p) = self.port {
            let mut temp_buf = [0u8; 1024];
            if let Ok(n) = p.read(&mut temp_buf) {
                if n > 0 {
                    self.rx_buffer.extend_from_slice(&temp_buf[..n]);
                }
            }
        }
    }

    fn process_buffer(&mut self) {
        while let Some(etx_pos) = self.rx_buffer.iter().position(|&x| x == ETX) {
            // 检查是否有足够的长度包含校验和字节 (etx_pos + 1)
            if self.rx_buffer.len() <= etx_pos + 1 {
                break; 
            }

            let frame_len = etx_pos + 2;
            let frame: Vec<u8> = self.rx_buffer.drain(..frame_len).collect();

            // 校验 Checksum
            let data_part = &frame[..frame_len - 1];
            let received_checksum = frame[frame_len - 1];
            let calculated_checksum = data_part.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));

            if received_checksum == calculated_checksum {
                self.handle_protocol_logic(frame);
            }
        }
    }

    /// 5. 任务模块：底层自动握手逻辑
    fn handle_protocol_logic(&mut self, frame: Vec<u8>) {
        // 提取内容字符串
        let msg_raw = String::from_utf8_lossy(&frame[..frame.len() - 2]);
        let msg = msg_raw.trim();

        // --- 状态机控制 ---
        // 如果是手闸或曝光动作 (最后一位不是0)，停止心跳以避免冲突
        if msg.starts_with("PR") || msg.starts_with("XR") || msg.starts_with("FLX") {
            if let Some(last_char) = msg.chars().last() {
                self.is_standby = last_char == '0';
            }
        }

        // 把收到的每一条指令都推送到事件队列（前端显示）
        self.log_io("RX", msg, &frame);
    }

    /// 6. 任务模块：智能心跳
    fn maintain_heartbeat(&mut self) {
        if self.is_standby && self.last_heartbeat.elapsed() >= HEARTBEAT_INTERVAL {
            let _ = self.send_command("ST", false);
            self.last_heartbeat = Instant::now();
        }
    }

    /// 7. 辅助：格式化日志输出并存入事件队列以便前端显示
    fn log_io(&mut self, dir: &str, ascii: &str, raw: &[u8]) {
        let hex_str: String = raw.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<String>>()
            .join(" ");
        let log_msg = if dir == "RX" {
            format!(" {} <- [{}] | HEX: {}", dir, ascii, hex_str)
        } else {
            format!(" {} -> [{}] | HEX: {}", dir, ascii, hex_str)
        };
        self.event_queue.push(log_msg);
    }
}