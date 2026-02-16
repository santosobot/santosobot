use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub agent: AgentConfig,

    #[serde(default)]
    pub provider: ProviderConfig,

    #[serde(default)]
    pub tools: ToolsConfig,

    #[serde(default)]
    pub channels: ChannelsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    #[serde(default = "default_memory_window")]
    pub memory_window: u32,
    #[serde(default = "default_workspace")]
    pub workspace: String,
}

fn default_max_tokens() -> u32 {
    8192
}
fn default_temperature() -> f32 {
    0.7
}
fn default_max_iterations() -> u32 {
    20
}
fn default_memory_window() -> u32 {
    50
}
fn default_workspace() -> String {
    "~/.santosobot/workspace".to_string()
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o-mini".to_string(),
            max_tokens: 8192,
            temperature: 0.7,
            max_iterations: 20,
            memory_window: 50,
            workspace: "~/.santosobot/workspace".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key: String,
    #[serde(default = "default_api_base")]
    pub api_base: String,
    pub model: String,
    #[serde(default)]
    pub brave_api_key: String,
}

fn default_api_base() -> String {
    "https://api.openai.com/v1".to_string()
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            api_base: "https://api.openai.com/v1".to_string(),
            model: String::new(),
            brave_api_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    #[serde(default = "default_shell_timeout")]
    pub shell_timeout: u64,
    #[serde(default)]
    pub restrict_to_workspace: bool,
}

fn default_shell_timeout() -> u64 {
    60
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            shell_timeout: 60,
            restrict_to_workspace: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelsConfig {
    #[serde(default)]
    pub telegram: TelegramConfig,
    #[serde(default)]
    pub cli: CliConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelegramConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl Default for CliConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl Config {
    pub fn load(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn workspace_path(&self) -> PathBuf {
        let path = self.agent.workspace.replace(
            "~",
            &dirs::home_dir().unwrap_or_default().display().to_string(),
        );
        PathBuf::from(path)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_config_defaults() {
        let config = super::Config::default();
        
        assert_eq!(config.agent.model, "gpt-4o-mini");
        assert_eq!(config.agent.max_tokens, 8192);
        assert_eq!(config.agent.temperature, 0.7);
        assert_eq!(config.agent.max_iterations, 20);
        assert_eq!(config.agent.memory_window, 50);
        assert_eq!(config.agent.workspace, "~/.santosobot/workspace");
        
        assert_eq!(config.provider.api_base, "https://api.openai.com/v1");
        assert!(config.provider.api_key.is_empty());
        assert!(config.provider.model.is_empty());
        assert!(config.provider.brave_api_key.is_empty());
        
        assert_eq!(config.tools.shell_timeout, 60);
        assert!(!config.tools.restrict_to_workspace);
        
        assert!(!config.channels.telegram.enabled);
        assert!(config.channels.telegram.token.is_empty());
        assert!(config.channels.telegram.allow_from.is_empty());
        
        assert!(config.channels.cli.enabled);
    }

    #[test]
    fn test_workspace_path_expansion() {
        let mut config = super::Config::default();
        config.agent.workspace = "~/test_workspace".to_string();
        
        let path = config.workspace_path();
        let home_dir = dirs::home_dir().unwrap_or_default();
        let expected = home_dir.join("test_workspace");
        
        assert_eq!(path, expected);
    }

    #[test]
    fn test_load_config_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"# Santosobot Configuration
[agent]
model = "gpt-4-test"
max_tokens = 4096
temperature = 0.5
max_iterations = 10
memory_window = 25

[provider]
api_key = "test-key-123"
api_base = "https://test-api.example.com/v1"
model = "test-model"
brave_api_key = "test-brave-key"

[tools]
shell_timeout = 30
restrict_to_workspace = true

[channels.telegram]
enabled = true
token = "test-token"
allow_from = ["123456789"]

[channels.cli]
enabled = false
"#;
        
        std::fs::write(&config_path, config_content).unwrap();
        
        let config = super::Config::load(&config_path).unwrap();
        
        assert_eq!(config.agent.model, "gpt-4-test");
        assert_eq!(config.agent.max_tokens, 4096);
        assert_eq!(config.agent.temperature, 0.5);
        assert_eq!(config.agent.max_iterations, 10);
        assert_eq!(config.agent.memory_window, 25);
        
        assert_eq!(config.provider.api_key, "test-key-123");
        assert_eq!(config.provider.api_base, "https://test-api.example.com/v1");
        assert_eq!(config.provider.model, "test-model");
        assert_eq!(config.provider.brave_api_key, "test-brave-key");
        
        assert_eq!(config.tools.shell_timeout, 30);
        assert!(config.tools.restrict_to_workspace);
        
        assert!(config.channels.telegram.enabled);
        assert_eq!(config.channels.telegram.token, "test-token");
        assert_eq!(config.channels.telegram.allow_from, vec!["123456789"]);
        
        assert!(!config.channels.cli.enabled);
    }
}
