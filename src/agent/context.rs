use crate::agent::memory::MemoryStore;
use std::path::{Path, PathBuf};

pub struct ContextBuilder {
    workspace: PathBuf,
    memory: MemoryStore,
}

impl ContextBuilder {
    pub fn new(workspace: &Path) -> Self {
        Self {
            workspace: workspace.to_path_buf(),
            memory: MemoryStore::new(workspace),
        }
    }

    pub fn build_system_prompt(&self) -> String {
        let identity = self.get_identity();
        let bootstrap = self.load_bootstrap_files();
        let memory = self.memory.get_memory_context();

        let mut parts = vec![identity];

        if !bootstrap.is_empty() {
            parts.push(bootstrap);
        }

        if !memory.is_empty() {
            parts.push(memory);
        }

        parts.join("\n\n---\n\n")
    }

    pub fn build_system_prompt_with_tools(&self, tools_json: &str) -> String {
        let base_prompt = self.build_system_prompt();
        
        format!(
            r#"{}

## Available Tools
You have access to the following tools. When you need to use a tool, respond with a JSON object in this format:
```json
{{
    "tool": "tool_name",
    "arguments": {{
        "arg1": "value1",
        "arg2": "value2"
    }}
}}
```

Available tools:
{}

After receiving the tool result, you can continue with your response or use another tool if needed.
If the user's request doesn't require any tools, just respond naturally with text."#,
            base_prompt,
            tools_json
        )
    }

    fn get_identity(&self) -> String {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M (%A)");
        let workspace_path = self.workspace.display();

        format!(
            r#"# Santoso 🤖

You are Santoso, a helpful AI assistant.

## Current Time
{}

## Workspace
Your workspace is at: {}
- Long-term memory: {}/memory/MEMORY.md
- History log: {}/memory/HISTORY.md

## Your Capabilities
You have access to tools that allow you to:
- Read, write, and edit files
- Execute shell commands
- Fetch web pages
- Send messages to users
- Spawn subagents for background tasks

IMPORTANT: When responding to direct questions or conversations, reply directly with your text response.
Only use the 'message' tool when you need to send a message to a specific chat channel.

Always be helpful, accurate, and concise. When using tools, think step by step.
When remembering something important, write to {}/memory/MEMORY.md"#,
            now, workspace_path, workspace_path, workspace_path, workspace_path
        )
    }

    fn load_bootstrap_files(&self) -> String {
        let files = ["AGENTS.md", "SOUL.md", "USER.md", "TOOLS.md", "IDENTITY.md"];

        let mut parts = Vec::new();

        for filename in files {
            let path = self.workspace.join(filename);
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if !content.trim().is_empty() {
                        parts.push(format!("## {}\n\n{}", filename, content.trim()));
                    }
                }
            }
        }

        parts.join("\n\n")
    }

    pub fn build_messages(
        &self,
        history: &[serde_json::Value],
        current_message: &str,
        channel: Option<&str>,
        chat_id: Option<&str>,
    ) -> Vec<crate::providers::ChatMessage> {
        let mut messages = Vec::new();

        let mut system_prompt = self.build_system_prompt();

        if let (Some(ch), Some(cid)) = (channel, chat_id) {
            system_prompt.push_str(&format!(
                "\n\n## Current Session\nChannel: {}\nChat ID: {}",
                ch, cid
            ));
        }

        messages.push(crate::providers::ChatMessage::system(system_prompt));

        for msg in history {
            let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("user");
            let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");

            messages.push(match role {
                "assistant" => crate::providers::ChatMessage::assistant(content),
                _ => crate::providers::ChatMessage::user(content),
            });
        }

        messages.push(crate::providers::ChatMessage::user(current_message));

        messages
    }

    pub fn build_messages_with_tools(
        &self,
        history: &[serde_json::Value],
        current_message: &str,
        channel: Option<&str>,
        chat_id: Option<&str>,
        tools_json: &str,
    ) -> Vec<crate::providers::ChatMessage> {
        let mut messages = Vec::new();

        let mut system_prompt = self.build_system_prompt_with_tools(tools_json);

        if let (Some(ch), Some(cid)) = (channel, chat_id) {
            system_prompt.push_str(&format!(
                "\n\n## Current Session\nChannel: {}\nChat ID: {}",
                ch, cid
            ));
        }

        messages.push(crate::providers::ChatMessage::system(system_prompt));

        for msg in history {
            let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("user");
            let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");

            messages.push(match role {
                "assistant" => crate::providers::ChatMessage::assistant(content),
                _ => crate::providers::ChatMessage::user(content),
            });
        }

        messages.push(crate::providers::ChatMessage::user(current_message));

        messages
    }

    #[allow(dead_code)]
    pub fn add_tool_result(
        &self,
        messages: &mut Vec<crate::providers::ChatMessage>,
        tool_call_id: &str,
        _tool_name: &str,
        result: &str,
    ) {
        messages.push(crate::providers::ChatMessage::tool(result, tool_call_id));
    }

    #[allow(dead_code)]
    pub fn add_assistant_message(
        &self,
        messages: &mut Vec<crate::providers::ChatMessage>,
        content: Option<&str>,
        _tool_calls: Option<&[serde_json::Value]>,
    ) {
        // For simplicity, we just add the content. Tool calls will be handled separately.
        if let Some(c) = content {
            messages.push(crate::providers::ChatMessage::assistant(c));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_context_builder_creation() {
        let temp_dir = TempDir::new().unwrap();
        let context_builder = ContextBuilder::new(temp_dir.path());

        assert_eq!(context_builder.workspace, temp_dir.path());
    }

    #[test]
    fn test_build_system_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let context_builder = ContextBuilder::new(temp_dir.path());

        let prompt = context_builder.build_system_prompt();
        
        // Check that the prompt contains expected elements
        assert!(prompt.contains("# Santoso 🤖"));
        assert!(prompt.contains("You are Santoso, a helpful AI assistant."));
        assert!(prompt.contains("Workspace"));
        assert!(prompt.contains("Your Capabilities"));
    }

    #[test]
    fn test_build_messages() {
        let temp_dir = TempDir::new().unwrap();
        let context_builder = ContextBuilder::new(temp_dir.path());

        let history = vec![
            serde_json::json!({"role": "user", "content": "Hello"}),
            serde_json::json!({"role": "assistant", "content": "Hi there!"}),
        ];
        
        let messages = context_builder.build_messages(&history, "How are you?", Some("cli"), Some("test-chat"));
        
        // Should have system message, history messages, and current message
        assert!(messages.len() >= 3); // At least system, 2 history items, and current
        
        // First message should be system
        assert_eq!(messages[0].role, "system");
        
        // Last message should be the current one
        assert_eq!(messages[messages.len()-1].role, "user");
        assert_eq!(messages[messages.len()-1].content, "How are you?");
    }

    #[test]
    fn test_load_bootstrap_files() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create a bootstrap file
        let agents_file = temp_dir.path().join("AGENTS.md");
        fs::write(&agents_file, "# Agents\nSpecialized agents for various tasks").unwrap();
        
        let context_builder = ContextBuilder::new(temp_dir.path());
        // Note: load_bootstrap_files is private, so we test it indirectly through build_system_prompt
        let prompt = context_builder.build_system_prompt();
        
        assert!(prompt.contains("## AGENTS.md"));
        assert!(prompt.contains("Specialized agents for various tasks"));
    }
}
