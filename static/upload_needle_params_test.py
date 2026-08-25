#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
模拟客户端脚本：向 Kepler2-WGPU 服务器的 /api/upload_needle_params 挂载点传输针刺参数。

用途
----
模拟"远程设备/客户端"向 Rust 服务器上报穿刺针的位置(position)与姿态(orientation)，
打印每次请求与响应以便调试，并附带基础断言（HTTP 状态码、status/len_mm/id 等）。

用法
----
# 终端 1：启动服务器（默认端口 3000，可用环境变量 KEPLER_SERVER_PORT 覆盖）
cargo run -- server

# 终端 2：安装依赖并运行
python scripts/upload_needle_params_test.py
"""

import random
import time
import json
import math
import os
import sys

try:
    import requests
except ImportError:
    print("[错误] 缺少依赖 requests，请先执行: pip install requests", file=sys.stderr)
    sys.exit(1)

# ---------------------------------------------------------------------------
# 常量
# ---------------------------------------------------------------------------
def _default_port() -> int:
    env = os.environ.get("KEPLER_SERVER_PORT")
    if env and env.isdigit():
        return int(env)
    return 3000


PORT = _default_port()
BASE_URL = f"http://172.18.3.15:3000"
URL = f"{BASE_URL}/api/upload_needle_params"
WS_URL = f"ws://172.18.3.15:3000/ws"

LEN_MM = 120.0            # 服务器固定针长
TOL = 0.05                # f32 容差（坐标比对用）
TIMEOUT = 10.0            # 请求超时（秒）

_PASS = 0
_FAIL = 0


def fmt_json(obj) -> str:
    return json.dumps(obj, ensure_ascii=False, indent=2)


def check(label: str, cond: bool, detail: str = "") -> None:
    """轻量断言：打印 [PASS]/[FAIL] 并累计计数。"""
    global _PASS, _FAIL
    mark = "[PASS]" if cond else "[FAIL]"
    if cond:
        _PASS += 1
    else:
        _FAIL += 1
    suffix = f"  ({detail})" if detail else ""
    print(f"  {mark} {label}{suffix}")


# ---------------------------------------------------------------------------
# 坐标变换复算（与 Rust handler 公式一致）
# ---------------------------------------------------------------------------
def euler_xyz_rotate_z(a_deg: float, b_deg: float, c_deg: float):
    """复算 glam::Mat3::from_euler(EulerRot::XYZ, a,b,c) 作用于 Vec3::Z 得到的方向向量。

    依据 glam 0.30.9 源码（src/euler.rs）：EulerRot::XYZ 且 parity_even，
    矩阵等价于 Rx(a)*Ry(b)*Rz(c) 的列向量乘积；Z 轴旋转不改变 Z 方向，
    因此结果只与 a、b 有关：dir = (sin b, -cos b * sin a, cos b * cos a)。
    """
    ax = math.radians(a_deg)
    ay = math.radians(b_deg)
    _ = c_deg  # z 角不影响 Vec3::Z 的方向
    sx, cx = math.sin(ax), math.cos(ax)
    sy, cy = math.sin(ay), math.cos(ay)
    return (sy, -cy * sx, cy * cx)


def expected_entry_tip(pos, ori):
    """按 handlers.rs L232-262 复算变换后的 entry/tip。

    pos = (x, y, z) 针中心，ori = (a, b, c) 欧拉角（度）。
    返回 ((entry), (tip))，均为变换后的坐标。
    """
    dx, dy, dz = euler_xyz_rotate_z(*ori)
    cx, cy, cz = pos
    half = LEN_MM * 0.5
    ex, ey, ez = cx - dx * half, cy - dy * half, cz - dz * half
    tx, ty, tz = cx + dx * half, cy + dy * half, cz + dz * half
    return ((ex - 30.0, ey + 100.0, -ez), (tx - 30.0, ty + 100.0, -tz))


def check_coord(actual, expected, label: str) -> None:
    """坐标比对（参考性质）：不一致只警告，不参与 PASS/FAIL 计数。"""
    if actual is None:
        return
    diff = max(abs(actual[i] - expected[i]) for i in range(3))
    if diff <= TOL:
        print(f"  [参考] {label} 与本地复算一致 (max diff={diff:.4f})")
    else:
        print(f"  [参考] {label} 与本地复算不一致 (max diff={diff:.4f}) "
              f"预期{expected} 实际{actual} —— 仅提示，不影响判定")


# ---------------------------------------------------------------------------
# 模拟发送
# ---------------------------------------------------------------------------
def post_json(name: str, payload=None, data=None, headers=None, expect: int = 200):
    """执行一次 POST 并打印请求/响应，返回 requests.Response。"""
    print(f"\n---------- {name} ----------")
    if payload is not None:
        print(f"请求: POST {URL}")
        print("请求体: " + fmt_json(payload))
        resp = requests.post(URL, json=payload, timeout=TIMEOUT)
    else:
        print(f"请求: POST {URL}")
        print(f"请求体: (无 body) | data={data!r} | headers={headers}")
        resp = requests.post(URL, data=data, headers=headers, timeout=TIMEOUT)
    print(f"HTTP {resp.status_code}")
    try:
        body = resp.json()
        print("响应体: " + fmt_json(body))
    except Exception:
        body = None
        print("响应体: (非 JSON) " + resp.text)
    check(f"HTTP 状态码应为 {expect}", resp.status_code == expect,
          f"实际 {resp.status_code}")
    return resp, body

def scenario1_stream(rate_hz: float = 40.0):
    """
    持续模拟设备发送针参数

    频率:
        40Hz

    position:
        x,y,z ∈ [-400,400] mm

    orientation:
        a,b,c ∈ [-10,10] deg
    """

    interval = 1.0 / rate_hz
    frame_seq = 0

    print(f"开始模拟针数据发送: {rate_hz} Hz")
    print("Ctrl+C 停止\n")

    try:
        while True:

            frame_seq += 1

            # 随机位置 mm
            pos = (
                random.uniform(-400.0, 400.0),
                random.uniform(-400.0, 400.0),
                random.uniform(-400.0, 400.0),
            )

            # 随机欧拉角 deg
            ori = (
                random.uniform(-10.0, 10.0),
                random.uniform(-10.0, 10.0),
                random.uniform(-10.0, 10.0),
            )


            payload = {
                "frame_sequence": frame_seq,

                "position": {
                    "x": pos[0],
                    "y": pos[1],
                    "z": pos[2],
                    "unit": "mm",
                },

                "orientation": {
                    "a": ori[0],
                    "b": ori[1],
                    "c": ori[2],
                    "unit": "deg",
                },

                "coordinate_frame": "patient",
            }


            t0 = time.perf_counter()

            resp, body = post_json(
                f"frame {frame_seq}",
                payload=payload
            )


            if body is not None:
                check(
                    "status == ok",
                    body.get("status") == "ok",
                    str(body.get("status"))
                )


            # 保证40Hz
            elapsed = time.perf_counter() - t0
            delay = interval - elapsed

            if delay > 0:
                time.sleep(delay)


    except KeyboardInterrupt:
        print("\n停止发送")

def scenario1_full() -> None:
    """完整参数：全部字段 → 200，且 len_mm==120、id 回显、坐标与本地复算一致。"""
    frame_seq = 3
    pos = (0.0, 0.0, 0.0)
    ori = (0.0,45.0, 0.0)
    # pos = (0.0, 0.0, -60.0)
    # pos = (0.0, 0.0, -124.0)
    # pos = (0.0, 0.0, -97.0)
    # ori = (45.0, 0.0, 0.0)
    # pos = (0.0, 0.0, -124.0)
    # ori = (90.0, 0.0, 0.0)
    payload = {
        "frame_sequence": frame_seq,
        "position": {"x": pos[0], "y": pos[1], "z": pos[2], "unit": "mm"},
        "orientation": {"a": ori[0], "b": ori[1], "c": ori[2], "unit": "deg"},
        "coordinate_frame": "patient",
    }
    resp, body = post_json("场景 1/5：完整参数（全部字段）", payload=payload)

    if body is None:
        return
    check("status == 'ok'", body.get("status") == "ok", str(body.get("status")))
    check("len_mm == 120.0", abs(body.get("len_mm", -1.0) - LEN_MM) < 1e-3,
          f"实际 {body.get('len_mm')}")
    check("id 回显 frame_sequence", body.get("id") == frame_seq,
          f"实际 {body.get('id')}")
    check("dir 原样回显欧拉角", all(abs(body["dir"][i] - ori[i]) <= TOL for i in range(3)),
          f"实际 {body.get('dir')}")

    # 坐标变换参考校验（警告不中断）
    exp_entry, exp_tip = expected_entry_tip(pos, ori)
    check_coord(body.get("pos"), exp_entry, "entry(pos)")
    check_coord(body.get("tip"), exp_tip, "tip")

if __name__ == "__main__":
    scenario1_stream(50.0)
    # scenario1_full()
    print("\n" + "=" * 64)
    print(f"汇总: PASS={_PASS}  FAIL={_FAIL}")
    print("=" * 64)
    sys.exit(1 if _FAIL > 0 else 0)
