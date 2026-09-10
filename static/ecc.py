import os
import sys
import argparse
import numpy as np
import subprocess
# from scipy import signal
# from scipy.linalg import inv
# import matplotlib.pyplot as plt

class Config:
    first_proj_index = 1
    last_proj_index = 621
    proj_indices = range(first_proj_index, last_proj_index)
    filter = "gaussian"

config = Config()

def save_mhd(file_name, raw):
    base_name = os.path.basename(file_name)
    basename_without_ext = os.path.splitext(base_name)[0]
    folder = os.path.dirname(file_name)
    os.makedirs(folder, exist_ok=True)
    raw_path = os.path.join(folder, f"{basename_without_ext}.raw")
    raw = np.ascontiguousarray(raw, dtype=np.float32)
    print(f"Writing in chunks: {raw_path} ({raw.nbytes/1e6:.1f} MB)")
    try:
        with open(raw_path, "wb") as f:
            chunk_size = 100_000_000  # 100 MB per write
            total = raw.nbytes
            view = memoryview(raw)
            for i in range(0, total, chunk_size):
                end = min(i + chunk_size, total)
                f.write(view[i:end])
    except Exception as e:
        raise OSError(f"写入失败: {raw_path}, {e}")
    num_of_slices = config.last_proj_index - config.first_proj_index
    with open(file_name, "w") as f:
        f.write(f"""ObjectType = Image
NDims = 3
BinaryData = True
BinaryDataByteOrderMSB = False
CompressedData = False
TransformMatrix = 1 0 0 0 1 0 0 0 1
Offset = -213.296 -213.296 0
CenterOfRotation = 0 0 0
AnatomicalOrientation = RAI
ElementSpacing = 0.417 0.417 1
ElementType = MET_FLOAT
DimSize = 1024 1024 {num_of_slices}
ElementDataFile = {basename_without_ext}.raw
""")

def process_raw(input_file_name):
    raw = np.fromfile(input_file_name, dtype="uint16").reshape(1024,1024)
    # raw = np.rot90(raw, 2)
    raw[raw < 50] = 50
    processed = raw
    processed = processed.astype("float32")
    max = np.max(processed)
    processed /= max
    processed = -np.log(processed)
    return processed

def gen_projs(file_path):
    projts = []
    for i in config.proj_indices:
        input_file_name = os.path.join(file_path, f"{i:03d}.raw")
        p = process_raw(input_file_name)
        projts.append(p)

    p = np.array(projts, dtype=np.float32)
    save_mhd(os.path.join(file_path, "p1_22.mhd"), p)

    p2 = np.power(p, 2)
    save_mhd(os.path.join(file_path, "p2_22.mhd"), p2)

    p3 = np.power(p, 3)
    save_mhd(os.path.join(file_path, "p3_22.mhd"), p3)

    p4 = np.power(p, 4)
    save_mhd(os.path.join(file_path, "p4_22.mhd"), p4)

# def angle_mask(angle, center, width=10):
#     d = np.abs((angle - center + 180) % 360 - 180)
#     return d < width

# def calc_c4():
    # ct0 = np.fromfile(r"E:\96-CBCT-catphan\cbct_p0.raw", dtype="float32").reshape(300,512,512)
    # ct1 = np.fromfile(r"E:\96-CBCT-catphan\cbct_p1_new.raw", dtype="float32").reshape(300,512,512)
    # ct2 = np.fromfile(r"E:\96-CBCT-catphan\cbct_p2_new.raw", dtype="float32").reshape(300,512,512)
    # ct3 = np.fromfile(r"E:\96-CBCT-catphan\cbct_p3_new.raw", dtype="float32").reshape(300,512,512)
    # ct4 = np.fromfile(r"E:\96-CBCT-catphan\cbct_p4_new.raw", dtype="float32").reshape(300,512,512)

    # f = []
    # f.append(1)
    # slice = 150
    # # f.append(np.mean(ct0[:,150,:]).flatten())
    # f.append(ct1[slice,:,:].flatten())
    # f.append(ct2[slice,:,:].flatten())
    # f.append(ct3[slice,:,:].flatten())
    # f.append(ct4[slice,:,:].flatten())

    # r = 161
    # inner_r = 5
    # center_x = 272
    # center_y = 266
    # t = []
    # w = []
    # u_water = 0.2269
    # u_air = 0.0
    # y, x = np.indices((512, 512))
    # dx = x - center_x
    # dy = y - center_y
    # radius2 = dx ** 2 + dy ** 2
    # radius = np.sqrt(radius2)
    # angle = np.degrees(np.arctan2(dy, dx)) % 360
    # water_mask = ((radius < r) & (radius > inner_r))
    # air_angle_mask = (angle_mask(angle, 150, 80))
    # air_mask = (
    #     air_angle_mask &
    #     (radius > r + 40) &
    #     (radius < r + 50)
    # )
    # air_mask &= ~water_mask
    # plt.figure(figsize=(8, 8))

    # plt.imshow(ct1[slice], cmap="gray")

    # # 水：红色
    # plt.contour(
    #     water_mask.astype(np.float32),
    #     levels=[0.5],
    #     colors="red",
    #     linewidths=1.5
    # )

    # # 空气：蓝色
    # plt.contour(
    #     air_mask.astype(np.float32),
    #     levels=[0.5],
    #     colors="blue",
    #     linewidths=1.5
    # )

    # plt.scatter(
    #     center_x,
    #     center_y,
    #     c="yellow",
    #     s=20
    # )

    # plt.axis("image")
    # plt.show()

    # t_water = np.where(water_mask.flatten(), u_water, 0)
    # w_water = water_mask.flatten().astype(np.float32)

    # # 空气区域
    # t_air = np.where(air_mask.flatten(), u_air, 0)
    # w_air = air_mask.flatten().astype(np.float32)
    # w = w_water + w_air

    # a = []
    # for i in range(5):
    #     value = (np.sum(f[i] * t_water * w_water) + np.sum(f[i] * t_air * w_air))
    #     a.append(value)

    # B = []        
    # for j in range(5):
    #     b = []
    #     for i in range(5):
    #         if i != 0 or j != 0:
    #             b.append(np.sum(f[i]*f[j]*w))
    #         else:
    #             b.append(1)
    #     B.append(b)
        
    # np.set_printoptions(precision=3)
    # B = np.array(B)
    # c = inv(B) @ a
    # print(c)
    # return c

if __name__ == "__main__":
    # file_path = r'E:\96-CBCT-catphan'
    file_path = r'E:\spine_4BBs'
    # p = gen_projs(file_path)
    # c = calc_c4()
    c = [-2.495e-03,  2.794e-01,  6.475e+00, -6.625e-01, -4.847e-02]
    proj1 = np.fromfile(r"E:\spine_4BBs\p1_22.raw", dtype="float32")
    proj2 = np.fromfile(r"E:\spine_4BBs\p2_22.raw", dtype="float32")
    proj3 = np.fromfile(r"E:\spine_4BBs\p3_22.raw", dtype="float32")
    proj4 = np.fromfile(r"E:\spine_4BBs\p4_22.raw", dtype="float32")
    proj = c[0] *proj1 + c[1]*proj2 + c[2]*proj3 + c[3]*proj4
    save_mhd(os.path.join(file_path, "projections_corrected_22.mhd"), proj)