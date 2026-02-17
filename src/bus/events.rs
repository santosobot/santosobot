use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct InboundMessage {
    pub channel: String,
    pub sender_id: String,
    pub chat_id: String,
    pub content: String,
    pub media: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl InboundMessage {
    pub fn new(channel: String, sender_id: String, chat_id: String, content: String) -> Self {
        Self {
            channel,
            sender_id,
            chat_id,
            content,
            media: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_media(mut self, media: Vec<String>) -> Self {
        self.media = media;
        self
    }

    #[allow(dead_code)]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct OutboundMessage {
    pub channel: String,
    pub chat_id: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub message_id: Option<i64>,
    pub is_streaming: bool,
}

impl OutboundMessage {
    pub fn new(channel: String, chat_id: String, content: String) -> Self {
        Self {
            channel,
            chat_id,
            content,
            metadata: HashMap::new(),
            message_id: None,
            is_streaming: false,
        }
    }

    #[allow(dead_code)]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    #[allow(dead_code)]
    pub fn with_message_id(mut self, message_id: i64) -> Self {
        self.message_id = Some(message_id);
        self
    }

    #[allow(dead_code)]
    pub fn streaming(mut self) -> Self {
        self.is_streaming = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inbound_message_creation() {
        let msg = InboundMessage::new(
            "telegram".to_string(),
            "user123".to_string(),
            "chat456".to_string(),
            "Hello!".to_string(),
        );

        assert_eq!(msg.channel, "telegram");
        assert_eq!(msg.sender_id, "user123");
        assert_eq!(msg.chat_id, "chat456");
        assert_eq!(msg.content, "Hello!");
        assert!(msg.media.is_empty());
        assert!(msg.metadata.is_empty());
    }

    #[test]
    fn test_outbound_message_creation() {
        let msg = OutboundMessage::new(
            "telegram".to_string(),
            "chat456".to_string(),
            "Hello back!".to_string(),
        );

        assert_eq!(msg.channel, "telegram");
        assert_eq!(msg.chat_id, "chat456");
        assert_eq!(msg.content, "Hello back!");
        assert!(msg.metadata.is_empty());
    }

    #[test]
    fn test_inbound_message_with_media_and_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("key1".to_string(), "value1".to_string());
        
        let msg = InboundMessage::new(
            "telegram".to_string(),
            "user123".to_string(),
            "chat456".to_string(),
            "Hello!".to_string(),
        )
        .with_media(vec!["image.jpg".to_string()])
        .with_metadata(metadata);

        assert_eq!(msg.media, vec!["image.jpg"]);
        assert_eq!(msg.metadata.get("key1").unwrap(), "value1");
    }

    #[test]
    fn test_outbound_message_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("key1".to_string(), "value1".to_string());
        
        let msg = OutboundMessage::new(
            "telegram".to_string(),
            "chat456".to_string(),
            "Hello back!".to_string(),
        )
        .with_metadata(metadata);

        assert_eq!(msg.metadata.get("key1").unwrap(), "value1");
    }
}
