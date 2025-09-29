use anyhow::Result;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[cfg(not(target_arch = "wasm32"))]
use tokio::fs::{self, File};
#[cfg(not(target_arch = "wasm32"))]
use tokio::io::AsyncReadExt;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::Mutex;

#[cfg(target_arch = "wasm32")]
use crate::ct_volume::CTVolume;

use super::*;

/// Parses DICOM files from a list of directories and constructs a `DicomRepo`.
///
/// This function traverses each provided directory asynchronously, collects file paths,
/// and delegates the parsing of files to `parse_dcm_files`. It ensures that only files
/// (not directories) are added to the list of file paths for processing.
///
/// # Arguments
/// - `directories`: A vector of directory paths containing DICOM files to process.
///
/// # Returns
/// A `Result` containing the constructed `DicomRepo` on success, or an error if any
/// directory cannot be read.
///
/// # Errors
/// - If a directory cannot be opened or traversed, the function logs the error and returns it.
/// - Non-existent directories or permission issues may result in an error.
///
/// # Example
/// ```rust
/// let directories = vec!["/path/to/dir1", "/path/to/dir2"];
/// let repo = parse_dcm_directories(directories).await?;
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub async fn parse_dcm_directories(directories: Vec<&str>) -> Result<DicomRepo> {
    let mut file_paths = Vec::new();

    // Collect all files from the provided directories
    for dir_path in directories {
        let mut entries = fs::read_dir(dir_path).await.map_err(|err| {
            eprintln!("Error reading directory {}: {}", dir_path, err);
            anyhow::Error::new(err)
        })?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                file_paths.push(path);
            }
        }
    }
    parse_dcm_files(file_paths).await
}

/// Parses a list of DICOM files concurrently and constructs a `DicomRepo`.
///
/// This function reads the contents of each file asynchronously, parses DICOM data,
/// and updates a shared `DicomRepo`. File processing is performed concurrently using
/// Tokio's task spawning, and the shared repository is updated in a thread-safe manner.
///
/// # Arguments
/// - `file_paths`: A vector of file paths to DICOM files to process.
///
/// # Returns
/// A `Result` containing the constructed `DicomRepo` on success, or an error if any
/// critical operation (e.g., file I/O) fails.
///
/// # Errors
/// - If a file cannot be opened, read, or parsed, an error will be logged, and the function
///   will continue processing the remaining files. Non-fatal errors will not stop the process.
///
/// # Concurrency
/// - Parsing of DICOM data is done outside of the critical section to minimize the time spent
///   holding the `Mutex` lock on the shared repository.
/// - Each file is processed in a separate Tokio task, enabling high concurrency.
///
/// # Example
/// ```rust
/// let files = vec![PathBuf::from("file1.dcm"), PathBuf::from("file2.dcm")];
/// let repo = parse_dcm_files(files).await?;
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub async fn parse_dcm_files(file_paths: Vec<std::path::PathBuf>) -> Result<DicomRepo> {
    // Shared repository and counter tracker
    let repo = Arc::new(Mutex::new(DicomRepo::new()));
    let count = Arc::new(AtomicUsize::new(0));

    // Process files concurrently
    let mut tasks = vec![];
    for file_path in file_paths {
        let repo_clone = Arc::clone(&repo);
        let count_clone = Arc::clone(&count);

        let task: tokio::task::JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {
            // Open the file asynchronously
            let mut file = File::open(&file_path).await.map_err(|err| {
                eprintln!("Error opening file {}: {}", file_path.display(), err);
                anyhow::Error::new(err)
            })?;

            // Read the file contents into a buffer
            let mut buffer = vec![];
            file.read_to_end(&mut buffer).await.map_err(|err| {
                eprintln!("Error reading file {}: {}", file_path.display(), err);
                anyhow::Error::new(err)
            })?;

            // Parse the DICOM data outside of the lock
            let parsed_patient = Patient::from_bytes(&buffer);
            let parsed_study = StudySet::from_bytes(&buffer);
            let parsed_series = ImageSeries::from_bytes(&buffer);
            let parsed_ct_image = CTImage::from_bytes(&buffer);

            // Update the repository with parsed data
            let mut repo = repo_clone.lock().await;

            if let Ok(patient) = parsed_patient {
                repo.add_patient(patient);
            }
            if let Ok(study) = parsed_study {
                repo.add_study(study);
            }
            if let Ok(series) = parsed_series {
                repo.add_image_series(series);
            }
            if let Ok(ct_image) = parsed_ct_image {
                repo.add_ct_image(ct_image);
            }

            count_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    for task in tasks {
        match task.await {
            Ok(Ok(())) => { /* Task succeeded */ }
            Ok(Err(err)) => eprintln!("Task error: {}", err),
            Err(join_err) => eprintln!("Task panicked or was cancelled: {:?}", join_err),
        }
    }

    // Extract the final repository and return it
    let repo = repo.lock().await;
    Ok((*repo).clone())
}

//------------------------------ WASM Code -------------------------------------

#[cfg(target_arch = "wasm32")]
use js_sys::{Array, Promise, Uint8Array};
use web_sys::{File, FileReader, ProgressEvent};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use image::{GrayImage, Luma};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg(target_arch = "wasm32")]
pub async fn parse_dcm_files_wasm(files: Array) -> Result<DicomRepo, JsValue> {
    // use futures::channel::oneshot;
    // use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    // use wasm_bindgen_futures::future_to_promise;

    // Shared repository and counter
    let repo = Arc::new(Mutex::new(DicomRepo::new()));
    let len = files.length() as usize;

    // Processing each file asynchronously
    let tasks: Vec<Promise> = (0..len)
        .map(|idx| {
            let file: File = files.get(idx as u32).dyn_into().unwrap();
            let file_reader = FileReader::new().unwrap();
            // let repo_clone = Arc::clone(&repo);

            // Create a promise for each file
            let promise = Promise::new(&mut |resolve, reject| {
                let repo_clone = Arc::clone(&repo);
                // The closure now correctly accepts the `ProgressEvent`
                let closure = Closure::once_into_js(move |event: ProgressEvent| {
                    let result: Result<(), String> = {
                        let buffer = event
                            .target()
                            .ok_or_else(|| JsValue::from("Failed to retrieve target"))?
                            .dyn_into::<FileReader>()?
                            .result()?;
                            // .map_err(|| JsValue::from("Failed to retrieve file result"))?;

                        let buffer = Uint8Array::new(&buffer).to_vec();

                        // Parse the DICOM and update repository
                        let mut repo = repo_clone.lock().unwrap();
                        if let Ok(patient) = Patient::from_bytes(&buffer) {
                            repo.add_patient(patient);
                        }
                        if let Ok(study) = StudySet::from_bytes(&buffer) {
                            repo.add_study(study);
                        }
                        if let Ok(series) = ImageSeries::from_bytes(&buffer) {
                            repo.add_image_series(series);
                        }
                        if let Ok(ct_image) = CTImage::from_bytes(&buffer) {
                            repo.add_ct_image(ct_image);
                        }

                        Ok(())
                    };

                    // Resolve or reject the promise based on the result
                    match result {
                        Ok(_) => resolve.call0(&JsValue::NULL),
                        Err(err) => reject.call0(&JsValue::from(err)),
                    }
                });

                file_reader.set_onload(Some(closure.as_ref().unchecked_ref()));
                file_reader.read_as_array_buffer(&file).expect("Failed to read file");
            });

            promise
        })
        .collect();

    // Wait for all tasks to complete (using Promise.all)
    let all_promise = js_sys::Promise::all(&tasks.into_iter().collect::<Array>());
    wasm_bindgen_futures::JsFuture::from(all_promise).await?;

    // Return the DicomRepo directly as a JsValue
    let repo = repo.lock().map_err(|e| JsValue::from(e.to_string()))?;
    Ok(repo.clone())
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg(target_arch = "wasm32")]
pub async fn parse_mha_files_wasm(files: Array,info: js_sys::Uint8Array,) -> Result<CTVolume, JsValue> {
    use std::sync::{Arc, Mutex};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use js_sys::{Promise, Uint8Array};
    use web_sys::{File, FileReader, ProgressEvent};

    // info JSON
    let mut buf = vec![0u8; info.length() as usize];
    info.copy_to(&mut buf[..]);
    let info_json = String::from_utf8(buf)
        .map_err(|e| JsValue::from_str(&format!("UTF8 decode error: {}", e)))?;
    let info: serde_json::Value = serde_json::from_str(&info_json)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
    let kv = info["kv"].as_f64().ok_or(JsValue::from_str("Missing kv in info"))?;
    let m_as = info["mAs"].as_f64().ok_or(JsValue::from_str("Missing mAs in info"))?;
    let slope = info["slope"].as_f64()
        .map(|v| v as f32)
        .ok_or(JsValue::from_str("Missing slope in info"))?;
    let intercept = info["intercept"].as_f64()
        .map(|v| v as f32)
        .ok_or(JsValue::from_str("Missing intercept in info"))?;
    log::info!("mha info: kv={}, mAs={}, slope = {:?}, intercept = {:?}", kv, m_as, slope, intercept);

    // Processing each file asynchronously
    let len = files.length() as usize;
    let results = Arc::new(Mutex::new(Vec::<Result<CTVolume, String>>::new()));
    let mut tasks = Vec::new();

    for idx in 0..len {
        let file: File = files.get(idx as u32).dyn_into()?;
        let file_name = file.name();
        
        // only process .mha files
        if !file_name.ends_with(".mha") {
            continue;
        }
        
        // Clone the results Arc for use in the async task
        let results_clone = Arc::clone(&results);
        let task = async move {
            let file_reader = FileReader::new().unwrap();
            
            // Create a promise for each file
            let promise = Promise::new(&mut |resolve, reject| {
                let resolve_clone = resolve.clone();
                let reject_clone = reject.clone();
                let results_inner = Arc::clone(&results_clone);
                
                let onload = Closure::<dyn FnMut(ProgressEvent)>::new(move |event: ProgressEvent| {
                    let result = || -> Result<(), JsValue> {
                        // Read file data
                        let target = event.target().ok_or("No target")?;
                        let file_reader = target.dyn_into::<FileReader>()?;
                        let array_buffer = file_reader.result()?;
                        let uint8_array = Uint8Array::new(&array_buffer);
                        let buffer = uint8_array.to_vec();
                        
                        // Parse MHA and generate CTVolume
                        let mha = MHXVolume::from_bytes(&buffer)
                            .map_err(|e| JsValue::from_str(&e.to_string()))?;
                        
                        let ct_volume = mha.generate_ct_volume_mha(slope, intercept)
                            .map_err(|e| JsValue::from_str(&e.to_string()))?;
                        
                        // store the result
                        let mut results_guard = results_inner.lock().unwrap();
                        results_guard.push(Ok(ct_volume));
                        
                        Ok(())
                    }();
                    
                    // Resolve or reject the promise based on the result
                    match result {
                        Ok(_) => {
                            let _ = resolve_clone.call0(&JsValue::NULL);
                        },
                        Err(err) => {
                            let _ = reject_clone.call1(&JsValue::NULL, &err);
                        },
                    }
                });
                file_reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();
                
                // error
                if let Err(e) = file_reader.read_as_array_buffer(&file) {
                    let _ = reject.call1(&JsValue::NULL, &JsValue::from(e));
                }
            });
            JsFuture::from(promise).await
        };
        
        tasks.push(task);
    }
    
    //if no MHA files found, return error
    if tasks.is_empty() {
        return Err(JsValue::from_str("No MHA files found"));
    }
    
    // wait for all tasks to complete
    for task in tasks {
        task.await?;
    }
    
    // return the first successful result 
    let results_guard = results.lock().unwrap();
    if let Some(Ok(volume)) = results_guard.first() {
        Ok(volume.clone())
    } else if let Some(Err(err)) = results_guard.first() {
        Err(JsValue::from_str(&format!("Failed to parse MHA file: {}", err)))
    } else {
        Err(JsValue::from_str("No valid results found"))
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg(target_arch = "wasm32")]
pub async fn build_ct_dicom_wasm(
    mha_bytes: js_sys::Uint8Array,
    patient: js_sys::Uint8Array,
    study: js_sys::Uint8Array,
    info: js_sys::Uint8Array,
) -> Result<JsValue,JsValue> {
    //patient JSON
    let mut p_buf = vec![0u8; patient.length() as usize];
    patient.copy_to(&mut p_buf[..]);
    let patient_json = String::from_utf8(p_buf)
        .map_err(|e| JsValue::from_str(&format!("UTF8 decode error: {}", e)))?;
    let patient: serde_json::Value = serde_json::from_str(&patient_json)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
    let patient = Patient {
        patient_id: patient["patient_id"].as_str().unwrap_or("").to_string(),
        name: patient["name"].as_str().unwrap_or("").to_string(),
        birthdate: patient["birthdate"].as_str().map(|s| s.to_string()),
        sex: patient["sex"].as_str().map(|s|s.to_string())
    };

    //study JSON
    let mut s_buf = vec![0u8; study.length() as usize];
    study.copy_to(&mut s_buf[..]);
    let study_json = String::from_utf8(s_buf)
        .map_err(|e| JsValue::from_str(&format!("UTF8 decode error: {}", e)))?;
    let study: serde_json::Value = serde_json::from_str(&study_json)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
    let uid = generate_uid();
    let study = StudySet {
        uid: uid.clone(),
        study_id: study["study_id"].as_str().unwrap_or("").to_string(),
        patient_id: study["patient_id"].as_str().unwrap_or("").to_string(),
        date: study["date"].as_str().unwrap_or("").to_string(),
        description: study["description"].as_str().map(|s| s.to_string()),
    };

    // info JSON
    let mut buf = vec![0u8; info.length() as usize];
    info.copy_to(&mut buf[..]);
    let info_json = String::from_utf8(buf)
        .map_err(|e| JsValue::from_str(&format!("UTF8 decode error: {}", e)))?;
    let info: serde_json::Value = serde_json::from_str(&info_json)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
    let kv = info["kv"].as_f64().ok_or(JsValue::from_str("Missing kv in info"))?;
    let m_as = info["mAs"].as_f64().ok_or(JsValue::from_str("Missing mAs in info"))?;
        let slope = info["slope"].as_f64()
        .map(|v| v as f32)
        .ok_or(JsValue::from_str("Missing slope in info"))?;
    let intercept = info["intercept"].as_f64()
        .map(|v| v as f32)
        .ok_or(JsValue::from_str("Missing intercept in info"))?;
    log::info!("dicom info: kv={}, mAs={}, slope = {:?}, intercept = {:?}", kv, m_as, slope, intercept);

    // MHA bytes
    let mut buf = vec![0u8; mha_bytes.length() as usize];
    mha_bytes.copy_to(&mut buf[..]);

    // Build DICOM files in memory
    let mut sink = MemSink::new();
    build_ct_dicom(
        &buf,
        &patient,
        &study,
        kv, m_as,
        slope, intercept,
        &mut sink
    ).map_err(|e| JsValue::from_str(&format!("build_ct_dicom failed: {}", e)))?;

    // Convert the files in MemSink to a JavaScript array of objects
    let js_array = js_sys::Array::new();
    for (filename, data) in sink.files {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"filename".into(), &JsValue::from_str(&filename))?;
        js_sys::Reflect::set(&obj, &"data".into(), &js_sys::Uint8Array::from(data.as_slice()))?;
        js_array.push(&obj);
    }

    Ok(js_array.into())
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg(target_arch = "wasm32")]
pub async fn export_slice_png(
    mha_bytes: js_sys::Uint8Array, // MHA 文件内容
    slice_index: usize,            // 切片索引
    orientation: u32,              // 0=axial, 1=coronal, 2=sagittal
    ww: f32,                       // window width
    wc: f32,                       // window center
    slope: f32,
    intercept: f32,
) -> Result<Vec<u8>, JsValue> {
    // 1. 读取 mha
    let mut buf = vec![0u8; mha_bytes.length() as usize];
    mha_bytes.copy_to(&mut buf[..]);
    let mha = MHXVolume::from_bytes(&buf).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let ct_volume = mha.generate_ct_volume_mha(slope, intercept) .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let width = ct_volume.dimensions.0;
    let height = ct_volume.dimensions.1;
    let depth = ct_volume.dimensions.2;
    log::info!("w:{:?},h:{:?},d:{:?}",width,height,depth);
    let data = &ct_volume.voxel_data; // i16 数组 (体素数据)

    // 2. 抽取切片
    let slice: Vec<i16> = match orientation {
        0 => {
            // axial: 固定 z
            if slice_index >= depth {
                log::info!("slice_index:{:?},d:{:?}",slice_index,depth);
                return Err(JsValue::from_str("axial index out of range"));
            }
            let mut v = Vec::with_capacity(width * height);
            for y in 0..height {
                for x in 0..width {
                    v.push(data[slice_index * width * height + y * width + x]);
                }
            }
            v
        }
        1 => {
            // coronal: 固定 y
            if slice_index >= height {
                return Err(JsValue::from_str("coronal index out of range"));
            }
            let mut v = Vec::with_capacity(width * depth);
            for z in 0..depth {
                for x in 0..width {
                    v.push(data[z * width * height + slice_index * width + x]);
                }
            }
            v
        }
        2 => {
            // sagittal: 固定 x
            if slice_index >= width {
                return Err(JsValue::from_str("sagittal index out of range"));
            }
            let mut v = Vec::with_capacity(height * depth);
            for z in 0..depth {
                for y in 0..height {
                    v.push(data[z * width * height + y * width + slice_index]);
                }
            }
            v
        }
        _ => return Err(JsValue::from_str("invalid orientation")),
    };

    // 3. 应用窗宽窗位
    let ww = ww.max(1.0);
    let wc = wc;
    let min_hu = wc - ww / 2.0;
    let max_hu = wc + ww / 2.0;

    let mut buf_u8 = Vec::with_capacity(slice.len());
    for &val in slice.iter() {
        let hu = val as f32;
        let norm = if hu <= min_hu {
            0
        } else if hu >= max_hu {
            255
        } else {
            (((hu - min_hu) / ww) * 255.0).round() as u8
        };
        buf_u8.push(norm);
    }

    // 4. 构造 GrayImage
    let (w, h) = match orientation {
        0 => (width as u32, height as u32),
        1 => (width as u32, depth as u32),
        2 => (height as u32, depth as u32),
        _ => unreachable!(),
    };

    let img = image::GrayImage::from_raw(w, h, buf_u8)
        .ok_or_else(|| JsValue::from_str("构造 GrayImage 失败"))?;

    // 5. 编码 PNG 到内存
    let mut png_buf: Vec<u8> = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_buf, w, h);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(img.as_raw()).unwrap();
    }

    Ok(png_buf)
}
