import SimpleITK as sitk
import numpy as np
import pydicom
from pydicom.dataset import FileDataset, FileMetaDataset
import datetime
import os
from pydicom.uid import generate_uid, ImplicitVRLittleEndian

def mha_file_to_dicom(mha_path, output_dir="dicom_output"):
    os.makedirs(output_dir, exist_ok=True)

    # ========================
    # 读取 MHA
    # ========================
    img = sitk.ReadImage(mha_path)
    
    # ========================
    # 🔥 核心：进行真实的 3D 重采样 (Resampling)
    # ========================
    # 前端传过来的变换矩阵 (列优先)
    # [409.3144, 21.45126, 0.0, 0.0, -21.45126, 409.3144, 0.0, 0.0, 0.0, 0.0, 409.87613, 0.0, -193.45073, -214.902, 0.99586487, 1.0]
    # 我们只提取纯旋转部分（去除缩放）来做重采样，因为缩放最好由 PixelSpacing 保持
    
    mat_col0 = np.array([409.3144, 21.45126, 0.0])
    mat_col1 = np.array([-21.45126, 409.3144, 0.0])
    mat_col2 = np.array([0.0, 0.0, 409.87613, 0.0])
    mat_offset = np.array([-193.45073, -214.902, 0.0])

    # 归一化提取纯旋转矩阵
    r0 = mat_col0 / np.linalg.norm(mat_col0)
    r1 = mat_col1 / np.linalg.norm(mat_col1)
    r2 = mat_col2 / np.linalg.norm(mat_col2)

    # 构建 3x3 旋转矩阵 (按行展开，SimpleITK 要求)
    rotation_matrix = [
        r0[0], r1[0], r2[0],
        r0[1], r1[1], r2[1],
        r0[2], r1[2], r2[2]
    ]

    # 创建仿射变换
    transform = sitk.AffineTransform(3)
    transform.SetMatrix(rotation_matrix)
    # 平移中心点 (可选，通常可以设在图像中心进行旋转)
    center = img.TransformContinuousIndexToPhysicalPoint(
        [sz/2.0 for sz in img.GetSize()]
    )
    transform.SetCenter(center)
    
    # 注意：如果想加上 offset，可以设置 transform.SetTranslation，
    # 但通常 DICOM 的位置由 Origin 决定，旋转即可。

    # 执行重采样！这会真实地旋转像素，而不仅仅是改 Header
    print("正在进行三维像素重采样旋转，请稍候...")
    resampled_img = sitk.Resample(
        img, 
        img, # 参考图像，保持原样大小和间距
        transform, 
        sitk.sitkLinear, # 线性插值
        -1000.0, # 默认背景值 (空气 HU)
        img.GetPixelID()
    )
    print("重采样完成！")

    # 从旋转后的图像中获取数据
    volume = sitk.GetArrayFromImage(resampled_img).astype(np.int16)
    spacing = np.array(resampled_img.GetSpacing(), dtype=np.float32)
    origin = np.array(resampled_img.GetOrigin(), dtype=np.float32)
    direction = np.array(resampled_img.GetDirection(), dtype=np.float32).reshape(3, 3)

    dim_z, dim_y, dim_x = volume.shape

    # 三个方向向量（列向量，现在它们应该是标准正交的）
    col0 = direction[:, 0]
    col1 = direction[:, 1]
    col2 = direction[:, 2]

    # ========================
    # UID（整套数据共享）
    # ========================
    study_uid = generate_uid()
    series_uid = generate_uid()
    frame_uid = generate_uid()

    now = datetime.datetime.now()

    for z in range(dim_z):
        filename = os.path.join(output_dir, f"slice_{z:04d}.dcm")

        # 每张 slice 唯一 SOP UID
        sop_uid = generate_uid()

        # ========================
        # File Meta（必须有）
        # ========================
        file_meta = FileMetaDataset()
        file_meta.MediaStorageSOPClassUID = pydicom.uid.CTImageStorage
        file_meta.MediaStorageSOPInstanceUID = sop_uid
        file_meta.TransferSyntaxUID = ImplicitVRLittleEndian

        ds = FileDataset(filename, {}, file_meta=file_meta, preamble=b"\0" * 128)

        # ========================
        # 基本信息
        # ========================
        ds.PatientName = "Test^Patient"
        ds.PatientID = "123456"
        ds.Modality = "CT"

        ds.StudyInstanceUID = study_uid
        ds.SeriesInstanceUID = series_uid
        ds.FrameOfReferenceUID = frame_uid

        ds.SOPInstanceUID = sop_uid
        ds.SOPClassUID = file_meta.MediaStorageSOPClassUID

        ds.StudyDate = now.strftime("%Y%m%d")
        ds.StudyTime = now.strftime("%H%M%S")

        # ========================
        # 图像维度
        # ========================
        ds.Rows = dim_y
        ds.Columns = dim_x

        ds.PixelSpacing = [float(spacing[0]), float(spacing[1])]
        ds.SliceThickness = float(spacing[2])
        ds.SpacingBetweenSlices = float(spacing[2])

        ds.PixelRepresentation = 0  # 0 = 无符号 (Unsigned)
        ds.BitsAllocated = 16
        ds.BitsStored = 16
        ds.HighBit = 15
        ds.SamplesPerPixel = 1
        ds.PhotometricInterpretation = "MONOCHROME2"

        # ========================
        # 🔥 关键：方向（决定是否“倾斜”）
        # ========================
        ds.ImageOrientationPatient = [
            float(col0[0]), float(col0[1]), float(col0[2]),
            float(col1[0]), float(col1[1]), float(col1[2])
        ]

        # ========================
        # 🔥 关键：位置（真正决定空间）
        # ========================
        image_position = origin + col2 * z * spacing[2]
        ds.ImagePositionPatient = [float(c) for c in image_position]

        # ========================
        # 排序 & 空间辅助
        # ========================
        ds.InstanceNumber = z + 1
        ds.SliceLocation = float(np.dot(image_position, col2))

        # ========================
        # CT 强度（关键修复点）
        # ========================
        ds.RescaleIntercept = "-1024"
        ds.RescaleSlope = "1"

        # ========================
        # Pixel 数据
        # ========================
        # 将有符号的 HU 值 (比如 -1024) 加上 1024 变成无符号 (比如 0) 存储
        # 这样配合 RescaleIntercept = -1024，查看器解析后仍然是正确的 HU 值
        pixel_array = volume[z].astype(np.int32) + 1024
        # 防止溢出，裁切到 uint16 的范围
        pixel_array = np.clip(pixel_array, 0, 65535).astype(np.uint16)
        ds.PixelData = pixel_array.tobytes()

        # 保存
        ds.save_as(filename)

    print(f"✅ 导出完成，DICOM 文件保存在 '{output_dir}' 文件夹。")


# ========================
# 示例调用
# ========================
if __name__ == "__main__":
    mha_path = "C:\\Users\\admin\\Downloads\\exported_noisy_volume_10.mha"
    mha_file_to_dicom(mha_path, output_dir="dicom_output")