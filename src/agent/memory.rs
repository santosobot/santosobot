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

        writeln!(file, "{}\n", entry.trim())
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
}
