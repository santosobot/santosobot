use std::io::{self, Write};
use tokio::sync::mpsc;
use crate::bus::{InboundMessage, OutboundMessage};

#[allow(dead_code)]
pub struct CliChannel {
    outbound_tx: mpsc::Sender<OutboundMessage>,
}

impl CliChannel {
    #[allow(dead_code)]
    pub fn new(outbound_tx: mpsc::Sender<OutboundMessage>) -> Self {
        Self { outbound_tx }
    }

    #[allow(dead_code)]
    pub async fn run(&self) {
        println!("Santoso CLI - Type 'exit' or 'quit' to end the session");
        println!("----------------------------------------------------------------");
        
        loop {
            print!("> ");
            io::stdout().flush().unwrap();
            
            let mut input = String::new();
            if io::stdin().read_line(&mut input).unwrap() == 0 {
                break;
            }
            
            let input = input.trim();
            if input.is_empty() {
                continue;
            }
            
            if input.eq_ignore_ascii_case("exit") 
                || input.eq_ignore_ascii_case("quit")
                || input.eq_ignore_ascii_case("/exit")
                || input.eq_ignore_ascii_case("/quit") {
                break;
            }
            
            let _msg = InboundMessage::new(
                "cli".to_string(),
                "user".to_string(),
                "cli".to_string(),
                input.to_string(),
            );
            
            // For CLI, we handle responses directly
            println!("\n[Waiting for response...]\n");
        }
        
        println!("Goodbye!");
    }

    #[allow(dead_code)]
    pub async fn send(&self, msg: OutboundMessage) -> Result<(), String> {
        println!("{}", msg.content);
        Ok(())
    }
}
