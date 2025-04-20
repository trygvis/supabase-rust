//! HTTP client abstraction for making requests to Supabase services

use reqwest::{Client, RequestBuilder, Method, header::{HeaderMap, HeaderValue}};
use serde::{Serialize, de::DeserializeOwned};
use crate::error::Error;
use std::collections::HashMap;
use url::Url;

/// Helper for building and executing HTTP requests
pub struct FetchBuilder<'a> {
    client: &'a Client,
    url: String,
    method: Method,
    headers: HeaderMap,
    query_params: Option<HashMap<String, String>>,
    body: Option<Vec<u8>>,
}

impl<'a> FetchBuilder<'a> {
    /// Create a new FetchBuilder
    pub fn new(client: &'a Client, url: &str, method: Method) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        
        Self {
            client,
            url: url.to_string(),
            method,
            headers,
            query_params: None,
            body: None,
        }
    }
    
    /// Add a header to the request
    pub fn header(mut self, name: &str, value: &str) -> Self {
        if let Ok(value) = HeaderValue::from_str(value) {
            self.headers.insert(name, value);
        }
        self
    }
    
    /// Add bearer token authentication to the request
    pub fn bearer_auth(self, token: &str) -> Self {
        self.header("Authorization", &format!("Bearer {}", token))
    }
    
    /// Add query parameters to the request
    pub fn query(mut self, params: HashMap<String, String>) -> Self {
        self.query_params = Some(params);
        self
    }
    
    /// Add a JSON body to the request
    pub fn json<T: Serialize>(mut self, body: &T) -> Result<Self, Error> {
        let json = serde_json::to_vec(body)?;
        self.body = Some(json);
        Ok(self)
    }
    
    /// Build the request
    fn build(&self) -> Result<RequestBuilder, Error> {
        let mut url = Url::parse(&self.url)?;
        
        // Add query parameters if present
        if let Some(params) = &self.query_params {
            let mut query_pairs = url.query_pairs_mut();
            for (key, value) in params {
                query_pairs.append_pair(key, value);
            }
        }
        
        let mut req = self.client.request(self.method.clone(), url.as_str());
        req = req.headers(self.headers.clone());
        
        if let Some(body) = &self.body {
            req = req.body(body.clone());
        }
        
        Ok(req)
    }
    
    /// Execute the request and parse the response as JSON
    pub async fn execute<T: DeserializeOwned>(&self) -> Result<T, Error> {
        let req = self.build()?;
        let response = req.send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(Error::general(format!("Request failed with status {}: {}", status, text)));
        }
        
        let result = response.json::<T>().await?;
        Ok(result)
    }
    
    /// Execute the request and return the raw response
    pub async fn execute_raw(&self) -> Result<reqwest::Response, Error> {
        let req = self.build()?;
        let response = req.send().await?;
        Ok(response)
    }
}

/// Helper for creating HTTP requests
pub struct Fetch;

impl Fetch {
    /// Create a GET request
    pub fn get<'a>(client: &'a Client, url: &str) -> FetchBuilder<'a> {
        FetchBuilder::new(client, url, Method::GET)
    }
    
    /// Create a POST request
    pub fn post<'a>(client: &'a Client, url: &str) -> FetchBuilder<'a> {
        FetchBuilder::new(client, url, Method::POST)
    }
    
    /// Create a PUT request
    pub fn put<'a>(client: &'a Client, url: &str) -> FetchBuilder<'a> {
        FetchBuilder::new(client, url, Method::PUT)
    }
    
    /// Create a PATCH request
    pub fn patch<'a>(client: &'a Client, url: &str) -> FetchBuilder<'a> {
        FetchBuilder::new(client, url, Method::PATCH)
    }
    
    /// Create a DELETE request
    pub fn delete<'a>(client: &'a Client, url: &str) -> FetchBuilder<'a> {
        FetchBuilder::new(client, url, Method::DELETE)
    }
}