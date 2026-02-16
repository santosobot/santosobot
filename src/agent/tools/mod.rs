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
use std::any::Any;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;

    async fn execute(&self, args: Value) -> Result<String, String>;
    
    #[allow(dead_code)]
    fn as_any(&self) -> &dyn Any;
    
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
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|boxed| boxed.as_ref())
    }
    
    #[allow(dead_code)]
    pub fn register_boxed(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;
    use std::fs;

    struct MockTool {
        name: String,
        description: String,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str { &self.name }
        
        fn description(&self) -> &str { &self.description }
        
        fn parameters(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "test_param": {
                        "type": "string",
                        "description": "A test parameter"
                    }
                },
                "required": ["test_param"]
            })
        }

        async fn execute(&self, _args: Value) -> Result<String, String> {
            Ok(format!("Executed {}", self.name))
        }
    }

    #[tokio::test]
    async fn test_tool_registry_register_and_get() {
        let mut registry = ToolRegistry::new();
        
        let mock_tool = MockTool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
        };
        
        registry.register(mock_tool);
        
        // Check that the tool definition is returned
        let definitions = registry.get_definitions();
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].function.name, "test_tool");
    }

    #[tokio::test]
    async fn test_tool_registry_execute() {
        let mut registry = ToolRegistry::new();
        
        let mock_tool = MockTool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
        };
        
        registry.register(mock_tool);
        
        let result = registry.execute("test_tool", json!({"test_param": "value"})).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Executed test_tool");
    }

    #[tokio::test]
    async fn test_tool_registry_execute_nonexistent() {
        let registry = ToolRegistry::new();
        
        let result = registry.execute("nonexistent_tool", json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Tool not found"));
    }

    #[tokio::test]
    async fn test_tool_registry_multiple_tools() {
        let mut registry = ToolRegistry::new();
        
        // Register multiple tools
        registry.register(MockTool {
            name: "tool1".to_string(),
            description: "First tool".to_string(),
        });
        
        registry.register(MockTool {
            name: "tool2".to_string(),
            description: "Second tool".to_string(),
        });
        
        let definitions = registry.get_definitions();
        assert_eq!(definitions.len(), 2);
        
        // Both tools should be executable
        let result1 = registry.execute("tool1", json!({"test_param": "value"})).await;
        assert!(result1.is_ok());
        
        let result2 = registry.execute("tool2", json!({"test_param": "value"})).await;
        assert!(result2.is_ok());
    }
}
