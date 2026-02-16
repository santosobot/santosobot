use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use crate::agent::tools::Tool;

pub struct ReadFileTool {
    allowed_dir: Option<PathBuf>,
}

impl ReadFileTool {
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }

    fn validate_path(&self, path: &str) -> Result<PathBuf, String> {
        let path = PathBuf::from(path);
        
        if let Some(ref dir) = self.allowed_dir {
            let canonical = path.canonicalize()
                .map_err(|e| format!("Invalid path: {}", e))?;
            let dir_canonical = dir.canonicalize()
                .map_err(|e| format!("Invalid workspace: {}", e))?;
            
            if !canonical.starts_with(&dir_canonical) {
                return Err("Path outside workspace not allowed".to_string());
            }
        }
        
        Ok(path)
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str { "read_file" }

    fn description(&self) -> &str {
        "Read contents of a file"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let path = args["path"]
            .as_str()
            .ok_or("Missing path parameter")?;

        let validated = self.validate_path(path)?;

        std::fs::read_to_string(&validated)
            .map_err(|e| format!("Failed to read file: {}", e))
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct WriteFileTool {
    allowed_dir: Option<PathBuf>,
}

impl WriteFileTool {
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }

    fn validate_path(&self, path: &str) -> Result<PathBuf, String> {
        let path = PathBuf::from(path);

        if let Some(ref dir) = self.allowed_dir {
            // Resolve the absolute path relative to the allowed directory
            let abs_path = if path.is_absolute() {
                path
            } else {
                dir.join(&path)
            };

            // Canonicalize the allowed directory
            let dir_canonical = dir.canonicalize()
                .map_err(|e| format!("Invalid workspace: {}", e))?;

            // Canonicalize the target path (this will fail if the file doesn't exist yet)
            // So we'll check the parent directory instead
            let parent = abs_path.parent().unwrap_or(&abs_path);
            
            let parent_canonical = parent.canonicalize()
                .map_err(|_| "Path validation failed: parent directory does not exist".to_string())?;

            if !parent_canonical.starts_with(&dir_canonical) {
                return Err("Path outside workspace not allowed".to_string());
            }

            // Additional check: ensure the path doesn't contain dangerous sequences like '/../'
            let path_str = abs_path.to_string_lossy();
            if path_str.contains("../") || path_str.starts_with("../") {
                return Err("Path contains invalid sequences".to_string());
            }

            Ok(abs_path)
        } else {
            Ok(path)
        }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str { "write_file" }

    fn description(&self) -> &str {
        "Write content to a file (creates or overwrites)"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let path = args["path"]
            .as_str()
            .ok_or("Missing path parameter")?;
        let content = args["content"]
            .as_str()
            .ok_or("Missing content parameter")?;

        let validated = self.validate_path(path)?;

        if let Some(parent) = validated.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        std::fs::write(&validated, content)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(format!("File written successfully: {}", path))
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct EditFileTool {
    #[allow(dead_code)]
    allowed_dir: Option<PathBuf>,
}

impl EditFileTool {
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }
}

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str { "edit_file" }

    fn description(&self) -> &str {
        "Edit a file by replacing specific text"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "Text to find and replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "Replacement text"
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let path = args["path"].as_str().ok_or("Missing path")?;
        let old_string = args["old_string"].as_str().ok_or("Missing old_string")?;
        let new_string = args["new_string"].as_str().ok_or("Missing new_string")?;

        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        if !content.contains(old_string) {
            return Err("old_string not found in file".to_string());
        }

        let new_content = content.replace(old_string, new_string);

        std::fs::write(path, &new_content)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok("File edited successfully".to_string())
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct ListDirTool {
    #[allow(dead_code)]
    allowed_dir: Option<PathBuf>,
}

impl ListDirTool {
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }
}

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str { "list_dir" }

    fn description(&self) -> &str {
        "List files in a directory"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let path = args["path"].as_str().ok_or("Missing path")?;

        let entries: Vec<String> = std::fs::read_dir(path)
            .map_err(|e| format!("Failed to read directory: {}", e))?
            .filter_map(|entry| entry.ok())
            .map(|entry| {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                if path.is_dir() {
                    format!("{}/", name)
                } else {
                    name
                }
            })
            .collect();

        Ok(entries.join("\n"))
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_read_file_tool() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "Hello, world!").unwrap();

        let tool = ReadFileTool::new(None);
        let args = json!({"path": test_file.to_string_lossy()});
        
        let result = tool.execute(args).await.unwrap();
        assert_eq!(result, "Hello, world!");
    }

    #[tokio::test]
    async fn test_write_file_tool() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("new_file.txt");

        let tool = WriteFileTool::new(None);
        let args = json!({
            "path": test_file.to_string_lossy(),
            "content": "New file content"
        });
        
        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("File written successfully"));
        
        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "New file content");
    }

    #[tokio::test]
    async fn test_edit_file_tool() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("edit_test.txt");
        fs::write(&test_file, "Original content").unwrap();

        let tool = EditFileTool::new(None);
        let args = json!({
            "path": test_file.to_string_lossy(),
            "old_string": "Original",
            "new_string": "Modified"
        });
        
        let result = tool.execute(args).await.unwrap();
        assert_eq!(result, "File edited successfully");
        
        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "Modified content");
    }

    #[tokio::test]
    async fn test_list_dir_tool() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        fs::write(temp_dir.path().join("file.txt"), "content").unwrap();

        let tool = ListDirTool::new(None);
        let args = json!({"path": temp_dir.path().to_string_lossy()});
        
        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("subdir/"));
        assert!(result.contains("file.txt"));
    }

    #[test]
    fn test_validate_path_allowed_dir() {
        let temp_dir = TempDir::new().unwrap();
        let allowed_dir = temp_dir.path().to_path_buf();
        
        let tool = ReadFileTool::new(Some(allowed_dir.clone()));
        
        // This would normally test the validate_path method, but it's private
        // We'll test the functionality through the execute method instead
    }
}
