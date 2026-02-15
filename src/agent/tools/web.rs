use async_trait::async_trait;
use serde_json::{json, Value};
use reqwest::Client;
use crate::agent::tools::Tool;

pub struct WebFetchTool {
    client: Client,
}

impl WebFetchTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str { "web_fetch" }
    
    fn description(&self) -> &str {
        "Fetch content from a URL"
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to fetch"
                },
                "max_length": {
                    "type": "integer",
                    "description": "Maximum characters to return",
                    "default": 10000
                }
            },
            "required": ["url"]
        })
    }
    
    async fn execute(&self, args: Value) -> Result<String, String> {
        let url = args["url"]
            .as_str()
            .ok_or("Missing url parameter")?;
        
        let max_length = args["max_length"]
            .as_u64()
            .unwrap_or(10000) as usize;
        
        let response = self.client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (compatible; Santosobot/1.0)")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch URL: {}", e))?;
        
        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }
        
        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;
        
        let text = extract_text(&text);
        
        if text.len() > max_length {
            return Ok(format!("{}...[truncated]", &text[..max_length]));
        }
        
        Ok(text)
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

fn extract_text(html: &str) -> String {
    let mut result = String::new();
    let mut in_script = false;
    let mut in_style = false;
    
    let chars: Vec<char> = html.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        if chars[i..].starts_with(&['<', 's', 'c', 'r', 'i', 'p', 't'][..]) {
            in_script = true;
        } else if chars[i..].starts_with(&['<', '/', 's', 'c', 'r', 'i', 'p', 't'][..]) {
            in_script = false;
        } else if chars[i..].starts_with(&['<', 's', 't', 'y', 'l', 'e'][..]) {
            in_style = true;
        } else if chars[i..].starts_with(&['<', '/', 's', 't', 'y', 'l', 'e'][..]) {
            in_style = false;
        } else if chars[i] == '<' {
            if let Some(end) = chars[i..].iter().position(|&c| c == '>') {
                i += end + 1;
                continue;
            }
        }
        
        if !in_script && !in_style {
            result.push(chars[i]);
        }
        
        i += 1;
    }
    
    result
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
