use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tokio::time::sleep;
use chrono::{DateTime, Utc, NaiveDateTime};
use crate::agent::tools::Tool;
use crate::bus::OutboundMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: String,
    pub user_id: String,
    pub channel: String,
    pub message: String,
    pub scheduled_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub repeat_pattern: Option<String>, // For recurring reminders (e.g., "daily", "weekly")
}

pub struct ReminderTool {
    reminders: Arc<RwLock<Vec<Reminder>>>,
    workspace_path: String,
    outbound_tx: Arc<Mutex<Option<tokio::sync::mpsc::Sender<OutboundMessage>>>>,
}

impl ReminderTool {
    pub fn new(workspace_path: String) -> Self {
        let reminders = Arc::new(RwLock::new(Vec::new()));
        let outbound_tx = Arc::new(Mutex::new(None));
        
        Self {
            reminders,
            workspace_path,
            outbound_tx,
        }
    }

    pub fn set_outbound_sender(&self, sender: tokio::sync::mpsc::Sender<OutboundMessage>) {
        let mut tx_guard = self.outbound_tx.blocking_lock();
        *tx_guard = Some(sender);
    }

    async fn save_reminders_to_file(&self) -> Result<(), String> {
        let reminders = self.reminders.read().await;
        let content = serde_json::to_string_pretty(&*reminders)
            .map_err(|e| format!("Failed to serialize reminders: {}", e))?;
        
        let file_path = format!("{}/reminders.json", self.workspace_path);
        tokio::fs::write(file_path, content)
            .await
            .map_err(|e| format!("Failed to write reminders to file: {}", e))?;
        
        Ok(())
    }

    async fn load_reminders_from_file(&self) -> Result<(), String> {
        let file_path = format!("{}/reminders.json", self.workspace_path);
        
        // Check if file exists
        if !tokio::fs::try_exists(&file_path).await.map_err(|e| e.to_string())? {
            return Ok(());
        }
        
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| format!("Failed to read reminders file: {}", e))?;
        
        let loaded_reminders: Vec<Reminder> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to deserialize reminders: {}", e))?;
        
        let mut reminders = self.reminders.write().await;
        *reminders = loaded_reminders;
        
        Ok(())
    }

    async fn start_reminder_task(&self, reminder: Reminder) {
        let outbound_tx_clone = Arc::clone(&self.outbound_tx);
        
        tokio::spawn(async move {
            let delay = (reminder.scheduled_time - Utc::now()).to_std()
                .unwrap_or(std::time::Duration::from_secs(0));
            
            // Sleep until the reminder time
            sleep(delay).await;
            
            // Send the reminder
            {
                let tx_guard = outbound_tx_clone.lock().await;
                if let Some(ref tx) = *tx_guard {
                    let msg = OutboundMessage::new(
                        reminder.channel.clone(),
                        reminder.user_id.clone(),
                        format!("⏰ **REMINDER**: {}", reminder.message)
                    );
                    
                    if let Err(e) = tx.send(msg).await {
                        eprintln!("Failed to send reminder: {}", e);
                    }
                }
            }
            
            // Handle recurring reminders
            let repeat_pattern = reminder.repeat_pattern.clone();
            if let Some(pattern) = repeat_pattern {
                // For now, we'll just create a new reminder with updated time based on pattern
                // In a real implementation, you'd parse the pattern (e.g., daily, weekly) and calculate next occurrence
                if pattern == "daily" {
                    let next_time = reminder.scheduled_time + chrono::Duration::days(1);
                    let new_reminder = Reminder {
                        id: format!("{}_repeat_{}", reminder.id, next_time.timestamp()),
                        scheduled_time: next_time,
                        created_at: Utc::now(),
                        repeat_pattern: Some(pattern),
                        user_id: reminder.user_id.clone(),
                        channel: reminder.channel.clone(),
                        message: reminder.message.clone(),
                    };
                    
                    // In a real implementation, we would add this to the active reminders
                    // For now, we'll just log that we would schedule a recurring reminder
                    println!("Would schedule recurring reminder: {}", new_reminder.message);
                }
            }
        });
    }
}

#[async_trait]
impl Tool for ReminderTool {
    fn name(&self) -> &str { "reminder" }

    fn description(&self) -> &str {
        "Schedule a reminder message to be sent at a specific time"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The reminder message content"
                },
                "time": {
                    "type": "string",
                    "description": "Time for the reminder in format YYYY-MM-DD HH:MM:SS UTC"
                },
                "user_id": {
                    "type": "string",
                    "description": "User ID to send the reminder to"
                },
                "channel": {
                    "type": "string",
                    "description": "Channel to send the reminder to (e.g., telegram)"
                },
                "repeat": {
                    "type": "string",
                    "description": "Repeat pattern (optional): daily, weekly"
                }
            },
            "required": ["message", "time", "user_id", "channel"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let message = args["message"]
            .as_str()
            .ok_or("Missing message parameter")?
            .to_string();

        let time_str = args["time"]
            .as_str()
            .ok_or("Missing time parameter")?;

        let user_id = args["user_id"]
            .as_str()
            .ok_or("Missing user_id parameter")?
            .to_string();

        let channel = args["channel"]
            .as_str()
            .ok_or("Missing channel parameter")?
            .to_string();

        let repeat_pattern = args["repeat"].as_str().map(|s| s.to_string());

        // Parse the time string to DateTime<Utc>
        let naive_dt = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S")
            .map_err(|e| format!("Failed to parse time: {}", e))?;
        let scheduled_time = DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc);

        // Check if the scheduled time is in the past
        if scheduled_time <= Utc::now() {
            return Err("Scheduled time must be in the future".to_string());
        }

        // Generate a unique ID for the reminder
        let id = format!("reminder_{}_{}", user_id, scheduled_time.timestamp());

        let reminder = Reminder {
            id: id.clone(),
            user_id,
            channel,
            message: message.clone(),
            scheduled_time,
            created_at: Utc::now(),
            repeat_pattern,
        };

        // Add to in-memory list
        {
            let mut reminders = self.reminders.write().await;
            reminders.push(reminder.clone());
        }

        // Save to file
        self.save_reminders_to_file().await?;

        // Start the reminder task
        self.start_reminder_task(reminder).await;

        Ok(format!("Reminder scheduled successfully for {}", time_str))
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_reminder_creation() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().to_str().unwrap().to_string();
        
        let tool = ReminderTool::new(workspace_path);
        
        // Test creating a reminder
        let args = json!({
            "message": "Test reminder",
            "time": "2099-12-31 23:59:59",
            "user_id": "test_user",
            "channel": "telegram"
        });
        
        let result = tool.execute(args).await;
        assert!(result.is_ok());
        
        // Check that the reminder was added
        let reminders = tool.reminders.read().await;
        assert_eq!(reminders.len(), 1);
        assert_eq!(reminders[0].message, "Test reminder");
    }
}