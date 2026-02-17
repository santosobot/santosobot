mod types;

pub use types::*;

use crate::config::ProviderConfig;
use reqwest::Client;
use tracing::{info, error};
use futures::stream::{StreamExt, BoxStream};

pub struct OpenAIProvider {
    client: Client,
    config: ProviderConfig,
}

impl OpenAIProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let client = Client::builder()
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    #[allow(dead_code)]
    pub async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<Vec<ToolDefinition>>,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<LLMResponse, Box<dyn std::error::Error + Send + Sync>> {
        let model = model.unwrap_or_else(|| self.config.model.clone());

        let request = ChatRequest {
            model: model.clone(),
            messages,
            tools,
            temperature,
            max_tokens,
        };

        info!(model = %model, "Sending chat request");
        tracing::debug!("Request payload: {:#?}", request);

        let url = format!("{}/chat/completions", self.config.api_base.trim_end_matches('/'));

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!(status = %status, body = %body, "LLM request failed");
            return Err(format!("LLM API error: {} - {}", status, body).into());
        }

        let chat_resp: ChatResponse = response.json().await?;
        tracing::debug!("Response from LLM: {:#?}", chat_resp);
        Ok(chat_resp.into())
    }

    #[allow(dead_code)]
    pub async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<Vec<ToolDefinition>>,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<BoxStream<'static, Result<String, Box<dyn std::error::Error + Send + Sync>>>, Box<dyn std::error::Error + Send + Sync>> {
        let model = model.unwrap_or_else(|| self.config.model.clone());

        let request = ChatRequest {
            model: model.clone(),
            messages,
            tools,
            temperature,
            max_tokens,
        };

        info!(model = %model, "Sending streaming chat request");
        tracing::debug!("Request payload: {:#?}", request);

        let url = format!("{}/chat/completions", self.config.api_base.trim_end_matches('/'));

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": request.model,
                "messages": request.messages,
                "tools": request.tools,
                "temperature": request.temperature,
                "max_tokens": request.max_tokens,
                "stream": true,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!(status = %status, body = %body, "LLM stream request failed");
            return Err(format!("LLM API error: {} - {}", status, body).into());
        }

        let stream = response.bytes_stream()
            .filter_map(|chunk_result| async move {
                let bytes = match chunk_result {
                    Ok(b) => b,
                    Err(e) => return Some(Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)),
                };

                let text = String::from_utf8_lossy(&bytes);
                // Parse SSE data lines
                for line in text.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            return None;
                        }
                        if let Ok(stream_resp) = serde_json::from_str::<StreamResponse>(data) {
                            if let Some(choice) = stream_resp.choices.first() {
                                if let Some(ref content) = choice.delta.content {
                                    if !content.is_empty() {
                                        return Some(Ok(content.clone()));
                                    }
                                }
                            }
                        }
                    }
                }
                None
            })
            .boxed();

        Ok(stream)
    }
}
