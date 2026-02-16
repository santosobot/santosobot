use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::agent::tools::Tool;

pub struct SpawnTool {
    subagents: Arc<RwLock<std::collections::HashMap<String, Subagent>>>,
    channel: Option<String>,
    chat_id: Option<String>,
}

pub struct Subagent {
    pub name: String,
    pub task: String,
    pub status: String,
}

impl SpawnTool {
    pub fn new() -> Self {
        Self {
            subagents: Arc::new(RwLock::new(std::collections::HashMap::new())),
            channel: None,
            chat_id: None,
        }
    }

    pub fn set_context(&mut self, channel: String, chat_id: String) {
        self.channel = Some(channel);
        self.chat_id = Some(chat_id);
    }
}

#[async_trait]
impl Tool for SpawnTool {
    fn name(&self) -> &str { "spawn" }
    
    fn description(&self) -> &str {
        "Spawn a background subagent to handle a task"
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name for the subagent"
                },
                "task": {
                    "type": "string",
                    "description": "Task description for the subagent"
                }
            },
            "required": ["name", "task"]
        })
    }
    
    async fn execute(&self, args: Value) -> Result<String, String> {
        let name = args["name"]
            .as_str()
            .ok_or("Missing name parameter")?;

        let task = args["task"]
            .as_str()
            .ok_or("Missing task parameter")?;

        let subagent = Subagent {
            name: name.to_string(),
            task: task.to_string(),
            status: "pending".to_string(),
        };

        self.subagents.write().await.insert(name.to_string(), subagent);

        Ok(format!("Subagent '{}' spawned with task: {}", name, task))
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Default for SpawnTool {
    fn default() -> Self {
        Self::new()
    }
}
