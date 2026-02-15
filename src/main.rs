mod agent;
mod bus;
mod channels;
mod config;
mod providers;
mod utils;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tokio::sync::mpsc;

use config::Config;
#[allow(unused_imports)]
use bus::{InboundMessage, OutboundMessage};
use agent::AgentLoop;
use channels::telegram::TelegramChannel;

#[derive(Parser)]
#[command(name = "santosobot")]
#[command(about = "Santoso - Ultra-Lightweight Personal AI Assistant")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Onboard,
    Agent {
        #[arg(short, long)]
        message: Option<String>,
    },
    Gateway,
    Status,
}

fn get_config_path() -> PathBuf {
    let path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("santosobot");
    let _ = std::fs::create_dir_all(&path);
    path.join("config.toml")
}

fn get_workspace_path() -> PathBuf {
    let path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".santosobot/workspace");
    let _ = std::fs::create_dir_all(&path);
    path
}

fn create_default_config(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let default_config = r#"# Santosobot Configuration

[agent]
model = "gpt-4o-mini"
max_tokens = 8192
temperature = 0.7
max_iterations = 20
memory_window = 50

[provider]
api_key = ""
api_base = "https://api.openai.com/v1"
model = ""

[tools]
shell_timeout = 60
restrict_to_workspace = false

[channels.telegram]
enabled = false
token = ""
allow_from = []

[channels.cli]
enabled = true
"#;
    std::fs::write(path, default_config)?;
    Ok(())
}

fn setup_logging() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(filter)
        .init();
}

fn print_banner() {
    println!(r#"
╔═══════════════════════════════════════════════════════════╗
║   🤖 S A N T O S O B O T                                ║
║   Ultra-Lightweight Personal AI Assistant               ║
╠═══════════════════════════════════════════════════════════╣
║   Version: 0.1.0  |  Rust-based  |  OpenAI-compatible   ║
╚═══════════════════════════════════════════════════════════╝
    "#);
}

fn print_success(message: &str) {
    println!("✅ {}", message);
}

fn print_info(message: &str) {
    println!("ℹ️  {}", message);
}

fn print_warning(message: &str) {
    println!("⚠️  {}", message);
}

async fn run_agent_mode(message: Option<String>, config: Config) {
    let (_inbound_tx, inbound_rx) = mpsc::channel(10);
    let (outbound_tx, _outbound_rx) = mpsc::channel(10);
    
    let agent = AgentLoop::new(&config, inbound_rx, outbound_tx);
    
    if let Some(msg) = message {
        match agent.process_direct(&msg).await {
            Ok(response) => println!("\n{}", response),
            Err(e) => eprintln!("❌ Error: {}", e),
        }
    } else {
        println!("\nInteractive mode - Type 'exit' or 'quit' to end\n");
        
        loop {
            print!("You: ");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            let mut input = String::new();
            if std::io::stdin().read_line(&mut input).unwrap() == 0 {
                break;
            }
            
            let input = input.trim();
            if input.is_empty() || input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                break;
            }
            
            match agent.process_direct(input).await {
                Ok(response) => println!("\nSantoso: {}", response),
                Err(e) => eprintln!("\n❌ Error: {}", e),
            }
        }
    }
}

async fn run_gateway_mode(config: Config) {
    print_banner();
    println!();
    
    let (inbound_tx, inbound_rx) = mpsc::channel(100);
    let (outbound_tx, mut outbound_rx) = mpsc::channel(100);
    
    let mut agent = AgentLoop::new(&config, inbound_rx, outbound_tx.clone());
    
    tokio::spawn(async move {
        agent.run().await;
    });
    
    let telegram_enabled = config.channels.telegram.enabled && !config.channels.telegram.token.is_empty();
    
    if telegram_enabled {
        let telegram = TelegramChannel::new(
            config.channels.telegram.token.clone(),
            inbound_tx.clone(),
            config.channels.telegram.allow_from.clone(),
        );
        
        tokio::spawn(async move {
            telegram.start().await;
        });
        
        print_success("Telegram channel started");
    }
    
    let telegram_config = config.channels.telegram;
    
    tokio::spawn(async move {
        while let Some(msg) = outbound_rx.recv().await {
            match msg.channel.as_str() {
                "telegram" => {
                    if telegram_config.enabled && !telegram_config.token.is_empty() {
                        let telegram = TelegramChannel::new(
                            telegram_config.token.clone(),
                            inbound_tx.clone(),
                            telegram_config.allow_from.clone(),
                        );
                        let _ = telegram.send(msg).await;
                    }
                }
                "cli" => println!("\nSantoso: {}", msg.content),
                _ => tracing::warn!("Unknown channel: {}", msg.channel),
            }
        }
    });
    
    println!();
    print_info("Gateway is running...");
    print_info("Press Ctrl+C to stop");
    println!();
    
    tokio::signal::ctrl_c().await.ok();
    print_warning("Gateway stopped");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_logging();
    
    let cli = Cli::parse();
    let config_path = get_config_path();
    
    match cli.command {
        Commands::Onboard => {
            if config_path.exists() {
                println!("Config already exists at {:?}", config_path);
                print!("Do you want to overwrite? (y/N): ");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();
                if !input.trim().eq_ignore_ascii_case("y") {
                    print_warning("Cancelled");
                    return Ok(());
                }
            }
            
            create_default_config(&config_path)?;
            print_success(&format!("Config created at {:?}", config_path));
            
            let workspace = get_workspace_path();
            print_success(&format!("Workspace created at {:?}", workspace));
            
            let bootstrap_files = [
                ("AGENTS.md", "# Agents\n\nYou are a helpful AI assistant."),
                ("SOUL.md", "# Soul\n\nYour core personality and values."),
                ("USER.md", "# User\n\nInformation about the user."),
                ("TOOLS.md", "# Tools\n\nAvailable tools and their descriptions."),
            ];
            
            for (name, content) in bootstrap_files {
                let path = workspace.join(name);
                if !path.exists() {
                    std::fs::write(&path, content)?;
                }
            }
            
            println!("\n🎉 Setup complete! Please edit the config file and add your API key.");
        }
        
        Commands::Agent { message } => {
            if !config_path.exists() {
                eprintln!("❌ Config not found. Run 'santosobot onboard' first.");
                return Ok(());
            }
            
            let config = Config::load(&config_path)?;
            
            if config.provider.api_key.is_empty() {
                eprintln!("❌ API key not configured. Edit {:?} and add your API key.", config_path);
                return Ok(());
            }
            
            if config.provider.model.is_empty() {
                eprintln!("❌ Model not configured. Edit {:?} and add your model.", config_path);
                return Ok(());
            }
            
            run_agent_mode(message, config).await;
        }
        
        Commands::Gateway => {
            if !config_path.exists() {
                eprintln!("❌ Config not found. Run 'santosobot onboard' first.");
                return Ok(());
            }
            
            let config = Config::load(&config_path)?;
            
            if config.provider.api_key.is_empty() {
                eprintln!("❌ API key not configured. Edit {:?} and add your API key.", config_path);
                return Ok(());
            }
            
            run_gateway_mode(config).await;
        }
        
        Commands::Status => {
            if !config_path.exists() {
                print_warning("Not configured. Run 'santosobot onboard' first.");
                return Ok(());
            }
            
            let config = Config::load(&config_path)?;
            
            println!("\n🤖 Santosobot Status");
            println!("═══════════════════════════════════════");
            println!("  Config:     {:?}", config_path);
            println!("  Model:      {}", config.agent.model);
            println!("  Provider:   {}", config.provider.api_base);
            println!("  Telegram:    {}", if config.channels.telegram.enabled { "✅ enabled" } else { "❌ disabled" });
            println!("  CLI:        {}", if config.channels.cli.enabled { "✅ enabled" } else { "❌ disabled" });
            println!("═══════════════════════════════════════\n");
        }
    }
    
    Ok(())
}
