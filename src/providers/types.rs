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

#[derive(Debug, Deserialize)]
pub struct StreamChoice {
    pub delta: StreamDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StreamDelta {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StreamResponse {
    pub id: String,
    pub choices: Vec<StreamChoice>,
}

impl LLMResponse {
    pub fn _has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

impl From<ChatResponse> for LLMResponse {
    fn from(resp: ChatResponse) -> Self {
        let choice = resp.choices.first();
        let msg = choice.map(|c| &c.message);

        let content = msg.and_then(|m| m.content().map(|s| s.to_string()));

        // Handle the case where finish_reason is "tool_calls" but no actual tool_calls are present in the message
        let tool_calls = if let Some(choice) = choice {
            if choice.finish_reason == "tool_calls" {
                // If finish_reason is "tool_calls" but no tool_calls in message, 
                // we might need to infer or handle this case specially
                // For now, we'll check if there are actual tool_calls in the message
                msg
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
                    .unwrap_or_default()
            } else {
                // Normal case - not a tool_calls finish reason
                msg
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
                    .unwrap_or_default()
            }
        } else {
            vec![]
        };

        let finish_reason = choice.map(|c| c.finish_reason.clone()).unwrap_or_default();

        Self {
            content,
            tool_calls,
            finish_reason,
            usage: resp.usage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_creation() {
        let system_msg = ChatMessage::system("System message");
        assert_eq!(system_msg.role, "system");
        assert_eq!(system_msg.content, "System message");

        let user_msg = ChatMessage::user("User message");
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content, "User message");

        let assistant_msg = ChatMessage::assistant("Assistant message");
        assert_eq!(assistant_msg.role, "assistant");
        assert_eq!(assistant_msg.content, "Assistant message");

        let tool_msg = ChatMessage::tool("Tool result", "call_123");
        assert_eq!(tool_msg.role, "tool");
        assert_eq!(tool_msg.content, "Tool result");
        assert_eq!(tool_msg.tool_call_id, Some("call_123".to_string()));
    }

    #[test]
    fn test_llm_response_has_tool_calls() {
        let mut response = LLMResponse {
            content: Some("Hello".to_string()),
            tool_calls: vec![],
            finish_reason: "stop".to_string(),
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
        };

        assert!(!response.has_tool_calls());

        response.tool_calls.push(ToolCallRequest {
            id: "call_123".to_string(),
            name: "test_tool".to_string(),
            arguments: HashMap::new(),
        });

        assert!(response.has_tool_calls());
    }

    #[test]
    fn test_usage_struct() {
        let usage = Usage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };

        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
    }
}
