#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
remedy_simulator.py - REMEDY高压发生器模拟器
"""

import threading
import serial
import time
import random
import argparse
from datetime import datetime

class RemedySimulator:
    def __init__(self, port="COM4", baudrate=19200):
        self.port = port
        self.baudrate = baudrate
        self.ser = None
        self.running = False
        self.thread = None
        
        # 模拟发生器状态
        self.state = {
            "kv": 120, "ma": 250, "ms": 2000, "mx": 49,
            "status": "002", 
            "anode_heat": 0, "flt": -1,
            "workstation": 1, "focus": 1, "technique": 1,
            "prep_state": 0, "exposure_state": 0,
            "aec_fields": "000", "film_screen": "000", "density": "+0",
            "fluoro_mode": 0,     # 0=RAD 2=Pulsed Fluoro
        }
        
        # 发生器限制参数
        self.limits = {
            "min_kv": 40, "max_kv": 150,
            "min_ma": 10, "max_ma": 10000,
            "min_ms": 10, "max_ms": 63000,
            "max_power": 50,
        }

    def start(self):
        """启动模拟器"""
        try:
            self.ser = serial.Serial(
                port=self.port,
                baudrate=self.baudrate,
                bytesize=8,
                parity='N',
                stopbits=1,
                timeout=0.1
            )
            self.running = True
            self.thread = threading.Thread(target=self._listen_loop, daemon=True)
            self.thread.start()
            print(f"🎯 REMEDY模拟器已启动在 {self.port}")
            print("📡 等待GUI连接...")
            return True
        except Exception as e:
            print(f"❌ 启动失败: {e}")
            return False
            
    def stop(self):
        """停止模拟器"""
        self.running = False
        if self.thread:
            self.thread.join(timeout=1.0)
        if self.ser and self.ser.is_open:
            self.ser.close()
        print("🛑 模拟器已停止")
        
    def _calculate_checksum(self, data):
        return sum(data) & 0xFF
        
    def _build_response(self, text):
        """构建协议响应帧"""
        data = text.encode('ascii') + b'\x03'
        checksum = self._calculate_checksum(data)
        return data + bytes([checksum])
        
    def _parse_frame(self, data):
        """解析接收到的数据帧"""
        try:
            if len(data) < 3:
                return None
                
            if data[-2] == 0x03:  # ETX
                received_checksum = data[-1]
                calculated_checksum = self._calculate_checksum(data[:-1])
                if received_checksum == calculated_checksum:
                    return data[:-2].decode('ascii', errors='ignore')
        except:
            pass
        return None
        
    def _handle_command(self, command):
        """处理命令并返回响应"""
        print(f"📨 收到命令: {command}")
        
        # 查询命令处理
        if command.endswith('?'):
            return self._handle_query_command(command)
            
        # 设置命令处理
        elif command.startswith(("KV", "MA", "MS", "FLK", "FLM", "FLF")):
            return self._handle_set_command(command)
            
        # 控制命令处理
        elif command.startswith(("PR", "XR", "FLP", "FLX")):
            return self._handle_control_command(command)
            
        # 系统命令
        elif command == "RE":
            return self._handle_reset_command()
        elif command == "RR":
            return self._handle_refresh_rad_command()
        elif command == "RS":
            return self._handle_refresh_system_command()
        elif command == "ST":
            return f"ST{self.state['status']}"
            
        # 默认返回原命令（确认回显）
        return command
            
    def _handle_query_command(self, command):
        """处理查询命令"""
        cmd_type = command[:-1]  # 去掉问号
        
        if cmd_type == "KV":
            return f"KV{self.state['kv']:03d}"
        elif cmd_type == "FLK":
            return f"FLK{self.state['kv']:05d}"
        elif cmd_type == "MA":
            return f"MA{self.state['ma']:05d}"
        elif cmd_type == "FLM":
            return f"FLM{self.state['ma']:05d}"
        elif cmd_type == "MS":
            return f"MS{self.state['ms']:05d}"
        elif cmd_type == "MX":
            return f"MX{self.state['mx']:05d}"
        elif cmd_type == "ST":
            return f"ST{self.state['status']}"
        elif cmd_type == "HE":
            return f"HE{self.state['anode_heat']:03d}"
        elif cmd_type == "FLF":
            return f"FLF{self.state['fluoro_mode']}"
        elif cmd_type == "WS":
            return f"WS{self.state['workstation']}"
        elif cmd_type == "FO":
            return f"FO{self.state['focus']}"
        elif cmd_type == "ET":
            return f"ET{self.state['technique']}"
        elif cmd_type == "AP":
            return f"AP{self.state['mx']:05d}"
        elif cmd_type == "AT":
            return f"AT{self.state['ms']:05d}"
        elif cmd_type == "FLT":
            return f"FLT{self.state['flt']:03d}"
        else:
            return command  # 未知查询命令，回显原命令
            
    def _handle_set_command(self, command):
        """处理设置命令"""
        try:
            if command.startswith("KV") and len(command) == 5:
                kv_value = int(command[2:])
                if 40 <= kv_value <= 150:
                    self.state['kv'] = kv_value
                return command

            elif command.startswith("FLK") and len(command) == 5:
                kv_value = int(command[2:])
                if 40 <= kv_value <= 150:
                    self.state['kv'] = kv_value
                return command
                
            elif command.startswith("MA") and len(command) == 7:
                ma_value = int(command[2:])
                self.state['ma'] = ma_value
                return command

            elif command.startswith("FLM") and len(command) == 7:
                ma_value = int(command[2:])
                self.state['ma'] = ma_value
                return command
                
            elif command.startswith("MS") and len(command) == 7:
                ms_value = int(command[2:])
                self.state['ms'] = ms_value
                return command
                
            elif command.startswith("FLF"):
                mode = int(command[3:])

                if mode == 0:
                    self.state["fluoro_mode"] = 0
                    self.state["status"] = "002"
                    print("📷 切换到 RAD 模式")

                elif mode == 2:
                    self.state["fluoro_mode"] = 2
                    self.state["status"] = "011"
                    print("🎥 脉冲透视模式")

                return command
                
        except ValueError:
            return command  # 解析失败也回显原命令
            
        return command
        
    def _handle_control_command(self, command):
        """处理控制命令"""
        if command == "PR1" and self.state['prep_state'] == 0:
            self.state['prep_state'] = 1
            self.state['status'] = "003"
            print(">>> [一档] 手闸按下 (PR1) - 等待 Ready...")
            return command

        elif command == "FLP1" and self.state['prep_state'] == 0 and self.state["fluoro_mode"] == 2:
            self.state["prep_state"] = 1
            self.state["status"] = "009"
            print(">>> [FLUORO] 准备透视 (FLP1)")
            return command

        elif self.state['prep_state'] == 1 and self.state['status'] == "003":
            command = "PR2"
            time.sleep(1)  # 模拟阳极启动时间
            self.state['prep_state'] = 2
            self.state['exposure_state'] = 3
            print(">>> [状态] 阳极启动完成 (PR2) - 请按二档曝光！")
            return command

        elif self.state['prep_state'] == 1 and self.state['status'] == "009":
            command = "FLX1"
            time.sleep(4)  # 模拟阳极启动时间
            self.state['prep_state'] = 2
            self.state['exposure_state'] = 56
            print(">>> [状态] 阳极启动完成")
            self.state['status'] = "011"
            return command
            
        elif command == "XR2" and self.state['prep_state'] == 2:
            self.state['status'] = "004"
            time.sleep(0.5)
            
            # 模拟曝光过程
            print("!!! [二档] 正在曝光 (XR2) !!!")
            self.state['status'] = "005"
            time.sleep(1)  # 模拟曝光持续时间
            
            self.state['exposure_state'] = self.state['exposure_state'] - 1
            print(self.state['exposure_state'])
            if self.state['exposure_state'] == 0 :
                # 生成曝光数据
                actual_ma = self.state['ma'] / 10.0
                actual_ms = self.state['ms'] / 10.0
                self.state['mx'] = int(actual_ma * actual_ms * 10)
                self.state['prep_state'] = 1
            
            return command

        elif command == "FLX1" and self.state['prep_state'] == 2 and self.state['status'] == "011":
            # 模拟曝光过程
            print("!!!正在曝光!!!")
            time.sleep(1)  # 模拟曝光持续时间
            
            self.state['exposure_state'] = self.state['exposure_state'] - 1
            print(self.state['exposure_state'])
            if self.state['exposure_state'] < 0 :
                self.state['prep_state'] = 0
                self.state['status'] = "002"
                print("⚡ 停止曝光")
                return "FLX0 FLP0"
            
            # 返回特定的状态信息，如果刚好处于特定的周期
            if (self.state['exposure_state'] % 7) == 0:
                self.state['flt'] = self.state['flt'] + 1
                return f"FLX1 FLT{self.state['flt']:03d} HE{self.state['anode_heat']:03d}"
            
            return command

            
        elif self.state['exposure_state'] == 0 and self.state['status'] == "005":
            command = "XR0"
            self.state['status'] = "004"
            print("⚡ 停止曝光")
            return command
        
        elif command == "PR0" and self.state['status'] == "004":
            self.state['prep_state'] = 0
            self.state['status'] = "002"
            print("<<< [复位] 回到待机 (PR0)")
            return command
        
    def _handle_reset_command(self):
        """处理复位命令"""
        print("🔄 系统复位")
        self.state['status'] = "002"
        time.sleep(0.5)
        return "RE"
        
    def _handle_refresh_rad_command(self):
        """处理刷新拍片数据命令"""
        return f"KV{self.state['kv']:03d} MA{self.state['ma']:05d} MS{self.state['ms']:05d} MX{self.state['mx']:05d}"
        
    def _handle_refresh_system_command(self):
        """处理刷新系统数据命令"""
        return (f"TU0 WS{self.state['workstation']} FO{self.state['focus']} "
                f"ET{self.state['technique']} FI{self.state['aec_fields']} "
                f"FS{self.state['film_screen']} FN{self.state['density']} "
                f"HE{self.state['anode_heat']:03d}")
        
    def _listen_loop(self):
        """监听循环"""
        buffer = bytearray()
        
        while self.running:
            try:
                # 读取数据
                data = self.ser.read(256)
                if data:
                    buffer.extend(data)
                    print(f"📥 原始数据: {data.hex(' ')}")
                    
                    # 处理缓冲区中的完整帧
                    while len(buffer) >= 3:
                        try:
                            etx_pos = buffer.index(0x03)
                            if len(buffer) >= etx_pos + 2:
                                frame = bytes(buffer[:etx_pos + 2])
                                buffer = buffer[etx_pos + 2:]
                                
                                command = self._parse_frame(frame)
                                if command:
                                    response = self._handle_command(command)
                                    if response:
                                        # 处理多命令响应
                                        if ' ' in response:
                                            for cmd in response.split():
                                                response_frame = self._build_response(cmd)
                                                self.ser.write(response_frame)
                                                self.ser.flush()
                                                print(f"📤 发送响应: {cmd}")
                                                time.sleep(0.05)
                                        else:
                                            response_frame = self._build_response(response)
                                            self.ser.write(response_frame)
                                            self.ser.flush()
                                            print(f"📤 发送响应: {response}")
                                            
                        except ValueError:
                            break
                            
            except Exception as e:
                print(f"❌ 通信错误: {e}")
                time.sleep(0.1)
                
            time.sleep(0.01)

# 启动函数
def main():
    parser = argparse.ArgumentParser(description="REMEDY 高压发生器模拟器")
    parser.add_argument("--port", default="COM4",
                        help="串口名（USB-TTL 接到 STM32 PD9/PD8 的那个口），默认 COM4")
    parser.add_argument("--baud", type=int, default=19200,
                        help="波特率，必须与 STM32 USART3 一致，默认 19200")
    parser.add_argument("--list", action="store_true",
                        help="列出本机可用串口后退出")
    args = parser.parse_args()

    if args.list:
        from serial.tools import list_ports
        for p in list_ports.comports():
            print(f"{p.device:8}  {p.description}")
        return

    simulator = RemedySimulator(args.port, args.baud)  # 修改为您的端口

    if simulator.start():
        try:
            print("模拟器运行中...按Ctrl+C退出")
            
            while True:
                time.sleep(1)
                # 随机更新热容量
                simulator.state['anode_heat'] = max(0, min(100, simulator.state['anode_heat'] + random.randint(-2, 2)))
                
        except KeyboardInterrupt:
            print("\n正在停止模拟器...")
        finally:
            simulator.stop()

if __name__ == "__main__":
    main()
