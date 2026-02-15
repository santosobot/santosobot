use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelsConfig {
    #[serde(default)]
    pub telegram: TelegramConfig,
    #[serde(default)]
    pub cli: CliConfig,
}

impl Default for ChannelsConfig {
    fn default() -> Self {
        Self {
            telegram: TelegramConfig::default(),
            cli: CliConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token: String::new(),
            allow_from: Vec::new(),
        }
    }
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
