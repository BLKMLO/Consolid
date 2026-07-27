use reqwest::blocking::Client;
use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE, RETRY_AFTER};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;

// L'analyse est confiée à un agent personnalisé créé dans Mistral Studio :
// le modèle, la température par défaut et les instructions système sont donc
// définis côté agent, pas dans l'application.
const ENDPOINT: &str = "https://api.mistral.ai/v1/conversations";
const MAX_RESPONSE_SIZE: u64 = 10 * 1024 * 1024;
const MAX_ATTEMPTS: usize = 3;
const MAX_OUTPUT_TOKENS: u32 = 32_768;

#[derive(Debug, Error)]
pub enum MistralError {
    #[error("clé API manquante")]
    MissingApiKey,
    #[error("format de clé API invalide")]
    InvalidApiKey,
    #[error("identifiant d’agent manquant")]
    MissingAgentId,
    #[error("identifiant d’agent invalide")]
    InvalidAgentId,
    #[error("opération annulée")]
    Cancelled,
    #[error("échec réseau : {0}")]
    Network(String),
    #[error("authentification Mistral refusée")]
    Authentication,
    #[error("agent introuvable : vérifiez l’identifiant dans Mistral Studio")]
    AgentNotFound,
    #[error("limite de requêtes Mistral atteinte")]
    RateLimited,
    #[error("Mistral a répondu HTTP {status} : {message}")]
    Http { status: u16, message: String },
    #[error("réponse Mistral invalide : {0}")]
    InvalidResponse(String),
    #[error("réponse Mistral trop volumineuse")]
    ResponseTooLarge,
}

/// Requête de conversation adressée à un agent Mistral Studio.
///
/// `store: false` demande explicitement à Mistral de ne pas conserver la
/// conversation côté serveur. `completion_args` ne fixe que le déterminisme et
/// la longueur maximale ; tout le reste provient de la configuration de l'agent.
#[derive(Serialize)]
struct Request<'a> {
    agent_id: &'a str,
    inputs: &'a str,
    stream: bool,
    store: bool,
    completion_args: CompletionArgs,
}

#[derive(Serialize)]
struct CompletionArgs {
    temperature: f32,
    max_tokens: u32,
}

#[derive(Deserialize)]
struct Response {
    #[serde(default)]
    outputs: Vec<Output>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Output {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    content: Option<ResponseContent>,
}

#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    completion_tokens: Option<u32>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ResponseContent {
    Text(String),
    Chunks(Vec<ResponseChunk>),
}

#[derive(Deserialize)]
struct ResponseChunk {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

pub fn audit(
    api_key: &str,
    agent_id: &str,
    prompt: &str,
    cancelled: &AtomicBool,
) -> Result<String, MistralError> {
    audit_with_endpoint(ENDPOINT, api_key, agent_id, prompt, cancelled)
}

fn audit_with_endpoint(
    endpoint: &str,
    api_key: &str,
    agent_id: &str,
    prompt: &str,
    cancelled: &AtomicBool,
) -> Result<String, MistralError> {
    validate_parameters(api_key, agent_id)?;

    let client = Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(180))
        .user_agent(concat!("Consolid/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| MistralError::Network(error.to_string()))?;
    let mut authorization = HeaderValue::from_str(&format!("Bearer {}", api_key.trim()))
        .map_err(|_| MistralError::InvalidApiKey)?;
    authorization.set_sensitive(true);
    let request = Request {
        agent_id,
        inputs: prompt,
        stream: false,
        store: false,
        completion_args: CompletionArgs {
            temperature: 0.0,
            max_tokens: MAX_OUTPUT_TOKENS,
        },
    };

    for attempt in 1..=MAX_ATTEMPTS {
        if cancelled.load(Ordering::Relaxed) {
            return Err(MistralError::Cancelled);
        }
        let response = client
            .post(endpoint)
            .header(AUTHORIZATION, authorization.clone())
            .header(CONTENT_TYPE, "application/json")
            .json(&request)
            .send();
        let response = match response {
            Ok(response) => response,
            Err(error) if attempt < MAX_ATTEMPTS && (error.is_connect() || error.is_timeout()) => {
                cancellable_wait(cancelled, Duration::from_secs(attempt as u64))?;
                continue;
            }
            Err(error) => return Err(MistralError::Network(error.to_string())),
        };

        let status = response.status();
        if is_transient_status(status.as_u16()) && attempt < MAX_ATTEMPTS {
            let delay = retry_delay(&response, attempt);
            drop(response);
            cancellable_wait(cancelled, delay)?;
            continue;
        }
        return parse_response(response);
    }

    Err(MistralError::Network(
        "nombre maximal de tentatives atteint".into(),
    ))
}

pub fn validate_parameters(api_key: &str, agent_id: &str) -> Result<(), MistralError> {
    if api_key.trim().is_empty() {
        return Err(MistralError::MissingApiKey);
    }
    if !valid_api_key(api_key.trim()) {
        return Err(MistralError::InvalidApiKey);
    }
    if agent_id.trim().is_empty() {
        return Err(MistralError::MissingAgentId);
    }
    if !valid_agent_id(agent_id) {
        return Err(MistralError::InvalidAgentId);
    }
    Ok(())
}

fn parse_response(response: reqwest::blocking::Response) -> Result<String, MistralError> {
    let status = response.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(MistralError::Authentication);
    }
    if status.as_u16() == 404 {
        return Err(MistralError::AgentNotFound);
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
    let bytes = read_limited(response, MAX_RESPONSE_SIZE)?;
    if !status.is_success() {
        let message = String::from_utf8_lossy(&bytes);
        return Err(MistralError::Http {
            status: status.as_u16(),
            message: sanitize_message(&message, 500),
        });
    }

    let parsed: Response = serde_json::from_slice(&bytes)
        .map_err(|error| MistralError::InvalidResponse(error.to_string()))?;

    // L'API de conversation ne renvoie pas de « finish_reason » : une génération
    // interrompue par la limite de jetons se reconnaît à la consommation
    // déclarée. Une troncature ferait perdre des jetons obligatoires et serait
    // de toute façon rejetée ensuite, mais le diagnostic doit rester explicite.
    if parsed
        .usage
        .as_ref()
        .and_then(|usage| usage.completion_tokens)
        .is_some_and(|used| used >= MAX_OUTPUT_TOKENS)
    {
        return Err(MistralError::InvalidResponse(
            "génération incomplète (limite de jetons de sortie atteinte)".into(),
        ));
    }

    // Un agent peut produire plusieurs entrées (appels d'outils, étapes
    // intermédiaires) ; seule la dernière réponse de l'assistant est retenue.
    parsed
        .outputs
        .into_iter()
        .filter(|output| output.role.as_deref() == Some("assistant"))
        .filter_map(|output| output.content.map(content_to_text))
        .rfind(|text| !text.trim().is_empty())
        .ok_or_else(|| MistralError::InvalidResponse("aucune réponse exploitable".into()))
}

fn content_to_text(content: ResponseContent) -> String {
    match content {
        ResponseContent::Text(text) => text,
        ResponseContent::Chunks(chunks) => chunks
            .into_iter()
            .filter(|chunk| chunk.kind == "text")
            .filter_map(|chunk| chunk.text)
            .collect::<Vec<_>>()
            .join(""),
    }
}

fn read_limited(response: impl Read, maximum: u64) -> Result<Vec<u8>, MistralError> {
    let mut bytes = Vec::new();
    response
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| MistralError::Network(error.to_string()))?;
    if bytes.len() as u64 > maximum {
        return Err(MistralError::ResponseTooLarge);
    }
    Ok(bytes)
}

fn valid_api_key(api_key: &str) -> bool {
    (8..=512).contains(&api_key.len()) && api_key.bytes().all(|value| value.is_ascii_graphic())
}

/// Les identifiants d'agent Mistral existent sous deux graphies
/// (`ag_<hexadécimal>` et `ag:<version>:<nom>:<révision>`). Le contrôle reste
/// volontairement générique et ne vise que l'injection de caractères.
fn valid_agent_id(agent_id: &str) -> bool {
    let trimmed = agent_id.trim();
    (3..=128).contains(&trimmed.len())
        && trimmed.bytes().all(|value| {
            value.is_ascii_alphanumeric() || matches!(value, b'-' | b'_' | b'.' | b':')
        })
}

fn is_transient_status(status: u16) -> bool {
    matches!(status, 429 | 502 | 503 | 504)
}

fn retry_delay(response: &reqwest::blocking::Response, attempt: usize) -> Duration {
    response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| Duration::from_secs(seconds.clamp(1, 15)))
        .unwrap_or_else(|| Duration::from_secs(attempt as u64))
}

fn cancellable_wait(cancelled: &AtomicBool, delay: Duration) -> Result<(), MistralError> {
    let deadline = Instant::now() + delay;
    while Instant::now() < deadline {
        if cancelled.load(Ordering::Relaxed) {
            return Err(MistralError::Cancelled);
        }
        std::thread::sleep(
            Duration::from_millis(100).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
    Ok(())
}

fn sanitize_message(value: &str, maximum: usize) -> String {
    value
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
        .take(maximum)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    const TEST_AGENT: &str = "ag_06827f9dd6ac7b8a80009d3f966b21eb";

    #[test]
    fn agent_id_validation_blocks_injected_values() {
        assert!(valid_agent_id(TEST_AGENT));
        assert!(valid_agent_id(
            "ag:3916a8a9:20260101:consolidation:1e4d2f5c"
        ));
        assert!(!valid_agent_id("https://evil.invalid"));
        assert!(!valid_agent_id("ag_123\nheader"));
        assert!(!valid_agent_id("ag"));
    }

    #[test]
    fn parameter_validation_reports_the_missing_agent() {
        assert!(matches!(
            validate_parameters("test-key-123456", "   "),
            Err(MistralError::MissingAgentId)
        ));
        assert!(matches!(
            validate_parameters("test-key-123456", "agent invalide"),
            Err(MistralError::InvalidAgentId)
        ));
        assert!(validate_parameters("test-key-123456", TEST_AGENT).is_ok());
    }

    #[test]
    fn api_key_validation_blocks_whitespace_and_control_characters() {
        assert!(valid_api_key("abcdefgh-12345678"));
        assert!(!valid_api_key("abc"));
        assert!(!valid_api_key("abcdefgh\n12345678"));
        assert!(!valid_api_key("abcdefgh 12345678"));
    }

    #[test]
    fn response_reader_enforces_limit_while_streaming() {
        let input = std::io::Cursor::new(vec![0_u8; 11]);
        assert!(matches!(
            read_limited(input, 10),
            Err(MistralError::ResponseTooLarge)
        ));
    }

    #[test]
    fn response_content_accepts_text_and_chunk_lists() {
        let text: ResponseContent = serde_json::from_str(r#""résultat""#).unwrap();
        assert_eq!(content_to_text(text), "résultat");

        let chunks: ResponseContent =
            serde_json::from_str(r#"[{"type":"text","text":"un"},{"type":"text","text":" deux"}]"#)
                .unwrap();
        assert_eq!(content_to_text(chunks), "un deux");
    }

    /// Serveur HTTP minimaliste sur 127.0.0.1 : lit une requête complète, renvoie
    /// la réponse fournie, puis rend la requête capturée au test pour inspection.
    fn spawn_mock_server(
        status: u16,
        body: &'static str,
    ) -> (String, std::thread::JoinHandle<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        let url = format!("http://{}", listener.local_addr().expect("adresse locale"));
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut received = Vec::new();
            let mut buffer = [0_u8; 8192];
            let mut header_end = None;
            while header_end.is_none() {
                let read = stream.read(&mut buffer).expect("lecture en-têtes");
                assert!(read > 0, "connexion fermée avant la fin des en-têtes");
                received.extend_from_slice(&buffer[..read]);
                header_end = received
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                    .map(|position| position + 4);
            }
            let header_end = header_end.expect("fin d'en-têtes détectée");
            let head = String::from_utf8_lossy(&received[..header_end]).into_owned();
            let content_length: usize = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    if name.trim().eq_ignore_ascii_case("content-length") {
                        value.trim().parse().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            while received.len() < header_end + content_length {
                let read = stream.read(&mut buffer).expect("lecture corps");
                if read == 0 {
                    break;
                }
                received.extend_from_slice(&buffer[..read]);
            }
            let reason = if status == 200 { "OK" } else { "Error" };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("écriture réponse");
            String::from_utf8_lossy(&received).into_owned()
        });
        (url, handle)
    }

    fn call_mock(url: &str) -> Result<String, MistralError> {
        let cancelled = AtomicBool::new(false);
        audit_with_endpoint(
            url,
            "test-key-123456",
            TEST_AGENT,
            "prompt de test",
            &cancelled,
        )
    }

    #[test]
    fn audit_sends_authorized_agent_request_and_parses_response() {
        let body = r#"{"conversation_id":"conv_1","outputs":[{"type":"message.output","role":"assistant","content":"résultat corrigé"}],"usage":{"completion_tokens":128}}"#;
        let (url, server) = spawn_mock_server(200, body);
        assert_eq!(
            call_mock(&url).expect("réponse mock valide"),
            "résultat corrigé"
        );

        let request = server.join().expect("thread serveur").to_lowercase();
        assert!(
            request.contains("authorization: bearer test-key-123456"),
            "en-tête d'authentification absent : {request}"
        );
        assert!(
            request.contains(&format!("\"agent_id\":\"{TEST_AGENT}\"")),
            "agent absent du corps : {request}"
        );
        assert!(
            request.contains("\"store\":false"),
            "la conversation doit être demandée sans conservation : {request}"
        );
        assert!(
            request.contains("prompt de test"),
            "prompt absent du corps : {request}"
        );
        assert!(
            !request.contains("\"model\""),
            "le modèle est défini par l'agent, pas par l'application : {request}"
        );
    }

    #[test]
    fn audit_keeps_the_last_assistant_output_and_ignores_tool_steps() {
        let body = r#"{"conversation_id":"conv_1","outputs":[
            {"type":"tool.execution","name":"code_interpreter"},
            {"type":"message.output","role":"assistant","content":[{"type":"text","text":"partie un "},{"type":"text","text":"et deux"}]}
        ]}"#;
        let (url, server) = spawn_mock_server(200, body);
        assert_eq!(
            call_mock(&url).expect("réponse mock valide"),
            "partie un et deux"
        );
        let _ = server.join();
    }

    #[test]
    fn audit_maps_401_to_authentication_error() {
        let (url, server) = spawn_mock_server(401, r#"{"error":"unauthorized"}"#);
        assert!(matches!(call_mock(&url), Err(MistralError::Authentication)));
        let _ = server.join();
    }

    #[test]
    fn audit_maps_403_to_authentication_error() {
        let (url, server) = spawn_mock_server(403, r#"{"error":"forbidden"}"#);
        assert!(matches!(call_mock(&url), Err(MistralError::Authentication)));
        let _ = server.join();
    }

    #[test]
    fn audit_maps_404_to_a_missing_agent() {
        let (url, server) = spawn_mock_server(404, r#"{"error":"agent not found"}"#);
        assert!(matches!(call_mock(&url), Err(MistralError::AgentNotFound)));
        let _ = server.join();
    }

    #[test]
    fn audit_rejects_incomplete_generation() {
        let body = r#"{"conversation_id":"conv_1","outputs":[{"type":"message.output","role":"assistant","content":"tronqué"}],"usage":{"completion_tokens":32768}}"#;
        let (url, server) = spawn_mock_server(200, body);
        assert!(matches!(
            call_mock(&url),
            Err(MistralError::InvalidResponse(_))
        ));
        let _ = server.join();
    }

    #[test]
    fn audit_rejects_a_response_without_assistant_output() {
        let body = r#"{"conversation_id":"conv_1","outputs":[{"type":"tool.execution","name":"web_search"}]}"#;
        let (url, server) = spawn_mock_server(200, body);
        assert!(matches!(
            call_mock(&url),
            Err(MistralError::InvalidResponse(_))
        ));
        let _ = server.join();
    }
}
