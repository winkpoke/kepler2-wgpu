import SimpleITK as sitk
import cv2
import numpy as np
import argparse

def main():
    parser = argparse.ArgumentParser(description="Crop water region from CT scan.")
    parser.add_argument("--input", type=str, required=True, help="Path to input MHA file.")
    parser.add_argument("--output", type=str, required=True, help="Path to output MHA file.")
    args = parser.parse_args()
    process_ct_mha(args.input, args.output)

def process_ct_mha(input_path: str, output_path: str):
    original_image = sitk.ReadImage(input_path)
    original_array = sitk.GetArrayFromImage(original_image)
    ct0 = original_array.copy()

    sample_slice = ct0[ct0.shape[0] // 2, :, :]  # 取中间层
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
    else:
        cx, cy = (w// 2 )+20, (h// 2 )
        r_inner = min(w, h) // 2

    # 创建统一的水区域 mask
    yy, xx = np.ogrid[:h, :w]
    dist_from_center = np.sqrt((xx - cx)**2 + (yy - cy)**2)
    water_outer_radius = r_inner - 100
    reliable_water_mask = (dist_from_center <= water_outer_radius)

    # ============================================================
    # 2. 所有层套用同一个 mask
    # ============================================================
    orig_h, orig_w = sample_slice.shape
    mask_bool = cv2.resize(reliable_water_mask.astype(np.uint8), (orig_w, orig_h)).astype(bool)

    for z in range(ct0.shape[0]):
        ct0[z, :, :][~mask_bool] = 0

    # ============================================================
    # 3. 保存为 MHD
    # ============================================================
    output = sitk.GetImageFromArray(ct0)
    output.SetSpacing(original_image.GetSpacing())
    output.SetOrigin(original_image.GetOrigin())
    output.SetDirection(original_image.GetDirection())
    sitk.WriteImage(output, output_path)

if __name__ == "__main__":
    main()