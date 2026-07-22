//! Mistral API client for sending anonymized data and receiving analysis

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use crate::AnonymizedFile;

/// Error type for Mistral API operations
#[derive(Error, Debug)]
pub enum MistralError {
    #[error("API key not set")]
    ApiKeyNotSet,
    
    #[error("API request failed: {0}")]
    RequestError(String),
    
    #[error("API returned error: {0}")]
    ApiError(String),
    
    #[error("Invalid response format")]
    InvalidResponse,
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    #[error("Timeout: {0}")]
    Timeout(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
}

/// Configuration for Mistral API
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MistralConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub endpoint: String,
    pub timeout: u64,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl Default for MistralConfig {
    fn default() -> Self {
        MistralConfig {
            api_key: None,
            model: "mistral-tiny".to_string(),
            endpoint: "https://api.mistral.ai/v1/chat/completions".to_string(),
            timeout: 60,
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

/// Request payload for Mistral API
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MistralRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub stream: bool,
}

impl Default for MistralRequest {
    fn default() -> Self {
        MistralRequest {
            model: "mistral-tiny".to_string(),
            messages: Vec::new(),
            temperature: 0.7,
            max_tokens: 4096,
            stream: false,
        }
    }
}

/// Chat message structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn new(role: &str, content: &str) -> Self {
        ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
        }
    }
    
    pub fn system(content: &str) -> Self {
        ChatMessage::new("system", content)
    }
    
    pub fn user(content: &str) -> Self {
        ChatMessage::new("user", content)
    }
    
    pub fn assistant(content: &str) -> Self {
        ChatMessage::new("assistant", content)
    }
}

/// Response from Mistral API
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MistralResponse {
    pub id: Option<String>,
    pub object: Option<String>,
    pub created: Option<u64>,
    pub model: Option<String>,
    pub choices: Option<Vec<Choice>>,
    pub usage: Option<Usage>,
    pub error: Option<ApiError>,
}

/// Choice in Mistral response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: Option<u32>,
    pub message: Option<ChatMessage>,
    pub finish_reason: Option<String>,
}

/// Usage information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// API error response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
    pub r#type: String,
    pub param: Option<String>,
    pub code: Option<String>,
}

/// Main Mistral client
pub struct MistralClient {
    client: Client,
    config: MistralConfig,
    is_connected: bool,
}

impl MistralClient {
    /// Create a new MistralClient instance
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");
        
        MistralClient {
            client,
            config: MistralConfig::default(),
            is_connected: false,
        }
    }
    
    /// Set API key
    pub fn set_api_key(&mut self, api_key: String) {
        self.config.api_key = Some(api_key);
        self.is_connected = false; // Reset connection status
    }
    
    /// Test connection to Mistral API
    pub async fn test_connection(&self) -> Result<bool, MistralError> {
        if self.config.api_key.is_none() {
            return Err(MistralError::ApiKeyNotSet);
        }
        
        // Make a simple request to test connection
        let request = MistralRequest {
            model: self.config.model.clone(),
            messages: vec![
                ChatMessage::system("You are a helpful assistant."),
                ChatMessage::user("Say 'OK' if you're working."),
            ],
            temperature: 0.0,
            max_tokens: 10,
            stream: false,
        };
        
        match self.send_request(&request).await {
            Ok(_) => Ok(true),
            Err(MistralError::AuthenticationFailed) => Err(MistralError::AuthenticationFailed),
            Err(_) => Ok(false), // Other errors might be temporary
        }
    }
    
    /// Send anonymized files to Mistral for analysis
    pub async fn send_to_mistral(
        &self,
        anonymized_files: Vec<AnonymizedFile>,
        template_content: String,
    ) -> Result<crate::MistralResponse, MistralError> {
        if self.config.api_key.is_none() {
            return Err(MistralError::ApiKeyNotSet);
        }
        
        // Build the prompt
        let prompt = self.build_prompt(anonymized_files, template_content)?;
        
        // Create request
        let request = MistralRequest {
            model: self.config.model.clone(),
            messages: vec![
                ChatMessage::system(self.get_system_prompt()),
                ChatMessage::user(&prompt),
            ],
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: false,
        };
        
        // Send request
        let response = self.send_request(&request).await?;
        
        // Process response
        self.process_response(response)
    }
    
    /// Build the prompt from anonymized files and template
    fn build_prompt(
        &self,
        anonymized_files: Vec<AnonymizedFile>,
        template_content: String,
    ) -> Result<String, MistralError> {
        let mut prompt = String::new();
        
        prompt.push_str("Analyse the following anonymized accounting data and fill in the template.\n\n");
        
        // Add each file content
        for (i, file) in anonymized_files.iter().enumerate() {
            prompt.push_str(&format!("=== File {}: {} ===\n", i + 1, file.original_name));
            prompt.push_str(&file.anonymized_content);
            prompt.push_str("\n\n");
        }
        
        prompt.push_str("\n=== Template to fill ===\n");
        prompt.push_str(&template_content);
        
        prompt.push_str("\n\nInstructions:\n");
        prompt.push_str("- Analyze the accounting data\n");
        prompt.push_str("- Fill in the template with the appropriate values\n");
        prompt.push_str("- Ensure all calculations are correct\n");
        prompt.push_str("- Return only the filled template, without any additional text\n");
        
        Ok(prompt)
    }
    
    /// Get system prompt for accounting analysis
    fn get_system_prompt(&self) -> String {
        "You are an expert accounting auditor. Your task is to:
1. Analyze the provided accounting data
2. Verify the consistency and correctness of the data
3. Fill in the provided template with accurate values
4. Ensure all calculations follow accounting standards
5. Return only the filled template without any additional explanations or text.

You must:
- Be precise and accurate
- Follow accounting principles
- Handle all data confidentially
- Return the template exactly as requested".to_string()
    }
    
    /// Send request to Mistral API
    async fn send_request(&self, request: &MistralRequest) -> Result<MistralResponse, MistralError> {
        let api_key = self.config.api_key.as_ref()
            .ok_or(MistralError::ApiKeyNotSet)?;
        
        let client = Client::builder()
            .timeout(Duration::from_secs(self.config.timeout))
            .build()
            .map_err(|e| MistralError::NetworkError(e.to_string()))?;
        
        let response = client
            .post(&self.config.endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("timeout") {
                    MistralError::Timeout(e.to_string())
                } else if e.to_string().contains("429") {
                    MistralError::RateLimitExceeded
                } else {
                    MistralError::RequestError(e.to_string())
                }
            })?;
        
        // Check status code
        let status = response.status();
        
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(MistralError::AuthenticationFailed);
        }
        
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(MistralError::RateLimitExceeded);
        }
        
        if !status.is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(MistralError::ApiError(error_text));
        }
        
        // Parse response
        let response: MistralResponse = response.json()
            .await
            .map_err(|e| MistralError::InvalidResponse)?;
        
        // Check for API errors
        if let Some(error) = response.error {
            if error.r#type == "BadRequestError" {
                return Err(MistralError::ApiError(error.message));
            }
            return Err(MistralError::ApiError(error.message));
        }
        
        Ok(response)
    }
    
    /// Process Mistral response
    fn process_response(&self, response: MistralResponse) -> Result<crate::MistralResponse, MistralError> {
        let choices = response.choices.ok_or(MistralError::InvalidResponse)?;
        
        if choices.is_empty() {
            return Err(MistralError::InvalidResponse);
        }
        
        let first_choice = &choices[0];
        let message = first_choice.message.as_ref()
            .ok_or(MistralError::InvalidResponse)?;
        
        let usage = response.usage.unwrap_or(Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        });
        
        Ok(crate::MistralResponse {
            success: true,
            message: message.content.clone(),
            filled_template: Some(message.content.clone()),
            analysis: None,
            tokens_used: usage.total_tokens,
        })
    }
    
    /// Check if client is connected
    pub fn is_connected(&self) -> bool {
        self.is_connected
    }
    
    /// Set connection status
    pub fn set_connected(&mut self, connected: bool) {
        self.is_connected = connected;
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> &MistralConfig {
        &self.config
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: MistralConfig) {
        self.config = config;
    }
}

impl Default for MistralClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chat_message_creation() {
        let system = ChatMessage::system("Test");
        assert_eq!(system.role, "system");
        assert_eq!(system.content, "Test");
        
        let user = ChatMessage::user("Hello");
        assert_eq!(user.role, "user");
        assert_eq!(user.content, "Hello");
    }
    
    #[test]
    fn test_mistral_request_default() {
        let request = MistralRequest::default();
        assert_eq!(request.model, "mistral-tiny");
        assert_eq!(request.temperature, 0.7);
        assert_eq!(request.max_tokens, 4096);
        assert!(!request.stream);
    }
    
    #[test]
    fn test_mistral_config_default() {
        let config = MistralConfig::default();
        assert_eq!(config.model, "mistral-tiny");
        assert_eq!(config.endpoint, "https://api.mistral.ai/v1/chat/completions");
        assert_eq!(config.timeout, 60);
    }
}
