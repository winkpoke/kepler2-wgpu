use super::{
    patient::Patient,
    studyset::StudySet
};

use crate::data::medical_imaging::{
    data::DicomSink,
    format::mha::*
};

use anyhow::{anyhow, Result};
use dicom_core::{value::PrimitiveValue, DataElement, VR};
use dicom_dictionary_std::tags;
use dicom_object::{DefaultDicomObject, FileMetaTableBuilder, InMemDicomObject};
use std::path::PathBuf;
use uuid::Uuid;
use chrono::Local;
use sha1::{Sha1, Digest};

/// generate dicom uid
pub fn generate_uid() -> String {
    let uuid = Uuid::new_v4();
    let mut hasher = Sha1::new();
    hasher.update(uuid.as_bytes());
    let hash = hasher.finalize();
    let hash_num = u128::from_be_bytes([
        hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7],
        hash[8], hash[9], hash[10], hash[11], hash[12], hash[13], hash[14], hash[15],
    ]);
    format!(
        "{}.{:03}.{:06}.{:04}.{:01}.{:01}.{:01}.{:03}.{:10}.{:10}.{:06}",
        "1.2",
        hash_num >> 112 & 0xFFF,
        hash_num >> 96 & 0xFF_FF_FF,
        hash_num >> 80 & 0xFFFF,
        hash_num >> 64 & 0xF,
        hash_num >> 48 & 0xF, 
        hash_num >> 32 & 0xF,
        hash_num >> 16 & 0xFFF,
        hash_num & 0xFF_FF_FF_FF_FF,
        (hash_num >> 8) & 0xFF_FF_FF_FF_FF,
        hash_num & 0xFF_FF_FF
    )
}

/// Change DICOM UID based on a root UID
fn change_dicom_uid(root: &str,twice:bool) -> String {
    // Generate a new UUID and split the root into parts
    let uuid = Uuid::new_v4();
    let uuid_num = uuid.as_u128();
    let parts_root: Vec<&str> = root.split('.').collect();

    let ten_digit_num = (uuid_num / 1_000_000) % 10_000_000_000;
    let six_digit_num = uuid_num % 1_000_000;

    // Combine all parts into final DICOM UID
    if twice {
        format!(
            "{}.{}.{}",
            &parts_root[0..10].join("."),
            ten_digit_num,
            six_digit_num
        )
    }else {
        format!(
            "{}.{}",
            &parts_root[0..11].join("."),
            six_digit_num
        )
    }
}

/// generate ct dicom
pub fn build_ct_dicom<S: DicomSink>(
    mha_path: &[u8],
    patient: &Patient,
    study: &StudySet,
    kv: f64,
    m_as: f64,
    slope: f32,
    intercept: f32,
    sink: &mut S,
) -> Result<()> {
    // generate series uid
    let series_uid= change_dicom_uid(&study.uid,true);
    let sop_class_uid = "1.2.840.10008.5.1.4.1.1.2"; // CT Image Storage
    let sop_instance_uid = change_dicom_uid(&series_uid, false);

    // generate meta
    let mut meta = FileMetaTableBuilder::new()
        .media_storage_sop_class_uid(sop_class_uid)
        .media_storage_sop_instance_uid(sop_instance_uid.clone())
        .transfer_syntax("1.2.840.10008.1.2.1") // Explicit VR Little Endian
        .implementation_class_uid("2.25.999")
        .implementation_version_name("DIRAC-CT")
    ;

    // generate dicom object
    let mut obj = InMemDicomObject::new_empty().with_meta(meta.clone())?;

    // generate basic info
    obj.put(DataElement::new(tags::SPECIFIC_CHARACTER_SET, VR::CS, PrimitiveValue::from("ISO_IR 192")));
    obj.put(DataElement::new(tags::SOP_CLASS_UID, VR::UI, PrimitiveValue::from(sop_class_uid)));
    obj.put(DataElement::new(tags::MODALITY, VR::CS, PrimitiveValue::from("CT")));
    obj.put(DataElement::new(tags::PHOTOMETRIC_INTERPRETATION, VR::CS, PrimitiveValue::from("MONOCHROME2")));
    obj.put(DataElement::new(tags::KVP, VR::DS, PrimitiveValue::from(kv))); // kVp
    obj.put(DataElement::new(tags::	X_RAY_TUBE_CURRENT, VR::IS, PrimitiveValue::from(m_as))); // mA
    obj.put(DataElement::new(tags::IMAGE_TYPE, VR::CS, PrimitiveValue::from("ORIGINAL\\PRIMARY\\AXIAL")));

    // generate patient info
    obj.put(DataElement::new(tags::PATIENT_NAME, VR::PN, PrimitiveValue::from(patient.name.clone())));
    obj.put(DataElement::new(tags::PATIENT_ID, VR::LO, PrimitiveValue::from(patient.patient_id.clone())));
    if let Some(birthdate) = &patient.birthdate {
        obj.put(DataElement::new(tags::PATIENT_BIRTH_DATE, VR::DA, PrimitiveValue::from(birthdate.clone())));
    }
    if let Some(sex) = &patient.sex {
        obj.put(DataElement::new(tags::PATIENT_SEX, VR::CS, PrimitiveValue::from(sex.clone())));
    }

    // generate study info
    obj.put(DataElement::new(tags::STUDY_INSTANCE_UID, VR::UI, PrimitiveValue::from(study.uid.clone())));
    obj.put(DataElement::new(tags::STUDY_ID, VR::SH, PrimitiveValue::from(study.study_id.clone())));
    obj.put(DataElement::new(tags::STUDY_DATE, VR::DA, PrimitiveValue::from(study.date.clone())));
    if let Some(description) = &study.description {
        obj.put(DataElement::new(tags::STUDY_DESCRIPTION, VR::LO, PrimitiveValue::from(description.clone())));
    }

    // generate series info
    obj.put(DataElement::new(tags::SERIES_INSTANCE_UID, VR::UI, PrimitiveValue::from(series_uid.clone())));
    let now = Local::now();
    obj.put(DataElement::new(tags::SERIES_DATE, VR::DA, PrimitiveValue::from(now.format("%Y%m%d").to_string())));
    obj.put(DataElement::new(tags::SERIES_TIME, VR::TM, PrimitiveValue::from(now.format("%H%M%S").to_string())));

    // generate image info
    inject_image(&mut obj, &mut meta, series_uid, mha_path, slope, intercept, sink)?;
    
    Ok(())
}

// generate image info
fn inject_image<S: DicomSink>(
    obj: &mut DefaultDicomObject,
    meta: &mut FileMetaTableBuilder,
    series_uid: String,
    mha_path: &[u8], 
    slope: f32,
    intercept: f32,
    sink: &mut S,
) -> Result<()> {
    let ct_volume = mha::MhaParser::from_bytes(mha_path).unwrap();
    let  patient_position = ct_volume.base.label.clone();

    let col = ct_volume.dimensions.0; 
    let row = ct_volume.dimensions.1; 
    let depth = ct_volume.dimensions.2; 
    let spacing: &[f32] = &[
        ct_volume.voxel_spacing.0, 
        ct_volume.voxel_spacing.1, 
        ct_volume.voxel_spacing.2
    ];

    // little endian
    let vol = ct_volume.voxel_data;
    let mut buffer = Vec::with_capacity(vol.len() * 2);
    let buffer = match header.element_type.as_str() {
        "MET_SHORT" | "MET_INT16" => {
            for val in vol{
                buffer.extend_from_slice(&val.to_le_bytes());
            }
            buffer
        }
        "MET_FLOAT" => {
            let mut rotated_i16 = vec![0i16; row * col * depth];
            for new_x in 0..depth {
                for new_y in 0..row {
                    for new_z in 0..col{
                        // 绕y轴顺时针旋转90度的坐标映射
                        let old_x = new_z; 
                        let old_y = new_y; 
                        let old_z = new_x;
                        
                        let old_idx = old_z * (row * col) + old_y * col + old_x;
                        let new_idx = new_z * (row * depth) + new_y * depth + new_x;
                        rotated_i16[new_idx] = vol[old_idx];
                    }
                }
            }
            for val in rotated_i16 {
                buffer.extend_from_slice(&val.to_le_bytes());
            }
            buffer
        }
        _ => return Err(anyhow!("不支持的 mha element type: {}", header.element_type)),
    };
    
    // generate dicom slice
    for z in 0..depth {
        let start = z * col * row * (16 as usize / 8);
        let end = start + col * row * (16 as usize / 8);
        let buf = &buffer[start..end];
        let buf_vec: Vec<u8> = buf.to_vec();

        // generate sop instance uid
        let sop_instance_uid = change_dicom_uid(&series_uid,false);

        // calculate slice location and direction
        let dz = *spacing.get(2).unwrap_or(&1.0);
        let instance_no = (z + 1) as i32;
        let slice_loc = (z as f32) * dz;
        let base = ct_volume.base.matrix.get_column(3);
        let (col_dir,row_dir, pos) = match header.element_type.as_str() {
            "MET_SHORT" | "MET_INT16" => {
                let transform: &[f32] = &header.transform;
                let (col_dir,row_dir, slice_dir) = orientation_dirs(&transform);
                let pos = [
                    base[0] + slice_loc * slice_dir[0],
                    base[1] + slice_loc * slice_dir[1],
                    base[2] + slice_loc * slice_dir[2],
                ];
                (col_dir, row_dir, pos)
            }
            "MET_FLOAT" => {
                let col_dir=[1.0, 0.0, 0.0];
                let row_dir=[0.0, 1.0, 0.0];
                let pos = [
                    base[0] ,
                    base[1] ,
                    base[2] + slice_loc,
                ];
                (col_dir, row_dir, pos)
            }
            _ => return Err(anyhow!("不支持的 mha element type: {}", header.element_type)),
        };

        if z == 5 {
            log::info!("➡️ CT 切片维度：col={}, row={}, depth={}", col, row, depth);
            log::info!("➡️ CT 切片体素间距：dx={:.3}, dy={:.3}, dz={:.3}", spacing[0], spacing[1], spacing[2]);
            log::info!("➡️ CT 切片位置：base={:?}, col_dir={:?}, row_dir={:?}", base, col_dir, row_dir);
            log::info!("➡️ CT 切片位置起点：pos={:?}", pos);
        }
        if z == 5 {
            println!("➡️ CT 切片维度：col={}, row={}, depth={}", col, row, depth);
            println!("➡️ CT 切片体素间距：dx={:.3}, dy={:.3}, dz={:.3}", spacing[0], spacing[1], spacing[2]);
            println!("➡️ CT 切片位置：base={:?}, col_dir={:?}, row_dir={:?}", base, col_dir, row_dir);
            println!("➡️ CT 切片位置起点：pos={:?}", pos);
        }

        // save DICOM slice
        let filename = format!("CT_{:04}.dcm", z + 1);
        write_ct_dicom_slice(
            obj.clone(),
            &mut meta.clone(),
            sop_instance_uid,
            patient_position.clone(),
            row, 
            col,
            spacing,
            row_dir, 
            col_dir,
            pos,
            instance_no,
            slice_loc,
            buf_vec,
            filename,
            sink,
        )?;
    }

    Ok(())
}

/// write CT dicom slice
fn write_ct_dicom_slice<S: DicomSink>(
    mut base: DefaultDicomObject,
    meta: &mut FileMetaTableBuilder,
    sop_instance_uid: String,
    patient_position: String,
    rows: usize,
    cols: usize,
    spacing: &[f32],    // [dx, dy, dz]
    row_dir: [f32; 3],
    col_dir: [f32; 3],
    pos_xyz: [f32; 3],
    instance_no: i32,
    slice_loc: f32,
    buf: Vec<u8>,
    filename: String,
    sink: &mut S,
) -> Result<()> {
    let slope = 1.0;
    let intercept=0.0;
    let win_center = (0.0 + 1.0) / 2.0;
    let win_width = f32::max(1.0 - 0.0, 1.0);
 
    // 更新文件元信息中的 MEDIA_STORAGE_SOP_INSTANCE_UID
    base.put(DataElement::new(tags::SOP_INSTANCE_UID, VR::UI, PrimitiveValue::from(sop_instance_uid.clone())));
    let new_meta = FileMetaTableBuilder::from(meta.clone())
        .media_storage_sop_instance_uid(sop_instance_uid.clone())
        .build()
        .unwrap();

    *base.meta_mut() = new_meta;
    
    // 像素结构（16 位有符号）
    base.put(DataElement::new(tags::ROWS, VR::US, PrimitiveValue::from(rows as u16)));
    base.put(DataElement::new(tags::COLUMNS, VR::US, PrimitiveValue::from(cols as u16)));
    base.put(DataElement::new(tags::BITS_ALLOCATED, VR::US, PrimitiveValue::from(16u16)));
    base.put(DataElement::new(tags::BITS_STORED, VR::US, PrimitiveValue::from(16u16)));
    base.put(DataElement::new(tags::HIGH_BIT, VR::US, PrimitiveValue::from(15u16)));
    base.put(DataElement::new(tags::SAMPLES_PER_PIXEL, VR::US, PrimitiveValue::from(1u16)));
    base.put(DataElement::new(tags::PIXEL_REPRESENTATION, VR::US, PrimitiveValue::from(1u16))); // signed

    // 像素间距（DICOM: Row\Col = dy\dx）
    let dx = spacing.get(0).copied().unwrap_or(1.0);
    let dy = spacing.get(1).copied().unwrap_or(1.0);
    let dz = spacing.get(2).copied().unwrap_or(1.0);

    base.put(DataElement::new(tags::PIXEL_SPACING, VR::DS, PrimitiveValue::from(format!("{:.3}\\{:.3}", dx, dy))));
    base.put(DataElement::new(tags::SLICE_THICKNESS, VR::DS, PrimitiveValue::from(format!("{:.3}", dz))));

    // orientation/position
    base.put(DataElement::new(tags::PATIENT_POSITION, VR::CS, PrimitiveValue::from(patient_position.clone())));
    base.put(DataElement::new(
        tags::IMAGE_ORIENTATION_PATIENT,
        VR::DS,
        PrimitiveValue::from(format!(
            "{:.3}\\{:.3}\\{:.3}\\{:.3}\\{:.3}\\{:.3}",
            col_dir[0], col_dir[1], col_dir[2],row_dir[0], row_dir[1], row_dir[2]
        )),
    ));
    base.put(DataElement::new(
        tags::IMAGE_POSITION_PATIENT,
        VR::DS,
        PrimitiveValue::from(format!("{:.3}\\{:.3}\\{:.3}", pos_xyz[0], pos_xyz[1], pos_xyz[2])),
    ));
    base.put(DataElement::new(tags::INSTANCE_NUMBER, VR::IS, PrimitiveValue::from(instance_no)));
    base.put(DataElement::new(tags::SLICE_LOCATION, VR::DS, PrimitiveValue::from(format!("{:.3}", slice_loc))));

    // Slope/Intercept
    base.put(DataElement::new(tags::RESCALE_SLOPE, VR::DS, PrimitiveValue::from(format!("{:.3}", slope))));
    base.put(DataElement::new(tags::RESCALE_INTERCEPT, VR::DS, PrimitiveValue::from(format!("{:.3}", intercept))));
    base.put(DataElement::new(tags::RESCALE_TYPE, VR::LO, PrimitiveValue::from("HU")));

    // ww/wc
    let ww = if win_width <= 0.0 { 1.0 } else { win_width };
    base.put(DataElement::new(tags::WINDOW_CENTER, VR::DS, PrimitiveValue::from(format!("{:.3}", win_center))));
    base.put(DataElement::new(tags::WINDOW_WIDTH, VR::DS, PrimitiveValue::from(format!("{:.3}", ww))));

    // Pixel Data
    base.put(DataElement::new(tags::PIXEL_DATA, VR::OW, PrimitiveValue::from(buf)));

    // save to sink
    let mut out_buf = Vec::new();
    base.write_all(&mut out_buf).map_err(|e| anyhow!("failed to serialize dicom to memory: {}", e))?;
    sink.save_slice(filename, out_buf)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_uuid() {
        let uid1 = generate_uid();
        let uid2 = change_dicom_uid(&uid1, true);
        let uid3 = change_dicom_uid(&uid1, false);
        assert_ne!(uid1, uid2);
        assert_ne!(uid2, uid3);
        println!("Generated UID: {}", uid1);
        println!("Changed UID: {}", uid2);
        println!("Final UID: {}", uid3);
    }

    #[test]
    fn test_build_ct_dicom() {
        // 测试路径
        let path = "C:/share/input/CT_new.mha";
        let data = fs::read(path);
        let mha_path = data.as_ref().map(|v| v.as_slice()).unwrap();
        let out_dir = Path::new("C:/share").join(Local::now().format("%Y-%m-%d").to_string());
        std::fs::create_dir_all(&out_dir).unwrap();

        // 生成一个 SOPInstanceUID
        let study_uid = generate_uid();
        println!("Generated Study UID: {}", study_uid);

        // 构造虚拟的 patient / study
        let patient = Patient {
            patient_id: "P001".to_string(),
            name: "zhangsan".to_string(),
            birthdate: Some("19800101".to_string()),
            sex: Some("M".to_string())
        };

        let study = StudySet {
            uid: study_uid.to_string(),
            study_id: "STUDY001".to_string(),
            patient_id: "P001".to_string(),
            date: "20250908".to_string(),
            description: Some("zhutiCT".to_string())
        };

        // 调用函数
        let mut sink = FsSink { out_dir };
        let result = build_ct_dicom(
            mha_path, 
            &patient, 
            &study,
            120.0,   // kV
            70.0,    // mAs
            1612.903, // slope
            -1016.129,   // intercept
            &mut sink);
        assert!(result.is_ok());

        println!("✅ build_ct_dicom 执行成功！");
    }
}
