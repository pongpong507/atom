use crate::error::{AppError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: "system".into(), content: content.into() }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".into(), content: content.into() }
    }
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    content: String,
}

#[derive(Debug, Clone)]
pub struct LlmClient {
    client: Client,
    base_url: String,
    pub model: String,
}

impl LlmClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
        }
    }

    pub async fn chat(&self, messages: Vec<Message>) -> Result<String> {
        let url = format!("{}/api/chat", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false
        });

        let mut last_err = None;
        for attempt in 1..=3 {
            match self.client.post(&url).json(&body).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    if !status.is_success() {
                        let err = AppError::Llm(format!("Ollama HTTP {status}: {text}"));
                        last_err = Some(err);
                        tokio::time::sleep(Duration::from_secs(attempt)).await;
                        continue;
                    }
                    let parsed: OllamaResponse = serde_json::from_str(&text)
                        .map_err(|e| AppError::Llm(format!("Parse Ollama response: {e}")))?;
                    return Ok(parsed.message.content);
                }
                Err(e) => {
                    tracing::warn!("Ollama request attempt {attempt} failed: {e}");
                    last_err = Some(AppError::Llm(e.to_string()));
                    tokio::time::sleep(Duration::from_secs(attempt)).await;
                }
            }
        }
        Err(last_err.unwrap_or_else(|| AppError::Llm("Unknown LLM error".into())))
    }
}
