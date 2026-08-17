import os
import numpy as np
import pydicom
from pydicom.dataset import Dataset, FileDataset
from datetime import datetime
import random

# --------------------------
# 配置
# --------------------------
output_dir = "abnormal_dicoms"
os.makedirs(output_dir, exist_ok=True)

num_slices = 10
image_size = (128, 128)  # 图像尺寸
modality = "CT"
manufacturer = "PythonSimulator"

# --------------------------
# 异常生成函数
# --------------------------
def generate_abnormal_pixel_array(size, anomaly_type="negative"):
    """生成异常像素矩阵"""
    if anomaly_type == "negative":
        return np.random.randint(-2000, 0, size=size).astype(np.int16)
    elif anomaly_type == "overflow":
        return np.random.randint(4000, 10000, size=size).astype(np.int16)
    elif anomaly_type == "random_nan":
        arr = np.random.randint(-1000, 3000, size=size).astype(np.float32)
        arr[random.randint(0, size[0]-1), random.randint(0, size[1]-1)] = np.nan
        return arr
    else:
        # 正常随机CT HU
        return np.random.randint(-1000, 3000, size=size).astype(np.int16)

def generate_abnormal_dicom(slice_index, anomaly_pixel_type="negative", missing_tags=False, bad_geometry=False):
    """生成单张异常DICOM"""
    filename = os.path.join(output_dir, f"ABNORMAL_{slice_index:03d}.dcm")
    
    # 基本 DICOM 元数据
    file_meta = pydicom.dataset.FileMetaDataset()
    file_meta.MediaStorageSOPClassUID = pydicom.uid.CTImageStorage
    file_meta.MediaStorageSOPInstanceUID = pydicom.uid.generate_uid()
    file_meta.TransferSyntaxUID = pydicom.uid.ExplicitVRLittleEndian

    ds = FileDataset(filename, {}, file_meta=file_meta, preamble=b"\0"*128)

    # 常规标签
    ds.PatientName = "Abnormal^Patient"
    ds.PatientID = f"PATIENT_{slice_index:03d}"
    ds.Modality = modality
    ds.Manufacturer = manufacturer
    ds.StudyInstanceUID = pydicom.uid.generate_uid()
    ds.SeriesInstanceUID = pydicom.uid.generate_uid()
    ds.SOPInstanceUID = pydicom.uid.generate_uid()
    ds.SOPClassUID = pydicom.uid.CTImageStorage
    ds.ImagePositionPatient = [0.0, 0.0, float(slice_index)] if not bad_geometry else [0.0, 0.0, float(slice_index*10)]
    ds.ImageOrientationPatient = [1.0,0.0,0.0,0.0,1.0,0.0] if not bad_geometry else [0.0,1.0,0.0,1.0,0.0,0.0]
    ds.PixelSpacing = [0.9765625, 0.9765625] if not bad_geometry else [0.1, 10.0]
    ds.SliceThickness = 2.5 if not bad_geometry else -1.0

    # 日期/时间
    ds.StudyDate = "20250101" if not missing_tags else ""
    ds.StudyTime = "120000" if not missing_tags else ""
    
    # 图像数据
    pixel_array = generate_abnormal_pixel_array(image_size, anomaly_pixel_type)
    ds.Rows, ds.Columns = pixel_array.shape
    ds.PixelData = pixel_array.tobytes()
    ds.BitsAllocated = 16
    ds.BitsStored = 16
    ds.HighBit = 15
    ds.PixelRepresentation = 1  # 有符号
    ds.RescaleIntercept = 0
    ds.RescaleSlope = 1

    if missing_tags:
        # 删除必需标签测试异常处理
        if "PatientID" in ds:
            del ds.PatientID
        if "Rows" in ds:
            del ds.Rows

    ds.save_as(filename)
    print(f"Saved {filename}")

# --------------------------
# 生成整个序列
# --------------------------
for i in range(num_slices):
    anomaly_type = random.choice(["negative", "overflow", "random_nan", "normal"])
    missing_tags_flag = random.choice([False, True])
    bad_geometry_flag = random.choice([False, True])
    generate_abnormal_dicom(
        i, 
        anomaly_pixel_type=anomaly_type, 
        missing_tags=missing_tags_flag, 
        bad_geometry=bad_geometry_flag
    )

print("All abnormal DICOMs generated in folder:", output_dir)