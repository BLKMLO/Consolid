use tauri::Manager;

mod anonymizer;
mod validator;
mod api;
mod file_handler;

use anonymizer::Anonymizer;
use validator::FileValidator;
use api::mistral_client::MistralClient;
use file_handler::FileProcessor;
use std::sync::Arc;
use tokio::sync::Mutex;

// Shared state for the application
#[derive(Clone)]
struct AppState {
    anonymizer: Arc<Mutex<Anonymizer>>,
    validator: Arc<Mutex<FileValidator>>,
    mistral_client: Arc<Mutex<MistralClient>>,
    file_processor: Arc<Mutex<FileProcessor>>,
}

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("consolid_audit=debug,tauri=info")
        .init();

    // Create shared state
    let state = AppState {
        anonymizer: Arc::new(Mutex::new(Anonymizer::new())),
        validator: Arc::new(Mutex::new(FileValidator::new())),
        mistral_client: Arc::new(Mutex::new(MistralClient::new())),
        file_processor: Arc::new(Mutex::new(FileProcessor::new())),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            handle_file_drop,
            process_files,
            validate_files,
            anonymize_files,
            send_to_mistral,
            get_app_status,
            set_api_key,
            test_api_connection
        ])
        .setup(|app| {
            // Create icons directory if it doesn't exist
            let _ = std::fs::create_dir_all(app.path().app_data_dir().unwrap().join("icons"));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Command handlers

#[tauri::command]
async fn handle_file_drop(
    state: tauri::State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Vec<FileInfo>, String> {
    let file_processor = state.file_processor.lock().await;
    file_processor.handle_file_drop(paths).await
}

#[tauri::command]
async fn process_files(
    state: tauri::State<'_, AppState>,
    file_paths: Vec<String>,
    template_path: String,
) -> Result<ProcessResult, String> {
    let mut file_processor = state.file_processor.lock().await;
    let mut anonymizer = state.anonymizer.lock().await;
    let mut validator = state.validator.lock().await;
    
    file_processor.process_files(file_paths, template_path, &mut anonymizer, &mut validator).await
}

#[tauri::command]
async fn validate_files(
    state: tauri::State<'_, AppState>,
    file_paths: Vec<String>,
) -> Result<ValidationResult, String> {
    let mut validator = state.validator.lock().await;
    validator.validate_files(file_paths).await
}

#[tauri::command]
async fn anonymize_files(
    state: tauri::State<'_, AppState>,
    file_paths: Vec<String>,
) -> Result<Vec<AnonymizedFile>, String> {
    let mut anonymizer = state.anonymizer.lock().await;
    anonymizer.anonymize_files(file_paths).await
}

#[tauri::command]
async fn send_to_mistral(
    state: tauri::State<'_, AppState>,
    anonymized_files: Vec<AnonymizedFile>,
    template_content: String,
    api_key: Option<String>,
) -> Result<MistralResponse, String> {
    let mut client = state.mistral_client.lock().await;
    
    // Set API key if provided
    if let Some(key) = api_key {
        client.set_api_key(key);
    }
    
    client.send_to_mistral(anonymized_files, template_content).await
}

#[tauri::command]
async fn get_app_status(
    state: tauri::State<'_, AppState>,
) -> Result<AppStatus, String> {
    let file_processor = state.file_processor.lock().await;
    let validator = state.validator.lock().await;
    let client = state.mistral_client.lock().await;
    
    Ok(AppStatus {
        files_loaded: file_processor.get_file_count(),
        validation_status: validator.get_status(),
        api_connected: client.is_connected(),
        ready_to_send: file_processor.get_file_count() > 0 
            && validator.get_status() == ValidationStatus::Valid
            && client.is_connected(),
    })
}

#[tauri::command]
async fn set_api_key(
    state: tauri::State<'_, AppState>,
    api_key: String,
) -> Result<bool, String> {
    let mut client = state.mistral_client.lock().await;
    client.set_api_key(api_key);
    Ok(client.test_connection().await.is_ok())
}

#[tauri::command]
async fn test_api_connection(
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    let client = state.mistral_client.lock().await;
    Ok(client.test_connection().await.is_ok())
}

// Data structures for communication with frontend

#[derive(serde::Serialize, Clone)]
struct FileInfo {
    path: String,
    name: String,
    size: u64,
    file_type: FileType,
    is_valid: bool,
    error: Option<String>,
}

#[derive(serde::Serialize, Clone)]
enum FileType {
    Csv,
    Excel,
    Pdf,
    Text,
    Unknown,
}

#[derive(serde::Serialize, Clone)]
struct ProcessResult {
    success: bool,
    message: String,
    processed_files: Vec<FileInfo>,
    filled_template: Option<String>,
    errors: Vec<String>,
}

#[derive(serde::Serialize, Clone)]
struct ValidationResult {
    is_valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
    file_results: Vec<FileValidationResult>,
}

#[derive(serde::Serialize, Clone)]
struct FileValidationResult {
    file_path: String,
    is_valid: bool,
    errors: Vec<String>,
}

#[derive(serde::Serialize, Clone)]
struct AnonymizedFile {
    original_name: String,
    anonymized_content: String,
    file_type: FileType,
    metadata: serde_json::Value,
}

#[derive(serde::Serialize, Clone)]
struct MistralResponse {
    success: bool,
    message: String,
    filled_template: Option<String>,
    analysis: Option<String>,
    tokens_used: u32,
}

#[derive(serde::Serialize, Clone)]
struct AppStatus {
    files_loaded: usize,
    validation_status: ValidationStatus,
    api_connected: bool,
    ready_to_send: bool,
}

#[derive(serde::Serialize, Clone)]
enum ValidationStatus {
    NotStarted,
    InProgress,
    Valid,
    Invalid,
}

impl Default for ValidationStatus {
    fn default() -> Self {
        ValidationStatus::NotStarted
    }
}

// Implement Default for FileType
impl Default for FileType {
    fn default() -> Self {
        FileType::Unknown
    }
}

// Implement Default for FileInfo
impl Default for FileInfo {
    fn default() -> Self {
        FileInfo {
            path: String::new(),
            name: String::new(),
            size: 0,
            file_type: FileType::Unknown,
            is_valid: false,
            error: None,
        }
    }
}
