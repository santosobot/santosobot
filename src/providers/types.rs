use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
            name: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            name: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            name: None,
            tool_call_id: None,
        }
    }

    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: content.into(),
            name: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Serialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ChatResponse {
    pub id: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Choice {
    pub message: ResponseMessage,
    #[serde(rename = "finish_reason")]
    pub finish_reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum ResponseMessage {
    Simple {
        role: String,
        content: Option<String>,
    },
    WithTools {
        role: String,
        content: Option<String>,
        #[serde(rename = "tool_calls")]
        tool_calls: Option<Vec<ToolCall>>,
    },
}

impl ResponseMessage {
    pub fn content(&self) -> Option<&str> {
        match self {
            ResponseMessage::Simple { content, .. } => content.as_deref(),
            ResponseMessage::WithTools { content, .. } => content.as_deref(),
        }
    }

    pub fn tool_calls(&self) -> Option<&Vec<ToolCall>> {
        match self {
            ResponseMessage::Simple { .. } => None,
            ResponseMessage::WithTools { tool_calls, .. } => tool_calls.as_ref(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Usage {
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: u32,
    #[serde(rename = "completion_tokens")]
    pub completion_tokens: u32,
    #[serde(rename = "total_tokens")]
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    pub arguments: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LLMResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCallRequest>,
    pub finish_reason: String,
    pub usage: Usage,
}

impl LLMResponse {
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

impl From<ChatResponse> for LLMResponse {
    fn from(resp: ChatResponse) -> Self {
        let choice = resp.choices.first();
        let msg = choice.map(|c| &c.message);

        let content = msg.and_then(|m| m.content().map(|s| s.to_string()));

        let tool_calls = msg
            .and_then(|m| m.tool_calls())
            .map(|calls| {
                calls
                    .iter()
                    .map(|tc| {
                        let args: HashMap<String, serde_json::Value> =
                            serde_json::from_str(&tc.function.arguments).unwrap_or_default();
                        ToolCallRequest {
                            id: tc.id.clone(),
                            name: tc.function.name.clone(),
                            arguments: args,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let finish_reason = choice.map(|c| c.finish_reason.clone()).unwrap_or_default();

        Self {
            content,
            tool_calls,
            finish_reason,
            usage: resp.usage,
        }
    }
}
