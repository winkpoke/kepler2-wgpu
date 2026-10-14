use serialport::SerialPort;
use std::io::{Read, Write};
use std::time::Duration;

pub struct HvDevice {
    port: Option<Box<dyn SerialPort>>,
    rx_buffer: Vec<u8>,
    pub event_queue: Vec<String>,
    port_name: String,            
    baud: u32,                    
}

impl HvDevice {
    pub fn new() -> Self {
        Self {
            port: None,
            rx_buffer: Vec::new(),
            event_queue: Vec::new(),
            port_name: String::new(),
            baud: 19200,
        }
    }

    /// 1. 基础配置：打开串口（重复调用会先断开旧口，支持运行时切换）
    pub fn connect(&mut self, port_name: &str, baud: u32) -> Result<(), String> {
        self.disconnect(); // 先释放旧句柄，避免占用
        let port = serialport::new(port_name, baud)
            .timeout(Duration::from_millis(10))
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .data_bits(serialport::DataBits::Eight)
            .open()
            .map_err(|e| format!("无法打开串口: {}", e))?;

        self.port = Some(port);
        self.port_name = port_name.to_string();
        self.baud = baud;
        println!("--- 串口 {} 已连接 ---", port_name);
        Ok(())
    }

    /// 断开串口
    pub fn disconnect(&mut self) {
        self.port = None;
        self.rx_buffer.clear();
    }

    /// 当前是否已连接
    pub fn is_connected(&self) -> bool {
        self.port.is_some()
    }

    /// 返回
    pub fn info(&self) -> (String, u32) {
        (self.port_name.clone(), self.baud)
    }

    // ========================================================================
    // 发送
    // ========================================================================
    
    pub fn send_lin(&mut self, body: &str) -> Result<(), String> {
        let frame = super::lin::encode(body);
        if let Some(ref mut p) = self.port {
            p.write_all(&frame).map_err(|e| e.to_string())?;
            p.flush().map_err(|e| e.to_string())?;
            // 日志里只显示前 64 字符（TUBE 帧很长）
            let shown = if body.len() > 64 {
                format!("{}…({}B)", &body[..40], body.len())
            } else {
                body.to_string()
            };
            self.event_queue.push(format!("TX: {}", shown));
            Ok(())
        } else {
            Err("串口未连接".to_string())
        }
    }

    // ========================================================================
    // 接收
    // ========================================================================

    /// 主循环更新（需在外部 loop 中调用）
    pub fn update(&mut self) {
        self.read_to_buffer();
        self.process_lin();
    }

    /// 抽取 STM32 推上来的**上行事件**（STATE/KV/MA/CAL/ACK/NAK/ERR/HV…）
    fn process_lin(&mut self) {
        for line in super::lin::drain(&mut self.rx_buffer) {
            self.event_queue.push(line);
        }
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
}