use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use url::Url;
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

    fn validate_url(&self, url_str: &str) -> Result<Url, String> {
        // Basic URL validation
        let url = Url::parse(url_str)
            .map_err(|_| "Invalid URL format".to_string())?;

        // Check scheme
        if url.scheme() != "http" && url.scheme() != "https" {
            return Err("Only http and https schemes are allowed".to_string());
        }

        // Block certain domains/IP ranges that are typically internal
        let host = url.host_str().ok_or("URL must have a host")?;
        
        // Block private IP ranges and localhost
        if host == "localhost" ||
           host.starts_with("127.") ||
           host.starts_with("10.") ||
           host.starts_with("192.168.") ||
           (host.starts_with("172.") && {
               // Check if it's in the 172.16.0.0 - 172.31.255.255 range
               let parts: Vec<&str> = host.split('.').collect();
               if parts.len() >= 2 {
                   if let Ok(second_octet) = parts[1].parse::<u8>() {
                       (16..=31).contains(&second_octet)
                   } else {
                       false
                   }
               } else {
                   false
               }
           }) ||
           host.starts_with("0.") ||
           host.starts_with("169.254.") {
            return Err("Access to local/network addresses is not allowed".to_string());
        }

        // Block URLs with suspicious patterns
        let dangerous_patterns = [
            r"(?i)(admin|root|passwd|shadow|etc|var|proc)",
        ];
        
        for pattern in &dangerous_patterns {
            let re = Regex::new(pattern).map_err(|e| format!("Regex error: {}", e))?;
            if re.is_match(host) {
                return Err(format!("URL contains potentially dangerous pattern: {}", pattern));
            }
        }

        Ok(url)
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

        // Validate the URL
        let validated_url = self.validate_url(url)?;

        let max_length = args["max_length"]
            .as_u64()
            .unwrap_or(10000) as usize;

        let response = self.client
            .get(validated_url)
            .header("User-Agent", "Mozilla/5.0 (compatible; Santosobot/1.0)")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch URL: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        // Limit response size to prevent large downloads
        let content_length = response.content_length().unwrap_or(0);
        if content_length > 10 * 1024 * 1024 { // 10MB limit
            return Err("Response too large (>10MB)".to_string());
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
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[test]
    fn test_validate_url_valid_urls() {
        let tool = WebFetchTool::new();

        // Test valid URLs
        let valid_urls = vec![
            "https://example.com",
            "http://example.com",
            "https://www.rust-lang.org",
            "https://api.github.com/users/octocat",
        ];

        for url in valid_urls {
            let result = tool.validate_url(url);
            assert!(result.is_ok(), "URL '{}' should be valid: {:?}", url, result.err());
        }
    }

    #[test]
    fn test_validate_url_invalid_urls() {
        let tool = WebFetchTool::new();

        // Test invalid URLs
        let invalid_urls = vec![
            "ftp://example.com",  // Invalid scheme
            "javascript:alert('xss')",  // Invalid scheme
            "",  // Empty URL
            "not-a-url",  // Not a URL
        ];

        for url in invalid_urls {
            let result = tool.validate_url(url);
            assert!(result.is_err(), "URL '{}' should be invalid", url);
        }
    }

    #[test]
    fn test_validate_url_local_addresses() {
        let tool = WebFetchTool::new();

        // Test local/network addresses that should be blocked
        let local_urls = vec![
            "http://localhost",
            "https://localhost:8080",
            "http://127.0.0.1",
            "https://10.0.0.1",
            "http://192.168.1.1",
            "https://172.16.0.1",
        ];

        for url in local_urls {
            let result = tool.validate_url(url);
            assert!(result.is_err(), "Local URL '{}' should be blocked", url);
        }
    }

    #[tokio::test]
    async fn test_web_fetch_tool_parameters() {
        let tool = WebFetchTool::new();

        // Check that the parameters are correctly defined
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["url"].is_object());
        assert!(params["properties"]["max_length"].is_object());
        assert!(params["required"][0] == "url");
    }

    #[test]
    fn test_extract_text_basic_html() {
        let html = "<html><head><title>Test</title></head><body><p>Hello world!</p></body></html>";
        let extracted = extract_text(html);
        assert!(extracted.contains("Hello world!"));
        assert!(!extracted.contains("<p>"));
        assert!(!extracted.contains("</p>"));
    }

    #[test]
    fn test_extract_text_with_script_and_style() {
        let html = r#"
        <html>
            <head>
                <style>body { color: red; }</style>
            </head>
            <body>
                <script>alert('test');</script>
                <p>Main content here</p>
            </body>
        </html>"#;
        
        let extracted = extract_text(html);
        assert!(extracted.contains("Main content here"));
        assert!(!extracted.contains("alert"));
        assert!(!extracted.contains("color: red"));
    }
}
