mod filesystem;
mod shell;
mod web;

#[allow(dead_code)]
mod message;

#[allow(dead_code)]
mod spawn;

pub use filesystem::{ReadFileTool, WriteFileTool, EditFileTool, ListDirTool};
pub use shell::ShellTool;
pub use web::WebFetchTool;

use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;
    
    async fn execute(&self, args: Value) -> Result<String, String>;
}

#[allow(dead_code)]
pub struct ToolRegistry {
    tools: std::collections::HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: std::collections::HashMap::new(),
        }
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
    }

    #[allow(dead_code)]
    pub fn get(&self, name: &str) -> Option<&Box<dyn Tool>> {
        self.tools.get(name)
    }

    pub fn get_definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|tool| ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: tool.name().to_string(),
                    description: tool.description().to_string(),
                    parameters: tool.parameters(),
                },
            })
            .collect()
    }

    pub async fn execute(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        let tool = self.tools.get(name).ok_or_else(|| format!("Tool not found: {}", name))?;
        tool.execute(args).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

use crate::providers::{ToolDefinition, FunctionDefinition};
