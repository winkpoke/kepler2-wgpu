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

use super::*;
use crate::data::ct_volume::CTVolume;
use crate::data::medical_imaging::formats::*;
use crate::data::medical_imaging::metadata::{volume::MedicalVolume, PixelData, ImageMetadata};

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
/// ```rust,ignore
/// // Requires async runtime and local filesystem with DICOM files.
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
/// ```rust,ignore
/// // Requires async runtime and real DICOM files on filesystem.
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
#[cfg(target_arch = "wasm32")]
use web_sys::{File, FileReader, ProgressEvent};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;


#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg(target_arch = "wasm32")]
pub async fn parse_dcm_files_wasm(files: Array) -> Result<DicomRepo, JsValue> {
    // use futures::channel::oneshot;
    // use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    // use wasm_bindgen_futures::future_to_promise;
    // use log::error;

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
pub async fn process_single_mha_file(file: File,slope: f32, intercept: f32) -> Result<CTVolume, JsValue> {
    use js_sys::{Promise, Uint8Array};
    use wasm_bindgen::closure::Closure;

    let file_reader = FileReader::new().map_err(|_| JsValue::from_str("Failed to create FileReader"))?;

    // Create a promise for each file
    let promise = Promise::new(&mut |resolve, reject| {
        let reject_clone = reject.clone();
        
        let onload = Closure::<dyn FnMut(ProgressEvent)>::new(move |event: ProgressEvent| {
            let result = || -> Result<CTVolume, JsValue> {
                // Read file data
                let target = event.target().ok_or("No target")?;
                let file_reader = target.dyn_into::<FileReader>()?;
                let array_buffer = file_reader.result()?;
                let uint8_array = Uint8Array::new(&array_buffer);
                let buffer = uint8_array.to_vec();
                
                // Parse MHA and generate CTVolume
                let medical_volume = MhaParser::parse_bytes(&buffer)
                    .map_err(|e| JsValue::from_str(&e.to_string()))?;
                
                // Convert MedicalVolume to CTVolume using the existing method
                let ct_volume = match medical_volume.pixel_data {
                    PixelData::UInt8(data) => {
                        // Extract dimensions from metadata
                        let dimensions = medical_volume.metadata.dimensions;
                        let spacing = medical_volume.metadata.spacing;
                        let offset = medical_volume.metadata.offset;
                        let orientation = medical_volume.metadata.orientation;
                        let transform: Vec<f32> = orientation.into_iter().flatten().collect();
                        
                        MedicalVolume::generate_ct_volume_mha(
                            [dimensions[0], dimensions[1], dimensions[2]],
                            data,
                            medical_volume.metadata.pixel_type,
                            spacing,
                            offset,
                            transform,
                            slope,
                            intercept
                        ).map_err(|e| JsValue::from_str(&e))?
                    },
                    _ => return Err(JsValue::from_str("Unsupported pixel data type")),
                };
                
                Ok(ct_volume)
            }();
            
            // Resolve or reject the promise based on the result
            match result {
                Ok(volume) => {
                        let js_value = serde_wasm_bindgen::to_value(&volume)
                    .unwrap_or_else(|_| JsValue::from_str("Serialization failed"));
                let _ = resolve.call1(&JsValue::NULL, &js_value);
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
    
    // return the first successful result 
    let results_guard = JsFuture::from(promise).await?;
    match serde_wasm_bindgen::from_value::<CTVolume>(results_guard) {
        Ok(volume) => Ok(volume),
        Err(e) => Err(JsValue::from_str(&format!("Deserialization error: {}", e))),
    }
}

/// Function-level comment: Parses common medical imaging files (MHA, MHD, etc.) for WASM
/// Uses the unified MedicalImageParser interface to support multiple formats
/// For MHD files, expects both header (.mhd) and data (.raw/.zraw) files in the array
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg(target_arch = "wasm32")]
pub async fn parse_common_files_wasm(files: Array, info: js_sys::Uint8Array) -> Result<CTVolume, JsValue> {
    // Parse info parameters once
    let mut buf = vec![0u8; info.length() as usize];
    info.copy_to(&mut buf[..]);
    let info_json = String::from_utf8(buf)
        .map_err(|e| JsValue::from_str(&format!("UTF8 decode error: {}", e)))?;
    let info: serde_json::Value = serde_json::from_str(&info_json)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
    let slope = info["slope"].as_f64()
        .map(|v| v as f32)
        .ok_or(JsValue::from_str("Missing slope in info"))?;
    let intercept = info["intercept"].as_f64()
        .map(|v| v as f32)
        .ok_or(JsValue::from_str("Missing intercept in info"))?;
    log::info!("Medical imaging info: slope = {:?}, intercept = {:?}", slope, intercept);

    // Collect and categorize medical imaging files
    let mut mha_files = Vec::new();
    let mut mhd_files = Vec::new();
    let mut raw_files = Vec::new();
    let len = files.length() as usize;
    
    for idx in 0..len {
        let file: File = files.get(idx as u32).dyn_into()?;
        let file_name = file.name().to_lowercase();
        
        // Categorize files by extension
        if file_name.ends_with(".mha") {
            mha_files.push(file);
        } else if file_name.ends_with(".mhd") {
            mhd_files.push(file);
        } else if file_name.ends_with(".raw") || file_name.ends_with(".zraw") {
            raw_files.push(file);
        } else {
            // Check using the extension parser for other formats
            match get_extension(&file.name()) {
                Ok(ImageFormat::NIfTI) | Ok(ImageFormat::DICOM) => {
                    log::warn!("Format {:?} not yet supported in WASM", get_extension(&file.name()).unwrap());
                },
                _ => {
                    log::debug!("Skipping unsupported file: {}", file.name());
                }
            }
        }
    }
    
    // Process files based on what's available
    if !mha_files.is_empty() {
        // Process MHA files (self-contained)
        log::info!("Processing {} MHA file(s)", mha_files.len());
        let first_mha = mha_files.into_iter().next().unwrap();
        process_single_mha_file(first_mha, slope, intercept).await
    } else if !mhd_files.is_empty() && !raw_files.is_empty() {
        // Process MHD files (require separate data files)
        log::info!("Processing MHD files: {} header(s), {} data file(s)", mhd_files.len(), raw_files.len());
        
        // Find matching MHD and data file pairs
        let mut matched_pair = None;
        for mhd_file in &mhd_files {
            let mhd_name = mhd_file.name();
            let base_name = mhd_name.trim_end_matches(".mhd").trim_end_matches(".MHD");
            
            // Look for corresponding data file
            for raw_file in &raw_files {
                let raw_name = raw_file.name();
                let raw_base = raw_name.trim_end_matches(".raw")
                    .trim_end_matches(".RAW")
                    .trim_end_matches(".zraw")
                    .trim_end_matches(".ZRAW");
                
                if base_name.eq_ignore_ascii_case(raw_base) {
                    matched_pair = Some((mhd_file.clone(), raw_file.clone()));
                    log::info!("Found matching MHD pair: {} + {}", mhd_name, raw_name);
                    break;
                }
            }
            
            if matched_pair.is_some() {
                break;
            }
        }
        
        match matched_pair {
            Some((header_file, data_file)) => {
                let header_data = read_file_as_bytes(header_file).await?;
                let metadata = MhdParser::parse_single_file(&header_data)
                    .map_err(|e| JsValue::from_str(&format!("MHD parse error: {}", e)))?;
                let data_bytes = read_file_as_bytes(data_file).await?;
                let pixel_data = PixelData::UInt8(data_bytes);
                let medical_volume = MedicalVolume::new(metadata, pixel_data, ImageFormat::MHD)
                    .map_err(|e| JsValue::from_str(&format!("Volume creation error: {}", e)))?;
                let ct_volume = match medical_volume.pixel_data {
                    PixelData::UInt8(data) => {
                        // Extract dimensions from metadata
                        let dimensions = medical_volume.metadata.dimensions;
                        let spacing = medical_volume.metadata.spacing;
                        let offset = medical_volume.metadata.offset;
                        let orientation = medical_volume.metadata.orientation;
                        let transform: Vec<f32> = orientation.into_iter().flatten().collect();
                        
                        MedicalVolume::generate_ct_volume_mha(
                            [dimensions[0], dimensions[1], dimensions[2]],
                            data,
                            medical_volume.metadata.pixel_type,
                            spacing,
                            offset,
                            transform,
                            slope,
                            intercept
                        ).map_err(|e| JsValue::from_str(&e))?
                    },
                    _ => return Err(JsValue::from_str("Unsupported pixel data type")),
                };
                
                Ok(ct_volume)
            },
            None => {
                Err(JsValue::from_str(
                    "MHD header files found but no corresponding data files (.raw/.zraw). \
                     MHD format requires both header and data files."
                ))
            }
        }
    } else if !mhd_files.is_empty() {
        // MHD files without data files - return error with helpful message
        Err(JsValue::from_str(
            "MHD header files found but no corresponding data files (.raw/.zraw). \
             MHD format requires both header and data files."
        ))
    } else {
        // No supported files found
        Err(JsValue::from_str(
            "No supported medical imaging files found. \
             Supported formats: MHA (self-contained), MHD+RAW (header+data pair)"
        ))
    }
}

pub async fn read_file_as_bytes(file: File) -> Result<Vec<u8>, JsValue> {
    let array_buffer = JsFuture::from(file.array_buffer()).await?;
    let uint8_array = Uint8Array::new(&array_buffer);
    Ok(uint8_array.to_vec())
}
