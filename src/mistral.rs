use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

const ENDPOINT: &str = "https://api.mistral.ai/v1/chat/completions";
const MAX_RESPONSE_SIZE: u64 = 10 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum MistralError {
    #[error("clé API manquante")]
    MissingApiKey,
    #[error("modèle invalide")]
    InvalidModel,
    #[error("échec réseau : {0}")]
    Network(String),
    #[error("authentification Mistral refusée")]
    Authentication,
    #[error("limite de requêtes Mistral atteinte")]
    RateLimited,
    #[error("Mistral a répondu HTTP {status} : {message}")]
    Http { status: u16, message: String },
    #[error("réponse Mistral invalide : {0}")]
    InvalidResponse(String),
    #[error("réponse Mistral trop volumineuse")]
    ResponseTooLarge,
}

#[derive(Serialize)]
struct Request<'a> {
    model: &'a str,
    temperature: f32,
    max_tokens: u32,
    messages: [Message<'a>; 2],
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct Response {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

pub fn audit(api_key: &str, model: &str, prompt: &str) -> Result<String, MistralError> {
    if api_key.trim().is_empty() {
        return Err(MistralError::MissingApiKey);
    }
    if !valid_model(model) {
        return Err(MistralError::InvalidModel);
    }

    let client = Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(180))
        .user_agent(concat!("Consolid/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| MistralError::Network(error.to_string()))?;
    let request = Request {
        model,
        temperature: 0.0,
        max_tokens: 8192,
        messages: [
            Message {
                role: "system",
                content: "Vous auditez une consolidation. Respectez strictement les jetons [[CONSOLID_*]] : ne les modifiez jamais. Retournez uniquement le document consolidé corrigé, sans commentaire périphérique.",
            },
            Message {
                role: "user",
                content: prompt,
            },
        ],
    };

    let response = client
        .post(ENDPOINT)
        .header(AUTHORIZATION, format!("Bearer {}", api_key.trim()))
        .header(CONTENT_TYPE, "application/json")
        .json(&request)
        .send()
        .map_err(|error| MistralError::Network(error.to_string()))?;
    let status = response.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(MistralError::Authentication);
    }
    if status.as_u16() == 429 {
        return Err(MistralError::RateLimited);
    }
    if response
        .content_length()
        .is_some_and(|size| size > MAX_RESPONSE_SIZE)
    {
        return Err(MistralError::ResponseTooLarge);
    }
    let bytes = response
        .bytes()
        .map_err(|error| MistralError::Network(error.to_string()))?;
    if bytes.len() as u64 > MAX_RESPONSE_SIZE {
        return Err(MistralError::ResponseTooLarge);
    }
    if !status.is_success() {
        let message = String::from_utf8_lossy(&bytes);
        return Err(MistralError::Http {
            status: status.as_u16(),
            message: truncate(&message, 500),
        });
    }

    let parsed: Response = serde_json::from_slice(&bytes)
        .map_err(|error| MistralError::InvalidResponse(error.to_string()))?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .filter(|content| !content.trim().is_empty())
        .ok_or_else(|| MistralError::InvalidResponse("contenu vide".into()))
}

fn valid_model(model: &str) -> bool {
    let trimmed = model.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 100
        && trimmed
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'-' | b'_' | b'.'))
}

fn truncate(value: &str, maximum: usize) -> String {
    value.chars().take(maximum).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_validation_blocks_injected_values() {
        assert!(valid_model("mistral-small-latest"));
        assert!(!valid_model("https://evil.invalid"));
        assert!(!valid_model("model\nheader"));
    }
}
