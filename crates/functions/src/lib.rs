//! Supabase Edge Functions client for Rust
//!
//! This crate provides functionality for invoking Supabase Edge Functions.

use reqwest::{Client, Response, StatusCode};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use serde_json::Value;
use thiserror::Error;
use url::Url;
use std::collections::HashMap;
use std::time::Duration;
use futures_util::{Stream, StreamExt};
use std::pin::Pin;
use bytes::{Bytes, BytesMut, BufMut};

/// エラー型の詳細
#[derive(Debug, Clone, Deserialize)]
pub struct FunctionErrorDetails {
    pub message: Option<String>,
    pub status: Option<u16>,
    pub code: Option<String>,
    pub details: Option<Value>,
}

/// エラー型
#[derive(Debug, Error)]
pub enum FunctionsError {
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
    
    #[error("URL parse error: {0}")]
    UrlError(#[from] url::ParseError),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("Function error (status: {status}): {message}")]
    FunctionError {
        message: String,
        status: StatusCode,
        details: Option<FunctionErrorDetails>,
    },
    
    #[error("Timeout error: Function execution exceeded timeout limit")]
    TimeoutError,
    
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

impl FunctionsError {
    pub fn new(message: String) -> Self {
        Self::FunctionError {
            message,
            status: StatusCode::INTERNAL_SERVER_ERROR,
            details: None,
        }
    }
    
    pub fn from_response(response: &Response) -> Self {
        Self::FunctionError {
            message: format!("Function returned error status: {}", response.status()),
            status: response.status(),
            details: None,
        }
    }
    
    pub fn with_details(response: &Response, details: FunctionErrorDetails) -> Self {
        Self::FunctionError {
            message: details.message.unwrap_or_else(|| format!("Function returned error status: {}", response.status())),
            status: response.status(),
            details: Some(details),
        }
    }
}

pub type Result<T> = std::result::Result<T, FunctionsError>;

/// 関数呼び出しオプション
#[derive(Clone, Debug)]
pub struct FunctionOptions {
    /// カスタムHTTPヘッダー
    pub headers: Option<HashMap<String, String>>,
    
    /// 関数タイムアウト（秒）
    pub timeout_seconds: Option<u64>,
    
    /// レスポンスのコンテンツタイプを指定（デフォルトはJSONとして処理）
    pub response_type: ResponseType,
    
    /// リクエストのコンテンツタイプ
    pub content_type: Option<String>,
}

impl Default for FunctionOptions {
    fn default() -> Self {
        Self {
            headers: None,
            timeout_seconds: None,
            response_type: ResponseType::Json,
            content_type: None,
        }
    }
}

/// レスポンスの処理方法
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseType {
    /// JSONとしてパースする（デフォルト）
    Json,
    
    /// テキストとして処理する
    Text,
    
    /// バイトデータとして処理する
    Binary,
    
    /// ストリームとして処理する
    Stream,
}

/// ストリーミングレスポンス用の型
pub type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>;

/// 関数レスポンス
#[derive(Debug, Clone)]
pub struct FunctionResponse<T> {
    /// レスポンスデータ
    pub data: T,
    
    /// HTTPステータスコード
    pub status: StatusCode,
    
    /// レスポンスヘッダー
    pub headers: HashMap<String, String>,
}

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
    pub async fn invoke<T: DeserializeOwned, B: Serialize>(
        &self,
        function_name: &str,
        body: Option<B>,
        options: Option<FunctionOptions>,
    ) -> Result<FunctionResponse<T>> {
        let opts = options.unwrap_or_default();
        
        // URLの構築
        let mut url = Url::parse(&self.base_url)?;
        url.path_segments_mut()
            .map_err(|_| FunctionsError::UrlError(url::ParseError::EmptyHost))?
            .push("functions")
            .push("v1")
            .push(function_name);

        // リクエストの構築
        let mut request_builder = self.http_client
            .post(url)
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", &self.api_key));
        
        // リクエストタイムアウトの設定
        if let Some(timeout) = opts.timeout_seconds {
            request_builder = request_builder.timeout(Duration::from_secs(timeout));
        }
        
        // コンテンツタイプの設定
        if let Some(content_type) = opts.content_type {
            request_builder = request_builder.header("Content-Type", content_type);
        }
        
        // カスタムヘッダーの追加
        if let Some(headers) = opts.headers {
            for (key, value) in headers {
                request_builder = request_builder.header(key, value);
            }
        }
        
        // リクエストボディの追加
        if let Some(body_data) = body {
            request_builder = request_builder.json(&body_data);
        }
        
        // リクエストの送信
        let response = request_builder.send().await
            .map_err(|e| {
                if e.is_timeout() {
                    FunctionsError::TimeoutError
                } else {
                    FunctionsError::from(e)
                }
            })?;
        
        // ステータスコードの確認
        let status = response.status();
        if !status.is_success() {
            // エラーレスポンスのパース
            let error_body = response.text().await
                .unwrap_or_else(|_| "Failed to read error response".to_string());

            if let Ok(error_details) = serde_json::from_str::<FunctionErrorDetails>(&error_body) {
                return Err(FunctionsError::with_details(&response, error_details));
            } else {
                return Err(FunctionsError::FunctionError {
                    message: error_body,
                    status,
                    details: None,
                });
            }
        }

        // レスポンスヘッダーの抽出
        let headers = response.headers().iter()
            .map(|(name, value)| {
                (
                    name.to_string(),
                    value.to_str().unwrap_or("").to_string()
                )
            })
            .collect::<HashMap<String, String>>();

        // レスポンスタイプに応じた処理
        match opts.response_type {
            ResponseType::Json => {
                let data = response.json::<T>().await
                    .map_err(|e| FunctionsError::JsonError(serde_json::Error::custom(e)))?;
                
                Ok(FunctionResponse {
                    data,
                    status,
                    headers,
                })
            },
            ResponseType::Text => {
                let text = response.text().await?;
                
                // T型にテキストを変換できるかをチェック
                let data = serde_json::from_value::<T>(Value::String(text))
                    .map_err(|_| FunctionsError::InvalidResponse("Cannot convert text response to requested type".to_string()))?;
                
                Ok(FunctionResponse {
                    data,
                    status,
                    headers,
                })
            },
            ResponseType::Binary => {
                let bytes = response.bytes().await?;
                
                // T型にバイトを変換できるかをチェック（通常はValue::Stringに変換されることはないため、エラーになりやすい）
                let binary_str = base64::encode(&bytes);
                let data = serde_json::from_value::<T>(Value::String(binary_str))
                    .map_err(|_| FunctionsError::InvalidResponse("Cannot convert binary response to requested type".to_string()))?;
                
                Ok(FunctionResponse {
                    data,
                    status,
                    headers,
                })
            }
        }
    }
    
    /// JSONを返すファンクションを呼び出す（シンプルなラッパー）
    pub async fn invoke_json<T: DeserializeOwned, B: Serialize>(
        &self,
        function_name: &str,
        body: Option<B>,
    ) -> Result<T> {
        let options = FunctionOptions {
            response_type: ResponseType::Json,
            ..Default::default()
        };
        
        let response = self.invoke::<T, B>(function_name, body, Some(options)).await?;
        Ok(response.data)
    }
    
    /// テキストを返すファンクションを呼び出す（シンプルなラッパー）
    pub async fn invoke_text<B: Serialize>(
        &self,
        function_name: &str,
        body: Option<B>,
    ) -> Result<String> {
        let options = FunctionOptions {
            response_type: ResponseType::Text,
            ..Default::default()
        };
        
        let response = self.invoke::<String, B>(function_name, body, Some(options)).await?;
        Ok(response.data)
    }
    
    /// バイナリデータを返すファンクションを呼び出す（シンプルなラッパー）
    pub async fn invoke_binary<B: Serialize>(
        &self,
        function_name: &str,
        body: Option<B>,
    ) -> Result<String> {
        let options = FunctionOptions {
            response_type: ResponseType::Binary,
            ..Default::default()
        };
        
        let response = self.invoke::<String, B>(function_name, body, Some(options)).await?;
        Ok(response.data)
    }
    
    /// ストリーミングレスポンスを取得するメソッド
    pub async fn invoke_stream<B: Serialize>(
        &self,
        function_name: &str,
        body: Option<B>,
        options: Option<FunctionOptions>,
    ) -> Result<ByteStream> {
        let opts = options.unwrap_or_default();
        
        // URLの構築
        let mut url = Url::parse(&self.base_url)?;
        url.path_segments_mut()
            .map_err(|_| FunctionsError::UrlError(url::ParseError::EmptyHost))?
            .push("functions")
            .push("v1")
            .push(function_name);

        // リクエスト構築
        let mut request_builder = self.http_client
            .post(url)
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", &self.api_key));
        
        // ストリーミングの設定
        request_builder = request_builder.header("Accept", "text/event-stream");
        
        // その他の設定（タイムアウト、ヘッダーなど）
        if let Some(timeout) = opts.timeout_seconds {
            request_builder = request_builder.timeout(Duration::from_secs(timeout));
        }
        
        if let Some(headers) = &opts.headers {
            for (key, value) in headers {
                request_builder = request_builder.header(key, value);
            }
        }
        
        // リクエストボディの追加
        if let Some(body_data) = body {
            request_builder = request_builder.json(&body_data);
        }
        
        // リクエストの送信とストリームの取得
        let response = request_builder.send().await
            .map_err(|e| {
                if e.is_timeout() {
                    FunctionsError::TimeoutError
                } else {
                    FunctionsError::from(e)
                }
            })?;
        
        // ステータスコードの確認
        if !response.status().is_success() {
            // エラー処理
            let error_body = response.text().await
                .unwrap_or_else(|_| "Failed to read error response".to_string());

            if let Ok(error_details) = serde_json::from_str::<FunctionErrorDetails>(&error_body) {
                return Err(FunctionsError::with_details(&response, error_details));
            } else {
                return Err(FunctionsError::FunctionError {
                    message: error_body,
                    status: response.status(),
                    details: None,
                });
            }
        }
        
        // ストリームを返す
        let stream = response.bytes_stream()
            .map(|result| {
                result.map_err(|e| FunctionsError::RequestError(e))
            });
            
        Ok(Box::pin(stream))
    }
    
    /// JSONストリームを取得するメソッド（SSE形式のJSONイベントを扱う）
    pub async fn invoke_json_stream<B: Serialize>(
        &self,
        function_name: &str,
        body: Option<B>,
        options: Option<FunctionOptions>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Value>> + Send>>> {
        let byte_stream = self.invoke_stream(function_name, body, options).await?;
        let json_stream = self.byte_stream_to_json(byte_stream);
        Ok(json_stream)
    }
    
    /// バイトストリームをJSONストリームに変換する
    fn byte_stream_to_json(&self, stream: ByteStream) -> Pin<Box<dyn Stream<Item = Result<Value>> + Send>> {
        Box::pin(async_stream::stream! {
            let mut line_stream = self.stream_to_lines(stream);
            
            while let Some(line_result) = line_stream.next().await {
                match line_result {
                    Ok(line) => {
                        // SSEフォーマットの処理: "data: {...}" の形式を期待
                        if line.starts_with("data: ") {
                            let json_str = line.trim_start_matches("data: ");
                            match serde_json::from_str::<Value>(json_str) {
                                Ok(json) => yield Ok(json),
                                Err(e) => yield Err(FunctionsError::JsonError(e)),
                            }
                        } else if !line.trim().is_empty() && !line.starts_with(":") {
                            // データ行でなく、コメントでもない場合はJSONとして解析を試みる
                            match serde_json::from_str::<Value>(&line) {
                                Ok(json) => yield Ok(json),
                                Err(e) => yield Err(FunctionsError::JsonError(e)),
                            }
                        }
                        // 空行やコメント行は無視
                    },
                    Err(e) => {
                        yield Err(e);
                        break;
                    }
                }
            }
        })
    }
    
    /// ストリームを行に変換する
    pub fn stream_to_lines(&self, stream: ByteStream) -> Pin<Box<dyn Stream<Item = Result<String>> + Send>> {
        Box::pin(async_stream::stream! {
            let mut buf = BytesMut::new();
            let mut stream = stream;
            
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buf.put(chunk);
                        
                        // 改行で分割して行を処理
                        loop {
                            if let Some(i) = buf.iter().position(|&b| b == b'\n') {
                                let line = String::from_utf8_lossy(&buf[..i]).to_string();
                                buf.advance(i + 1);
                                yield Ok(line);
                            } else {
                                break;
                            }
                        }
                    },
                    Err(e) => {
                        yield Err(e);
                        break;
                    }
                }
            }
            
            // 残りのバッファがあれば行として返す
            if !buf.is_empty() {
                let remaining = String::from_utf8_lossy(&buf).to_string();
                if !remaining.trim().is_empty() {
                    yield Ok(remaining);
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_invoke() {
        // TODO: モック実装を用いたテスト
    }
}