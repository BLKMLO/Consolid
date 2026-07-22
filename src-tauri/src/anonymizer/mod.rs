//! Module for anonymizing sensitive data in files

use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

mod patterns;
use patterns::ANONYMIZATION_PATTERNS;

/// Error type for anonymization operations
#[derive(Error, Debug)]
pub enum AnonymizationError {
    #[error("Failed to read file: {0}")]
    FileReadError(String),
    
    #[error("Failed to write file: {0}")]
    FileWriteError(String),
    
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),
    
    #[error("Anonymization pattern error: {0}")]
    PatternError(String),
    
    #[error("No content to anonymize")]
    EmptyContent,
}

/// Represents an anonymized file
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AnonymizedFile {
    pub original_name: String,
    pub anonymized_content: String,
    pub file_type: crate::FileType,
    pub metadata: serde_json::Value,
}

/// Tracks replacements for potential reversal (if needed for debugging)
#[derive(Clone, Debug)]
pub struct ReplacementLog {
    pub original: String,
    pub replacement: String,
    pub pattern_name: String,
    pub file: String,
}

/// Main anonymizer struct
pub struct Anonymizer {
    replacement_map: HashMap<String, String>,
    replacement_log: Vec<ReplacementLog>,
    patterns: Vec<(String, Regex)>,
}

impl Anonymizer {
    /// Create a new Anonymizer instance
    pub fn new() -> Self {
        let mut patterns = Vec::new();
        
        // Compile all patterns
        for (name, pattern_str) in ANONYMIZATION_PATTERNS.iter() {
            match Regex::new(pattern_str) {
                Ok(regex) => patterns.push((name.clone(), regex)),
                Err(e) => {
                    eprintln!("Failed to compile pattern '{}': {}", name, e);
                }
            }
        }
        
        Anonymizer {
            replacement_map: HashMap::new(),
            replacement_log: Vec::new(),
            patterns,
        }
    }
    
    /// Anonymize a single file
    pub async fn anonymize_file(&mut self, file_path: &str) -> Result<AnonymizedFile, AnonymizationError> {
        let path = Path::new(file_path);
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Determine file type
        let file_type = Self::detect_file_type(file_path);
        
        // Read file content
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| AnonymizationError::FileReadError(e.to_string()))?;
        
        if content.is_empty() {
            return Err(AnonymizationError::EmptyContent);
        }
        
        // Anonymize content
        let anonymized_content = self.anonymize_text(&content, &file_name)?;
        
        // Create metadata
        let metadata = serde_json::json!({
            "original_path": file_path,
            "anonymized_at": chrono::Utc::now().to_rfc3339(),
            "replacements_count": self.replacement_log.len(),
            "file_size": content.len(),
            "anonymized_size": anonymized_content.len(),
        });
        
        Ok(AnonymizedFile {
            original_name: file_name,
            anonymized_content,
            file_type,
            metadata,
        })
    }
    
    /// Anonymize multiple files
    pub async fn anonymize_files(&mut self, file_paths: Vec<String>) -> Result<Vec<AnonymizedFile>, AnonymizationError> {
        let mut results = Vec::new();
        
        for path in file_paths {
            match self.anonymize_file(&path).await {
                Ok(file) => results.push(file),
                Err(e) => {
                    eprintln!("Failed to anonymize {}: {}", path, e);
                    // Continue with other files
                }
            }
        }
        
        Ok(results)
    }
    
    /// Anonymize text content
    pub fn anonymize_text(&mut self, text: &str, file_name: &str) -> Result<String, AnonymizationError> {
        let mut content = text.to_string();
        self.replacement_log.clear();
        
        // Apply all patterns
        for (pattern_name, regex) in &self.patterns {
            let replacements: Vec<_> = regex.find_iter(&content).collect();
            
            for mat in replacements {
                let original = mat.as_str().to_string();
                let replacement = self.generate_replacement(pattern_name);
                
                // Store in replacement map for potential reversal
                self.replacement_map.insert(original.clone(), replacement.clone());
                
                // Log the replacement
                self.replacement_log.push(ReplacementLog {
                    original: original.clone(),
                    replacement: replacement.clone(),
                    pattern_name: pattern_name.clone(),
                    file: file_name.to_string(),
                });
                
                // Replace in content
                content = content.replace(&original, &replacement);
            }
        }
        
        Ok(content)
    }
    
    /// Generate a replacement token based on pattern type
    fn generate_replacement(&mut self, pattern_name: &str) -> String {
        match pattern_name {
            "email" => format!("EMAIL_{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
            "phone" => format!("PHONE_{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
            "ssn" | "siren" | "siret" => format!("ID_{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
            "name" | "first_name" | "last_name" => format!("NAME_{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
            "address" => format!("ADDR_{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
            "iban" => format!("IBAN_{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
            "credit_card" => format!("CARD_{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
            _ => format!("ANON_{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
        }
    }
    
    /// Detect file type from path
    fn detect_file_type(file_path: &str) -> crate::FileType {
        let path = Path::new(file_path);
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());
        
        match ext.as_deref() {
            Some("csv") => crate::FileType::Csv,
            Some("xlsx") | Some("xls") | Some("ods") => crate::FileType::Excel,
            Some("pdf") => crate::FileType::Pdf,
            Some("txt") | Some("md") | Some("json") => crate::FileType::Text,
            _ => crate::FileType::Unknown,
        }
    }
    
    /// Get replacement statistics
    pub fn get_replacement_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        
        for log in &self.replacement_log {
            *stats.entry(log.pattern_name.clone()).or_insert(0) += 1;
        }
        
        stats
    }
    
    /// Clear replacement logs
    pub fn clear_logs(&mut self) {
        self.replacement_log.clear();
        self.replacement_map.clear();
    }
}

impl Default for Anonymizer {
    fn default() -> Self {
        Self::new()
    }
}
