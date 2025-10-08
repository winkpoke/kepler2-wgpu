# MHA/MHD Medical Imaging Architecture

## Overview

This document describes the architecture for reading and processing medical imaging files in MHA (MetaImage) and MHD (MetaIO) formats within the Kepler WGPU medical imaging framework. The solution provides a robust, modular, and extensible system for handling various medical imaging formats while maintaining high performance and accuracy.

## Architecture Goals

### Primary Objectives
- **Robust File Parsing**: Support both MHA and MHD formats with comprehensive metadata extraction
- **Data Integrity**: Validate file integrity and handle various compression schemes
- **Extensibility**: Modular design to support future medical imaging formats
- **Performance**: Efficient memory usage and processing for large medical datasets
- **Cross-Platform**: Native and WebAssembly compatibility
- **Medical Accuracy**: Preserve spatial information and metadata critical for medical applications

### Design Principles
- **Separation of Concerns**: Clear separation between parsing, processing, and representation
- **Error Resilience**: Comprehensive error handling with clear diagnostic messages
- **Memory Safety**: Rust's ownership system ensures safe handling of large medical datasets
- **Zero-Copy Operations**: Minimize data copying for performance-critical operations
- **Async Support**: Non-blocking I/O for large file processing

## System Architecture

### Module Structure

```
src/data/medical_imaging/
├── mod.rs                    # Module exports and public API
├── formats/                  # Format-specific parsers
│   ├── mod.rs               # Format registry and common traits
│   ├── mha.rs               # MHA format parser
│   ├── mhd.rs               # MHD format parser
│   └── common.rs            # Shared parsing utilities
├── metadata/                 # Metadata handling
│   ├── mod.rs               # Metadata types and validation
│   ├── spatial.rs           # Spatial information (spacing, origin, orientation)
│   └── image_info.rs        # Image properties (dimensions, data type, etc.)
├── data/                     # Data processing and representation
│   ├── mod.rs               # Data processing API
│   ├── volume.rs            # 3D volume representation
│   ├── pixel_data.rs        # Pixel data handling and conversion
│   └── compression.rs       # Compression/decompression support
├── error.rs                  # Medical imaging specific errors
└── validation.rs            # File integrity and validation
```

## Core Components

### 1. File Parser Module (`formats/`)

#### MHA Parser (`formats/mha.rs`)
```rust
/// Function-level comment: Parses MHA (MetaImage) files with embedded data
/// Handles both ASCII and binary headers with inline image data
pub struct MhaParser {
    /// Validates MHA file signature and format
    validator: FormatValidator,
    /// Handles different compression schemes
    compression_handler: CompressionHandler,
    /// Manages endianness conversion
    endian_converter: EndianConverter,
}

impl MhaParser {
    /// Parses complete MHA file including header and embedded data
    pub async fn parse_file<P: AsRef<Path>>(path: P) -> MedicalImagingResult<MedicalVolume>;
    
    /// Parses MHA from byte buffer for WASM compatibility
    pub fn parse_bytes(data: &[u8]) -> MedicalImagingResult<MedicalVolume>;
    
    /// Extracts only metadata without loading pixel data
    pub fn parse_metadata_only<P: AsRef<Path>>(path: P) -> MedicalImagingResult<ImageMetadata>;
}
```

#### MHD Parser (`formats/mhd.rs`)
```rust
/// Function-level comment: Parses MHD (MetaIO) files with separate data files
/// Handles header files that reference external raw or compressed data
pub struct MhdParser {
    /// Validates MHD header format
    validator: FormatValidator,
    /// Resolves data file paths relative to header
    path_resolver: PathResolver,
    /// Handles various data file formats
    data_loader: DataFileLoader,
}

impl MhdParser {
    /// Parses MHD header and loads associated data file
    pub async fn parse_file<P: AsRef<Path>>(header_path: P) -> MedicalImagingResult<MedicalVolume>;
    
    /// Parses header only without loading data file
    pub fn parse_header<P: AsRef<Path>>(path: P) -> MedicalImagingResult<ImageMetadata>;
    
    /// Loads data file separately (useful for lazy loading)
    pub async fn load_data_file<P: AsRef<Path>>(
        metadata: &ImageMetadata, 
        data_path: P
    ) -> MedicalImagingResult<PixelData>;
}
```

#### Common Format Traits (`formats/common.rs`)
```rust
/// Function-level comment: Common interface for all medical imaging format parsers
/// Ensures consistent API across different format implementations
pub trait MedicalImageParser {
    /// Parses complete medical image file
    async fn parse<P: AsRef<Path>>(path: P) -> MedicalImagingResult<MedicalVolume>;
    
    /// Validates file format without full parsing
    fn validate_format<P: AsRef<Path>>(path: P) -> MedicalImagingResult<bool>;
    
    /// Extracts metadata without pixel data
    fn extract_metadata<P: AsRef<Path>>(path: P) -> MedicalImagingResult<ImageMetadata>;
    
    /// Returns supported file extensions
    fn supported_extensions() -> &'static [&'static str];
}

/// Function-level comment: Registry for format-specific parsers
/// Enables automatic format detection and parser selection
pub struct FormatRegistry {
    parsers: HashMap<String, Box<dyn MedicalImageParser>>,
}

impl FormatRegistry {
    /// Automatically detects format and selects appropriate parser
    pub fn parse_auto<P: AsRef<Path>>(path: P) -> MedicalImagingResult<MedicalVolume>;
    
    /// Registers new format parser for extensibility
    pub fn register_parser<T: MedicalImageParser + 'static>(
        &mut self, 
        extensions: &[&str], 
        parser: T
    );
}
```

### 2. Data Processing Module (`data/`)

#### Medical Volume Representation (`data/volume.rs`)
```rust
/// Function-level comment: Standardized 3D medical volume representation
/// Preserves all metadata and provides efficient access to pixel data
#[derive(Debug, Clone)]
pub struct MedicalVolume {
    /// Image metadata including spatial information
    pub metadata: ImageMetadata,
    /// Pixel data with type-safe access
    pub pixel_data: PixelData,
    /// Original file format for provenance tracking
    pub source_format: ImageFormat,
    /// Validation status and integrity checks
    pub validation_status: ValidationStatus,
}

impl MedicalVolume {
    /// Creates new medical volume with validation
    pub fn new(
        metadata: ImageMetadata, 
        pixel_data: PixelData, 
        source_format: ImageFormat
    ) -> MedicalImagingResult<Self>;
    
    /// Converts to different pixel data type
    pub fn convert_pixel_type<T: PixelType>(&self) -> MedicalImagingResult<MedicalVolume>;
    
    /// Extracts 2D slice at specified index
    pub fn extract_slice(&self, axis: Axis, index: usize) -> MedicalImagingResult<MedicalSlice>;
    
    /// Resamples volume to new spacing
    pub fn resample(&self, new_spacing: [f64; 3]) -> MedicalImagingResult<MedicalVolume>;
    
    /// Validates data integrity
    pub fn validate(&self) -> ValidationResult;
}
```

#### Pixel Data Handling (`data/pixel_data.rs`)
```rust
/// Function-level comment: Type-safe pixel data container with efficient access
/// Supports various pixel types common in medical imaging
#[derive(Debug, Clone)]
pub enum PixelData {
    /// 8-bit unsigned integer (common for segmentations)
    UInt8(Vec<u8>),
    /// 16-bit signed integer (common for CT)
    Int16(Vec<i16>),
    /// 16-bit unsigned integer
    UInt16(Vec<u16>),
    /// 32-bit signed integer
    Int32(Vec<i32>),
    /// 32-bit floating point (common for processed data)
    Float32(Vec<f32>),
    /// 64-bit floating point (high precision)
    Float64(Vec<f64>),
}

impl PixelData {
    /// Creates pixel data from raw bytes with type conversion
    pub fn from_bytes(
        bytes: &[u8], 
        pixel_type: PixelType, 
        endianness: Endianness
    ) -> MedicalImagingResult<Self>;
    
    /// Converts to different pixel type with proper scaling
    pub fn convert_to<T: PixelType>(&self) -> MedicalImagingResult<PixelData>;
    
    /// Gets pixel value at 3D coordinates
    pub fn get_pixel(&self, x: usize, y: usize, z: usize, dims: [usize; 3]) -> Option<f64>;
    
    /// Sets pixel value at 3D coordinates
    pub fn set_pixel(&mut self, x: usize, y: usize, z: usize, dims: [usize; 3], value: f64) -> MedicalImagingResult<()>;
    
    /// Returns raw byte representation
    pub fn as_bytes(&self) -> &[u8];
    
    /// Returns data statistics (min, max, mean, std)
    pub fn statistics(&self) -> PixelStatistics;
}
```

#### Metadata Management (`metadata/`)
```rust
/// Function-level comment: Comprehensive medical image metadata
/// Preserves all spatial and acquisition information
#[derive(Debug, Clone, PartialEq)]
pub struct ImageMetadata {
    /// Image dimensions [width, height, depth]
    pub dimensions: [usize; 3],
    /// Pixel spacing in mm [x, y, z]
    pub spacing: [f64; 3],
    /// Image origin in world coordinates [x, y, z]
    pub origin: [f64; 3],
    /// Orientation matrix (3x3)
    pub orientation: [[f64; 3]; 3],
    /// Pixel data type
    pub pixel_type: PixelType,
    /// Number of components per pixel
    pub components: usize,
    /// Endianness of pixel data
    pub endianness: Endianness,
    /// Compression type if any
    pub compression: Option<CompressionType>,
    /// Additional format-specific metadata
    pub custom_fields: HashMap<String, MetadataValue>,
}

impl ImageMetadata {
    /// Validates metadata consistency
    pub fn validate(&self) -> ValidationResult;
    
    /// Calculates total number of pixels
    pub fn total_pixels(&self) -> usize;
    
    /// Calculates data size in bytes
    pub fn data_size_bytes(&self) -> usize;
    
    /// Converts world coordinates to voxel indices
    pub fn world_to_voxel(&self, world_pos: [f64; 3]) -> [f64; 3];
    
    /// Converts voxel indices to world coordinates
    pub fn voxel_to_world(&self, voxel_pos: [f64; 3]) -> [f64; 3];
}

/// Function-level comment: Spatial information utilities
/// Handles coordinate transformations and spatial relationships
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialInfo {
    pub spacing: [f64; 3],
    pub origin: [f64; 3],
    pub orientation: [[f64; 3]; 3],
}

impl SpatialInfo {
    /// Creates spatial info with validation
    pub fn new(spacing: [f64; 3], origin: [f64; 3], orientation: [[f64; 3]; 3]) -> MedicalImagingResult<Self>;
    
    /// Validates orientation matrix orthogonality
    pub fn validate_orientation(&self) -> ValidationResult;
    
    /// Calculates transformation matrix
    pub fn transformation_matrix(&self) -> [[f64; 4]; 4];
}
```

### 3. Error Handling System (`error.rs`)

```rust
/// Function-level comment: Comprehensive error types for medical imaging operations
/// Provides detailed error information for debugging and user feedback
#[derive(Debug, thiserror::Error)]
pub enum MedicalImagingError {
    /// File format not supported
    #[error("Unsupported file format: {format}")]
    UnsupportedFormat { format: String },
    
    /// Invalid file header
    #[error("Invalid file header: {reason}")]
    InvalidHeader { reason: String },
    
    /// Metadata validation failed
    #[error("Metadata validation failed: {field} - {reason}")]
    MetadataValidation { field: String, reason: String },
    
    /// Pixel data corruption detected
    #[error("Pixel data corruption: expected {expected} bytes, found {actual}")]
    DataCorruption { expected: usize, actual: usize },
    
    /// Compression/decompression error
    #[error("Compression error: {algorithm} - {reason}")]
    CompressionError { algorithm: String, reason: String },
    
    /// Endianness conversion error
    #[error("Endianness conversion failed: {reason}")]
    EndiannessError { reason: String },
    
    /// File I/O error
    #[error("File I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Memory allocation error
    #[error("Memory allocation failed: requested {size} bytes")]
    MemoryAllocation { size: usize },
    
    /// Data type conversion error
    #[error("Data type conversion failed: from {from} to {to}")]
    TypeConversion { from: String, to: String },
    
    /// Validation error
    #[error("Validation failed: {0}")]
    Validation(String),
}

/// Result type alias for medical imaging operations
pub type MedicalImagingResult<T> = Result<T, MedicalImagingError>;

/// Function-level comment: Error context for detailed diagnostics
/// Provides additional context for error analysis and debugging
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub file_path: Option<PathBuf>,
    pub operation: String,
    pub line_number: Option<usize>,
    pub additional_info: HashMap<String, String>,
}

impl ErrorContext {
    /// Creates new error context
    pub fn new(operation: &str) -> Self;
    
    /// Adds file path context
    pub fn with_file<P: AsRef<Path>>(mut self, path: P) -> Self;
    
    /// Adds line number context
    pub fn with_line(mut self, line: usize) -> Self;
    
    /// Adds additional context information
    pub fn with_info(mut self, key: &str, value: &str) -> Self;
}
```

### 4. Validation System (`validation.rs`)

```rust
/// Function-level comment: Comprehensive file and data validation
/// Ensures data integrity and format compliance
pub struct MedicalImageValidator {
    /// Format-specific validators
    format_validators: HashMap<ImageFormat, Box<dyn FormatValidator>>,
    /// Data integrity checkers
    integrity_checkers: Vec<Box<dyn IntegrityChecker>>,
}

impl MedicalImageValidator {
    /// Validates complete medical image file
    pub fn validate_file<P: AsRef<Path>>(path: P) -> ValidationResult;
    
    /// Validates metadata consistency
    pub fn validate_metadata(metadata: &ImageMetadata) -> ValidationResult;
    
    /// Validates pixel data integrity
    pub fn validate_pixel_data(data: &PixelData, metadata: &ImageMetadata) -> ValidationResult;
    
    /// Performs checksum validation if available
    pub fn validate_checksum<P: AsRef<Path>>(path: P) -> ValidationResult;
}

/// Function-level comment: Validation result with detailed diagnostics
/// Provides comprehensive validation feedback
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub warnings: Vec<ValidationWarning>,
    pub errors: Vec<ValidationError>,
    pub metadata_issues: Vec<MetadataIssue>,
    pub data_issues: Vec<DataIssue>,
}

impl ValidationResult {
    /// Creates successful validation result
    pub fn success() -> Self;
    
    /// Creates failed validation result with error
    pub fn failure(error: ValidationError) -> Self;
    
    /// Adds warning to result
    pub fn with_warning(mut self, warning: ValidationWarning) -> Self;
    
    /// Combines multiple validation results
    pub fn combine(results: Vec<ValidationResult>) -> Self;
}
```

## Integration with Existing System

### Error System Integration

The medical imaging errors will be integrated into the existing `KeplerError` enum:

```rust
// In src/core/error.rs
#[derive(Debug)]
pub enum KeplerError {
    // ... existing variants ...
    
    /// Medical imaging file processing errors
    MedicalImaging(MedicalImagingError),
}

impl From<MedicalImagingError> for KeplerError {
    fn from(err: MedicalImagingError) -> Self {
        KeplerError::MedicalImaging(err)
    }
}
```

### Data Module Integration

The medical imaging module will be added to the existing data module:

```rust
// In src/data/mod.rs
pub mod medical_imaging;
pub use medical_imaging::*;
```

### Rendering Integration

Medical volumes can be converted to existing rendering formats:

```rust
impl From<MedicalVolume> for CTVolume {
    fn from(volume: MedicalVolume) -> Self {
        // Convert medical volume to CT volume format
        // Preserve spatial information and metadata
    }
}
```

## Performance Considerations

### Memory Management
- **Lazy Loading**: Load pixel data only when needed
- **Memory Mapping**: Use memory-mapped files for large datasets
- **Streaming**: Process large files in chunks
- **Zero-Copy**: Minimize data copying operations

### Async Operations
- **Non-Blocking I/O**: Use async file operations
- **Progress Reporting**: Provide progress callbacks for large files
- **Cancellation**: Support operation cancellation

### Optimization Strategies
- **SIMD**: Use SIMD instructions for pixel data processing
- **Parallel Processing**: Utilize multiple cores for data conversion
- **Caching**: Cache frequently accessed metadata
- **Compression**: Support efficient compression algorithms

## Cross-Platform Compatibility

### Native Platforms
- **File System Access**: Direct file system operations
- **Memory Mapping**: Efficient large file handling
- **Threading**: Multi-threaded processing

### WebAssembly (WASM)
- **Byte Array Input**: Process files from byte arrays
- **Memory Constraints**: Efficient memory usage
- **Browser APIs**: Integration with File API

## Testing Strategy

### Unit Tests
- **Parser Tests**: Validate format-specific parsing
- **Metadata Tests**: Test metadata extraction and validation
- **Data Conversion Tests**: Verify pixel data conversions
- **Error Handling Tests**: Test error conditions and recovery

### Integration Tests
- **End-to-End Tests**: Complete file processing workflows
- **Format Compatibility**: Test with real medical imaging files
- **Performance Tests**: Benchmark large file processing
- **Cross-Platform Tests**: Validate WASM and native compatibility

### Test Data
- **Sample Files**: Curated set of MHA/MHD test files
- **Edge Cases**: Malformed files and edge conditions
- **Performance Datasets**: Large files for performance testing

## Future Extensibility

### Additional Formats
The architecture supports easy addition of new medical imaging formats:

1. **Implement `MedicalImageParser` trait**
2. **Register parser in `FormatRegistry`**
3. **Add format-specific validation**
4. **Update documentation**

### Potential Future Formats
- **NIfTI**: Neuroimaging Informatics Technology Initiative
- **ANALYZE**: Mayo Clinic format
- **MINC**: Medical Image NetCDF
- **ITK**: Insight Toolkit formats
- **VTK**: Visualization Toolkit formats

### Plugin Architecture
Future enhancement could include a plugin system for format parsers:

```rust
pub trait FormatPlugin {
    fn name(&self) -> &str;
    fn supported_extensions(&self) -> &[&str];
    fn create_parser(&self) -> Box<dyn MedicalImageParser>;
}
```

## Security Considerations

### Input Validation
- **File Size Limits**: Prevent memory exhaustion attacks
- **Header Validation**: Strict validation of file headers
- **Metadata Bounds**: Validate metadata ranges
- **Buffer Overflow Protection**: Safe buffer handling

### Memory Safety
- **Rust Ownership**: Leverage Rust's memory safety guarantees
- **Bounds Checking**: Prevent buffer overruns
- **Integer Overflow**: Handle large dimension calculations safely

## Documentation and Examples

### API Documentation
- **Comprehensive rustdoc**: Document all public APIs
- **Usage Examples**: Practical usage examples
- **Error Handling**: Error handling best practices
- **Performance Tips**: Optimization guidelines

### Example Usage
```rust
use kepler_wgpu::data::medical_imaging::*;

// Parse MHA file
let volume = MhaParser::parse_file("scan.mha").await?;

// Validate data integrity
let validation = volume.validate();
if !validation.is_valid {
    eprintln!("Validation warnings: {:?}", validation.warnings);
}

// Convert pixel data type
let float_volume = volume.convert_pixel_type::<f32>()?;

// Extract axial slice
let slice = volume.extract_slice(Axis::Z, 100)?;

// Access metadata
println!("Dimensions: {:?}", volume.metadata.dimensions);
println!("Spacing: {:?}", volume.metadata.spacing);
println!("Origin: {:?}", volume.metadata.origin);
```

## Conclusion

This architecture provides a robust, extensible foundation for medical imaging file support in the Kepler WGPU framework. The modular design ensures maintainability while the comprehensive error handling and validation systems provide reliability for medical applications. The cross-platform compatibility and performance optimizations make it suitable for both research and clinical applications.

The implementation follows Rust best practices and leverages the language's safety guarantees to handle large medical datasets reliably. The extensible design allows for future format additions while maintaining backward compatibility and consistent APIs.