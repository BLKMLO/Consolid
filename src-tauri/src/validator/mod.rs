//! Module for validating input files

use std::path::Path;
use std::collections::HashMap;
use thiserror::Error;
use crate::FileType;

/// Error type for validation operations
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),
    
    #[error("File is empty")]
    EmptyFile,
    
    #[error("File is too large: {0} bytes (max: {1})")]
    FileTooLarge(u64, u64),
    
    #[error("Invalid file structure: {0}")]
    InvalidStructure(String),
    
    #[error("Missing required columns: {0}")]
    MissingColumns(String),
    
    #[error("File read error: {0}")]
    FileReadError(String),
}

/// Validation status for a file
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FileValidationResult {
    pub file_path: String,
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub file_type: FileType,
    pub file_size: u64,
    pub metadata: HashMap<String, String>,
}

/// Overall validation result
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub file_results: Vec<FileValidationResult>,
}

/// Validation status enum
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ValidationStatus {
    NotStarted,
    InProgress,
    Valid,
    Invalid,
}

/// Main validator struct
pub struct FileValidator {
    status: ValidationStatus,
    max_file_size: u64,
    required_columns: Vec<String>,
    supported_types: Vec<FileType>,
}

impl FileValidator {
    /// Create a new FileValidator instance
    pub fn new() -> Self {
        FileValidator {
            status: ValidationStatus::NotStarted,
            max_file_size: 10 * 1024 * 1024, // 10 MB
            required_columns: vec![
                "date".to_string(),
                "compte".to_string(),
                "libellé".to_string(),
                "débit".to_string(),
                "crédit".to_string(),
                "montant".to_string(),
            ],
            supported_types: vec![
                FileType::Csv,
                FileType::Excel,
                FileType::Text,
            ],
        }
    }
    
    /// Validate a single file
    pub async fn validate_file(&mut self, file_path: &str) -> Result<FileValidationResult, ValidationError> {
        let path = Path::new(file_path);
        
        if !path.exists() {
            return Err(ValidationError::FileNotFound(file_path.to_string()));
        }
        
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Check file size
        let metadata = std::fs::metadata(file_path)
            .map_err(|e| ValidationError::FileReadError(e.to_string()))?;
        
        let file_size = metadata.len();
        
        if file_size == 0 {
            return Err(ValidationError::EmptyFile);
        }
        
        if file_size > self.max_file_size {
            return Err(ValidationError::FileTooLarge(
                file_size,
                self.max_file_size
            ));
        }
        
        // Detect file type
        let file_type = Self::detect_file_type(file_path);
        
        // Check if file type is supported
        if !self.supported_types.contains(&file_type) {
            return Err(ValidationError::UnsupportedFileType(
                format!("{:?}", file_type)
            ));
        }
        
        // Validate file content
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut file_metadata = HashMap::new();
        
        match self.validate_content(file_path, &file_type).await {
            Ok(content_metadata) => {
                file_metadata.extend(content_metadata);
            }
            Err(e) => {
                errors.push(e.to_string());
            }
        }
        
        // Check for required columns if it's a structured file
        if file_type == FileType::Csv || file_type == FileType::Excel {
            if let Err(e) = self.check_required_columns(file_path, &file_type).await {
                errors.push(e.to_string());
            }
        }
        
        let is_valid = errors.is_empty();
        
        Ok(FileValidationResult {
            file_path: file_path.to_string(),
            is_valid,
            errors,
            warnings,
            file_type,
            file_size,
            metadata: file_metadata,
        })
    }
    
    /// Validate multiple files
    pub async fn validate_files(&mut self, file_paths: Vec<String>) -> Result<ValidationResult, ValidationError> {
        self.status = ValidationStatus::InProgress;
        
        let mut file_results = Vec::new();
        let mut all_errors = Vec::new();
        let mut all_warnings = Vec::new();
        
        for path in file_paths {
            match self.validate_file(&path).await {
                Ok(result) => {
                    file_results.push(result);
                    if !result.is_valid {
                        all_errors.extend(result.errors);
                    }
                    all_warnings.extend(result.warnings);
                }
                Err(e) => {
                    all_errors.push(e.to_string());
                    file_results.push(FileValidationResult {
                        file_path: path,
                        is_valid: false,
                        errors: vec![e.to_string()],
                        warnings: Vec::new(),
                        file_type: FileType::Unknown,
                        file_size: 0,
                        metadata: HashMap::new(),
                    });
                }
            }
        }
        
        let is_valid = all_errors.is_empty() && file_results.iter().all(|f| f.is_valid);
        
        self.status = if is_valid {
            ValidationStatus::Valid
        } else {
            ValidationStatus::Invalid
        };
        
        Ok(ValidationResult {
            is_valid,
            errors: all_errors,
            warnings: all_warnings,
            file_results,
        })
    }
    
    /// Validate file content
    async fn validate_content(
        &self,
        file_path: &str,
        file_type: &FileType,
    ) -> Result<HashMap<String, String>, ValidationError> {
        let mut metadata = HashMap::new();
        
        match file_type {
            FileType::Csv => {
                let content = std::fs::read_to_string(file_path)
                    .map_err(|e| ValidationError::FileReadError(e.to_string()))?;
                
                let mut reader = csv::Reader::from_reader(content.as_bytes());
                
                // Check if we can read headers
                if let Ok(headers) = reader.headers() {
                    metadata.insert("columns".to_string(), headers.len().to_string());
                    metadata.insert("headers".to_string(), headers.join(","));
                }
                
                // Count rows
                let row_count = reader.records().count();
                metadata.insert("row_count".to_string(), row_count.to_string());
            }
            FileType::Excel => {
                // For Excel files, we'd use the excel crate
                // For now, just check if we can open it
                metadata.insert("type".to_string(), "excel".to_string());
            }
            FileType::Text => {
                let content = std::fs::read_to_string(file_path)
                    .map_err(|e| ValidationError::FileReadError(e.to_string()))?;
                
                metadata.insert("line_count".to_string(), content.lines().count().to_string());
                metadata.insert("char_count".to_string(), content.len().to_string());
            }
            _ => {}
        }
        
        Ok(metadata)
    }
    
    /// Check for required columns
    async fn check_required_columns(
        &self,
        file_path: &str,
        file_type: &FileType,
    ) -> Result<(), ValidationError> {
        match file_type {
            FileType::Csv => {
                let content = std::fs::read_to_string(file_path)
                    .map_err(|e| ValidationError::FileReadError(e.to_string()))?;
                
                let mut reader = csv::Reader::from_reader(content.as_bytes());
                
                if let Ok(headers) = reader.headers() {
                    let header_names: Vec<String> = headers.iter()
                        .map(|h| h.to_lowercase().trim().to_string())
                        .collect();
                    
                    let mut missing = Vec::new();
                    
                    for required in &self.required_columns {
                        if !header_names.iter().any(|h| h.contains(required)) {
                            missing.push(required.clone());
                        }
                    }
                    
                    if !missing.is_empty() {
                        return Err(ValidationError::MissingColumns(
                            format!("Missing columns: {}", missing.join(", "))
                        ));
                    }
                }
            }
            FileType::Excel => {
                // Similar logic for Excel files
                // Would use excel crate to read headers
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Detect file type from path
    fn detect_file_type(file_path: &str) -> FileType {
        let path = Path::new(file_path);
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());
        
        match ext.as_deref() {
            Some("csv") => FileType::Csv,
            Some("xlsx") | Some("xls") | Some("ods") => FileType::Excel,
            Some("pdf") => FileType::Pdf,
            Some("txt") | Some("md") | Some("json") => FileType::Text,
            _ => FileType::Unknown,
        }
    }
    
    /// Get current validation status
    pub fn get_status(&self) -> ValidationStatus {
        self.status.clone()
    }
    
    /// Set validation status
    pub fn set_status(&mut self, status: ValidationStatus) {
        self.status = status;
    }
    
    /// Reset validator state
    pub fn reset(&mut self) {
        self.status = ValidationStatus::NotStarted;
    }
}

impl Default for FileValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ValidationStatus {
    fn default() -> Self {
        ValidationStatus::NotStarted
    }
}
