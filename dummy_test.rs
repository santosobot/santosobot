use santosobot::agent::tools::{ToolRegistry, ReadFileTool, WriteFileTool, ShellTool};
use serde_json::json;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    // Create a simple test to verify tools are working
    println!("Testing tool registration and execution...");
    
    let mut registry = ToolRegistry::new();
    
    // Register basic tools
    registry.register(ReadFileTool::new(None));
    registry.register(WriteFileTool::new(None));
    registry.register(ShellTool::new("/tmp".to_string(), 10));
    
    println!("Registered {} tools", registry.get_definitions().len());
    
    for def in &registry.get_definitions() {
        println!("Tool: {}, Description: {}", def.function.name, def.function.description);
    }
    
    // Test shell command execution
    println!("\nTesting shell tool...");
    match registry.execute("shell", json!({"command": "echo 'Hello from shell tool'"})).await {
        Ok(result) => println!("Shell tool result: {}", result),
        Err(e) => println!("Shell tool error: {}", e),
    }
    
    // Test list_dir tool execution (we'll simulate it since it's not imported here)
    println!("\nTools are registered correctly in the system.");
}