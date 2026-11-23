//! C-arm PLC Modbus TCP session layer (native only).
//!
//! 架构 B：Modbus 只承载五轴运动 / 整机回位 / 联锁状态 / CBCT 扫描几何与脉冲时序；
//! 曝光参数与热状态归高压发生器直连链路（`super::hv`），采集归探测器直连。
//!
//! 操控模型（协议 Sheet3）：控制权申请 → 心跳维持 → 参数暂存 + PARAM_COMMIT →
//! 轴命令（COMMAND + COMMAND_ID 去重）→ ACK/RESULT/STATUS 闭环反馈。
//!
//! 待 PLC 侧收口的事项见 `super::plc` 常量注释；心跳周期/方式未定，
//! 此处做成可配置参数（默认按"递增+回显"实现）。

#![cfg(not(target_arch = "wasm32"))]

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex as AsyncMutex;
use tokio_modbus::client::Context;
use tokio_modbus::prelude::*;

use crate::device::plc as regs;

const IO_TIMEOUT: Duration = Duration::from_millis(1500);
const COMMIT_POLL: Duration = Duration::from_millis(50);
const RESULT_POLL: Duration = Duration::from_millis(100);

/// 共享句柄：handlers 与后台 task 共用同一个客户端
pub type PlcHandle = Arc<PlcClient>;

pub struct PlcClient {
    addr: std::sync::Mutex<SocketAddr>,
    unit: AtomicU8,
    ctx: AsyncMutex<Option<Context>>,
    next_command_id: AtomicU16,
    connected: AtomicBool,
    heartbeat_running: AtomicBool,
    jog_running: AtomicBool,
}

impl PlcClient {
    /// 从环境变量构造，不自动连接：
    /// `PLC_ADDR`（如 "192.168.1.10"）、`PLC_PORT`（协议要求不得默认 502，
    /// 未配置则连接时报错）、`PLC_UNIT`（默认 1）。
    pub fn from_env() -> PlcHandle {
        let addr_str = std::env::var("PLC_ADDR").unwrap_or_default();
        let port = std::env::var("PLC_PORT").ok().and_then(|s| s.parse::<u16>().ok()).unwrap_or(0);
        let unit = std::env::var("PLC_UNIT").ok().and_then(|s| s.parse::<u8>().ok()).unwrap_or(1);
        let addr: SocketAddr = format!("{}:{}", addr_str, port).parse().unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap());
        Arc::new(Self {
            addr: std::sync::Mutex::new(addr),
            unit: AtomicU8::new(unit),
            ctx: AsyncMutex::new(None),
            next_command_id: AtomicU16::new(1),
            connected: AtomicBool::new(false),
            heartbeat_running: AtomicBool::new(false),
            jog_running: AtomicBool::new(false),
        })
    }

    /// 运行时覆盖端点（"点击连接"时可由前端传入 addr/port）
    pub fn set_endpoint(&self, ip: &str, port: u16, unit: u8) -> Result<(), String> {
        let addr: SocketAddr = format!("{}:{}", ip, port)
            .parse()
            .map_err(|e| format!("非法 PLC 地址 {ip}:{port}: {e}"))?;
        *self.addr.lock().unwrap() = addr;
        self.unit.store(unit, Ordering::SeqCst);
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    pub fn info(&self) -> (String, u16, u8) {
        let addr = *self.addr.lock().unwrap();
        let unit = self.unit.load(Ordering::SeqCst);
        (addr.ip().to_string(), addr.port(), unit)
    }

    /// 建立连接（幂等：已连接直接返回）
    pub async fn connect(&self) -> Result<(), String> {
        let addr = *self.addr.lock().unwrap();
        if addr.port() == 0 || addr.ip().is_unspecified() {
            return Err("PLC_ADDR / PLC_PORT 未配置（协议要求端口不得默认 502）".into());
        }
        if self.connected.load(Ordering::SeqCst) {
            return Ok(());
        }
        let mut guard = self.ctx.lock().await;
        let mut ctx = tcp::connect(addr)
            .await
            .map_err(|e| format!("无法连接 PLC {addr}: {e}"))?;
        ctx.set_slave(Slave(self.unit.load(Ordering::SeqCst)));        log::info!("[plc] connected {addr} unit={}", self.unit.load(Ordering::SeqCst));
        *guard = Some(ctx);
        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub async fn disconnect(&self) {
        self.connected.store(false, Ordering::SeqCst);
        self.heartbeat_running.store(false, Ordering::SeqCst);
        self.jog_running.store(false, Ordering::SeqCst);
        let mut guard = self.ctx.lock().await;
        if let Some(mut ctx) = guard.take() {
            let _ = ctx.disconnect().await;
        }
        log::info!("[plc] disconnected");
    }

    // ------------------------------------------------------------------
    // 读写原语（带超时；错误统一 String，对齐 hv.rs 风格）
    // ------------------------------------------------------------------
    async fn read_regs(&self, addr: u16, cnt: u16) -> Result<Vec<u16>, String> {
        let mut guard = self.ctx.lock().await;
        let ctx = guard.as_mut().ok_or("PLC 未连接")?;
        let fut = ctx.read_holding_registers(addr, cnt);
        // 0.17: Result<Result<Vec<u16>, ExceptionCode>, Error>，外加超时一层
        match tokio::time::timeout(IO_TIMEOUT, fut).await {
            Err(_) => Err(format!("modbus 读超时 @ {addr}")),
            Ok(Err(e)) => Err(format!("modbus 读失败 @ {addr}: {e}")),
            Ok(Ok(Err(exc))) => Err(format!("modbus 异常响应 @ {addr}: {exc:?}")),
            Ok(Ok(Ok(data))) => Ok(data),
        }
    }

    async fn write_reg(&self, addr: u16, value: u16) -> Result<(), String> {
        let mut guard = self.ctx.lock().await;
        let ctx = guard.as_mut().ok_or("PLC 未连接")?;
        let fut = ctx.write_single_register(addr, value);
        match tokio::time::timeout(IO_TIMEOUT, fut).await {
            Err(_) => Err(format!("modbus 写超时 @ {addr}")),
            Ok(Err(e)) => Err(format!("modbus 写失败 @ {addr}: {e}")),
            Ok(Ok(Err(exc))) => Err(format!("modbus 异常响应 @ {addr}: {exc:?}")),
            Ok(Ok(Ok(()))) => Ok(()),
        }
    }

    /// 0x10 写多寄存器（连续参数组 / 命令+编号原子下发）
    async fn write_regs(&self, addr: u16, values: &[u16]) -> Result<(), String> {
        let mut guard = self.ctx.lock().await;
        let ctx = guard.as_mut().ok_or("PLC 未连接")?;
        let fut = ctx.write_multiple_registers(addr, values);
        match tokio::time::timeout(IO_TIMEOUT, fut).await {
            Err(_) => Err(format!("modbus 写超时 @ {addr} ({} regs)", values.len())),
            Ok(Err(e)) => Err(format!("modbus 写失败 @ {addr}: {e}")),
            Ok(Ok(Err(exc))) => Err(format!("modbus 异常响应 @ {addr}: {exc:?}")),
            Ok(Ok(Ok(()))) => Ok(()),
        }
    }

    // ------------------------------------------------------------------
    // 控制权 / 会话
    // ------------------------------------------------------------------
    pub async fn session_id(&self) -> Result<u16, String> {
        Ok(self.read_regs(regs::SESSION_ID, 1).await?[0])
    }

    pub async fn take_control(&self) -> Result<(), String> {
        self.write_reg(regs::CONTROL_REQUEST, 1).await?;
        let owner = self.read_regs(regs::CONTROL_OWNER, 1).await?[0];
        if owner != 1 {
            log::warn!("[plc] CONTROL_OWNER={owner}（非上位机），控制权仲裁未通过");
        }
        Ok(())
    }

    pub async fn release_control(&self) -> Result<(), String> {
        self.write_reg(regs::CONTROL_REQUEST, 0).await
    }

    // ------------------------------------------------------------------
    // 心跳 task：周期递增写 HEARTBEAT_TX，检查 HEARTBEAT_ACK 回显。
    // 方式/周期 Sheet2 未定（递增 vs 清零握手），先按递增实现，
    // PLC 侧收口后只改这个 task。
    // ------------------------------------------------------------------
    pub fn spawn_heartbeat(self: &PlcHandle, period: Duration) {
        if self.heartbeat_running.swap(true, Ordering::SeqCst) {
            return; // 已在运行
        }
        let plc = Arc::clone(self);
        tokio::spawn(async move {
            let mut counter: u16 = 0;
            let mut misses: u32 = 0;
            while plc.heartbeat_running.load(Ordering::SeqCst) {
                counter = counter.wrapping_add(1);
                match plc.write_reg(regs::HEARTBEAT_TX, counter).await {
                    Ok(()) => match plc.read_regs(regs::HEARTBEAT_ACK, 1).await {
                        Ok(ack) if ack[0] == counter => misses = 0,
                        Ok(ack) => {
                            misses += 1;
                            log::debug!("[plc] heartbeat ack {} != tx {counter}", ack[0]);
                        }
                        Err(e) => {
                            misses += 1;
                            log::debug!("[plc] heartbeat ack 读失败: {e}");
                        }
                    },
                    Err(e) => {
                        misses += 1;
                        log::debug!("[plc] heartbeat 写失败: {e}");
                    }
                }
                if misses == 5 {
                    log::warn!("[plc] 心跳连续 {misses} 次无确认，链路可能已失联");
                }
                tokio::time::sleep(period).await;
            }
        });
    }

    // ------------------------------------------------------------------
    // 状态快照：系统块 0..=17 + 扩展块 180..=191 + 五轴反馈（供 REST / WS）
    // ------------------------------------------------------------------
    pub async fn snapshot(&self) -> Result<PlcStatus, String> {
        let sys = self.read_regs(0, 18).await?; // 0..=17
        let ext = self.read_regs(180, 12).await?; // 180..=191
        let mut axes = Vec::with_capacity(5);
        for n in 1..=5u8 {
            let base = regs::axis_base(n).unwrap();
            // base+18..=base+25 → [实际位置, 实际速度, 生效速度, 限速原因, 状态, 有效性, 结果, 错误码]
            let a = self.read_regs(base + regs::ax::ACTUAL_POSITION, 8).await?;
            axes.push(AxisStatus {
                axis: n,
                actual_position: regs::dec_signed(a[0] as i16),
                actual_speed: regs::dec_signed(a[1] as i16),
                status: a[4],
                feedback_valid: a[5] == 1,
                result: a[6],
                error_code: a[7],
            });
        }
        Ok(PlcStatus {
            connected: true,
            system_state: ext[6],      // 186 SYSTEM_STATE
            interlock: sys[6],         // 6   INTERLOCK_STATE
            system_error: sys[7],      // 7   SYSTEM_ERROR
            mains: sys[8],             // 8   MAINS_STATE
            collision: sys[9],         // 9   COLLISION_STATE
            control_owner: ext[2],     // 182 CONTROL_OWNER
            actual_mode: ext[5],       // 185 ACTUAL_MODE
            param_result: ext[11],     // 191 PARAM_RESULT
            acquisition_state: None,   // 162 属流程中间态，按需扩展
            axes,
        })
    }

    // ------------------------------------------------------------------
    // 参数暂存 → 整组提交（协议核心时序；提交不触发任何动作）
    // ------------------------------------------------------------------
    async fn commit_staged(&self, timeout: Duration) -> Result<(), String> {
        self.write_reg(regs::PARAM_COMMIT, 1).await?;
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            tokio::time::sleep(COMMIT_POLL).await;
            let r = self.read_regs(regs::PARAM_RESULT, 1).await?[0];
            match r {
                2 => return Ok(()), // Committed
                3 => {
                    let field = self.read_regs(regs::PARAM_ERROR_FIELD, 1).await?[0];
                    let code = self.read_regs(regs::PARAM_ERROR_CODE, 1).await?[0];
                    return Err(format!("参数被拒: field={field} code={code}"));
                }
                _ => {
                    if tokio::time::Instant::now() >= deadline {
                        return Err("参数提交超时（PARAM_RESULT 未变成功）".into());
                    }
                }
            }
        }
    }

    /// 单轴绝对定位：暂存目标位/速度 → COMMIT → COMMAND=1 + COMMAND_ID。
    /// 返回 command_id；完成态通过 `wait_command` / 状态泵获取。
    pub async fn move_axis_absolute(&self, axis: u8, position: f64, speed: Option<f64>) -> Result<u16, String> {
        let base = regs::axis_base(axis).ok_or_else(|| format!("非法轴号 {axis}"))?;
        let travel = regs::travel(axis);
        if position < travel.min || position > travel.max {
            return Err(format!(
                "AX{axis} 目标 {position} 超出机械行程 [{}, {}]（最终以 PLC 生效范围为准）",
                travel.min, travel.max
            ));
        }
        // 暂存：目标位（×10 INT16，位型转换）与定位速度（×10）
        let target = regs::enc_signed(position) as u16;
        self.write_regs(base + regs::ax::TARGET_POSITION, &[target]).await?;
        if let Some(s) = speed {
            self.write_regs(base + regs::ax::MOVE_SPEED, &[regs::enc_u16(s)]).await?;
        }
        self.commit_staged(Duration::from_secs(2)).await?;
        // 命令 + 命令编号一次 0x10 下发（base+14=COMMAND, base+15=COMMAND_ID）
        let cmd_id = self.next_command_id.fetch_add(1, Ordering::SeqCst);
        self.write_regs(base + regs::ax::COMMAND, &[regs::AxCommand::MoveAbsolute as u16, cmd_id]).await?;
        Ok(cmd_id)
    }

    pub async fn stop_axis(&self, axis: u8) -> Result<(), String> {
        let base = regs::axis_base(axis).ok_or_else(|| format!("非法轴号 {axis}"))?;
        let cmd_id = self.next_command_id.fetch_add(1, Ordering::SeqCst);
        self.write_regs(base + regs::ax::COMMAND, &[regs::AxCommand::Stop as u16, cmd_id]).await
    }

    pub async fn home_axis(&self, axis: u8) -> Result<u16, String> {
        let base = regs::axis_base(axis).ok_or_else(|| format!("非法轴号 {axis}"))?;
        let cmd_id = self.next_command_id.fetch_add(1, Ordering::SeqCst);
        self.write_regs(base + regs::ax::COMMAND, &[regs::AxCommand::ReturnToReference as u16, cmd_id]).await?;
        Ok(cmd_id)
    }

    /// 点动启动：写点动速度 → 发命令 → spawn 续期 task（JOG_KEEPALIVE 周期重写，
    /// 超时由 PLC 自动停止）。direction: +1=正点动，-1=负点动。
    pub async fn jog_start(self: &PlcHandle, axis: u8, direction: i8, speed: f64, keepalive_ms: u64) -> Result<(), String> {
        let base = regs::axis_base(axis).ok_or_else(|| format!("非法轴号 {axis}"))?;
        let cmd = if direction > 0 { regs::AxCommand::JogPositive } else { regs::AxCommand::JogNegative };
        self.write_regs(base + regs::ax::JOG_SPEED, &[regs::enc_u16(speed)]).await?;
        let cmd_id = self.next_command_id.fetch_add(1, Ordering::SeqCst);
        self.write_regs(base + regs::ax::COMMAND, &[cmd as u16, cmd_id]).await?;

        self.jog_running.store(true, Ordering::SeqCst);
        let plc = Arc::clone(self);
        tokio::spawn(async move {
            let period = Duration::from_millis(keepalive_ms.max(50));
            while plc.jog_running.load(Ordering::SeqCst) {
                let _ = plc.write_reg(base + regs::ax::JOG_KEEPALIVE, keepalive_ms as u16).await;
                tokio::time::sleep(period / 2).await;
            }
        });
        Ok(())
    }

    pub async fn jog_stop(&self, axis: u8) -> Result<(), String> {
        self.jog_running.store(false, Ordering::SeqCst);
        self.stop_axis(axis).await
    }

    /// 全轴停止（优先受理；不替代硬件急停链）
    pub async fn all_axes_stop(&self) -> Result<(), String> {
        self.jog_running.store(false, Ordering::SeqCst);
        self.write_reg(regs::ALL_AXES_STOP, 1).await
    }

    /// 整机回参考位（PLC 侧协调轨迹与碰撞检查，非五轴齐动）
    pub async fn machine_reset(&self) -> Result<u16, String> {
        let cmd_id = self.next_command_id.fetch_add(1, Ordering::SeqCst);
        self.write_reg(regs::RESET_COMMAND_ID, cmd_id).await?;
        self.write_reg(regs::RESET_SEQUENCE, 1).await?;
        Ok(cmd_id)
    }

    pub async fn fault_reset(&self) -> Result<(), String> {
        self.write_reg(regs::FAULT_RESET, 1).await
    }

    pub async fn set_mode(&self, mode: u16) -> Result<(), String> {
        if !(1..=4).contains(&mode) {
            return Err(format!("非法模式 {mode}（1=CBCT 2=DR 3=FLU 4=维护）"));
        }
        self.write_reg(regs::REQUESTED_MODE, mode).await?;
        self.commit_staged(Duration::from_secs(2)).await
    }

    /// CBCT 扫描参数组：168..=173 连续 6 个寄存器，一次 0x10 写入
    pub async fn set_cbct_params(&self, p: CbctParams) -> Result<(), String> {
        let start = regs::enc_signed(p.start_deg) as u16;
        let end = regs::enc_signed(p.end_deg) as u16;
        let speed = regs::enc_u16(p.speed_dps);
        self.write_regs(regs::SCAN_START_ANGLE, &[start, end, p.direction, speed, p.pulses, p.trigger_mode]).await?;
        self.commit_staged(Duration::from_secs(2)).await
    }

    /// 扫描进度：(已发脉冲, 已采帧数(PLC 参考), 扫描阶段)
    pub async fn scan_progress(&self) -> Result<(u16, u16, u16), String> {
        let data = self.read_regs(regs::SCAN_PULSES_SENT, 3).await?;
        Ok((data[0], data[1], data[2]))
    }

    /// 轮询命令结果：ACK 与 command_id 不匹配 → None（PLC 尚未受理）。
    /// 读 base+16 起连续 10 个：[ACK, KEEPALIVE, POS, SPD, EFF, LIMIT, STATUS, VALID, RESULT, ERR]
    pub async fn command_result(&self, axis: u8, command_id: u16) -> Result<Option<u16>, String> {
        let base = regs::axis_base(axis).ok_or_else(|| format!("非法轴号 {axis}"))?;
        let data = self.read_regs(base + regs::ax::ACK_COMMAND_ID, 10).await?;
        if data[0] != command_id {
            return Ok(None);
        }
        Ok(Some(data[8])) // RESULT
    }

    /// 阻塞等待命令终态：3=完成 4=拒绝 5=失败/取消 提前返回
    pub async fn wait_command(&self, axis: u8, command_id: u16, timeout: Duration) -> Result<u16, String> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if let Some(r) = self.command_result(axis, command_id).await? {
                if r >= 3 {
                    return Ok(r);
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("AX{axis} 命令 {command_id} 等待超时"));
            }
            tokio::time::sleep(RESULT_POLL).await;
        }
    }
}

// ======================================================================
// 状态快照结构（serde，供 REST JSON 与 WS 推送共用）
// ======================================================================
#[derive(Debug, Clone, serde::Serialize)]
pub struct AxisStatus {
    pub axis: u8,
    /// 解码后物理值（角度 ° 或 mm，×10 已还原）
    pub actual_position: f64,
    pub actual_speed: f64,
    /// AxStatus 原始编码
    pub status: u16,
    pub feedback_valid: bool,
    /// AxResult 原始编码
    pub result: u16,
    pub error_code: u16,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PlcStatus {
    pub connected: bool,
    pub system_state: u16,
    pub interlock: u16,
    pub system_error: u16,
    pub mains: u16,
    pub collision: u16,
    pub control_owner: u16,
    pub actual_mode: u16,
    pub param_result: u16,
    pub acquisition_state: Option<u16>,
    pub axes: Vec<AxisStatus>,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct CbctParams {
    pub start_deg: f64,
    pub end_deg: f64,
    pub direction: u16,
    pub speed_dps: f64,
    pub pulses: u16,
    pub trigger_mode: u16,
}
