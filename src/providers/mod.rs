mod types;

pub use types::*;

use crate::config::ProviderConfig;
use reqwest::Client;
use tracing::{info, error};

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

        let url = format!("{}/chat/completions", self.config.api_base.trim_end_matches('/'));
        
        info!(model = %model, "Sending chat request");

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
        Ok(chat_resp.into())
    }
}
