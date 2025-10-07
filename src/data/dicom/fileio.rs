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
use web_sys::{File, FileReader, ProgressEvent, console};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;


#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg(target_arch = "wasm32")]
pub async fn parse_dcm_files_wasm(files: Array) -> Result<DicomRepo, JsValue> {
    // use futures::channel::oneshot;
    // use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    // use wasm_bindgen_futures::future_to_promise;
    use log::error;

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

