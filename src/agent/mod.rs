mod context;
mod memory;
mod tools;

pub use context::ContextBuilder;
pub use memory::MemoryStore;

use std::path::PathBuf;
use tokio::sync::RwLock;
use serde::Deserialize;

#[allow(dead_code)]
use crate::bus::{InboundMessage, OutboundMessage};
use crate::config::Config;
use crate::providers::{ChatMessage, OpenAIProvider};
use crate::agent::tools::{EditFileTool, ListDirTool, ReadFileTool, ShellTool, ToolRegistry, WebFetchTool, WriteFileTool};

#[derive(Deserialize)]
struct ToolCallRequest {
    id: String,
    name: String,
    arguments: serde_json::Value,
}

pub struct AgentLoop {
    inbound_rx: tokio::sync::mpsc::Receiver<InboundMessage>,
    provider: OpenAIProvider,
    workspace: PathBuf,
    model: String,
    max_iterations: u32,
    temperature: f32,
    max_tokens: u32,
    memory_window: u32,
    tools: RwLock<ToolRegistry>,
    context: ContextBuilder,
    session_history: RwLock<Vec<serde_json::Value>>,
    #[allow(dead_code)]
    outbound_tx: tokio::sync::mpsc::Sender<OutboundMessage>,
}

impl AgentLoop {
    pub fn new(
        config: &Config,
        inbound_rx: tokio::sync::mpsc::Receiver<InboundMessage>,
        outbound_tx: tokio::sync::mpsc::Sender<OutboundMessage>,
    ) -> Self {
        let workspace = config.workspace_path();
        let provider = OpenAIProvider::new(config.provider.clone());
        
        let tools = Self::create_tools(&config, &workspace);
        
        Self {
            inbound_rx,
            provider,
            workspace,
            model: config.agent.model.clone(),
            max_iterations: config.agent.max_iterations,
            temperature: config.agent.temperature,
            max_tokens: config.agent.max_tokens,
            memory_window: config.agent.memory_window,
            tools: RwLock::new(tools),
            context: ContextBuilder::new(&config.workspace_path()),
            session_history: RwLock::new(Vec::new()),
            outbound_tx,
        }
    }

    fn create_tools(config: &Config, workspace: &PathBuf) -> ToolRegistry {
        let mut tools = ToolRegistry::new();
        
        let allowed_dir = if config.tools.restrict_to_workspace {
            Some(workspace.clone())
        } else {
            None
        };
        
        tools.register(ReadFileTool::new(allowed_dir.clone()));
        tools.register(WriteFileTool::new(allowed_dir.clone()));
        tools.register(EditFileTool::new(allowed_dir.clone()));
        tools.register(ListDirTool::new(allowed_dir));
        
        tools.register(ShellTool::new(
            workspace.display().to_string(),
            config.tools.shell_timeout,
        ));
        
        tools.register(WebFetchTool::new());
        
        tools
    }

    #[allow(dead_code)]
    pub fn register_message_tool(&self, _sender: tokio::sync::mpsc::Sender<OutboundMessage>) {
        // This would need to be done differently in actual implementation
    }

    pub async fn run(&mut self) {
        tracing::info!("Agent loop started");
        
        loop {
            tokio::select! {
                msg = self.inbound_rx.recv() => {
                    match msg {
                        Some(msg) => {
                            if let Err(e) = self.process_message(msg).await {
                                tracing::error!("Error processing message: {}", e);
                            }
                        }
                        None => break,
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                    // Brief pause to prevent busy loop
                }
            }
        }
        
        tracing::info!("Agent loop stopped");
    }

    async fn process_message(&mut self, msg: InboundMessage) -> Result<(), String> {
        tracing::info!("Processing message from {}: {}", msg.channel, &msg.content[..msg.content.len().min(50)]);

        let tools = self.tools.read().await;
        let tool_defs = tools.get_definitions();
        drop(tools);

        let messages = if !tool_defs.is_empty() {
            // Use system prompt with tools information
            let tools_json = serde_json::to_string_pretty(&tool_defs).unwrap_or_default();
            self.context.build_messages_with_tools(
                &self.session_history.read().await,
                &msg.content,
                Some(&msg.channel),
                Some(&msg.chat_id),
                &tools_json,
            )
        } else {
            self.context.build_messages(
                &self.session_history.read().await,
                &msg.content,
                Some(&msg.channel),
                Some(&msg.chat_id),
            )
        };

        let (final_content, tools_used) = self.run_agent_loop(messages, self.outbound_tx.clone(), msg.channel.clone(), msg.chat_id.clone()).await?;

        let response = final_content.unwrap_or_else(|| "I've completed processing but have no response to give.".to_string());

        tracing::info!("Agent response generated ({} chars)", response.len());

        self.session_history.write().await.push(serde_json::json!({
            "role": "user",
            "content": msg.content,
        }));

        self.session_history.write().await.push(serde_json::json!({
            "role": "assistant",
            "content": response.clone(),
            "tools_used": tools_used,
        }));

        if self.session_history.read().await.len() > self.memory_window as usize * 2 {
            self.consolidate_memory().await;
        }

        Ok(())
    }

    async fn run_agent_loop(&self, mut messages: Vec<ChatMessage>, outbound_tx: tokio::sync::mpsc::Sender<OutboundMessage>, channel: String, chat_id: String) -> Result<(Option<String>, Vec<String>), String> {
        let mut iteration = 0;
        let mut final_content: Option<String> = None;
        let mut tools_used = Vec::new();
        let mut last_tool_results: Vec<String> = Vec::new();

        while iteration < self.max_iterations {
            iteration += 1;

            let tools = self.tools.read().await;
            let _tool_defs = tools.get_definitions();

            tracing::info!("Iteration {}: Sending request", iteration);

            // Use streaming chat
            let mut stream = self.provider.chat_stream(
                messages.clone(),
                None,
                Some(self.model.clone()),
                Some(self.temperature),
                Some(self.max_tokens),
            ).await.map_err(|e| e.to_string())?;

            let mut content = String::new();
            use futures::StreamExt;
            
            // Send chunks in real-time
            let mut chunk_count = 0;
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        content.push_str(&chunk);
                        chunk_count += 1;
                        
                        // Send chunk every 4 chunks to avoid too many updates
                        if chunk_count % 4 == 0 {
                            let _ = outbound_tx.send(OutboundMessage::new(channel.clone(), chat_id.clone(), content.clone()).streaming()).await;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Stream error: {}", e);
                        break;
                    }
                }
            }

            // Send final content
            let _ = outbound_tx.send(OutboundMessage::new(channel.clone(), chat_id.clone(), content.clone()).streaming()).await;

            tracing::info!("LLM response: content length={:?}", content.len());

            // Check if response contains a tool call in JSON format
            if let Some(tool_call) = self.parse_tool_call_from_json(&content, &tools).await {
                tracing::info!("Parsed tool call: {}({:?})", tool_call.name, tool_call.arguments);
                tools_used.push(tool_call.name.clone());

                let result = tools
                    .execute(&tool_call.name, serde_json::to_value(&tool_call.arguments).unwrap_or_default())
                    .await;

                let result_str = match result {
                    Ok(r) => r,
                    Err(e) => format!("Error: {}", e),
                };

                last_tool_results.push(result_str.clone());
                messages.push(ChatMessage::assistant(content.clone()));
                messages.push(ChatMessage::tool(&result_str, &tool_call.id));
                messages.push(ChatMessage::user("Tool executed. Continue with your response or use another tool if needed."));

                continue;
            }

            // No tool call, use content as final response
            final_content = Some(content);
            break;
        }

        // If we have tool results but no final content, use the tool results as the response
        if final_content.is_none() && !last_tool_results.is_empty() {
            final_content = Some(last_tool_results.join("\n"));
        }

        Ok((final_content, tools_used))
    }

    async fn parse_tool_call_from_json(&self, content: &str, tools: &crate::agent::tools::ToolRegistry) -> Option<ToolCallRequest> {
        // Try to find JSON object in the content
        let json_start = content.find("```json")?;
        
        // Find the closing ``` after json_start
        let remaining = &content[json_start + 7..];
        let json_end_in_remaining = remaining.find("```")?;
        let json_end = json_start + 7 + json_end_in_remaining;
        
        let json_str = &content[json_start + 7..json_end].trim();
        
        #[derive(serde::Deserialize)]
        struct ToolCallJson {
            tool: String,
            arguments: serde_json::Value,
        }

        match serde_json::from_str::<ToolCallJson>(json_str) {
            Ok(call) => {
                // Verify tool exists - use the 'tool' field, not 'name'
                if tools.get(&call.tool).is_some() {
                    Some(ToolCallRequest {
                        id: format!("call_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
                        name: call.tool,
                        arguments: call.arguments,
                    })
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    async fn consolidate_memory(&self) {
        let history = self.session_history.read().await;
        
        if history.len() < self.memory_window as usize {
            return;
        }
        
        // Keep only the most recent messages
        let keep = history.len() - (self.memory_window as usize / 2);
        
        // Save older messages to history file
        let memory = MemoryStore::new(&self.workspace);
        
        for msg in history.iter().take(keep) {
            if let (Some(role), Some(content)) = (
                msg.get("role").and_then(|v| v.as_str()),
                msg.get("content").and_then(|v| v.as_str()),
            ) {
                let entry = format!("[{}] {}: {}", 
                    chrono::Local::now().format("%Y-%m-%d %H:%M"),
                    role.to_uppercase(),
                    content
                );
                let _ = memory.append_history(&entry);
            }
        }
        
        tracing::info!("Memory consolidated");
    }

    pub async fn process_direct(&self, content: &str) -> Result<String, String> {
        let messages = self.context.build_messages(
            &self.session_history.read().await,
            content,
            Some("cli"),
            Some("direct"),
        );

        let (final_content, _) = self.run_agent_loop(messages, self.outbound_tx.clone(), "cli".to_string(), "direct".to_string()).await?;

        Ok(final_content.unwrap_or_else(|| "No response".to_string()))
    }
}
