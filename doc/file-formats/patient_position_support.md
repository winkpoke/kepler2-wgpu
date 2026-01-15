# DICOM PatientPosition (0018,5100) 支持

## 概述

本软件现已支持识别并解析DICOM中的PatientPosition(0018,5100)标签，并能根据识别结果正确导入各种位姿的CT影像。

## 功能特性

### 1. PatientPosition标签解析
- **正确的DICOM标签**: 使用标准的(0018,5100)标签
- **多格式支持**: 支持标准DICOM格式和描述性文本格式
- **大小写不敏感**: 自动处理不同大小写的输入
- **默认回退**: 对于无法识别的位姿，默认使用HFS (Head First Supine)

### 2. 支持的患者位姿

| 位姿代码 | 描述 | 中文描述 |
|---------|------|----------|
| HFS | Head First Supine | 头先进仰卧位 |
| HFP | Head First Prone | 头先进俯卧位 |
| HFDR | Head First Decubitus Right | 头先进右侧卧位 |
| HFDL | Head First Decubitus Left | 头先进左侧卧位 |
| FFS | Feet First Supine | 足先进仰卧位 |
| FFP | Feet First Prone | 足先进俯卧位 |
| FFDR | Feet First Decubitus Right | 足先进右侧卧位 |
| FFDL | Feet First Decubitus Left | 足先进左侧卧位 |

### 3. 坐标系变换

根据不同的患者位姿，系统会自动应用相应的坐标系变换：

```rust,no_run
// 示例：不同位姿的坐标变换标志
let transforms = [
    (PatientPosition::HFS, (false, false, false)), // 无变换
    (PatientPosition::HFP, (false, true, true)),   // Y轴和Z轴翻转
    (PatientPosition::FFS, (false, false, true)),  // Z轴翻转
    (PatientPosition::FFP, (false, true, false)),  // Y轴翻转
    // ... 其他位姿
];
```

## 实现细节

### 1. 核心组件

#### PatientPosition
位于 `src/data/medical_imaging/metadata.rs`，提供以下功能：
- `to_string()`: 将位姿转换为字符串
- `from_str()`: 解析位姿字符串
- `validate_position_consistency()`: 验证位姿与图像方向的一致性
- `get_coordinate_transform()`: 获取坐标变换标志

#### 更新的数据结构
- **CTImage**: 添加了patient_position字段
- **DicomRepo**: 集成了位姿解析和坐标变换

### 2. 使用示例

```rust,no_run
use kepler_wgpu::data::medical_imaging::image_info::PatientPosition;

// 解析患者位姿
let position = PatientPosition::from_str(Some("HFP".to_string()))?;
assert_eq!(position, PatientPosition::HFP);

// 获取坐标变换
let (flip_x, flip_y, flip_z) = position.get_coordinate_transform();

// 验证位姿一致性
let orientation = (1.0, 0.0, 0.0, 0.0, 1.0, 0.0);
position.validate_position_consistency(Some(orientation))?;
```

### 3. CTVolume生成集成

在生成CTVolume时，系统会：
1. 解析PatientPosition标签
2. 验证与ImageOrientationPatient的一致性
3. 应用相应的坐标变换
4. 调整方向矩阵以确保正确的空间定位

## 测试覆盖

### 单元测试
- 标准位姿解析测试
- 大小写不敏感测试
- 回退机制测试
- 坐标变换测试
- 一致性验证测试

### 集成测试
- CTImage与PatientPosition的集成
- CTVolume生成的完整工作流
- 不同位姿的端到端测试

## 日志记录

系统会记录以下信息：
- **INFO**: 成功解析的PatientPosition
- **WARN**: 位姿与图像方向不一致的警告
- **DEBUG**: 坐标变换标志的详细信息

## 兼容性

- **原生构建**: 完全支持所有功能
- **WebAssembly**: 支持所有核心功能
- **跨平台**: Windows、macOS、Linux全平台支持

## 注意事项

1. **数值精度**: 医学影像要求高精度，所有坐标变换都使用f64精度
2. **内存效率**: 大型CT数据集的处理经过优化，避免不必要的数据复制
3. **错误处理**: 提供详细的错误信息和回退机制
4. **向后兼容**: 对于缺失PatientPosition标签的旧DICOM文件，默认使用HFS

## 更新日志

- 在CTImage结构体中添加PatientPosition字段支持
- 实现PatientPosition解析器，支持多种格式和回退机制
- 更新CTVolume生成逻辑，集成位姿识别和坐标变换
- 添加全面的单元测试和集成测试覆盖