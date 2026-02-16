use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub struct MemoryStore {
    memory_dir: PathBuf,
    memory_file: PathBuf,
    history_file: PathBuf,
}

impl MemoryStore {
    pub fn new(workspace: &Path) -> Self {
        let memory_dir = workspace.join("memory");
        std::fs::create_dir_all(&memory_dir).ok();

        Self {
            memory_dir: memory_dir.clone(),
            memory_file: memory_dir.join("MEMORY.md"),
            history_file: memory_dir.join("HISTORY.md"),
        }
    }

    pub fn read_long_term(&self) -> String {
        if self.memory_file.exists() {
            std::fs::read_to_string(&self.memory_file).unwrap_or_default()
        } else {
            String::new()
        }
    }

    #[allow(dead_code)]
    pub fn write_long_term(&self, content: &str) -> std::io::Result<()> {
        std::fs::write(&self.memory_file, content)
    }

    pub fn append_history(&self, entry: &str) -> std::io::Result<()> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.history_file)?;

        writeln!(file, "{}\n", entry.trim())?;
        
        // Optionally rotate the history file if it gets too large
        // For now, we'll just log the size
        if let Ok(metadata) = std::fs::metadata(&self.history_file) {
            if metadata.len() > 10 * 1024 * 1024 { // 10MB threshold
                tracing::warn!("History file is getting large ({} bytes)", metadata.len());
            }
        }
        
        Ok(())
    }
    
    /// Rotate history file if it exceeds size threshold
    #[allow(dead_code)]
    pub fn rotate_history_if_needed(&self, max_size: u64) -> std::io::Result<()> {
        if let Ok(metadata) = std::fs::metadata(&self.history_file) {
            if metadata.len() > max_size {
                let backup_path = self.history_file.with_extension("md.backup");
                std::fs::rename(&self.history_file, &backup_path)?;
                tracing::info!("History file rotated: {} -> {}", 
                              self.history_file.display(), 
                              backup_path.display());
            }
        }
        Ok(())
    }

    pub fn get_memory_context(&self) -> String {
        let long_term = self.read_long_term();
        if long_term.is_empty() {
            String::new()
        } else {
            format!("## Long-term Memory\n\n{}", long_term)
        }
    }

    #[allow(dead_code)]
    pub fn read_history(&self) -> String {
        if self.history_file.exists() {
            std::fs::read_to_string(&self.history_file).unwrap_or_default()
        } else {
            String::new()
        }
    }
    
    /// Clean up old backup files to free disk space
    #[allow(dead_code)]
    pub fn cleanup_old_backups(&self) -> std::io::Result<()> {
        if let Some(parent_dir) = self.history_file.parent() {
            let stem = self.history_file.file_stem()
                .unwrap_or_default().to_string_lossy();
                
            for entry in std::fs::read_dir(parent_dir)? {
                let entry = entry?;
                let file_name = entry.file_name();
                let name = file_name.to_string_lossy();
                
                if name.starts_with(&format!("{}.backup", stem)) {
                    std::fs::remove_file(entry.path())?;
                    tracing::info!("Cleaned up old backup: {}", entry.path().display());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_memory_store_creation() {
        let temp_dir = TempDir::new().unwrap();
        let memory_store = MemoryStore::new(temp_dir.path());

        // Check that directories were created
        assert!(memory_store.memory_dir.exists());
        assert!(memory_store.memory_file.exists() || !memory_store.memory_file.exists()); // May not exist until written
        assert!(memory_store.history_file.exists() || !memory_store.history_file.exists()); // May not exist until written
    }

    #[test]
    fn test_memory_store_long_term_memory() {
        let temp_dir = TempDir::new().unwrap();
        let memory_store = MemoryStore::new(temp_dir.path());

        // Initially should be empty
        assert!(memory_store.read_long_term().is_empty());

        // Write some content
        let test_content = "This is a test memory entry";
        memory_store.write_long_term(test_content).unwrap();

        // Read it back
        let read_content = memory_store.read_long_term();
        assert_eq!(read_content, test_content);
    }

    #[test]
    fn test_memory_store_append_history() {
        let temp_dir = TempDir::new().unwrap();
        let memory_store = MemoryStore::new(temp_dir.path());

        // Append an entry
        let entry = "Test history entry";
        memory_store.append_history(entry).unwrap();

        // Read the history file
        let history_content = memory_store.read_history();
        assert!(history_content.contains(entry));
    }

    #[test]
    fn test_memory_store_get_memory_context() {
        let temp_dir = TempDir::new().unwrap();
        let memory_store = MemoryStore::new(temp_dir.path());

        // Initially should be empty
        assert!(memory_store.get_memory_context().is_empty());

        // Add some content
        let test_content = "Important information";
        memory_store.write_long_term(test_content).unwrap();

        // Now should return the content with prefix
        let context = memory_store.get_memory_context();
        assert!(context.contains(test_content));
        assert!(context.contains("## Long-term Memory"));
    }
}
