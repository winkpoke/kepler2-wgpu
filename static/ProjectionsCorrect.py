import os
import sys
import argparse
import numpy as np
import subprocess
import SimpleITK as sitk
import cv2

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
    return p

def crop_water_region(input_path: str, output_path: str):
    """对重建后的 CT 做圆形水区域裁剪"""
    print(f"开始裁剪水区域: {input_path}")
    original_image = sitk.ReadImage(input_path)
    original_array = sitk.GetArrayFromImage(original_image)
    ct0 = original_array.copy()

    sample_slice = ct0[ct0.shape[0] // 2, :, :]
    slice_512 = cv2.resize(sample_slice, (512, 512))

    img = cv2.normalize(slice_512, None, 0, 255, cv2.NORM_MINMAX).astype(np.uint8)
    img_blur = cv2.medianBlur(img, 5)

    h, w = img.shape

    circles = cv2.HoughCircles(
        img_blur, cv2.HOUGH_GRADIENT, dp=1.2, minDist=100,
        param1=100, param2=50,
        minRadius=50, maxRadius=700
    )

    if circles is not None:
        circles = np.uint16(np.around(circles))
        cx, cy, r_inner = circles[0][0]
        print(f"检测到圆: center=({cx}, {cy}), radius={r_inner}")
    else:
        cx, cy = 272, 240
        r_inner = min(w, h) // 2 + 50
        print(f"未检测到圆，使用默认: center=({cx}, {cy}), radius={r_inner}")

    yy, xx = np.ogrid[:h, :w]
    dist_from_center = np.sqrt((xx - cx)**2 + (yy - cy)**2)
    water_outer_radius = r_inner - 100
    reliable_water_mask = (dist_from_center <= water_outer_radius)

    orig_h, orig_w = sample_slice.shape
    mask_bool = cv2.resize(reliable_water_mask.astype(np.uint8), (orig_w, orig_h)).astype(bool)

    for z in range(ct0.shape[0]):
        ct0[z, :, :][~mask_bool] = 0

    output = sitk.GetImageFromArray(ct0)
    output.SetSpacing(original_image.GetSpacing())
    output.SetOrigin(original_image.GetOrigin())
    output.SetDirection(original_image.GetDirection())
    sitk.WriteImage(output, output_path)
    print(f"裁剪完成，已保存: {output_path}")

def rewrite_ct_mhd(mhd_path):
    """强制把最终 CT.mhd 改成指定头信息"""
    folder = os.path.dirname(mhd_path) or "."
    basename = os.path.splitext(os.path.basename(mhd_path))[0]
    raw_name = f"{basename}.raw"

    content = f"""ObjectType = Image
NDims = 3
BinaryData = True
BinaryDataByteOrderMSB = False
CompressedData = False
TransformMatrix = 1 0 0 0 1 0 0 0 1
Offset = 127.75 127.75 -127.075
CenterOfRotation = 0 0 0
AnatomicalOrientation = LSA
ElementSpacing = 0.5 0.5 0.5
DimSize = 512 512 512
ElementType = MET_FLOAT
ElementDataFile = {raw_name}
"""
    with open(mhd_path, "w") as f:
        f.write(content)
    print(f"已重写最终 MHD 头信息: {mhd_path}")

def main():
    parser = argparse.ArgumentParser(description="投影校正 + 重建 + 水区域裁剪")
    parser.add_argument("-i", "--input", required=True, help="输入文件夹（需包含 p1_new.raw ~ p4_new.raw）")
    parser.add_argument("-O", "--output", required=True, help="最终裁剪后的 CT.mhd 路径")
    parser.add_argument("--cali", default="cali.csv", help="标定文件路径")
    parser.add_argument("--proj-mhd", default="projections_corrected.mhd", help="中间投影 MHD 文件名")
    args = parser.parse_args()

    file_path = args.input
    final_ct_mhd = args.output
    cali_csv = args.cali
    proj_mhd = args.proj_mhd

    print("开始处理投影数据...")
    p = gen_projs(file_path)
    c = [-2.495e-03,  2.794e-01,  6.475e+00, -6.625e-01, -4.847e-02]
    proj = (c[0] * p +
            c[1] * np.power(p, 2) +
            c[2] * np.power(p, 3) +
            c[3] * np.power(p, 4))

    # 如果 -O 是目录，自动补文件名
    if os.path.isdir(final_ct_mhd) or final_ct_mhd.endswith(('/', '\\')):
        final_ct_mhd = os.path.join(final_ct_mhd, "CT_cropped.mha")

    out_dir = os.path.dirname(os.path.abspath(final_ct_mhd)) or "."
    os.makedirs(out_dir, exist_ok=True)

    # 中间重建文件（未裁剪）
    temp_ct_mhd = os.path.join(out_dir, "CT_temp.mhd")

    if not os.path.isabs(proj_mhd) and os.path.dirname(proj_mhd) == "":
        proj_mhd = os.path.join(out_dir, proj_mhd)

    print("正在导出校正后的投影 MHD...")
    save_mhd(proj_mhd, proj)

    # ============================================================
    # 2. 调用 reconstruct
    # ============================================================
    proj_dir = os.path.dirname(os.path.abspath(proj_mhd)) or "."
    proj_basename = os.path.basename(proj_mhd)

    cmd = [
        "reconstruct",
        "-c", cali_csv,
        "-r", proj_basename,
        "-p", proj_dir,
        "--output", temp_ct_mhd,
        "--spacing", "0.5,0.5,0.5",
        "--dimension", "512,512,512",
        "--verbose",
        "--hardware", "cuda",
        "--direction", "-1,0,0,0,0,1,0,-1,0",
        "--origin", "127.75,-127.75,127.075"
    ]

    print("开始执行重建命令:")
    print(" ".join(cmd))
    try:
        result = subprocess.run(cmd, check=True, capture_output=True, text=True)
        print(result.stdout)
        if result.stderr:
            print(result.stderr)
    except subprocess.CalledProcessError as e:
        print("重建失败！")
        print("stdout:", e.stdout)
        print("stderr:", e.stderr)
        sys.exit(1)
    except FileNotFoundError:
        print("错误：找不到 reconstruct 可执行文件")
        sys.exit(1)

    rewrite_ct_mhd(temp_ct_mhd)

    # ============================================================
    # 3. 裁剪水区域
    # ============================================================
    crop_water_region(temp_ct_mhd, final_ct_mhd)

    # ============================================================
    # 4. 清理临时文件
    # ============================================================
    print("清理临时文件...")
    try:
        os.remove(temp_ct_mhd)
        raw_temp = temp_ct_mhd.replace(".mhd", ".raw")
        if os.path.exists(raw_temp):
            os.remove(raw_temp)
        print(f"已删除临时重建文件: {temp_ct_mhd}")
    except Exception as e:
        print(f"删除临时 CT 失败: {e}")

    print(f"\n全部完成！最终裁剪结果：{final_ct_mhd}")


if __name__ == "__main__":
    main()