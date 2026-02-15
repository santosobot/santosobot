use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use crate::agent::tools::Tool;

pub struct ShellTool {
    working_dir: PathBuf,
    timeout_secs: u64,
    #[allow(dead_code)]
    restrict_to_workspace: bool,
}

impl ShellTool {
    pub fn new(working_dir: String, timeout_secs: u64, restrict_to_workspace: bool) -> Self {
        Self {
            working_dir: PathBuf::from(working_dir),
            timeout_secs,
            restrict_to_workspace,
        }
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str { "shell" }
    
    fn description(&self) -> &str {
        "Execute a shell command"
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                }
            },
            "required": ["command"]
        })
    }
    
    async fn execute(&self, args: Value) -> Result<String, String> {
        let command = args["command"]
            .as_str()
            .ok_or("Missing command parameter")?;
        
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
           .arg(command)
           .current_dir(&self.working_dir)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_secs),
            cmd.output()
        )
        .await
        .map_err(|_| "Command timed out")?
        .map_err(|e| format!("Failed to execute command: {}", e))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        let result = if output.status.success() {
            if stdout.is_empty() && !stderr.is_empty() {
                stderr.to_string()
            } else {
                stdout.to_string()
            }
        } else {
            format!("Error (exit {}): {}\n{}", 
                output.status.code().unwrap_or(-1), 
                stdout, 
                stderr
            )
        };
        
        if result.len() > 50000 {
            return Ok(format!("{}...[truncated]", &result[..50000]));
        }
        
        Ok(result)
    }
}
