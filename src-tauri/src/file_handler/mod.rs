//! Module for handling file operations

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use thiserror::Error;
use crate::{FileInfo, FileType, AnonymizedFile, ProcessResult, ValidationResult};
use crate::anonymizer::Anonymizer;
use crate::validator::FileValidator;

/// Error type for file operations
#[derive(Error, Debug)]
pub enum FileHandlerError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Invalid file path: {0}")]
    InvalidPath(String),
    
    #[error("File read error: {0}")]
    FileReadError(String),
    
    #[error("File write error: {0}")]
    FileWriteError(String),
    
    #[error("No files provided")]
    NoFilesProvided,
    
    #[error("Template file not found")]
    TemplateNotFound,
}

/// Main file processor struct
pub struct FileProcessor {
    files: Vec<FileInfo>,
    template_path: Option<String>,
    template_content: Option<String>,
    processed_files: Vec<AnonymizedFile>,
    output_dir: PathBuf,
}

impl FileProcessor {
    /// Create a new FileProcessor instance
    pub fn new() -> Self {
        FileProcessor {
            files: Vec::new(),
            template_path: None,
            template_content: None,
            processed_files: Vec::new(),
            output_dir: PathBuf::from("./output"),
        }
    }
    
    /// Handle file drop event
    pub async fn handle_file_drop(&mut self, paths: Vec<String>) -> Result<Vec<FileInfo>, String> {
        let mut new_files = Vec::new();
        
        for path in paths {
            match self.add_file(&path).await {
                Ok(file_info) => new_files.push(file_info),
                Err(e) => {
                    eprintln!("Failed to add file {}: {}", path, e);
                }
            }
        }
        
        Ok(new_files)
    }
    
    /// Add a single file
    pub async fn add_file(&mut self, path: &str) -> Result<FileInfo, FileHandlerError> {
        let path_obj = Path::new(path);
        
        if !path_obj.exists() {
            return Err(FileHandlerError::FileNotFound(path.to_string()));
        }
        
        let metadata = std::fs::metadata(path)
            .map_err(|e| FileHandlerError::FileReadError(e.to_string()))?;
        
        let file_name = path_obj.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let file_type = Self::detect_file_type(path);
        
        let file_info = FileInfo {
            path: path.to_string(),
            name: file_name,
            size: metadata.len(),
            file_type,
            is_valid: true,
            error: None,
        };
        
        self.files.push(file_info.clone());
        
        Ok(file_info)
    }
    
    /// Set template file
    pub async fn set_template(&mut self, path: &str) -> Result<(), FileHandlerError> {
        let path_obj = Path::new(path);
        
        if !path_obj.exists() {
            return Err(FileHandlerError::TemplateNotFound);
        }
        
        let content = std::fs::read_to_string(path)
            .map_err(|e| FileHandlerError::FileReadError(e.to_string()))?;
        
        self.template_path = Some(path.to_string());
        self.template_content = Some(content);
        
        Ok(())
    }
    
    /// Process all files: validate, anonymize, and prepare for sending
    pub async fn process_files(
        &mut self,
        file_paths: Vec<String>,
        template_path: String,
        anonymizer: &mut Anonymizer,
        validator: &mut FileValidator,
    ) -> Result<ProcessResult, String> {
        // Clear previous state
        self.files.clear();
        self.processed_files.clear();
        self.template_path = None;
        self.template_content = None;
        
        // Add files
        for path in &file_paths {
            self.add_file(path).await
                .map_err(|e| e.to_string())?;
        }
        
        // Set template
        self.set_template(&template_path).await
            .map_err(|e| e.to_string())?;
        
        // Validate files
        let file_paths_clone = file_paths.clone();
        let validation_result = validator.validate_files(file_paths_clone).await
            .map_err(|e| e.to_string())?;
        
        if !validation_result.is_valid {
            return Ok(ProcessResult {
                success: false,
                message: "Validation failed".to_string(),
                processed_files: Vec::new(),
                filled_template: None,
                errors: validation_result.errors,
            });
        }
        
        // Anonymize files
        let anonymized_files = anonymizer.anonymize_files(file_paths).await
            .map_err(|e| e.to_string())?;
        
        self.processed_files = anonymized_files.clone();
        
        // Update file info with validation status
        for (i, file_result) in validation_result.file_results.iter().enumerate() {
            if i < self.files.len() {
                self.files[i].is_valid = file_result.is_valid;
                self.files[i].error = if file_result.is_valid {
                    None
                } else {
                    Some(file_result.errors.join(", "))
                };
            }
        }
        
        Ok(ProcessResult {
            success: true,
            message: "Files processed successfully".to_string(),
            processed_files: self.files.clone(),
            filled_template: None,
            errors: Vec::new(),
        })
    }
    
    /// Get all files
    pub fn get_files(&self) -> &[FileInfo] {
        &self.files
    }
    
    /// Get file count
    pub fn get_file_count(&self) -> usize {
        self.files.len()
    }
    
    /// Get processed files
    pub fn get_processed_files(&self) -> &[AnonymizedFile] {
        &self.processed_files
    }
    
    /// Get template content
    pub fn get_template_content(&self) -> Option<&String> {
        self.template_content.as_ref()
    }
    
    /// Clear all files
    pub fn clear_files(&mut self) {
        self.files.clear();
        self.processed_files.clear();
    }
    
    /// Save processed file to disk
    pub async fn save_processed_file(
        &self,
        file: &AnonymizedFile,
        output_path: Option<&str>,
    ) -> Result<PathBuf, FileHandlerError> {
        let output_dir = if let Some(path) = output_path {
            Path::new(path).parent().unwrap_or(Path::new("."))
        } else {
            &self.output_dir
        };
        
        // Create output directory if it doesn't exist
        if !output_dir.exists() {
            std::fs::create_dir_all(output_dir)
                .map_err(|e| FileHandlerError::FileWriteError(e.to_string()))?;
        }
        
        let file_name = if let Some(path) = output_path {
            Path::new(path).file_name().unwrap_or_else(|| {
                Path::new(&format!("anonymized_{}", file.original_name))
            })
        } else {
            Path::new(&format!("anonymized_{}", file.original_name))
        };
        
        let output_path = output_dir.join(file_name);
        
        // Write content based on file type
        match file.file_type {
            FileType::Csv | FileType::Text => {
                std::fs::write(&output_path, &file.anonymized_content)
                    .map_err(|e| FileHandlerError::FileWriteError(e.to_string()))?;
            }
            FileType::Excel => {
                // For Excel, we'd need to use the excel crate
                // For now, save as CSV
                std::fs::write(&output_path, &file.anonymized_content)
                    .map_err(|e| FileHandlerError::FileWriteError(e.to_string()))?;
            }
            _ => {
                std::fs::write(&output_path, &file.anonymized_content)
                    .map_err(|e| FileHandlerError::FileWriteError(e.to_string()))?;
            }
        }
        
        Ok(output_path)
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
    
    /// Set output directory
    pub fn set_output_dir(&mut self, dir: &str) {
        self.output_dir = PathBuf::from(dir);
    }
}

impl Default for FileProcessor {
    fn default() -> Self {
        Self::new()
    }
}
