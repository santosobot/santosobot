use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use crate::bus::{InboundMessage, OutboundMessage};

const TELEGRAM_MAX_MESSAGE_LENGTH: usize = 4096;

pub struct TelegramChannel {
    token: String,
    client: Client,
    inbound_tx: mpsc::Sender<InboundMessage>,
    allow_from: Vec<String>,
}

#[derive(Serialize)]
struct SendMessageRequest {
    chat_id: i64,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to_message_id: Option<i64>,
}

#[derive(Serialize)]
struct SendChatActionRequest {
    chat_id: i64,
    action: String,
}

#[derive(Serialize)]
#[allow(dead_code)]
struct EditMessageRequest {
    chat_id: i64,
    message_id: i64,
    text: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Update {
    update_id: i64,
    message: Option<Message>,
    edited_message: Option<Message>,
    my_chat_member: Option<ChatMember>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Message {
    message_id: i64,
    from: Option<User>,
    chat: Chat,
    text: Option<String>,
    bot_command: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct User {
    id: i64,
    is_bot: bool,
    username: Option<String>,
    first_name: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Chat {
    id: i64,
    #[serde(rename = "type")]
    chat_type: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ChatMember {
    new_chat_member: Option<ChatMemberInfo>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ChatMemberInfo {
    user: Option<User>,
    status: String,
}

impl TelegramChannel {
    pub fn new(
        token: String,
        inbound_tx: mpsc::Sender<InboundMessage>,
        allow_from: Vec<String>,
    ) -> Self {
        Self {
            token,
            client: Client::new(),
            inbound_tx,
            allow_from,
        }
    }

    pub async fn start(&self) {
        tracing::info!("Telegram channel starting...");
        
        // Get latest update offset first to skip old messages
        let mut offset: i64 = self.get_latest_update_id().await.unwrap_or(0) + 1;
        tracing::info!("Starting from offset: {}", offset);
        
        loop {
            match self.get_updates(offset).await {
                Ok(updates) => {
                    for update in updates {
                        // Handle new members (when bot is added to groups)
                        if let Some(member) = update.my_chat_member {
                            if let Some(user) = member.new_chat_member.as_ref().and_then(|m| m.user.as_ref()) {
                                tracing::info!("Bot added to chat by user: {}", user.id);
                            }
                            // Update offset to avoid reprocessing
                            offset = update.update_id + 1;
                            continue;
                        }
                        
                        if let Some(ref message) = update.message {
                            // Skip messages from bots
                            if message.from.as_ref().map(|u| u.is_bot).unwrap_or(false) {
                                offset = update.update_id + 1;
                                continue;
                            }
                            
                            // Check allow_from whitelist
                            if !self.allow_from.is_empty() {
                                let sender_id = message.from
                                    .as_ref()
                                    .map(|u| u.id.to_string())
                                    .unwrap_or_default();
                                
                                if !self.allow_from.contains(&sender_id) {
                                    tracing::debug!("Message from {} not in allow list, skipping", sender_id);
                                    offset = update.update_id + 1;
                                    continue;
                                }
                            }
                            
                            if let Some(text) = &message.text {
                                let sender_id = message.from
                                    .as_ref()
                                    .map(|u| u.id.to_string())
                                    .unwrap_or_default();
                                
                                tracing::info!("Received message from {}: {}", sender_id, text);
                                
                                let msg = InboundMessage::new(
                                    "telegram".to_string(),
                                    sender_id,
                                    message.chat.id.to_string(),
                                    text.to_string(),
                                );
                                
                                if self.inbound_tx.send(msg).await.is_err() {
                                    tracing::error!("Failed to send message to channel");
                                }
                            }
                        }
                        
                        // Update offset
                        offset = update.update_id + 1;
                    }
                }
                Err(e) => {
                    tracing::error!("Error getting updates: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
            
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    async fn get_updates(&self, offset: i64) -> Result<Vec<Update>, String> {
        let url = format!(
            "https://api.telegram.org/bot{}/getUpdates?timeout=60&offset={}",
            self.token, offset
        );
        
        #[derive(Deserialize)]
        struct Response {
            ok: bool,
            result: Vec<Update>,
        }
        
        let resp = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        let data: Response = resp.json().await.map_err(|e| e.to_string())?;
        
        if data.ok {
            Ok(data.result)
        } else {
            Err("Telegram API error".to_string())
        }
    }

    async fn get_latest_update_id(&self) -> Result<i64, String> {
        // Get updates with limit=1 to get the latest update_id
        let url = format!(
            "https://api.telegram.org/bot{}/getUpdates?limit=1",
            self.token
        );
        
        #[derive(Deserialize)]
        struct Response {
            ok: bool,
            result: Vec<Update>,
        }
        
        let resp = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        let data: Response = resp.json().await.map_err(|e| e.to_string())?;
        
        if data.ok {
            Ok(data.result.last().map(|u| u.update_id).unwrap_or(0))
        } else {
            Ok(0)
        }
    }

    pub async fn send(&self, msg: OutboundMessage) -> Result<(), String> {
        let chat_id: i64 = msg.chat_id.parse().map_err(|_| "Invalid chat_id")?;

        // Send typing status first
        let _ = self.send_chat_action(chat_id, "typing").await;

        // Split large messages
        let chunks = self.split_message(&msg.content);

        for (i, chunk) in chunks.iter().enumerate() {
            let reply_to = if i > 0 { Some(msg.chat_id.parse().unwrap_or(0)) } else { None };
            self.send_message(chat_id, chunk.to_string(), reply_to).await?;
        }

        Ok(())
    }

    pub async fn send_streaming(&self, msg: OutboundMessage) -> Result<i64, String> {
        let chat_id: i64 = msg.chat_id.parse().map_err(|_| "Invalid chat_id")?;

        // Send typing status first
        let _ = self.send_chat_action(chat_id, "typing").await;

        // Send initial empty message
        let message_id = self.send_message(chat_id, "⏳ Generating response...".to_string(), None).await?;

        Ok(message_id)
    }

    pub async fn edit_message(&self, chat_id: i64, message_id: i64, text: String) -> Result<(), String> {
        // Split and edit only the first chunk if text is too long
        let chunks = self.split_message(&text);
        let text = chunks.first().unwrap_or(&text).clone();

        let url = format!("https://api.telegram.org/bot{}/editMessageText", self.token);

        let request = EditMessageRequest {
            chat_id,
            message_id,
            text,
        };

        self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn finalize_streaming(&self, chat_id: i64, message_id: i64, final_text: String) -> Result<(), String> {
        // Split large messages for final response
        let chunks = self.split_message(&final_text);

        // Edit the original message with first chunk
        self.edit_message(chat_id, message_id, chunks.first().unwrap_or(&final_text).clone()).await?;

        // Send additional chunks as replies
        for (_i, chunk) in chunks.iter().enumerate().skip(1) {
            self.send_message(chat_id, chunk.to_string(), Some(message_id)).await?;
        }

        Ok(())
    }

    pub async fn send_chat_action(&self, chat_id: i64, action: &str) -> Result<(), String> {
        let url = format!("https://api.telegram.org/bot{}/sendChatAction", self.token);

        let request = SendChatActionRequest {
            chat_id,
            action: action.to_string(),
        };

        self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn send_message(&self, chat_id: i64, text: String, reply_to_message_id: Option<i64>) -> Result<i64, String> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);

        let request = SendMessageRequest {
            chat_id,
            text,
            reply_to_message_id,
        };

        let resp = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        #[derive(Deserialize)]
        struct TelegramResponse {
            ok: bool,
            result: TelegramMessage,
        }

        #[derive(Deserialize)]
        struct TelegramMessage {
            message_id: i64,
        }

        let data: TelegramResponse = resp.json().await.map_err(|e| e.to_string())?;

        if data.ok {
            Ok(data.result.message_id)
        } else {
            Err("Failed to send message".to_string())
        }
    }

    fn split_message(&self, content: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current = String::new();

        for line in content.lines() {
            if current.len() + line.len() + 1 > TELEGRAM_MAX_MESSAGE_LENGTH {
                if !current.is_empty() {
                    chunks.push(current);
                    current = String::new();
                }
                
                // If single line is too long, split it
                if line.len() > TELEGRAM_MAX_MESSAGE_LENGTH {
                    let mut start = 0;
                    while start < line.len() {
                        let end = start + TELEGRAM_MAX_MESSAGE_LENGTH;
                        if end >= line.len() {
                            chunks.push(line[start..].to_string());
                            break;
                        } else {
                            // Try to split at word boundary
                            let split_point = line[start..end].rfind(' ')
                                .map(|p| start + p)
                                .unwrap_or(end);
                            chunks.push(line[start..split_point].to_string());
                            start = split_point + 1;
                        }
                    }
                } else {
                    current.push_str(line);
                }
            } else {
                if !current.is_empty() {
                    current.push('\n');
                }
                current.push_str(line);
            }
        }

        if !current.is_empty() {
            chunks.push(current);
        }

        chunks
    }
}
