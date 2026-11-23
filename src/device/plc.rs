//! C-arm PLC Modbus TCP register map.
//!
//! 架构约定：
//! - 上位机直连高压发生器（`super::hv`，串口）与平板探测器,s
//!   曝光参数（kV/mA/time/mAs）、PREPARE/READY、热状态**不走 Modbus**。
//! - Modbus TCP 只承载：五轴运动、整机回位、安全联锁/系统状态、
//!   CBCT 扫描几何与脉冲发射时序（PLC 按角度/时间同步发脉冲，触发发生器外触发输入）。
//!
//! 寻址口径：F 列 = 0 基 PDU 保持寄存器编号，直接用于 0x03/0x06/0x10 报文；
//! PLC 内部地址 VW = 3100 + 2 * F。

#![allow(dead_code)]

// ===========================================================================
// 系统类（F 0..=17）
// ===========================================================================
pub const PROTOCOL_VERSION: u16 = 0; // R  UINT16 协议版本
pub const FIRMWARE_VERSION: u16 = 1; // R  UINT16 固件版本
pub const DEVICE_CAPABILITY: u16 = 2; // R  UINT16 能力位
pub const SESSION_ID: u16 = 3; // R  UINT16 会话编号，重启后更新，用于旧命令失效
pub const HEARTBEAT_TX: u16 = 4; // RW 计数递增；周期/超时 TBD（PLC 侧未定，先做成启动参数）
pub const HEARTBEAT_ACK: u16 = 5; // RW 控制器回显
pub const INTERLOCK_STATE: u16 = 6; // R  0=急停 1=碰撞 2=限位 3=门未关
pub const SYSTEM_ERROR: u16 = 7; // R  0=正常，其余故障码（PLC 与上位机共同定义）
pub const MAINS_STATE: u16 = 8; // R  0=停电 1=正常
pub const COLLISION_STATE: u16 = 9; // R  碰床状态；极性两表相反，TBD 电气确认
pub const COLLISION_VALID: u16 = 10; // R  0=无效 1=有效
pub const STATUS_AGE: u16 = 11; // R  ms 距最近有效更新
pub const SHUTDOWN_STATE: u16 = 12; // R  0=空闲 1=准备 2=执行中 3=完成 4=失败
pub const ALL_AXES_STOP: u16 = 13; // W  1=停止全部轴（优先受理；不替代硬件急停）
pub const FAULT_RESET: u16 = 14; // W  1=清除已消除的故障
pub const RESTORE_USER_DEFAULTS: u16 = 15; // W  1=恢复用户默认参数集
pub const PARAM_ERROR_FIELD: u16 = 16; // R  被拒字段的寄存器地址
pub const PARAM_ERROR_CODE: u16 = 17; // R  见 ParamErrorCode

// ===========================================================================
// 状态快照 / 模式 / 控制权（F 180..=187）
// ===========================================================================
pub const STATUS_VALID: u16 = 180; // R  状态快照有效性
pub const STATUS_SEQUENCE: u16 = 181; // R  快照编号，防读取撕裂
pub const CONTROL_OWNER: u16 = 182; // R  0=无 1=上位机 2=面板 3=手柄/脚闸
pub const CONTROL_REQUEST: u16 = 183; // W  1=申请控制权 0=释放
pub const REQUESTED_MODE: u16 = 184; // RW 1=CBCT 2=DR 3=FLU 4=维护
pub const ACTUAL_MODE: u16 = 185; // R  实际生效模式
pub const SYSTEM_STATE: u16 = 186; // R  见 SystemState
pub const SHUTDOWN_REQUEST: u16 = 187; // W  1=请求有序关机

// ===========================================================================
// 参数暂存 / 提交（F 188..=191）
// ===========================================================================
pub const PARAM_SET_ID: u16 = 188; // RW 0=无暂存
pub const PARAM_COMMIT: u16 = 189; // W  1=整组校验并提交（不触发动作）
pub const ACTIVE_PARAM_SET_ID: u16 = 190; // R  当前生效参数集编号
pub const PARAM_RESULT: u16 = 191; // R  见 ParamResult

// ===========================================================================
// 五轴统一接口：每轴 26 个寄存器，base = AXIS1_BASE + (n-1) * 26
// ===========================================================================
pub const AXIS_BLOCK_LEN: u16 = 26;
pub const AXIS1_BASE: u16 = 18; // VW3136 扫描旋转 ±110°
pub const AXIS2_BASE: u16 = 44; // VW3188 侧转 ±180°
pub const AXIS3_BASE: u16 = 70; // VW3240 上下升降 0..300mm
pub const AXIS4_BASE: u16 = 96; // VW3292 左右摆动 ±20°
pub const AXIS5_BASE: u16 = 122; // VW3344 前后平移 0..200mm

/// 轴块内偏移（地址 = axis_base + offset）
pub mod ax {
    pub const CAPABILITY: u16 = 0; // R  bit0=调速 bit1=定位 bit2=点动 bit3=回参考位 bit4=机械回零 bit5=速度反馈
    pub const TARGET_POSITION: u16 = 1; // RW INT16 绝对目标位置（Sheet2 范围列疑似 ×10 编码值，TBD）
    pub const RELATIVE_DISTANCE: u16 = 2; // RW INT16 相对位移（协议明确：运动用绝对位置，增量仅备用）
    pub const MOVE_SPEED: u16 = 3; // RW INT16 定位速度
    pub const JOG_SPEED: u16 = 4; // RW INT16 点动速度
    pub const RETURN_POSITION: u16 = 5; // RW INT16 参考位位置
    pub const RETURN_SPEED: u16 = 6; // RW INT16 回参考位速度
    pub const ACCEL: u16 = 7; // RW INT16 加速度（PLC 批注：让控制器定）
    pub const DECEL: u16 = 8; // RW INT16 减速度
    pub const HOME_SEARCH_SPEED: u16 = 9; // RW（PLC 批注：不需要，编码器直接知道）
    pub const HOME_APPROACH_SPEED: u16 = 10; // RW
    pub const HOME_BACKOFF: u16 = 11; // RW
    pub const HOME_DIRECTION: u16 = 12; // RW 0=正向 1=负向
    pub const HOME_TIMEOUT: u16 = 13; // RW ms 1000..=120000 默认 30000
    pub const COMMAND: u16 = 14; // W  见 AxCommand
    pub const COMMAND_ID: u16 = 15; // RW 命令编号，与 SESSION_ID 配合去重
    pub const ACK_COMMAND_ID: u16 = 16; // R  已受理命令编号
    pub const JOG_KEEPALIVE: u16 = 17; // W  点动续期，按周期重写，超时自动停止
    pub const ACTUAL_POSITION: u16 = 18; // R  INT16 实际位置，需检查 FEEDBACK_VALID
    pub const ACTUAL_SPEED: u16 = 19; // R  INT16 实际速度
    pub const EFFECTIVE_SPEED: u16 = 20; // R  本动作实际采用的速度
    pub const SPEED_LIMIT_REASON: u16 = 21; // R  0=无 1=模式限制 2=行程限制 3=联锁限制
    pub const STATUS: u16 = 22; // R  见 AxStatus
    pub const FEEDBACK_VALID: u16 = 23; // R  0=无效 1=有效
    pub const RESULT: u16 = 24; // R  见 AxResult
    pub const ERROR_CODE: u16 = 25; // R  0=正常
}

/// 轴号 → 块基址
#[inline]
pub const fn axis_base(n: u8) -> Option<u16> {
    match n {
        1 => Some(AXIS1_BASE),
        2 => Some(AXIS2_BASE),
        3 => Some(AXIS3_BASE),
        4 => Some(AXIS4_BASE),
        5 => Some(AXIS5_BASE),
        _ => None,
    }
}

// ===========================================================================
// 整机回参考位（F 148..=152, 179）
// 注意：多轴需协调轨迹与碰撞检查，不能简单同时触发五个轴。
// ===========================================================================
pub const RESET_SEQUENCE: u16 = 148; // W  1=启动整机回参考位
pub const RESET_PHASE: u16 = 149; // R  0=空闲 1=排队 2=执行中 3=完成 4=失败/取消
pub const RESET_COMPLETED_AXES: u16 = 150; // R  bit0..=4 = Axis1..=5 已完成
pub const RESET_FAILED_AXIS: u16 = 151; // R  0=无 1..=5
pub const RESET_RESULT: u16 = 152; // R
pub const RESET_COMMAND_ID: u16 = 179; // RW

// ===========================================================================
// 采集公共（F 153..=162）
// 架构 B：PREPARE / ACQUISITION_ID / EXPOSURE_PARAM_SET_ID / GENERATOR_READY /
// DETECTOR_READY / EXPOSURE_PERMITTED 六项归发生器/探测器直连链路（super::hv 等），
// 不进 Modbus——Sheet2 未分配地址，属预期行为，不是遗漏。
// 下面仅保留 PLC 侧仍需要的流程状态与中止通道。
// ===========================================================================
pub const INHIBIT_REASON: u16 = 153; // R  0=无 1=联锁 2=热状态 3=参数错误 4=未就绪
pub const TRIGGER_SOURCE: u16 = 154; // RW 0=软件 1=手柄 2=脚闸
pub const TRIGGER_STATE: u16 = 155; // R  触发输入状态（Sheet2 类型笔误 "UINT17"，实为 UINT16）
pub const EXPOSURE_ACTIVE: u16 = 156; // R  实际曝光中（"UINT18" 同为 UINT16 笔误）
pub const EXPOSURE_DONE: u16 = 157; // R  0=未曝光 1=曝光中 2=曝光结束
pub const ACQUISITION_DONE: u16 = 158; // R  采集完成（图像侧仍以探测器直连数据为准）
pub const ACQUISITION_RESULT: u16 = 159; // R  0=无 1=成功 2=失败 3=取消
pub const CANCEL_PREPARE: u16 = 160; // W  1=撤销准备
pub const ABORT_EXPOSURE: u16 = 161; // W  1=中止曝光（优先受理）
pub const ACQUISITION_STATE: u16 = 162; // R  见 AcquisitionState

// ===========================================================================
// 曝光参数（F 163..=167）—— 架构 B 下归高压发生器直连链路（super::hv）。
// 常量保留备查（例如 PLC 需要本地联锁判断），上位机正常流程不写。
// mA*time 与 mAs 两方案择一，不得同时下发。
// ===========================================================================
pub const DR_KV: u16 = 163; // UNUSED(generator-domain) INT16 kV 40..=120
pub const DR_MA: u16 = 164; // UNUSED(generator-domain) UINT16 mA 1..=500
pub const DR_TIME: u16 = 165; // UNUSED(generator-domain) UINT16 ms 1..=10000
pub const DR_MAS: u16 = 166; // UNUSED(generator-domain) INT16 mAs*10
pub const DR_ACQUISITION_TYPE: u16 = 167; // UNUSED(generator-domain) 1=静态单次 2=移动扫描

// ===========================================================================
// CBCT 扫描（F 168..=177）—— PLC 核心职责：按时间/角度同步发脉冲。
// 脉冲输出线接发生器外触发输入（电气侧已确认走外触发，接线细节 TBD）。
// ===========================================================================
pub const SCAN_START_ANGLE: u16 = 168; // RW INT16 角度*10，默认 -550
pub const SCAN_END_ANGLE: u16 = 169; // RW INT16 角度*10，默认 550
pub const SCAN_DIRECTION: u16 = 170; // RW 0=递增 1=递减
pub const SCAN_SPEED: u16 = 171; // RW (°/s)*10，2..=11，默认 5
pub const SCAN_PULSE_COUNT: u16 = 172; // RW 1..=1000，默认 220
pub const SCAN_TRIGGER_MODE: u16 = 173; // RW 0=时间同步 1=角度同步（编码器）
pub const SCAN_PULSES_SENT: u16 = 174; // R  本次已发脉冲数（扫描进度对账用）
pub const SCAN_FRAMES_ACQUIRED: u16 = 175; // R  帧数权威来源是探测器直连；此寄存器仅作 PLC 侧参考
pub const SCAN_PHASE: u16 = 176; // R  0=空闲 1=定位 2=扫描中 3=结束 4=中断
pub const SCAN_ERROR: u16 = 177; // R  0=正常，其余由 PLC 定义

// 架构 B：透视脉冲频率归发生器直连；此寄存器仅在 PLC 参与 FLU 触发时使用。
pub const FLU_PULSE_RATE: u16 = 178; // TBD 0..=40 脉冲/s，0 的含义未定义

// ===========================================================================
// 线上枚举（repr(u16)，取值来自 Sheet2）
// ===========================================================================
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum AxCommand {
    MoveAbsolute = 1,
    MoveRelative = 2,
    JogPositive = 3,
    JogNegative = 4,
    Stop = 5,
    ReturnToReference = 6,
    HomeMechanical = 7,
    ClearFault = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum AxStatus {
    Waiting = 0,
    Moving = 1,
    InPosition = 2,
    Homed = 3,
    PositiveLimit = 4,
    NegativeLimit = 5,
    Fault = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum AxResult {
    Idle = 0,
    Accepted = 1,
    Running = 2,
    Completed = 3,
    Rejected = 4,
    FailedOrCancelled = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ParamResult {
    None = 0,
    Accepted = 1,
    Committed = 2,
    Rejected = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ParamErrorCode {
    None = 0,
    OutOfRange = 1,
    ModeNotAllowed = 2,
    TypeMismatch = 3,
    CombinationConflict = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum RequestedMode {
    Cbct = 1,
    Dr = 2,
    Flu = 3,
    Maintenance = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum SystemState {
    Idle = 0,
    Preparing = 1,
    Ready = 2,
    Moving = 3,
    Exposing = 4,
    Fault = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum AcquisitionState {
    Idle = 0,
    Preparing = 1,
    Ready = 2,
    WaitingTrigger = 3,
    Exposing = 4,
    ExposureEnded = 5,
    AcquisitionDone = 6,
    Cancelled = 7,
    Fault = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum InterlockState {
    EmergencyStop = 0,
    Collision = 1,
    LimitSwitch = 2,
    DoorOpen = 3,
}

// ===========================================================================
// 编码换算：角度/速度/mAs 一律 ×10。饱和转换，避免 UI 越界输入静默回绕。
// 位移：AX3/AX5 目标位 0.1mm 步进；相对位移/实际位整数 mm
// ===========================================================================
pub const fn enc_signed(v: f64) -> i16 {
    let r = (v * 10.0).round();
    if r > i16::MAX as f64 {
        i16::MAX
    } else if r < i16::MIN as f64 {
        i16::MIN
    } else {
        r as i16
    }
}

pub const fn dec_signed(raw: i16) -> f64 {
    raw as f64 / 10.0
}

pub const fn enc_u16(v: f64) -> u16 {
    let r = (v * 10.0).round();
    if r > u16::MAX as f64 {
        u16::MAX
    } else if r < 0.0 {
        0
    } else {
        r as u16
    }
}

pub const fn dec_u16(raw: u16) -> f64 {
    raw as f64 / 10.0
}

// ===========================================================================
// 机械行程（Sheet4 轴定义）。⚠ Sheet2 目标位范围列（±550/±900/±200、
// 0..3000、0..2000）与机械行程矛盾，疑似把 ×10 编码值填进了范围列；
// 收口前前端预检以此为准，最终钳位以 PLC 生效范围（PARAM_RESULT）为准。
// ===========================================================================
#[derive(Debug, Clone, Copy)]
pub struct AxisTravel {
    pub min: f64,
    pub max: f64,
    /// true = 角度轴（编码 ×10），false = 直线轴
    pub angular: bool,
}

pub fn travel(axis: u8) -> AxisTravel {
    match axis {
        1 => AxisTravel { min: -110.0, max: 110.0, angular: true },
        2 => AxisTravel { min: -180.0, max: 180.0, angular: true },
        3 => AxisTravel { min: 0.0, max: 300.0, angular: false },
        4 => AxisTravel { min: -20.0, max: 20.0, angular: true },
        5 => AxisTravel { min: 0.0, max: 200.0, angular: false },
        _ => AxisTravel { min: 0.0, max: 0.0, angular: false },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_addresses_match_sheet2() {
        assert_eq!(axis_base(1).unwrap() + ax::COMMAND, 32); // VW3164
        assert_eq!(axis_base(3).unwrap() + ax::TARGET_POSITION, 71); // VW3242
        assert_eq!(axis_base(5).unwrap() + ax::ACTUAL_POSITION, 140);
        assert_eq!(axis_base(5).unwrap() + AXIS_BLOCK_LEN - 1, 147); // VW3394
    }

    #[test]
    fn encoding_roundtrip() {
        assert_eq!(enc_signed(45.0), 450);
        assert_eq!(dec_signed(450), 45.0);
        assert_eq!(enc_signed(-110.0), -1100);
        assert_eq!(enc_signed(f64::INFINITY), i16::MAX); // 饱和
        assert_eq!(enc_u16(11.0), 110);
    }
}
