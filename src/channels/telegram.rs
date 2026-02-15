use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use crate::bus::{InboundMessage, OutboundMessage};

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
        
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
        
        let request = SendMessageRequest {
            chat_id,
            text: msg.content,
            reply_to_message_id: None,
        };
        
        self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        Ok(())
    }
}
