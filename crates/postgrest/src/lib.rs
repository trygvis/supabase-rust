//! Supabase PostgREST client for Rust
//!
//! This crate provides database functionality for Supabase,
//! allowing for querying, filtering, and manipulating data in PostgreSQL.

use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue, HeaderName};
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
    is_rpc: bool,
    rpc_params: Option<Value>,
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
            is_rpc: false,
            rpc_params: None,
        }
    }

    /// RPCリクエストを作成
    pub fn rpc(base_url: &str, api_key: &str, function_name: &str, params: Value, http_client: Client) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("apikey", HeaderValue::from_str(api_key).unwrap());
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        
        Self {
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            table: function_name.to_string(),
            http_client,
            headers,
            query_params: HashMap::new(),
            path: None,
            is_rpc: true,
            rpc_params: Some(params),
        }
    }
    
    /// ヘッダーを追加
    pub fn with_header(mut self, key: &str, value: &str) -> Result<Self, PostgrestError> {
        let header_value = HeaderValue::from_str(value)
            .map_err(|_| PostgrestError::InvalidParameters(format!("Invalid header value: {}", value)))?;
        
        // ヘッダー名を文字列として所有し、HeaderNameに変換する
        let header_name = HeaderName::from_bytes(key.as_bytes())
            .map_err(|_| PostgrestError::InvalidParameters(format!("Invalid header name: {}", key)))?;
        
        self.headers.insert(header_name, header_value);
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
        if self.is_rpc {
            // RPCの場合は単一の結果を配列に入れて返す
            let result = self.execute_rpc().await?;
            let single_result: T = serde_json::from_value(result)?;
            return Ok(vec![single_result]);
        }

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
    
    /// RPC関数を実行
    async fn execute_rpc(&self) -> Result<Value, PostgrestError> {
        let url = format!("{}/rest/v1/rpc/{}", self.base_url, self.table);
        
        let response = self.http_client.post(url)
            .headers(self.headers.clone())
            .json(&self.rpc_params.clone().unwrap_or(Value::Null))
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(PostgrestError::ApiError(error_text));
        }
        
        let data = response.json::<Value>().await?;
        
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
    
    #[tokio::test]
    async fn test_select() {
        // TODO: モック実装を用いたテスト
    }

    #[tokio::test]
    async fn test_rpc() {
        // TODO: モック実装を用いたテスト
    }
}