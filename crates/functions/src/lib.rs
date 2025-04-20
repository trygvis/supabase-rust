//! Supabase Edge Functions client for Rust
//!
//! This crate provides functionality for invoking Supabase Edge Functions.

use reqwest::Client;
use std::sync::Arc;
use serde::Serialize;
use serde_json::{json, Value};
use thiserror::Error;
use url::Url;

/// エラー型
#[derive(Debug, Error)]
pub enum FunctionsError {
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
    
    #[error("URL parse error: {0}")]
    UrlError(#[from] url::ParseError),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("Function error: {0}")]
    FunctionError(String),
}

pub type Result<T> = std::result::Result<T, FunctionsError>;

/// Edge Functions クライアント
pub struct FunctionsClient {
    base_url: String,
    api_key: String,
    http_client: Client,
}

impl FunctionsClient {
    /// 新しい Edge Functions クライアントを作成
    pub fn new(supabase_url: &str, supabase_key: &str, http_client: Client) -> Self {
        Self {
            base_url: supabase_url.to_string(),
            api_key: supabase_key.to_string(),
            http_client,
        }
    }
    
    /// Edge Function を呼び出す
    pub async fn invoke<T: Serialize>(
        &self,
        function_name: &str,
        body: Option<T>,
        options: Option<FunctionOptions>,
    ) -> Result<Value> {
        let mut url = Url::parse(&self.base_url)?;
        url.path_segments_mut()
            .map_err(|_| FunctionsError::UrlError(url::ParseError::EmptyHost))?
            .push("functions")
            .push("v1")
            .push(function_name);
        
        let opts = options.unwrap_or_default();
        
        let mut request = self.http_client
            .post(url)
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", &self.api_key));
        
        // Add custom headers if provided
        if let Some(headers) = opts.headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }
        
        // Add body if provided
        if let Some(body_data) = body {
            request = request.json(&body_data);
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(FunctionsError::FunctionError(error_text));
        }
        
        let json_response = response.json::<Value>().await?;
        Ok(json_response)
    }
}

#[derive(Default)]
pub struct FunctionOptions {
    pub headers: Option<std::collections::HashMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, header};
    use serde_json::json;
    
    #[tokio::test]
    async fn test_invoke() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/functions/v1/hello-world"))
            .and(header("apikey", "test_key"))
            .respond_with(ResponseTemplate::new(200).json(json!({
                "message": "Hello, World!"
            })))
            .mount(&mock_server)
            .await;
        
        let client = FunctionsClient::new(
            &mock_server.uri(),
            "test_key",
            Client::new(),
        );
        
        #[derive(Deserialize)]
        struct Response {
            message: String,
        }
        
        let result = client.invoke::<Response>("hello-world", None, None).await.unwrap();
        
        assert_eq!(result["message"].as_str(), Some("Hello, World!"));
    }
}