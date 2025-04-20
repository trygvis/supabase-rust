//! Edge Functions client for Supabase

use reqwest::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;

use crate::error::Error;
use crate::fetch::Fetch;

/// Client for Supabase Edge Functions
pub struct FunctionsClient {
    /// The base URL for the Supabase project
    url: String,
    
    /// The anonymous API key for the Supabase project
    key: String,
    
    /// HTTP client
    client: Client,
}

/// Response from an Edge Function
#[derive(Debug, Clone)]
pub struct FunctionResponse<T> {
    /// Response data
    pub data: T,
    
    /// Response status
    pub status: u16,
    
    /// Response status text
    pub status_text: String,
    
    /// Response headers
    pub headers: HashMap<String, String>,
}

impl FunctionsClient {
    /// Create a new FunctionsClient
    pub(crate) fn new(url: &str, key: &str, client: Client) -> Self {
        Self {
            url: url.to_string(),
            key: key.to_string(),
            client,
        }
    }
    
    /// Get the base URL for edge function operations
    fn get_url(&self, function_name: &str) -> String {
        format!("{}/functions/v1/{}", self.url, function_name)
    }
    
    /// Invoke an edge function
    pub async fn invoke<T: Serialize, R: DeserializeOwned>(
        &self,
        function_name: &str,
        invoke_options: &FunctionInvokeOptions<T>,
    ) -> Result<FunctionResponse<R>, Error> {
        let url = self.get_url(function_name);
        
        let mut fetch = Fetch::post(&self.client, &url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0");
        
        // Add custom headers if provided
        if let Some(headers) = &invoke_options.headers {
            for (key, value) in headers {
                fetch = fetch.header(key, value);
            }
        }
        
        // Add authorization header if provided
        if let Some(token) = &invoke_options.authorization {
            fetch = fetch.header("Authorization", &format!("Bearer {}", token));
        }
        
        // Add body if provided
        let fetch = if let Some(body) = &invoke_options.body {
            fetch.json(body)?
        } else {
            fetch
        };
        
        let response = fetch.execute_raw().await?;
        
        let status = response.status().as_u16();
        let status_text = response.status().to_string();
        
        // Extract headers
        let headers = response.headers().iter()
            .map(|(key, value)| (key.to_string(), value.to_str().unwrap_or("").to_string()))
            .collect();
        
        if !response.status().is_success() {
            let text = response.text().await?;
            return Err(Error::function(format!("Function {} failed with status {}: {}", function_name, status, text)));
        }
        
        let data = response.json::<R>().await?;
        
        Ok(FunctionResponse {
            data,
            status,
            status_text,
            headers,
        })
    }
}

/// Options for invoking an edge function
#[derive(Debug, Clone)]
pub struct FunctionInvokeOptions<T> {
    /// Request body
    pub body: Option<T>,
    
    /// Request headers
    pub headers: Option<HashMap<String, String>>,
    
    /// Authorization token
    pub authorization: Option<String>,
}

impl<T> Default for FunctionInvokeOptions<T> {
    fn default() -> Self {
        Self {
            body: None,
            headers: None,
            authorization: None,
        }
    }
}

impl<T> FunctionInvokeOptions<T> {
    /// Create new empty invoke options
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the request body
    pub fn with_body(mut self, body: T) -> Self {
        self.body = Some(body);
        self
    }
    
    /// Set a request header
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        let headers = self.headers.get_or_insert_with(HashMap::new);
        headers.insert(key.to_string(), value.to_string());
        self
    }
    
    /// Set the authorization token
    pub fn with_auth(mut self, token: &str) -> Self {
        self.authorization = Some(token.to_string());
        self
    }
}