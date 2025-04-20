//! Supabase PostgREST client for Rust
//!
//! This crate provides database functionality for Supabase,
//! allowing for querying, filtering, and manipulating data in PostgreSQL.

use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;
use url::Url;

/// エラー型
#[derive(Error, Debug)]
pub enum PostgrestError {
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
    
    #[error("JSON serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
}

/// ソート方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

/// PostgreST クライアント
pub struct PostgrestClient {
    base_url: String,
    api_key: String,
    table: String,
    http_client: Client,
    headers: HeaderMap,
    query_params: HashMap<String, String>,
    path: Option<String>,
}

impl PostgrestClient {
    /// 新しい PostgreST クライアントを作成
    pub fn new(base_url: &str, api_key: &str, table: &str, http_client: Client) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("apikey", HeaderValue::from_str(api_key).unwrap());
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        
        Self {
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            table: table.to_string(),
            http_client,
            headers,
            query_params: HashMap::new(),
            path: None,
        }
    }
    
    /// ヘッダーを追加
    pub fn with_header(mut self, key: &str, value: &str) -> Result<Self, PostgrestError> {
        let header_value = HeaderValue::from_str(value)
            .map_err(|_| PostgrestError::InvalidParameters(format!("Invalid header value: {}", value)))?;
        
        self.headers.insert(key, header_value);
        Ok(self)
    }
    
    /// 認証トークンを設定
    pub fn with_auth(self, token: &str) -> Result<Self, PostgrestError> {
        self.with_header("Authorization", &format!("Bearer {}", token))
    }
    
    /// 取得するカラムを指定
    pub fn select(mut self, columns: &str) -> Self {
        self.query_params.insert("select".to_string(), columns.to_string());
        self
    }
    
    /// 等価フィルター
    pub fn eq(mut self, column: &str, value: &str) -> Self {
        self.query_params.insert(column.to_string(), format!("eq.{}", value));
        self
    }
    
    /// より大きいフィルター
    pub fn gt(mut self, column: &str, value: &str) -> Self {
        self.query_params.insert(column.to_string(), format!("gt.{}", value));
        self
    }
    
    /// 以上フィルター
    pub fn gte(mut self, column: &str, value: &str) -> Self {
        self.query_params.insert(column.to_string(), format!("gte.{}", value));
        self
    }
    
    /// より小さいフィルター
    pub fn lt(mut self, column: &str, value: &str) -> Self {
        self.query_params.insert(column.to_string(), format!("lt.{}", value));
        self
    }
    
    /// 以下フィルター
    pub fn lte(mut self, column: &str, value: &str) -> Self {
        self.query_params.insert(column.to_string(), format!("lte.{}", value));
        self
    }
    
    /// LIKE フィルター
    pub fn like(mut self, column: &str, pattern: &str) -> Self {
        self.query_params.insert(column.to_string(), format!("like.{}", pattern));
        self
    }
    
    /// ILIKE フィルター（大文字小文字を区別しない）
    pub fn ilike(mut self, column: &str, pattern: &str) -> Self {
        self.query_params.insert(column.to_string(), format!("ilike.{}", pattern));
        self
    }
    
    /// IN フィルター
    pub fn in_list(mut self, column: &str, values: &[&str]) -> Self {
        let value_list = values.join(",");
        self.query_params.insert(column.to_string(), format!("in.({})", value_list));
        self
    }
    
    /// NOT フィルター
    pub fn not(mut self, column: &str, operator_with_value: &str) -> Self {
        self.query_params.insert(column.to_string(), format!("not.{}", operator_with_value));
        self
    }
    
    /// ソート順を指定
    pub fn order(mut self, column: &str, order: SortOrder) -> Self {
        let order_str = match order {
            SortOrder::Ascending => "asc",
            SortOrder::Descending => "desc",
        };
        self.query_params.insert("order".to_string(), format!("{}:{}", column, order_str));
        self
    }
    
    /// 取得件数を制限
    pub fn limit(mut self, count: i32) -> Self {
        self.query_params.insert("limit".to_string(), count.to_string());
        self
    }
    
    /// オフセットを指定
    pub fn offset(mut self, count: i32) -> Self {
        self.query_params.insert("offset".to_string(), count.to_string());
        self
    }
    
    /// データを取得
    pub async fn execute<T: for<'de> Deserialize<'de>>(&self) -> Result<Vec<T>, PostgrestError> {
        let url = self.build_url()?;
        
        let response = self.http_client.get(url)
            .headers(self.headers.clone())
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(PostgrestError::ApiError(error_text));
        }
        
        let data = response.json::<Vec<T>>().await?;
        
        Ok(data)
    }
    
    /// データを挿入
    pub async fn insert<T: Serialize>(&self, values: T) -> Result<Value, PostgrestError> {
        let url = self.build_url()?;
        
        let response = self.http_client.post(url)
            .headers(self.headers.clone())
            .json(&values)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(PostgrestError::ApiError(error_text));
        }
        
        let data = response.json::<Value>().await?;
        
        Ok(data)
    }
    
    /// データを更新
    pub async fn update<T: Serialize>(&self, values: T) -> Result<Value, PostgrestError> {
        let url = self.build_url()?;
        
        // PATCH メソッドで更新
        let response = self.http_client.patch(url)
            .headers(self.headers.clone())
            .json(&values)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(PostgrestError::ApiError(error_text));
        }
        
        let data = response.json::<Value>().await?;
        
        Ok(data)
    }
    
    /// データを削除
    pub async fn delete(&self) -> Result<Value, PostgrestError> {
        let url = self.build_url()?;
        
        let response = self.http_client.delete(url)
            .headers(self.headers.clone())
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(PostgrestError::ApiError(error_text));
        }
        
        let data = response.json::<Value>().await?;
        
        Ok(data)
    }
    
    // URLを構築
    fn build_url(&self) -> Result<String, PostgrestError> {
        let mut url = Url::parse(&format!("{}/rest/v1/{}", self.base_url, self.table))?;
        
        for (key, value) in &self.query_params {
            url.query_pairs_mut().append_pair(key, value);
        }
        
        Ok(url.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, query_param};
    use serde_json::json;
    
    #[tokio::test]
    async fn test_select() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/rest/v1/users"))
            .and(query_param("select", "id,name"))
            .and(query_param("limit", "10"))
            .respond_with(ResponseTemplate::new(200).json(json!([
                {"id": 1, "name": "User 1"},
                {"id": 2, "name": "User 2"}
            ])))
            .mount(&mock_server)
            .await;
        
        let client = PostgrestClient::new(
            &mock_server.uri(),
            "test_key",
            "users",
            Client::new(),
        );
        
        let result: Vec<HashMap<String, Value>> = client
            .select("id,name")
            .limit(10)
            .execute()
            .await
            .unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].get("name").unwrap().as_str().unwrap(), "User 1");
        assert_eq!(result[1].get("name").unwrap().as_str().unwrap(), "User 2");
    }
}