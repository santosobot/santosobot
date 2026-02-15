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
            let dir_canonical = dir.canonicalize()
                .map_err(|e| format!("Invalid workspace: {}", e))?;
            
            if !path.canonicalize().unwrap_or(path.clone()).starts_with(&dir_canonical) {
                return Err("Path outside workspace not allowed".to_string());
            }
        }
        
        Ok(path)
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
}
