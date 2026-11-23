#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
C-arm PLC Modbus TCP 离线协议模拟器（纯 Python 标准库，零 pip 依赖）

按 docs/c-arm-ct-modbus-protocol-draft.md（Sheet2 "新版命令表待确认"）模拟：
- 0x03 读保持寄存器 / 0x06 写单寄存器 / 0x10 写多寄存器，含 Modbus 异常响应
- 七步握手：控制权仲裁 → 心跳回显 → 参数暂存 + PARAM_COMMIT 闭环 →
  COMMAND + COMMAND_ID → ACK/RESULT/STATUS 状态机
- 五轴位置按速度渐变运动（前端状态页可看到实际角度实时变化）
- 点动 JOG_KEEPALIVE 超时自动停；全停 ALL_AXES_STOP；整机回位 RESET_SEQUENCE
- CBCT 扫描模拟：提交扫描参数后自动 定位→扫描→完成，PULSES_SENT 递增
- 故障注入：--reject-prob 参数拒绝 / --drop-heartbeat 丢心跳回显

寄存器地址与 src/device/plc.rs 严格一致（F 列 = 0 基 PDU 编号）。

用法（本文件位于 static/，从仓库根目录运行）：
  python static/plc_sim.py                    # 监听 127.0.0.1:1502
  python static/plc_sim.py --port 1502 -v     # 逐报文打印收发
  python static/plc_sim.py --selftest         # 起服务后内置客户端自测七步握手
  python static/plc_sim.py --reject-prob 0.3 --drop-heartbeat 2   # 故障注入

上位机对接：
  前端设备页 PLC 卡片点"连接"默认连接 127.0.0.1:1502（index.html 传 addr/port），
  无需再设环境变量；若要从服务端统一配置端点，可用：
  PLC_ADDR=127.0.0.1 PLC_PORT=1502 cargo run --bin kepler
  → 状态灯 300ms 刷新，SET 下发后 AX1 实际角度渐变。
"""

import argparse
import random
import socket
import struct
import sys
import threading
import time

# ===========================================================================
# 寄存器地址（与 src/device/plc.rs 一字不差）
# ===========================================================================
PROTOCOL_VERSION = 0
FIRMWARE_VERSION = 1
DEVICE_CAPABILITY = 2
SESSION_ID = 3
HEARTBEAT_TX = 4
HEARTBEAT_ACK = 5
INTERLOCK_STATE = 6
SYSTEM_ERROR = 7
MAINS_STATE = 8
COLLISION_STATE = 9
COLLISION_VALID = 10
STATUS_AGE = 11
SHUTDOWN_STATE = 12
ALL_AXES_STOP = 13
FAULT_RESET = 14
RESTORE_USER_DEFAULTS = 15
PARAM_ERROR_FIELD = 16
PARAM_ERROR_CODE = 17

AXIS1_BASE, AXIS2_BASE, AXIS3_BASE, AXIS4_BASE, AXIS5_BASE = 18, 44, 70, 96, 122
AXES = {1: AXIS1_BASE, 2: AXIS2_BASE, 3: AXIS3_BASE, 4: AXIS4_BASE, 5: AXIS5_BASE}
AXIS_BLOCK_LEN = 26

# 轴块内偏移
axCAP = 0
axTARGET = 1
axRELATIVE = 2
axMOVE_SPEED = 3
axJOG_SPEED = 4
axRETURN_POS = 5
axRETURN_SPEED = 6
axHOME_TIMEOUT = 13
axCOMMAND = 14
axCOMMAND_ID = 15
axACK_ID = 16
axJOG_KEEPALIVE = 17
axACTUAL_POS = 18
axACTUAL_SPD = 19
axSTATUS = 22
axFEEDBACK_VALID = 23
axRESULT = 24
axERROR_CODE = 25

RESET_SEQUENCE = 148
RESET_PHASE = 149
RESET_COMPLETED_AXES = 150
RESET_FAILED_AXIS = 151
RESET_RESULT = 152
RESET_COMMAND_ID = 179

INHIBIT_REASON = 153
TRIGGER_SOURCE = 154
TRIGGER_STATE = 155
EXPOSURE_ACTIVE = 156
EXPOSURE_DONE = 157
ACQUISITION_DONE = 158
ACQUISITION_RESULT = 159
CANCEL_PREPARE = 160
ABORT_EXPOSURE = 161
ACQUISITION_STATE = 162

SCAN_START_ANGLE = 168
SCAN_END_ANGLE = 169
SCAN_DIRECTION = 170
SCAN_SPEED = 171
SCAN_PULSE_COUNT = 172
SCAN_TRIGGER_MODE = 173
SCAN_PULSES_SENT = 174
SCAN_FRAMES_ACQUIRED = 175
SCAN_PHASE = 176
SCAN_ERROR = 177

STATUS_VALID = 180
STATUS_SEQUENCE = 181
CONTROL_OWNER = 182
CONTROL_REQUEST = 183
REQUESTED_MODE = 184
ACTUAL_MODE = 185
SYSTEM_STATE = 186
SHUTDOWN_REQUEST = 187

PARAM_SET_ID = 188
PARAM_COMMIT = 189
ACTIVE_PARAM_SET_ID = 190
PARAM_RESULT = 191

REG_COUNT = 192  # 有效寄存器 0..191，越界回 Modbus 异常码 2

# 机械行程（Sheet4，物理域；编码域 ×10）——与 plc.rs::travel 一致
TRAVEL = {1: (-110.0, 110.0), 2: (-180.0, 180.0),
          3: (0.0, 300.0), 4: (-20.0, 20.0), 5: (0.0, 200.0)}


def i16(v: int) -> int:
    """int16 → u16 位型（补码），与 Rust 侧 enc_signed(v) as u16 对齐"""
    return v & 0xFFFF


def s16(v: int) -> int:
    """u16 → int16 符号解释"""
    return v - 0x10000 if v >= 0x8000 else v


def log(msg: str):
    print(msg, flush=True)


# ===========================================================================
# 寄存器组：数组 + 全局锁
# ===========================================================================
class Bank:
    def __init__(self, args):
        self.lock = threading.Lock()
        self.r = [0] * REG_COUNT
        r = self.r
        r[PROTOCOL_VERSION] = 1
        r[FIRMWARE_VERSION] = 0x0064
        r[DEVICE_CAPABILITY] = 0x3F
        r[SESSION_ID] = random.randint(1, 0xFFFE)
        r[INTERLOCK_STATE] = args.interlock
        r[MAINS_STATE] = 1
        r[COLLISION_STATE] = 0
        r[COLLISION_VALID] = 1
        for base in AXES.values():
            r[base + axCAP] = 0x3F           # 支持调速/定位/点动/回参考/回零/速度反馈
            r[base + axMOVE_SPEED] = 100     # 默认定位速度（编码 = 10°/s 或 10mm/s）
            r[base + axJOG_SPEED] = 50       # 默认点动速度
            r[base + axRETURN_SPEED] = 200
            r[base + axHOME_TIMEOUT] = 30000
            r[base + axFEEDBACK_VALID] = 1
        r[STATUS_VALID] = 1
        r[STATUS_SEQUENCE] = 1

    def read(self, addr: int, count: int):
        if addr < 0 or count < 1 or addr + count > REG_COUNT:
            raise IndexError(f"非法地址 {addr}..{addr + count - 1}")
        return self.r[addr:addr + count]

    def write(self, addr: int, values):
        if addr < 0 or addr + len(values) > REG_COUNT:
            raise IndexError(f"非法地址 {addr}..{addr + len(values) - 1}")
        self.r[addr:addr + len(values)] = list(values)


# ===========================================================================
# 物理过程模拟：轴运动 / 心跳回显 / 参数提交闭环 / CBCT 扫描
# ===========================================================================
class Simulator:
    DT = 0.02  # tick 周期（秒）

    def __init__(self, bank: Bank, args):
        self.bank = bank
        self.verbose = args.verbose
        self.reject_prob = args.reject_prob
        self.hb_drop = args.drop_heartbeat  # 剩余不回显次数
        # 每轴运动状态: mode=move/jog/idle, speed=编码值/s, home=回参考标记
        self.axes = {n: {"mode": "idle", "dir": 0, "speed": 0, "home": False,
                         "res_at": 0.0, "ka_ts": 0.0, "keepalive_ms": 1000}
                     for n in AXES}
        self.scan_active = False
        self.scan_rate = 0.0
        self.scan_sent = 0.0
        self.reset_active = False
        threading.Thread(target=self._tick_loop, daemon=True).start()

    # ------------------------------------------------------------------
    def _tick_loop(self):
        while True:
            with self.bank.lock:
                self._tick()
            time.sleep(self.DT)

    def _tick(self):
        b, now = self.bank, time.monotonic()
        any_moving = False
        for n, base in AXES.items():
            st = self.axes[n]
            cur = s16(b.r[base + axACTUAL_POS])
            if st["mode"] == "move":
                tgt = s16(b.r[base + axTARGET])
                step = max(st["speed"], 1) * self.DT
                if abs(tgt - cur) <= step:
                    b.r[base + axACTUAL_POS] = i16(tgt)
                    b.r[base + axACTUAL_SPD] = 0
                    b.r[base + axSTATUS] = 3 if st["home"] else 2   # Homed / InPosition
                    b.r[base + axRESULT] = 3                        # Completed
                    st["mode"] = "idle"
                else:
                    cur += step if tgt > cur else -step
                    lo, hi = TRAVEL[n]
                    cur = max(min(cur, hi * 10), lo * 10)
                    b.r[base + axACTUAL_POS] = i16(int(cur))
                    b.r[base + axACTUAL_SPD] = i16(int(st["speed"]))
                    if b.r[base + axRESULT] == 1 and now >= st["res_at"]:
                        b.r[base + axRESULT] = 2                    # Running
                    any_moving = True
            elif st["mode"] == "jog":
                ka = max(st["keepalive_ms"] * 2, 1000) / 1000.0
                if now - st["ka_ts"] > ka:
                    # keepalive 超时：PLC 自动停止点动
                    b.r[base + axACTUAL_SPD] = 0
                    b.r[base + axSTATUS] = 0
                    b.r[base + axRESULT] = 3
                    st["mode"] = "idle"
                else:
                    lo, hi = TRAVEL[n]
                    cur += st["dir"] * max(st["speed"], 1) * self.DT
                    cur = max(min(cur, hi * 10), lo * 10)
                    b.r[base + axACTUAL_POS] = i16(int(cur))
                    b.r[base + axACTUAL_SPD] = i16(int(st["speed"]))
                    if b.r[base + axRESULT] == 1 and now >= st["res_at"]:
                        b.r[base + axRESULT] = 2
                    any_moving = True
        # SYSTEM_STATE 联动：故障 > 运动 > 空闲
        b.r[SYSTEM_STATE] = 5 if b.r[SYSTEM_ERROR] else (3 if any_moving else 0)
        # CBCT 扫描脉冲
        if self.scan_active:
            self.scan_sent += self.scan_rate * self.DT
            b.r[SCAN_PULSES_SENT] = int(self.scan_sent)
            b.r[SCAN_FRAMES_ACQUIRED] = int(self.scan_sent)  # 简化：1 脉冲 = 1 帧
            if self.scan_sent >= b.r[SCAN_PULSE_COUNT]:
                b.r[SCAN_PHASE] = 3
                self.scan_active = False
        # 整机回位完成检查
        if self.reset_active:
            done = all(s16(b.r[base + axACTUAL_POS]) == s16(b.r[base + axRETURN_POS])
                       for base in AXES.values())
            if done:
                b.r[RESET_PHASE] = 3
                b.r[RESET_COMPLETED_AXES] = 0b11111
                b.r[RESET_RESULT] = 1
                self.reset_active = False

    # ------------------------------------------------------------------
    # 写副作用（Bank 写入后、持锁调用）
    # ------------------------------------------------------------------
    def on_write(self, addr: int, count: int):
        b = self.bank
        covered = lambda a: addr <= a < addr + count

        if covered(HEARTBEAT_TX):
            if self.hb_drop > 0:
                self.hb_drop -= 1  # 故障注入：不回显
            else:
                b.r[HEARTBEAT_ACK] = b.r[HEARTBEAT_TX]

        if covered(CONTROL_REQUEST):
            b.r[CONTROL_OWNER] = b.r[CONTROL_REQUEST]  # 1=上位机 0=释放

        if covered(ALL_AXES_STOP) and b.r[ALL_AXES_STOP] == 1:
            for n in AXES:
                self._halt(n, 3)

        if covered(FAULT_RESET) and b.r[FAULT_RESET] == 1:
            b.r[SYSTEM_ERROR] = 0
            for base in AXES.values():
                b.r[base + axERROR_CODE] = 0

        for n, base in AXES.items():
            if covered(base + axCOMMAND):
                self._handle_command(n)
            if covered(base + axJOG_KEEPALIVE):
                st = self.axes[n]
                st["ka_ts"] = time.monotonic()
                if b.r[base + axJOG_KEEPALIVE] > 0:
                    st["keepalive_ms"] = b.r[base + axJOG_KEEPALIVE]

        if covered(PARAM_COMMIT) and b.r[PARAM_COMMIT] == 1:
            self._commit()

        if covered(RESET_SEQUENCE) and b.r[RESET_SEQUENCE] == 1:
            self._machine_reset()

    # ------------------------------------------------------------------
    def _handle_command(self, n: int):
        b, base, st = self.bank, AXES[n], self.axes[n]
        cmd = b.r[base + axCOMMAND]
        b.r[base + axACK_ID] = b.r[base + axCOMMAND_ID]
        b.r[base + axERROR_CODE] = 0
        now = time.monotonic()
        st["home"] = False
        if cmd == 1:  # MoveAbsolute
            st.update(mode="move", speed=b.r[base + axMOVE_SPEED] or 100, res_at=now + 0.15)
        elif cmd == 2:  # MoveRelative
            cur = s16(b.r[base + axACTUAL_POS])
            tgt = cur + s16(b.r[base + axRELATIVE])
            b.r[base + axTARGET] = i16(tgt)
            st.update(mode="move", speed=b.r[base + axMOVE_SPEED] or 100, res_at=now + 0.15)
        elif cmd in (3, 4):  # Jog±
            st.update(mode="jog", dir=1 if cmd == 3 else -1,
                      speed=b.r[base + axJOG_SPEED] or 50, res_at=now + 0.15)
        elif cmd == 5:  # Stop
            self._halt(n, 3)
            return
        elif cmd in (6, 7):  # ReturnToReference / HomeMechanical
            b.r[base + axTARGET] = b.r[base + axRETURN_POS]
            st.update(mode="move", home=True,
                      speed=b.r[base + axRETURN_SPEED] or b.r[base + axMOVE_SPEED] or 100,
                      res_at=now + 0.15)
        elif cmd == 8:  # ClearFault
            b.r[base + axRESULT] = 3
            return
        else:
            b.r[base + axERROR_CODE] = 3  # TypeMismatch
            b.r[base + axRESULT] = 4      # Rejected
            return
        b.r[base + axSTATUS] = 1  # Moving
        b.r[base + axRESULT] = 1  # Accepted

    def _halt(self, n: int, result: int):
        b, base, st = self.bank, AXES[n], self.axes[n]
        st["mode"] = "idle"
        b.r[base + axACTUAL_SPD] = 0
        b.r[base + axSTATUS] = 0
        b.r[base + axRESULT] = result

    # ------------------------------------------------------------------
    def _commit(self):
        """PARAM_COMMIT=1：整组行程校验 → 受理 → 60ms 后 Committed"""
        b = self.bank
        for n, base in AXES.items():
            tgt = s16(b.r[base + axTARGET])
            lo, hi = TRAVEL[n]
            if tgt < lo * 10 or tgt > hi * 10:
                b.r[PARAM_RESULT] = 3
                b.r[PARAM_ERROR_FIELD] = base + axTARGET
                b.r[PARAM_ERROR_CODE] = 1  # OutOfRange
                return
        if random.random() < self.reject_prob:
            base = AXES[1]
            b.r[PARAM_RESULT] = 3
            b.r[PARAM_ERROR_FIELD] = base + axTARGET
            b.r[PARAM_ERROR_CODE] = 4  # CombinationConflict（注入）
            return
        b.r[PARAM_SET_ID] = (b.r[PARAM_SET_ID] + 1) & 0xFFFF
        b.r[ACTIVE_PARAM_SET_ID] = b.r[PARAM_SET_ID]
        b.r[PARAM_RESULT] = 1  # Accepted
        mode = b.r[REQUESTED_MODE]
        threading.Timer(0.06, self._committed, args=(mode,)).start()

    def _committed(self, mode: int):
        with self.bank.lock:
            self.bank.r[PARAM_RESULT] = 2  # Committed
            if mode in (1, 2, 3, 4):
                self.bank.r[ACTUAL_MODE] = mode
            if mode == 1 and self.bank.r[SCAN_PULSE_COUNT] > 0:
                self._start_scan()

    def _start_scan(self):
        """CBCT 扫描演示：0.8s 后定位 → 1.6s 后扫描，脉冲按角度跨度/速度均布"""
        b = self.bank
        start, end = s16(b.r[SCAN_START_ANGLE]), s16(b.r[SCAN_END_ANGLE])
        speed = max(b.r[SCAN_SPEED], 1)
        duration = max(abs(end - start) / speed, 1.0)
        count = max(b.r[SCAN_PULSE_COUNT], 1)
        log(f"[sim] CBCT 扫描开始：{start / 10:.1f}°→{end / 10:.1f}° "
            f"{count} 脉冲 / {duration:.1f}s")
        threading.Timer(0.8, lambda: self._set_scan_phase(1)).start()
        threading.Timer(1.6, lambda: self._set_scan_phase(2)).start()
        self.scan_rate = count / duration
        self.scan_sent = 0.0
        threading.Timer(1.6, self._scan_go).start()

    def _set_scan_phase(self, ph: int):
        with self.bank.lock:
            if ph == 2:
                self.bank.r[SCAN_PULSES_SENT] = 0
                self.scan_active = True
            self.bank.r[SCAN_PHASE] = ph

    def _scan_go(self):
        pass  # phase/速率已在 _set_scan_phase(2) 与 __init__ 处理，占位防 Timer 报错

    def _machine_reset(self):
        b = self.bank
        b.r[RESET_PHASE] = 2
        b.r[RESET_COMPLETED_AXES] = 0
        b.r[RESET_RESULT] = 0
        for n, base in AXES.items():
            b.r[base + axTARGET] = b.r[base + axRETURN_POS]
            st = self.axes[n]
            st.update(mode="move", home=True,
                      speed=b.r[base + axRETURN_SPEED] or 200, res_at=time.monotonic() + 0.15)
            b.r[base + axSTATUS] = 1
            b.r[base + axRESULT] = 1
        self.reset_active = True


# ===========================================================================
# Modbus TCP 服务端：MBAP 解析 + PDU 分发
# ===========================================================================
class Server:
    def __init__(self, host, port, unit, sim: Simulator, verbose: bool):
        self.host, self.port, self.unit = host, port, unit
        self.sim, self.verbose = sim, verbose
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.sock.bind((host, port))
        self.sock.listen(4)
        # Windows 上阻塞 accept() 无法被 Ctrl-C 中断（主线程拿不到 KeyboardInterrupt）。
        # 设 0.5s 超时让 accept 周期性返回，主循环因此有机会检查并响应 Ctrl-C。
        self.sock.settimeout(0.5)

    def serve_forever(self):
        while True:
            try:
                conn, addr = self.sock.accept()
            except socket.timeout:
                continue  # 超时属正常轮询，回到循环顶部；Ctrl-C 在此时被捕获退出
            threading.Thread(target=self._client, args=(conn, addr), daemon=True).start()

    @staticmethod
    def _recv_exact(conn, n):
        buf = b""
        while len(buf) < n:
            chunk = conn.recv(n - len(buf))
            if not chunk:
                return None
            buf += chunk
        return buf

    def _client(self, conn, addr):
        log(f"[tcp] 客户端接入 {addr[0]}:{addr[1]}")
        try:
            while True:
                head = self._recv_exact(conn, 7)
                if head is None:
                    break
                txn, proto, length, unit = struct.unpack(">HHHB", head)
                if proto != 0 or length < 2:
                    break
                pdu = self._recv_exact(conn, length - 1)
                if pdu is None:
                    break
                resp = self._dispatch(pdu)
                conn.sendall(struct.pack(">HHHB", txn, 0, len(resp) + 1, unit) + resp)
        except (ConnectionResetError, OSError):
            pass
        finally:
            conn.close()
            log(f"[tcp] 客户端断开 {addr[0]}:{addr[1]}")

    def _dispatch(self, pdu: bytes) -> bytes:
        b, fc = self.sim.bank, pdu[0]
        try:
            if fc == 3:  # 读保持寄存器
                addr, cnt = struct.unpack(">HH", pdu[1:5])
                if self.verbose:
                    log(f"[modbus] FC03 read addr={addr} cnt={cnt}")
                vals = b.read(addr, cnt)
                return bytes([fc, cnt * 2]) + struct.pack(f">{cnt}H", *vals)
            if fc == 6:  # 写单寄存器
                addr, val = struct.unpack(">HH", pdu[1:5])
                if self.verbose:
                    log(f"[modbus] FC06 write addr={addr} val={val} (i16={s16(val)})")
                with b.lock:
                    b.write(addr, [val])
                    self.sim.on_write(addr, 1)
                return pdu[:5]
            if fc == 16:  # 写多寄存器
                addr, cnt = struct.unpack(">HH", pdu[1:5])
                bc = pdu[5]
                vals = list(struct.unpack(f">{cnt}H", pdu[6:6 + bc]))
                if self.verbose:
                    log(f"[modbus] FC10 write addr={addr} cnt={cnt} vals={vals}")
                with b.lock:
                    b.write(addr, vals)
                    self.sim.on_write(addr, cnt)
                return bytes([fc]) + struct.pack(">HH", addr, cnt)
            return bytes([fc | 0x80, 1])  # 非法功能
        except IndexError:
            return bytes([fc | 0x80, 2])  # 非法地址


# ===========================================================================
# 内置自测客户端：不依赖 Rust 侧即可验证七步握手
# ===========================================================================
class SelfTest:
    def __init__(self, port, unit):
        self.sock = socket.create_connection(("127.0.0.1", port), timeout=3)
        self.unit, self.txn = unit, 0

    def _pdu(self, req: bytes) -> bytes:
        self.txn = (self.txn + 1) & 0xFFFF
        self.sock.sendall(struct.pack(">HHHB", self.txn, 0, len(req) + 1, self.unit) + req)
        head = self._recv(7)
        _, _, length, _ = struct.unpack(">HHHB", head)
        return self._recv(length - 1)

    def _recv(self, n):
        buf = b""
        while len(buf) < n:
            chunk = self.sock.recv(n - len(buf))
            if not chunk:
                raise ConnectionError("连接中断")
            buf += chunk
        return buf

    def read(self, addr, cnt):
        resp = self._pdu(struct.pack(">BHH", 3, addr, cnt))
        assert resp[0] == 3, f"FC03 异常响应: {resp.hex()}"
        return list(struct.unpack(f">{cnt}H", resp[2:2 + cnt * 2]))

    def write1(self, addr, val):
        resp = self._pdu(struct.pack(">BHH", 6, addr, val))
        assert resp[0] == 6, f"FC06 异常响应: {resp.hex()}"

    def writeN(self, addr, vals):
        payload = struct.pack(f">{len(vals)}H", *vals)
        resp = self._pdu(struct.pack(">BHHB", 16, addr, len(vals), len(vals) * 2) + payload)
        assert resp[0] == 16, f"FC10 异常响应: {resp.hex()}"

    def poll(self, addr, want, timeout):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self.read(addr, 1)[0] == want:
                return True
            time.sleep(0.05)
        return False

    def run(self) -> bool:
        base, results = AXES[1], []
        def step(desc, ok, detail=""):
            results.append(ok)
            log(f"  [{'PASS' if ok else 'FAIL'}] {desc}" + (f" —— {detail}" if detail else ""))

        log("[selftest] 七步握手自测开始")
        sid = self.read(SESSION_ID, 1)[0]
        step("① 读 SESSION_ID", sid != 0, f"sid={sid}")

        self.write1(CONTROL_REQUEST, 1)
        step("② 申请控制权 → OWNER==1", self.read(CONTROL_OWNER, 1)[0] == 1)

        self.write1(HEARTBEAT_TX, 0x1234)
        step("③ 心跳 TX=0x1234 → ACK 回显", self.read(HEARTBEAT_ACK, 1)[0] == 0x1234)

        self.write1(base + axTARGET, 100)       # 10.0°（编码 ×10）
        self.write1(base + axMOVE_SPEED, 300)   # 30°/s
        self.write1(PARAM_COMMIT, 1)
        step("④ 暂存+COMMIT → PARAM_RESULT==2", self.poll(PARAM_RESULT, 2, 3))

        self.writeN(base + axCOMMAND, [1, 7])   # MoveAbsolute + COMMAND_ID=7
        ack_ok = self.poll(base + axACK_ID, 7, 2)
        done = self.poll(base + axRESULT, 3, 8)
        pos = s16(self.read(base + axACTUAL_POS, 1)[0])
        step("⑤ 命令受理 ACK==7", ack_ok)
        step("⑥ 运动到位 RESULT==3", done, f"实际位置={pos / 10:.1f}°")
        step("⑦ 位置到达目标 10.0°", pos == 100)

        self.write1(CONTROL_REQUEST, 0)
        step("⑧ 释放控制权 → OWNER==0", self.read(CONTROL_OWNER, 1)[0] == 0)

        # 异常路径：越界读应回 Modbus 异常码 2
        resp = self._pdu(struct.pack(">BHH", 3, 250, 1))
        step("⑨ 越界读回异常码 2", resp[0] == 0x83 and resp[1] == 2, resp.hex())

        self.sock.close()
        ok = all(results)
        log(f"[selftest] {'全部通过' if ok else '存在失败项'} ({sum(results)}/{len(results)})")
        return ok


# ===========================================================================
def main():
    ap = argparse.ArgumentParser(description="C-arm PLC Modbus TCP 离线协议模拟器")
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=1502, help="监听端口（协议要求不默认 502）")
    ap.add_argument("--unit", type=int, default=1, help="Unit ID")
    ap.add_argument("--interlock", type=int, default=0,
                    help="初始联锁状态 0=急停 1=碰撞 2=限位 3=门未关")
    ap.add_argument("--reject-prob", type=float, default=0.0, help="参数提交随机拒绝概率 0..1")
    ap.add_argument("--drop-heartbeat", type=int, default=0, help="故意不回显前 N 次心跳")
    ap.add_argument("--selftest", action="store_true", help="启动后内置客户端自测七步握手")
    ap.add_argument("-v", "--verbose", action="store_true", help="逐报文打印")
    args = ap.parse_args()

    bank = Bank(args)
    sim = Simulator(bank, args)
    server = Server(args.host, args.port, args.unit, sim, args.verbose)
    log(f"[sim] PLC 模拟器监听 {args.host}:{args.port} unit={args.unit}"
        f" session={bank.r[SESSION_ID]}"
        + (f"（注入：丢心跳×{args.drop_heartbeat} 拒绝率{args.reject_prob:.0%}）"
           if args.drop_heartbeat or args.reject_prob else ""))

    if args.selftest:
        threading.Thread(target=server.serve_forever, daemon=True).start()
        time.sleep(0.3)
        try:
            ok = SelfTest(args.port, args.unit).run()
        except (AssertionError, ConnectionError, OSError) as e:
            log(f"[selftest] 异常中止: {e}")
            ok = False
        sys.exit(0 if ok else 1)

    log("[sim] 等待上位机连接…（Ctrl-C 退出）")
    log("[sim] 上位机对接：PLC_ADDR=%s PLC_PORT=%d cargo run" % (args.host, args.port))
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        log("\n[sim] 已退出")


if __name__ == "__main__":
    main()
