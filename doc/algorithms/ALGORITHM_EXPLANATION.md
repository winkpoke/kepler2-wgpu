# Kepler2-WGPU 医学影像渲染算法说明文档

> **版本**: 1.0  
> **更新日期**: 2026-05-25  
> **作者**: Sisyphus (AI Agent)

---

## 目录

1. [概述](#1-概述)
2. [窗宽窗位（Window Width/Level）](#2-窗宽窗位window-widthlevel)
   - [2.1 基本概念](#21-基本概念)
   - [2.2 窗宽（Window Width）](#22-窗宽window-width)
   - [2.3 窗位（Window Level）](#23-窗位window-level)
   - [2.4 偏移量（Bias）](#24-偏移量bias)
   - [2.5 在 MIP 视图中的作用](#25-在-mip-视图中的作用)
   - [2.6 在 MESH 视图中的作用](#26-在-mesh-视图中的作用)
3. [MIP 渲染算法](#3-mip-渲染算法)
   - [3.1 什么是 MIP](#31-什么是-mip)
   - [3.2 光线投射（Ray Casting）原理](#32-光线投射ray-casting原理)
   - [3.3 MIP/MINIP/AVGIP 三种模式](#33-mipminipavgip-三种模式)
   - [3.4 旋转操作](#34-旋转操作)
4. [MESH 渲染算法](#4-mesh-渲染算法)
   - [4.1 什么是 MESH 渲染](#41-什么是-mesh-渲染)
   - [4.2 等值面提取（Iso-surface）](#42-等值面提取iso-surface)
   - [4.3 直接体渲染（DVR）](#43-直接体渲染dvr)
   - [4.4 光照模型](#44-光照模型)
   - [4.5 旋转操作](#45-旋转操作)
5. [交互操作汇总](#5-交互操作汇总)
6. [预设参数参考](#6-预设参数参考)
7. [数学公式与实现细节](#7-数学公式与实现细节)

---

## 1. 概述

Kepler2-WGPU 是一个基于 Rust + WGPU 的医学影像三维可视化系统，主要用于 CT 数据的重建、多平面重建（MPR）、最大密度投影（MIP）以及三维体渲染（MESH）。

本文档详细解释系统中核心渲染算法的数学原理和实现细节。

---

## 2. 窗宽窗位（Window Width/Level）

### 2.1 基本概念

CT 图像使用 **Hounsfield Unit (HU)** 来表示组织的 X 射线衰减系数。CT 值的范围通常是 **-1024 HU（空气）到 +3071 HU（骨骼）**。

人眼只能分辨约 256 级灰度，无法直接观察整个 HU 范围。因此，医学影像使用**窗宽窗位**技术来选择性地显示感兴趣区域的组织。

### 2.2 窗宽（Window Width）

**窗宽**控制**对比度（Contrast）**，决定显示的 HU 值范围：

- **窗宽越窄** → 对比度越高，灰度变化更明显
  - 例如脑窗：WW = 80 HU，可以清晰区分脑组织细微差异
- **窗宽越宽** → 对比度越低，能同时看到更多组织类型
  - 例如骨窗：WW = 1500 HU，可以同时看到软组织和骨骼

**数学定义**：
```
min_val = level - 0.5 - (width - 1.0) × 0.5
max_val = level - 0.5 + (width - 1.0) × 0.5
```

### 2.3 窗位（Window Level）

**窗位**控制**亮度中心**，决定以哪个 HU 值为中心进行显示：

- **高窗位** → 偏向观察高密度组织（骨骼）
- **低窗位** → 偏向观察低密度组织（肺、空气）

### 2.4 偏移量（Bias）

系统额外提供 **bias** 参数，对窗位进行微调偏移：

```
effective_level = window_level + bias
```

这在需要临时微调显示而不改变主窗位时非常有用。

### 2.5 在 MIP 视图中的作用

在 MIP 渲染中，窗宽窗位作用于**投影后的最终像素值**：

```glsl
// mip.wgsl 中的 DICOM 窗宽窗位映射
fn apply_window_level(value: f32) -> f32 {
    let center = u_mip.level;
    let width = max(u_mip.window, 1e-6);
    let min_val = center - 0.5 - (width - 1.0) * 0.5;
    let max_val = center - 0.5 + (width - 1.0) * 0.5;
    let v = (value - min_val) / (max_val - min_val);
    return clamp(v, 0.0, 1.0);
}
```

**流程**：
1. GPU 沿光线方向遍历体积数据
2. 对每个像素，取路径上的最大/最小/平均 HU 值
3. 将最终 HU 值通过窗宽窗位映射到 [0, 1] 显示灰度

**调整窗宽窗位的影响**：
- 改变**哪些 HU 值可见**，但**不会改变投影结果本身**
- 例如：设置肺窗（WW=1500, WL=-600）后，只有 -1024 到 0 左右的 HU 值会显示为不同灰度

### 2.6 在 MESH 视图中的作用

在 MESH 渲染中，窗宽窗位的作用更加复杂：

1. **控制可见性阈值**：低于窗底的值会被完全跳过
   ```glsl
   if (hu < min_val) {
       t = t + dt * 2.0;  // 加速跳过不可见区域
       continue;
   }
   ```

2. **控制颜色映射**（DVR 模式）：
   ```glsl
   fn transfer_function(hu: f32, grad_mag: f32) -> vec4<f32> {
       let norm = apply_window_level(hu);
       if (norm <= 0.0) {
           return vec4<f32>(0.0);  // 完全透明
       }
       // 颜色从深棕 → 肉色 → 骨白
       ...
   }
   ```

3. **控制不透明度**（剥离感效果）：
   ```glsl
   var base_alpha = pow(norm, 2.5);  // 低值更透明，高值更不透明
   ```

**调整窗宽窗位的影响**：
- 在 **ISO 模式**：窗位决定等值面阈值，直接影响哪些组织表面被提取
- 在 **DVR 模式**：窗宽窗位同时控制颜色、透明度和可见性
窗位：
· 上调显示密度、亮度数值更高的体素
· 下调显示密度、亮度数值更低的体素
窗宽：
· 向左缩小体素显示数值范围
· 向右扩大体素显示数值范围

---

## 3. MIP 渲染算法

### 3.1 什么是 MIP

**MIP（Maximum Intensity Projection，最大密度投影）** 是一种体渲染技术：

- 对每个屏幕像素，沿视线方向穿过三维体积
- 记录路径上的**最大 HU 值**作为该像素的输出
- 特别适合观察高密度结构（血管、骨骼、造影剂增强区域）

### 3.2 光线投射（Ray Casting）原理

系统使用 GPU 光线投射算法：

```
对每个屏幕像素 (u, v):
  1. 计算穿过体积的光线：origin + t × direction
  2. 计算光线与体积边界的交点 [t_start, t_end]
  3. 沿光线步进采样：
     for t in t_start to t_end:
         intensity = sample_volume(origin + t × direction)
         max_intensity = max(max_intensity, intensity)
  4. 应用窗宽窗位 → 显示灰度
```

**关键参数**：
- `ray_step_size`：步进间隔（mm），默认 0.005mm
- `max_steps`：最大步数，默认 1000 步
- `lower_threshold` / `upper_threshold`：HU 值过滤范围

### 3.3 旋转操作

MIP 视图的旋转通过**旋转投影光线的方向**来实现：

#### 3.3.1 旋转矩阵构建

旋转矩阵由三个欧拉角组合而成：
```rust
// 顺序：Roll (绕Z) → Yaw (绕Y) → Pitch (绕X)
let rot = Mat4::from_rotation_x(roll) * 
          Mat4::from_rotation_y(yaw) * 
          Mat4::from_rotation_z(pitch);
```

或者使用四元数（避免万向锁）：
```rust
pub fn set_rotation_quat(&mut self, rotation: [f32; 4]) {
    self.rotation_quat = Quat::from_array(rotation);
}
```

#### 3.3.2 旋转在 Shader 中的应用

```glsl
// 基础光线方向 (沿 Z 轴)
let base_ray_origin = vec3<f32>(uv.x, 1.0 - uv.y, -0.5);

// 应用旋转矩阵变换光线起点和方向
let volume_ray_origin = (u_mip.rotation * vec4<f32>(base_ray_origin - center, 1.0)).xyz + center;
let volume_ray_dir = normalize((u_mip.rotation * vec4<f32>(0.0, 0.0, 1.0, 0.0)).xyz);
```

**直观理解**：
- 旋转操作**不改变体积数据**
- 旋转改变的是**观察角度**，即光线穿过体积的方向
- 例如：绕 Y 轴旋转 90° 相当于从侧面看患者

#### 3.3.3 交互式旋转

```rust
pub fn set_rotation_angle_degrees(&mut self, degrees_x: f32, degrees_y: f32) {
    // 基于当前姿态的增量旋转
    let right = self.rotation_quat * Vec3::Y;
    let up = self.rotation_quat * Vec3::X;
    let dx = degrees_x.to_radians();
    let dy = degrees_y.to_radians();
    let qx = Quat::from_axis_angle(up.normalize(), dx);
    let qy = Quat::from_axis_angle(right.normalize(), dy);
    let delta = qy * qx;
    self.rotation_quat = (delta * self.rotation_quat).normalize();
}
```

这种设计确保旋转始终**相对于当前观察方向**，而不是固定的世界坐标系。

---

## 4. MESH 渲染算法

### 4.1 什么是 MESH 渲染

MESH 渲染是**三维体渲染（Volume Rendering）**，在 GPU 中通过光线步进实时计算每个像素的颜色。与 MIP 的投影不同，MESH 渲染：

- 模拟光线穿过体积时的**吸收和散射**
- 计算**光照和阴影**效果
- 支持**半透明和颜色映射**

系统支持两种体渲染模式：
- **ISO（等值面提取）**：提取特定 HU 值的表面（类似三维重建骨骼）
- **DVR（直接体渲染）**：逐体素计算颜色和透明度

### 4.2 等值面提取（Iso-surface）

等值面渲染查找 HU 值等于阈值的位置，并提取其表面：

```glsl
fn iso_ray_march(ray_origin, ray_dir, t0, t1, iso) -> vec4<f32> {
    var v_prev = sample_volume(ray_origin + t * ray_dir) - iso;
    
    for (var i = 0u; i < max_steps; i++) {
        let v_cur = sample_volume(p) - iso;
        
        // 零交叉检测：表面在这里！
        if (v_prev * v_cur <= 0.0) {
            // 二分法精确定位表面位置
            for (var j = 0u; j < 4u; j++) {
                let m = 0.5 * (a + b);
                let vm = sample_volume(ray_origin + m * ray_dir) - iso;
                // 二分迭代...
            }
            
            // 计算表面法线和光照
            let n = compute_normal(hit_pos);
            let col = compute_lighting(n, v, base);
            return vec4<f32>(col * sample_alpha, sample_alpha);
        }
        v_prev = v_cur;
    }
}
```

**等值面阈值选择**：
- **BONE 预设**：ISO = 300 HU（提取骨骼表面）
- **SOFT 预设**：ISO = window_level（使用窗位作为阈值）

### 4.3 直接体渲染（DVR）

DVR 模式对路径上每个采样点计算颜色和透明度，并进行累积：

```glsl
fn dvr_ray_march(ray_origin, ray_dir, t0, t1) -> vec4<f32> {
    var accum_rgb = vec3<f32>(0.0);
    var accum_a = 0.0;
    
    for (var i = 0u; i < max_steps; i++) {
        let hu = sample_volume(pos);
        
        // 传递函数：HU → 颜色 + 透明度
        let tf = transfer_function(hu, grad_mag);
        
        if (tf.a > 0.005) {
            let vdir = normalize(-ray_dir);
            let lit_color = compute_lighting(n, vdir, tf.rgb);
            
            // 体积密度计算
            let mapped_opacity = pow(u_vol.opacity_multiplier, 6.0);
            let density = tf.a * mapped_opacity * 3.0;
            let volume_alpha = 1.0 - exp(-density * step_len);
            
            // 前向累积合成
            accum_rgb += (1.0 - accum_a) * final_color * final_alpha;
            accum_a += (1.0 - accum_a) * final_alpha;
        }
    }
}
```

**传递函数（Transfer Function）**：

DVR 模式下，系统使用多级颜色映射：

```glsl
fn transfer_function(hu: f32, grad_mag: f32) -> vec4<f32> {
    let norm = apply_window_level(hu);
    
    // 颜色映射：深棕 → 肉色 → 骨白
    let color_low = vec3<f32>(0.18, 0.14, 0.12);   // 深灰褐色
    let color_mid = vec3<f32>(0.72, 0.55, 0.44);   // 肉色
    let color_high = vec3<f32>(0.98, 0.96, 0.92);  // 骨白色
    
    color = mix(color_low, color_mid, smoothstep(0.0, 0.5, norm));
    color = mix(color, color_high, smoothstep(0.5, 0.9, norm));
    
    // 透明度：低值透明，高值不透明（剥离感）
    var base_alpha = pow(norm, 2.5);
}
```

**效果**：
- 低 HU 值（软组织）→ 较透明，产生"剥离"效果
- 高 HU 值（骨骼）→ 较不透明，清晰显示
- 梯度边缘增强：表面交界处不透明度更高

### 4.4 光照模型

系统使用**三光源 Phong 光照模型**：

| 光源 | 名称 | 位置 | 作用 |
|------|------|------|------|
| Light 0 | 主光（Key） | 相机右侧前方 | 主要照明 |
| Light 1 | 补光（Fill） | 相机左后方 | 填充阴影 |
| Light 2 | 边缘光（Rim） | 相机后方 | 突出轮廓 |

```glsl
fn compute_lighting(normal_in, view_dir_in, base_color) -> vec3<f32> {
    // 主光：镜面反射强度 16 次方
    let spec0 = pow(max(dot(n, h0), 0.0), 16.0);
    
    // 补光：柔和填充，镜面反射 4 次方
    let spec1 = pow(max(dot(n, h1), 0.0), 4.0);
    
    // 边缘光：轮廓高光，镜面反射 24 次方
    let spec2 = pow(max(dot(n, h2), 0.0), 24.0);
    
    let ambient = 0.5;
    let diffuse = ...; // 三光源漫反射
    let specular = ...; // 三光源镜面反射
    
    return base_color * (ambient + diffuse) + specular;
}
```

**法线计算**：

使用中心差分近似梯度作为表面法线：

```glsl
fn compute_normal(pos: vec3<f32>) -> vec3<f32> {
    let h = 1.0 / u_vol.vol_dims;
    let dx = sample_volume(pos + vec3(h.x,0,0)) - sample_volume(pos - vec3(h.x,0,0));
    let dy = sample_volume(pos + vec3(0,h.y,0)) - sample_volume(pos - vec3(0,h.y,0));
    let dz = sample_volume(pos + vec3(0,0,h.z)) - sample_volume(pos - vec3(0,0,h.z));
    return normalize(vec3<f32>(dx, dy, dz));
}
```

### 4.5 旋转操作

MESH 视图的旋转机制与 MIP 类似，但更复杂：

#### 4.5.1 旋转矩阵的构建

```rust
pub fn update_uniforms(&mut self, queue: &wgpu::Queue) {
    // 可选的自动旋转动画
    if self.rotation_enabled {
        let delta_time = current_time.duration_since(self.last_frame_time).as_secs_f32();
        let angle_delta = self.rotation_speed * delta_time;
        let rot_delta = Quat::from_rotation_y(angle_delta);
        self.rotation_quat = (rot_delta * self.rotation_quat).normalize();
    }
    
    // 构建最终变换矩阵
    let rotation = Mat4::from_quat(self.rotation_quat);
    let final_matrix = scale_texture * rotation * scale_viewport;
}
```

#### 4.5.2 自动旋转

MESH 视图支持自动旋转动画（默认 90°/秒）：

```rust
pub fn set_rotation_speed(&mut self, speed_rad_per_sec: f32) {
    self.rotation_speed = speed_rad_per_sec;
}
```

#### 4.5.3 Shader 中的旋转应用

```glsl
// 应用旋转到光线起点和方向
let ray_origin = (u_vol.rotation * vec4<f32>(base_ray_origin - center, 1.0)).xyz + center;
let ray_dir = normalize((u_vol.rotation * vec4<f32>(0.0, 0.0, 1.0, 0.0)).xyz);
```

与 MIP 的区别：
- MESH 的旋转矩阵还包含**纹理缩放**和**视口缩放**
- 光照计算始终基于**旋转后的法线和视线方向**，确保光照效果正确

---

## 5. 交互操作汇总

| 操作 | MIP | MESH | 说明 |
|------|-----|------|------|
| **窗宽调整** | ✅ | ✅ | 控制对比度 |
| **窗位调整** | ✅ | ✅ | 控制亮度中心 |
| **手动旋转** | ✅ | ✅ | 鼠标拖拽或 API 调用 |
| **缩放** | ✅ | ✅ | 鼠标滚轮 |
| **平移** | ✅ | ✅ | 鼠标中键或右键 |
| **透明度控制** | ❌ | ✅ | DVR 模式专用 |
| **ROI 选择** | ❌ | ✅ | 区域裁剪 |
---

## 6. 预设参数参考

### 6.1 窗宽窗位预设

| 预设 | 窗宽 (WW) | 窗位 (WL) | 显示范围 | 适用场景 |
|------|-----------|-----------|----------|----------|
| **软组织（Soft Tissue）** | 400 | 40 | -160 到 240 HU | 肌肉、器官 |
| **骨骼（Bone）** | 1500 | 400 | -350 到 1150 HU | 骨骼结构 |
| **肺（Lung）** | 1500 | -600 | -1350 到 150 HU | 肺部、气道 |
| **脑（Brain）** | 80 | 40 | 0 到 80 HU | 脑组织 |
| **肝脏（Liver）** | 150 | 30 | -45 到 105 HU | 肝脏病变 |

### 6.2 MIP 模式参数

| 模式 | 默认窗位 | 步进大小 | 最大步数 | HU 范围 |
|------|----------|----------|----------|---------|
| **MIP (Mode 0)** | 骨骼窗 | 0.005 mm | 1000 | -1024 ~ 3071 |
| **MinIP (Mode 1)** | 肺窗 | 0.004 mm | 1500 | -1024 ~ 300 |
| **AvgIP (Mode 2)** | 软组织窗 | 0.003 mm | 2000 | -200 ~ 300 |

### 6.3 MESH 模式

| 模式 | preset 值 | 渲染类型 | 等值面阈值 |
|------|-----------|----------|-----------|
| **BONE** | 0 | ISO 等值面 | 300 HU |
| **SOFT** | ≥ 0.5 | DVR 体渲染 | 窗位 |

---

## 7. 数学公式与实现细节

### 7.1 窗宽窗位映射公式

```
min_val = level - 0.5 - (width - 1.0) × 0.5
max_val = level - 0.5 + (width - 1.0) × 0.5
display_value = clamp((value - min_val) / (max_val - min_val), 0.0, 1.0)
```

### 7.2 光线与轴对齐包围盒求交

```glsl
fn intersect_volume(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> vec2<f32> {
    let t0 = (vec3<f32>(0.0) - ray_origin) / ray_dir;
    let t1 = (vec3<f32>(1.0) - ray_origin) / ray_dir;
    let tmin = max(min(t0.x,t1.x), min(t0.y,t1.y), min(t0.z,t1.z));
    let tmax = min(max(t0.x,t1.x), max(t0.y,t1.y), max(t0.z,t1.z));
    return vec2<f32>(tmin, tmax);
}
```

### 7.3 体积合成公式（前向累积）

```glsl
accum_rgb += (1.0 - accum_a) × color × alpha;
accum_a += (1.0 - accum_a) × alpha;
```

### 7.4 指数不透明度映射

```glsl
density = alpha × pow(opacity_multiplier, 6.0) × 3.0;
final_alpha = 1.0 - exp(-density × step_len);
```

### 7.5 四元数旋转

```rust
// 增量旋转：在局部坐标系中旋转
let delta = Quat::from_axis_angle(axis, angle);
new_rotation = delta * current_rotation;
```
