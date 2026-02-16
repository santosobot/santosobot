use async_trait::async_trait;
use serde_json::{json, Value};
use crate::agent::tools::Tool;
use crate::bus::OutboundMessage;

pub struct MessageTool {
    sender: Option<tokio::sync::mpsc::Sender<OutboundMessage>>,
    channel: Option<String>,
    chat_id: Option<String>,
}

impl MessageTool {
    pub fn new() -> Self {
        Self {
            sender: None,
            channel: None,
            chat_id: None,
        }
    }

    pub fn set_sender(&mut self, sender: tokio::sync::mpsc::Sender<OutboundMessage>) {
        self.sender = Some(sender);
    }

    pub fn set_context(&mut self, channel: String, chat_id: String) {
        self.channel = Some(channel);
        self.chat_id = Some(chat_id);
    }
}

#[async_trait]
impl Tool for MessageTool {
    fn name(&self) -> &str { "message" }
    
    fn description(&self) -> &str {
        "Send a message to a user on a chat channel"
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "Message content to send"
                },
                "channel": {
                    "type": "string",
                    "description": "Channel to send to (telegram, cli, etc.)"
                },
                "chat_id": {
                    "type": "string",
                    "description": "Chat/user ID to send to"
                }
            },
            "required": ["content"]
        })
    }
    
    async fn execute(&self, args: Value) -> Result<String, String> {
        let content = args["content"]
            .as_str()
            .ok_or("Missing content parameter")?;

        let channel = args["channel"]
            .as_str()
            .or(self.chat_id.as_deref())
            .ok_or("Missing channel parameter")?;

        let chat_id = args["chat_id"]
            .as_str()
            .or(self.chat_id.as_deref())
            .unwrap_or("default")
            .to_string();

        if let Some(ref sender) = self.sender {
            let msg = OutboundMessage::new(channel.to_string(), chat_id, content.to_string());
            sender.send(msg).await
                .map_err(|e| format!("Failed to send message: {}", e))?;
            Ok("Message sent".to_string())
        } else {
            Err("Message sender not configured".to_string())
        }
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Default for MessageTool {
    fn default() -> Self {
        Self::new()
    }
}
