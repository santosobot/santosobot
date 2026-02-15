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
