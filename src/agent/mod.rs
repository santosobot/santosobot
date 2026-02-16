mod context;
mod memory;
mod tools;

pub use context::ContextBuilder;
pub use memory::MemoryStore;

use std::path::PathBuf;
use tokio::sync::RwLock;

#[allow(dead_code)]
use crate::bus::{InboundMessage, OutboundMessage};
use crate::config::Config;
use crate::providers::{ChatMessage, OpenAIProvider};
use crate::agent::tools::{EditFileTool, ListDirTool, ReadFileTool, ShellTool, ToolRegistry, WebFetchTool, WriteFileTool};

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
        
        let messages = self.context.build_messages(
            &self.session_history.read().await,
            &msg.content,
            Some(&msg.channel),
            Some(&msg.chat_id),
        );
        
        let (final_content, tools_used) = self.run_agent_loop(messages).await?;
        
        let response = final_content.unwrap_or_else(|| "I've completed processing but have no response to give.".to_string());
        
        self.session_history.write().await.push(serde_json::json!({
            "role": "user",
            "content": msg.content,
        }));
        
        self.session_history.write().await.push(serde_json::json!({
            "role": "assistant",
            "content": response,
            "tools_used": tools_used,
        }));
        
        if self.session_history.read().await.len() > self.memory_window as usize * 2 {
            self.consolidate_memory().await;
        }
        
        let outbound = OutboundMessage::new(msg.channel, msg.chat_id, response);
        self.outbound_tx.send(outbound).await.map_err(|e| e.to_string())?;
        
        Ok(())
    }

    async fn run_agent_loop(&self, mut messages: Vec<ChatMessage>) -> Result<(Option<String>, Vec<String>), String> {
        let mut iteration = 0;
        let mut final_content: Option<String> = None;
        let mut tools_used = Vec::new();
        
        while iteration < self.max_iterations {
            iteration += 1;
            
            let tools = self.tools.read().await;
            let tool_defs = tools.get_definitions();
            
            let response = self.provider.chat(
                messages.clone(),
                if tool_defs.is_empty() { None } else { Some(tool_defs) },
                Some(self.model.clone()),
                Some(self.temperature),
                Some(self.max_tokens),
            ).await.map_err(|e| e.to_string())?;
            
            if response.has_tool_calls() {
                let content = response.content.as_deref().unwrap_or("");
                messages.push(ChatMessage::assistant(content));
                
                for tc in &response.tool_calls {
                    tools_used.push(tc.name.clone());
                    tracing::info!("Tool call: {}({:?})", tc.name, tc.arguments);
                    
                    let result = self.tools.read().await
                        .execute(&tc.name, serde_json::to_value(&tc.arguments).unwrap_or_default())
                        .await;
                    
                    let result_str = match result {
                        Ok(r) => r,
                        Err(e) => format!("Error: {}", e),
                    };
                    
                    messages.push(ChatMessage::tool(&result_str, &tc.id));
                }
                
                messages.push(ChatMessage::user("Reflect on the results and decide next steps."));
            } else {
                final_content = response.content;
                break;
            }
        }
        
        Ok((final_content, tools_used))
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
        
        let (final_content, _) = self.run_agent_loop(messages).await?;
        
        Ok(final_content.unwrap_or_else(|| "No response".to_string()))
    }
}
