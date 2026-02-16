use async_trait::async_trait;
use serde_json::{json, Value};
use reqwest::Client;
use crate::agent::tools::Tool;

pub struct BraveSearchTool {
    client: Client,
    api_key: String,
}

impl BraveSearchTool {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            api_key,
        }
    }

    fn validate_query(&self, query: &str) -> Result<String, String> {
        // Basic validation
        if query.is_empty() {
            return Err("Query cannot be empty".to_string());
        }

        if query.len() > 500 {
            return Err("Query too long (max 500 characters)".to_string());
        }

        // Sanitize query - remove potentially harmful characters
        let sanitized = query.trim();
        if sanitized.contains('\0') {
            return Err("Query contains null characters".to_string());
        }

        Ok(sanitized.to_string())
    }
}

#[async_trait]
impl Tool for BraveSearchTool {
    fn name(&self) -> &str { "brave_search" }

    fn description(&self) -> &str {
        "Search the web using Brave Search API to find relevant information"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query to find information about"
                },
                "count": {
                    "type": "integer",
                    "description": "Number of results to return (default: 5, max: 10)",
                    "minimum": 1,
                    "maximum": 10
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let query = args["query"]
            .as_str()
            .ok_or("Missing query parameter")?;

        // Validate the query
        let validated_query = self.validate_query(query)?;

        let count = args["count"]
            .as_u64()
            .unwrap_or(5)
            .min(10) as usize; // Max 10 results

        // Construct the API request
        let url = "https://api.search.brave.com/res/v1/web/search";
        
        let response = self.client
            .get(url)
            .header("X-Subscription-Token", &self.api_key)
            .header("Accept", "application/json")
            .query(&[("q", &validated_query)])
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(format!("Brave Search API error: {} - {}", status, error_body));
        }

        let search_results: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Extract relevant information from the response
        let mut results = Vec::new();
        
        if let Some(web_results) = search_results.pointer("/web/results").and_then(|v| v.as_array()) {
            for (index, result) in web_results.iter().enumerate() {
                if index >= count {
                    break;
                }

                let title = result.pointer("/title").and_then(|v| v.as_str()).unwrap_or("No title");
                let url = result.pointer("/url").and_then(|v| v.as_str()).unwrap_or("No URL");
                let description = result.pointer("/description").and_then(|v| v.as_str()).unwrap_or("No description");

                results.push(format!(
                    "Title: {}\nURL: {}\nDescription: {}\n",
                    title, url, description
                ));
            }
        }

        if results.is_empty() {
            Ok("No search results found for the given query.".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Default for BraveSearchTool {
    fn default() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            api_key: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brave_search_tool_parameters() {
        let tool = BraveSearchTool::new("test-key".to_string());

        // Check that the parameters are correctly defined
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["query"].is_object());
        assert!(params["properties"]["count"].is_object());
        assert!(params["required"][0] == "query");
    }

    #[test]
    fn test_validate_query_valid() {
        let tool = BraveSearchTool::new("test-key".to_string());

        // Test valid queries
        let valid_queries = vec![
            "hello world",
            "Rust programming language",
            "weather forecast",
        ];

        for query in valid_queries {
            let result = tool.validate_query(query);
            assert!(result.is_ok(), "Query '{}' should be valid: {:?}", query, result.err());
        }
    }

    #[test]
    fn test_validate_query_invalid() {
        let tool = BraveSearchTool::new("test-key".to_string());

        // Test invalid queries
        let long_query = "a".repeat(501); // Too long query
        let invalid_queries = vec![
            ("", "Empty query"),
            (&long_query, "Too long query"),
        ];

        for (query, desc) in invalid_queries {
            let result = tool.validate_query(query);
            assert!(result.is_err(), "Query '{}' ({}) should be invalid", query, desc);
        }
    }

    #[test]
    fn test_validate_query_null_bytes() {
        let tool = BraveSearchTool::new("test-key".to_string());

        // Test query with null bytes
        let query_with_null = "hello\0world";
        let result = tool.validate_query(query_with_null);
        assert!(result.is_err());
    }
}