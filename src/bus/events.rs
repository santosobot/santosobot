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
}

impl OutboundMessage {
    pub fn new(channel: String, chat_id: String, content: String) -> Self {
        Self {
            channel,
            chat_id,
            content,
            metadata: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }
}
