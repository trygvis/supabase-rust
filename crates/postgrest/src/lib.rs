//! Supabase PostgREST client for Rust
//!
//! This crate provides database functionality for Supabase,
//! allowing for querying, filtering, and manipulating data in PostgreSQL.
//!
//! # Features
//!
//! - Query API (`select`, `insert`, `update`, `delete`)
//! - Filtering (`eq`, `gt`, `lt`, etc.)
//! - Ordering and pagination
//! - Transactions
//! - RPC function calls
//! - CSV export

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;
use url::Url;

use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// PostgREST APIエラーの詳細情報
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PostgrestApiErrorDetails {
    pub code: Option<String>,
    pub message: Option<String>,
    pub details: Option<String>,
    pub hint: Option<String>,
}

// エラー詳細を整形して表示するための Display 実装
impl fmt::Display for PostgrestApiErrorDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if let Some(code) = &self.code {
            parts.push(format!("Code: {}", code));
        }
        if let Some(message) = &self.message {
            parts.push(format!("Message: {}", message));
        }
        if let Some(details) = &self.details {
            parts.push(format!("Details: {}", details));
        }
        if let Some(hint) = &self.hint {
            parts.push(format!("Hint: {}", hint));
        }
        write!(f, "{}", parts.join(", "))
    }
}

/// エラー型
#[derive(Error, Debug)]
pub enum PostgrestError {
    #[error("API error: {details} (Status: {status})")]
    ApiError {
        details: PostgrestApiErrorDetails,
        status: reqwest::StatusCode,
    },

    #[error("API error (unparsed): {message} (Status: {status})")]
    UnparsedApiError {
        message: String,
        status: reqwest::StatusCode,
    },

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("JSON serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}

/// ソート方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

/// トランザクションの分離レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

impl IsolationLevel {
    /// 分離レベルを文字列に変換
    fn display(&self) -> &'static str {
        match self {
            IsolationLevel::ReadCommitted => "read committed",
            IsolationLevel::RepeatableRead => "repeatable read",
            IsolationLevel::Serializable => "serializable",
        }
    }
}

/// トランザクションの読み取り/書き込みモード
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionMode {
    ReadWrite,
    ReadOnly,
}

impl TransactionMode {
    /// トランザクションモードを文字列に変換
    fn display(&self) -> &'static str {
        match self {
            TransactionMode::ReadWrite => "read write",
            TransactionMode::ReadOnly => "read only",
        }
    }
}

/// トランザクションの状態
#[allow(dead_code)]
enum TransactionState {
    Inactive,
    Active,
    Committed,
    RolledBack,
}

/// PostgreST クライアント
pub struct PostgrestClient {
    base_url: String,
    api_key: String,
    table: String,
    http_client: Client,
    headers: HeaderMap,
    query_params: HashMap<String, String>,
    #[allow(dead_code)]
    path: Option<String>,
    #[allow(dead_code)]
    is_rpc: bool,
    #[allow(dead_code)]
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
    pub fn rpc(
        base_url: &str,
        api_key: &str,
        function_name: &str,
        params: Value,
        http_client: Client,
    ) -> Self {
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
        let header_value = HeaderValue::from_str(value).map_err(|_| {
            PostgrestError::InvalidParameters(format!("Invalid header value: {}", value))
        })?;

        // ヘッダー名を文字列として所有し、HeaderNameに変換する
        let header_name = HeaderName::from_bytes(key.as_bytes()).map_err(|_| {
            PostgrestError::InvalidParameters(format!("Invalid header name: {}", key))
        })?;

        self.headers.insert(header_name, header_value);
        Ok(self)
    }

    /// 認証トークンを設定
    pub fn with_auth(self, token: &str) -> Result<Self, PostgrestError> {
        self.with_header("Authorization", &format!("Bearer {}", token))
    }

    /// 取得するカラムを指定
    pub fn select(mut self, columns: &str) -> Self {
        self.query_params
            .insert("select".to_string(), columns.to_string());
        self
    }

    /// 結合クエリ: 参照テーブルとの内部結合
    pub fn inner_join(mut self, foreign_table: &str, column: &str, foreign_column: &str) -> Self {
        // 選択列にリレーションを追加
        let current_select = self
            .query_params
            .get("select")
            .cloned()
            .unwrap_or_else(|| "*".to_string());
        let new_select = if current_select == "*" {
            format!("*,{}!inner({})", foreign_table, foreign_column)
        } else {
            format!(
                "{},{},{}!inner({})",
                current_select, column, foreign_table, foreign_column
            )
        };

        self.query_params.insert("select".to_string(), new_select);
        self
    }

    /// 結合クエリ: 参照テーブルとの左外部結合
    pub fn left_join(mut self, foreign_table: &str, column: &str, foreign_column: &str) -> Self {
        // 選択列にリレーションを追加
        let current_select = self
            .query_params
            .get("select")
            .cloned()
            .unwrap_or_else(|| "*".to_string());
        let new_select = if current_select == "*" {
            format!("*,{}!left({})", foreign_table, foreign_column)
        } else {
            format!(
                "{},{},{}!left({})",
                current_select, column, foreign_table, foreign_column
            )
        };

        self.query_params.insert("select".to_string(), new_select);
        self
    }

    /// 結合クエリ: 一対多関係の子テーブルを含める
    pub fn include(
        mut self,
        foreign_table: &str,
        _foreign_column: &str,
        columns: Option<&str>,
    ) -> Self {
        // 選択列にリレーションを追加
        let current_select = self
            .query_params
            .get("select")
            .cloned()
            .unwrap_or_else(|| "*".to_string());
        let columns_str = columns.unwrap_or("*");
        let new_select = if current_select == "*" {
            format!("*,{}({})", foreign_table, columns_str)
        } else {
            format!("{},{}({})", current_select, foreign_table, columns_str)
        };

        self.query_params.insert("select".to_string(), new_select);
        self
    }

    /// 結合クエリ: 外部キーの参照先テーブルを含める
    pub fn referenced_by(mut self, foreign_table: &str, foreign_column: &str) -> Self {
        // 選択列にリレーションを追加
        let current_select = self
            .query_params
            .get("select")
            .cloned()
            .unwrap_or_else(|| "*".to_string());
        let new_select = if current_select == "*" {
            format!("*,{}!fk({})", foreign_table, foreign_column)
        } else {
            format!(
                "{},{}!fk({})",
                current_select, foreign_table, foreign_column
            )
        };

        self.query_params.insert("select".to_string(), new_select);
        self
    }

    /// 等価フィルター
    pub fn eq(mut self, column: &str, value: &str) -> Self {
        self.query_params
            .insert(column.to_string(), format!("eq.{}", value));
        self
    }

    /// より大きいフィルター
    pub fn gt(mut self, column: &str, value: &str) -> Self {
        self.query_params
            .insert(column.to_string(), format!("gt.{}", value));
        self
    }

    /// 以上フィルター
    pub fn gte(mut self, column: &str, value: &str) -> Self {
        self.query_params
            .insert(column.to_string(), format!("gte.{}", value));
        self
    }

    /// より小さいフィルター
    pub fn lt(mut self, column: &str, value: &str) -> Self {
        self.query_params
            .insert(column.to_string(), format!("lt.{}", value));
        self
    }

    /// 以下フィルター
    pub fn lte(mut self, column: &str, value: &str) -> Self {
        self.query_params
            .insert(column.to_string(), format!("lte.{}", value));
        self
    }

    /// LIKE フィルター
    pub fn like(mut self, column: &str, pattern: &str) -> Self {
        self.query_params
            .insert(column.to_string(), format!("like.{}", pattern));
        self
    }

    /// ILIKE フィルター（大文字小文字を区別しない）
    pub fn ilike(mut self, column: &str, pattern: &str) -> Self {
        self.query_params
            .insert(column.to_string(), format!("ilike.{}", pattern));
        self
    }

    /// IN フィルター
    pub fn in_list(mut self, column: &str, values: &[&str]) -> Self {
        let value_list = values.join(",");
        self.query_params
            .insert(column.to_string(), format!("in.({})", value_list));
        self
    }

    /// NOT フィルター
    pub fn not(mut self, column: &str, operator_with_value: &str) -> Self {
        self.query_params
            .insert(column.to_string(), format!("not.{}", operator_with_value));
        self
    }

    /// JSON/JSONB カラムが指定した値を含むか (`cs`, `@>`) フィルター
    /// value は serde_json::Value で指定します
    pub fn contains(mut self, column: &str, value: &Value) -> Result<Self, PostgrestError> {
        let value_str = serde_json::to_string(value)?;
        self.query_params
            .insert(column.to_string(), format!("cs.{}", value_str));
        Ok(self)
    }

    /// JSON/JSONB カラムが指定した値に含まれるか (`cd`, `<@`) フィルター
    /// value は serde_json::Value で指定します
    pub fn contained_by(mut self, column: &str, value: &Value) -> Result<Self, PostgrestError> {
        let value_str = serde_json::to_string(value)?;
        self.query_params
            .insert(column.to_string(), format!("cd.{}", value_str));
        Ok(self)
    }

    /// ソート順を指定
    pub fn order(mut self, column: &str, order: SortOrder) -> Self {
        let order_str = match order {
            SortOrder::Ascending => "asc",
            SortOrder::Descending => "desc",
        };
        self.query_params
            .insert("order".to_string(), format!("{}.{}", column, order_str));
        self
    }

    /// 取得件数を制限
    pub fn limit(mut self, count: i32) -> Self {
        self.query_params
            .insert("limit".to_string(), count.to_string());
        self
    }

    /// オフセットを指定
    pub fn offset(mut self, count: i32) -> Self {
        self.query_params
            .insert("offset".to_string(), count.to_string());
        self
    }

    /// 全文検索
    pub fn text_search(mut self, column: &str, query: &str, config: Option<&str>) -> Self {
        let search_param = match config {
            Some(cfg) => format!("fts({}).{}", cfg, query),
            None => format!("fts.{}", query),
        };

        self.query_params.insert(column.to_string(), search_param);
        self
    }

    /// 地理空間データの距離ベース検索
    pub fn geo_distance(
        mut self,
        column: &str,
        lat: f64,
        lng: f64,
        distance: f64,
        unit: &str,
    ) -> Self {
        self.query_params.insert(
            column.to_string(),
            format!("st_dwithin.POINT({} {}).{}.{}", lng, lat, distance, unit),
        );
        self
    }

    /// グループ化
    pub fn group_by(mut self, columns: &str) -> Self {
        self.query_params
            .insert("group".to_string(), columns.to_string());
        self
    }

    /// 行数カウント
    pub fn count(mut self, exact: bool) -> Self {
        let count_method = if exact { "exact" } else { "planned" };
        self.query_params
            .insert("count".to_string(), count_method.to_string());
        self
    }

    /// RLS（行レベルセキュリティ）ポリシーを無視
    pub fn ignore_rls(mut self) -> Self {
        self.headers.insert(
            reqwest::header::HeaderName::from_static("x-supabase-admin-role"),
            reqwest::header::HeaderValue::from_static("service_role"),
        );
        self
    }

    /// スキーマを指定（デフォルトのpublicスキーマではない場合）
    pub fn schema(mut self, schema_name: &str) -> Self {
        self.query_params
            .insert("schema".to_string(), schema_name.to_string());
        self
    }

    /// CSVとしてデータをエクスポート
    pub async fn export_csv(&self) -> Result<String, PostgrestError> {
        let mut url = self.build_url()?;

        // CSVフォーマットを指定
        if url.contains('?') {
            url.push('&');
        } else {
            url.push('?');
        }
        url.push_str("accept=text/csv");

        let mut headers = self.headers.clone();
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("text/csv"),
        );

        let response = self.http_client.get(url).headers(headers).send().await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            let details = serde_json::from_str::<PostgrestApiErrorDetails>(&error_text)
                .unwrap_or_else(|_| PostgrestApiErrorDetails {
                    code: None,
                    message: Some(error_text.clone()),
                    details: None,
                    hint: None,
                });
            return Err(PostgrestError::ApiError { details, status });
        }

        let csv_data = response.text().await?;

        Ok(csv_data)
    }

    /// データを取得
    pub async fn execute<T: for<'de> Deserialize<'de>>(&self) -> Result<Vec<T>, PostgrestError> {
        let url = self.build_url()?;

        let response = self
            .http_client
            .get(&url)
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(PostgrestError::NetworkError)?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());

            // Attempt to parse specific error details
            if let Ok(details) = serde_json::from_str::<PostgrestApiErrorDetails>(&error_text) {
                return Err(PostgrestError::ApiError { details, status });
            } else {
                // If parsing fails, return a less specific error with the raw message
                return Err(PostgrestError::UnparsedApiError {
                    message: error_text,
                    status,
                });
            }
        }

        response
            .json::<Vec<T>>()
            .await
            .map_err(|e| PostgrestError::DeserializationError(e.to_string()))
    }

    /// データを挿入
    pub async fn insert<T: Serialize>(&self, values: T) -> Result<Value, PostgrestError> {
        let url = self.build_url()?;

        let response = self
            .http_client
            .post(&url)
            .headers(self.headers.clone())
            .json(&values)
            .send()
            .await
            .map_err(PostgrestError::NetworkError)?;

        let status = response.status();

        // Check for success first (e.g., 201 Created)
        if status.is_success() {
            // Read the body as text first to handle potential empty responses
            let body_text = response.text().await.map_err(|e| {
                PostgrestError::DeserializationError(format!("Failed to read response body: {}", e))
            })?;

            // If body is empty but status was success (e.g., 201), return Null.
            // PostgREST usually returns the inserted row(s), so empty is unexpected.
            if body_text.trim().is_empty() {
                // Consider returning Value::Array(vec![]) if an array is expected
                Ok(Value::Null)
            } else {
                // If body is not empty, try to parse it as JSON
                serde_json::from_str::<Value>(&body_text)
                    .map_err(|e| PostgrestError::DeserializationError(e.to_string()))
            }
        } else {
            // Handle non-success status codes as before
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());

            let details_result: Result<PostgrestApiErrorDetails, _> =
                serde_json::from_str(&error_text);
            match details_result {
                Ok(details) => Err(PostgrestError::ApiError { details, status }),
                Err(_) => Err(PostgrestError::UnparsedApiError {
                    message: error_text,
                    status,
                }),
            }
        }
    }

    /// データを更新
    pub async fn update<T: Serialize>(&self, values: T) -> Result<Value, PostgrestError> {
        let url = self.build_url()?;

        let response = self
            .http_client
            .patch(&url)
            .headers(self.headers.clone())
            .json(&values)
            .send()
            .await
            .map_err(PostgrestError::NetworkError)?;

        let status = response.status();

        // Check for success (e.g., 200 OK, 204 No Content)
        if status.is_success() {
            // Read the body as text first
            let body_text = response.text().await.map_err(|e| {
                PostgrestError::DeserializationError(format!("Failed to read response body: {}", e))
            })?;

            // If body is empty, return Null. Update might return 204 No Content.
            if body_text.trim().is_empty() {
                Ok(Value::Null)
            } else {
                // If body is not empty, try to parse it as JSON
                serde_json::from_str::<Value>(&body_text)
                    .map_err(|e| PostgrestError::DeserializationError(e.to_string()))
            }
        } else {
            // Handle non-success status codes
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());

            let details_result: Result<PostgrestApiErrorDetails, _> =
                serde_json::from_str(&error_text);
            match details_result {
                Ok(details) => Err(PostgrestError::ApiError { details, status }),
                Err(_) => Err(PostgrestError::UnparsedApiError {
                    message: error_text,
                    status,
                }),
            }
        }
    }

    /// データを削除
    pub async fn delete(&self) -> Result<Value, PostgrestError> {
        let url = self.build_url()?;

        let response = self
            .http_client
            .delete(&url)
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(PostgrestError::NetworkError)?;

        let status = response.status();

        // Check for success (e.g., 200 OK, 204 No Content)
        if status.is_success() {
            // Read the body as text first
            let body_text = response.text().await.map_err(|e| {
                PostgrestError::DeserializationError(format!("Failed to read response body: {}", e))
            })?;

            // If body is empty, return Null. Delete often returns 204 No Content.
            if body_text.trim().is_empty() {
                Ok(Value::Null)
            } else {
                // If body is not empty, try to parse it as JSON
                serde_json::from_str::<Value>(&body_text)
                    .map_err(|e| PostgrestError::DeserializationError(e.to_string()))
            }
        } else {
            // Handle non-success status codes
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());

            let details_result: Result<PostgrestApiErrorDetails, _> =
                serde_json::from_str(&error_text);
            match details_result {
                Ok(details) => Err(PostgrestError::ApiError { details, status }),
                Err(_) => Err(PostgrestError::UnparsedApiError {
                    message: error_text,
                    status,
                }),
            }
        }
    }

    /// RPC関数を呼び出す (POSTリクエスト)
    pub async fn call_rpc<T: for<'de> Deserialize<'de>>(&self) -> Result<T, PostgrestError> {
        if !self.is_rpc {
            return Err(PostgrestError::InvalidParameters(
                "Client was not created for RPC. Use PostgrestClient::rpc().".to_string(),
            ));
        }
        // RPCの場合はテーブル名が関数名として扱われる
        let url = format!("{}/rest/v1/rpc/{}", self.base_url, self.table);
        let params = self.rpc_params.as_ref().ok_or_else(|| {
            PostgrestError::InvalidParameters("RPC parameters are missing.".to_string())
        })?;

        let response = self
            .http_client
            .post(&url)
            .headers(self.headers.clone())
            .json(params)
            .send()
            .await
            .map_err(PostgrestError::NetworkError)?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());

            let details_result: Result<PostgrestApiErrorDetails, _> =
                serde_json::from_str(&error_text);
            return match details_result {
                Ok(details) => Err(PostgrestError::ApiError { details, status }),
                Err(_) => Err(PostgrestError::UnparsedApiError {
                    message: error_text,
                    status,
                }),
            };
        }

        response.json::<T>().await.map_err(|e| {
            PostgrestError::DeserializationError(format!(
                "Failed to deserialize RPC response: {}",
                e
            ))
        })
    }

    // URLを構築
    fn build_url(&self) -> Result<String, PostgrestError> {
        let mut url = Url::parse(&format!("{}/rest/v1/{}", self.base_url, self.table))?;

        for (key, value) in &self.query_params {
            url.query_pairs_mut().append_pair(key, value);
        }

        Ok(url.to_string())
    }

    /// トランザクションを開始
    pub async fn begin_transaction(
        &self,
        isolation_level: Option<IsolationLevel>,
        transaction_mode: Option<TransactionMode>,
        timeout_seconds: Option<u64>,
    ) -> Result<PostgrestTransaction, PostgrestError> {
        // トランザクションオプションを構築
        let isolation = isolation_level.unwrap_or(IsolationLevel::ReadCommitted);
        let mode = transaction_mode.unwrap_or(TransactionMode::ReadWrite);

        // トランザクション開始リクエストを構築
        let mut request_body = json!({
            "isolation_level": isolation.display(),
            "mode": mode.display(),
        });

        if let Some(timeout) = timeout_seconds {
            request_body["timeout_seconds"] = json!(timeout);
        }

        // トランザクション開始APIを呼び出し
        let transaction_url = format!("{}/rpc/begin_transaction", self.base_url);

        let response = self
            .http_client
            .post(&transaction_url)
            .headers(self.headers.clone())
            .json(&request_body)
            .send()
            .await
            .map_err(PostgrestError::NetworkError)?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());

            // Transaction begin might not return standard PostgREST JSON error, treat as TransactionError
            return Err(PostgrestError::TransactionError(format!(
                "Failed to begin transaction: {} (Status: {})",
                error_text, status
            )));
        }

        #[derive(Debug, Deserialize)]
        struct TransactionResponse {
            transaction_id: String,
        }

        let response_data = response
            .json::<TransactionResponse>()
            .await
            .map_err(|e| PostgrestError::DeserializationError(e.to_string()))?;

        // トランザクションオブジェクトを作成して返す
        Ok(PostgrestTransaction::new(
            &self.base_url,
            &self.api_key,
            self.http_client.clone(),
            self.headers.clone(),
            response_data.transaction_id,
        ))
    }
}

/// トランザクションクライアント
pub struct PostgrestTransaction {
    base_url: String,
    api_key: String,
    http_client: Client,
    headers: HeaderMap,
    transaction_id: String,
    state: Arc<AtomicBool>, // トランザクションがアクティブかどうか
}

impl PostgrestTransaction {
    /// 新しいトランザクションを作成
    fn new(
        base_url: &str,
        api_key: &str,
        http_client: Client,
        headers: HeaderMap,
        transaction_id: String,
    ) -> Self {
        Self {
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            http_client,
            headers,
            transaction_id,
            state: Arc::new(AtomicBool::new(true)), // トランザクションは初期状態でアクティブ
        }
    }

    /// トランザクション内で指定したテーブルに対するクライアントを取得
    pub fn from(&self, table: &str) -> PostgrestClient {
        // トランザクションIDをクエリパラメータとして追加するクライアントを作成
        let mut client = PostgrestClient::new(
            &self.base_url,
            &self.api_key,
            table,
            self.http_client.clone(),
        );

        // トランザクションヘッダーを設定
        for (key, value) in self.headers.iter() {
            // HeaderNameをStr形式に変換
            if let Ok(value_str) = value.to_str() {
                if let Ok(client_with_header) = PostgrestClient::new(
                    &self.base_url,
                    &self.api_key,
                    table,
                    self.http_client.clone(),
                )
                .with_header(key.as_str(), value_str)
                {
                    client = client_with_header;
                }
            }
        }

        // トランザクションIDをクエリパラメータに追加
        client
            .query_params
            .insert("transaction".to_string(), self.transaction_id.clone());

        client
    }

    /// トランザクションをコミット
    pub async fn commit(&self) -> Result<(), PostgrestError> {
        // トランザクションがアクティブかチェック
        if !self.state.load(Ordering::SeqCst) {
            return Err(PostgrestError::TransactionError(
                "Cannot commit: transaction is no longer active".to_string(),
            ));
        }

        // コミットAPIを呼び出し
        let commit_url = format!("{}/rpc/commit_transaction", self.base_url);

        let commit_body = json!({
            "transaction_id": self.transaction_id
        });

        let response = self
            .http_client
            .post(&commit_url)
            .headers(self.headers.clone())
            .json(&commit_body)
            .send()
            .await
            .map_err(PostgrestError::NetworkError)?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());

            // Treat transaction commit/rollback errors specifically
            return Err(PostgrestError::TransactionError(format!(
                "Failed to commit transaction: {} (Status: {})",
                error_text, status
            )));
        }

        // トランザクションを非アクティブに設定
        self.state.store(false, Ordering::SeqCst);

        Ok(())
    }

    /// トランザクションをロールバック
    pub async fn rollback(&self) -> Result<(), PostgrestError> {
        // トランザクションがアクティブかチェック
        if !self.state.load(Ordering::SeqCst) {
            return Err(PostgrestError::TransactionError(
                "Cannot rollback: transaction is no longer active".to_string(),
            ));
        }

        // ロールバックAPIを呼び出し
        let rollback_url = format!("{}/rpc/rollback_transaction", self.base_url);

        let rollback_body = json!({
            "transaction_id": self.transaction_id
        });

        let response = self
            .http_client
            .post(&rollback_url)
            .headers(self.headers.clone())
            .json(&rollback_body)
            .send()
            .await
            .map_err(PostgrestError::NetworkError)?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
            return Err(PostgrestError::TransactionError(format!(
                "Failed to rollback transaction: {} (Status: {})",
                error_text, status
            )));
        }

        // トランザクションを非アクティブに設定
        self.state.store(false, Ordering::SeqCst);

        Ok(())
    }

    /// セーブポイントを作成
    pub async fn savepoint(&self, name: &str) -> Result<(), PostgrestError> {
        // トランザクションがアクティブかチェック
        if !self.state.load(Ordering::SeqCst) {
            return Err(PostgrestError::TransactionError(
                "Cannot create savepoint: transaction is no longer active".to_string(),
            ));
        }

        // セーブポイントAPIを呼び出し
        let savepoint_url = format!("{}/rpc/create_savepoint", self.base_url);

        let savepoint_body = json!({
            "transaction_id": self.transaction_id,
            "name": name
        });

        let response = self
            .http_client
            .post(&savepoint_url)
            .headers(self.headers.clone())
            .json(&savepoint_body)
            .send()
            .await
            .map_err(PostgrestError::NetworkError)?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
            return Err(PostgrestError::TransactionError(format!(
                "Failed to create savepoint '{}': {} (Status: {})",
                name, error_text, status
            )));
        }
        Ok(())
    }

    /// セーブポイントにロールバック
    pub async fn rollback_to_savepoint(&self, name: &str) -> Result<(), PostgrestError> {
        // トランザクションがアクティブかチェック
        if !self.state.load(Ordering::SeqCst) {
            return Err(PostgrestError::TransactionError(
                "Cannot rollback to savepoint: transaction is no longer active".to_string(),
            ));
        }

        // セーブポイントへのロールバックAPIを呼び出し
        let rollback_url = format!("{}/rpc/rollback_to_savepoint", self.base_url);

        let rollback_body = json!({
            "transaction_id": self.transaction_id,
            "name": name
        });

        let response = self
            .http_client
            .post(&rollback_url)
            .headers(self.headers.clone())
            .json(&rollback_body)
            .send()
            .await
            .map_err(PostgrestError::NetworkError)?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
            return Err(PostgrestError::TransactionError(format!(
                "Failed to rollback to savepoint '{}': {} (Status: {})",
                name, error_text, status
            )));
        }
        Ok(())
    }
}

// デストラクタに相当する実装（トランザクションが終了するとロールバック）
impl Drop for PostgrestTransaction {
    fn drop(&mut self) {
        // トランザクションがまだアクティブな場合は自動ロールバック
        if self.state.load(Ordering::SeqCst) {
            eprintln!("Warning: Active transaction is being dropped without commit or rollback. Performing automatic rollback.");

            // ブロッキング呼び出しが推奨されませんが、Dropコンテキストでは非同期関数を呼び出せないため
            let url = format!("{}/rest/v1/rpc/rollback_transaction", self.base_url);

            let client = Client::new();
            // Using drop to explicitly drop the future and avoid the warning
            let future = client
                .post(url)
                .headers(self.headers.clone())
                .json(&json!({ "transaction_id": self.transaction_id }))
                .send();
            std::mem::drop(future);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_select() {
        let mock_server = MockServer::start().await;
        println!("Mock server started at: {}", mock_server.uri());

        // Selectクエリのモック
        Mock::given(method("GET"))
            .and(path("/rest/v1/items"))
            .and(query_param("select", "*")) // select=* を想定
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                { "id": 1, "name": "Test Item 1" },
                { "id": 2, "name": "Test Item 2" }
            ])))
            .mount(&mock_server)
            .await;
        println!("Select mock set up");

        let client = PostgrestClient::new(
            &mock_server.uri(),
            "fake-key",
            "items", // テーブル名
            reqwest::Client::new(),
        );
        println!("Client created for select test");

        let result = client.select("*").execute::<serde_json::Value>().await;

        if let Err(e) = &result {
            println!("Select query failed: {:?}", e);
        }

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(
            data.first()
                .and_then(|v: &Value| v.get("name"))
                .and_then(Value::as_str),
            Some("Test Item 1")
        );
        assert_eq!(
            data.first()
                .and_then(|v: &Value| v.get("id"))
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[tokio::test]
    async fn test_rpc() {
        let mock_server = MockServer::start().await;
        println!("Mock server started at: {}", mock_server.uri());

        // RPC呼び出しのモック (POST)
        let rpc_params = json!({ "arg1": "value1", "arg2": 123 });
        Mock::given(method("POST"))
            .and(path("/rest/v1/rpc/my_rpc_function"))
            .and(body_json(&rpc_params)) // リクエストボディを検証
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "result": "success",
                "data": 456
            })))
            .mount(&mock_server)
            .await;
        println!("RPC mock set up");

        // RPC 用クライアント作成
        let client = PostgrestClient::rpc(
            &mock_server.uri(),
            "fake-key",
            "my_rpc_function", // RPC関数名
            rpc_params.clone(),
            reqwest::Client::new(),
        );
        println!("Client created for RPC test");

        // RPC呼び出し
        #[derive(Deserialize, Debug, PartialEq)]
        struct RpcResponse {
            result: String,
            data: i32,
        }

        let result = client.call_rpc::<RpcResponse>().await; // 新しいメソッドを使用

        if let Err(e) = &result {
            println!("RPC call failed: {:?}", e);
        }

        assert!(result.is_ok());
        let response_data = result.unwrap();
        assert_eq!(
            response_data,
            RpcResponse {
                result: "success".to_string(),
                data: 456
            }
        );
    }

    #[tokio::test]
    async fn test_join_queries() {
        let mock_server = MockServer::start().await;
        println!("Mock server started at: {}", mock_server.uri());

        // 結合クエリの戻り値をモック
        Mock::given(method("GET"))
            .and(path("/rest/v1/posts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "id": 1,
                    "title": "First Post",
                    "content": "Content",
                    "comments": [
                        { "id": 1, "text": "Comment 1", "user_id": 2 },
                        { "id": 2, "text": "Comment 2", "user_id": 3 }
                    ],
                    "users": { "id": 1 }
                }
            ])))
            .mount(&mock_server)
            .await;
        println!("Join query mock set up");

        let client = PostgrestClient::new(
            &mock_server.uri(),
            "fake-key",
            "posts",
            reqwest::Client::new(),
        );
        println!("Client created");

        let result = client
            .select("id,title,content")
            .include("comments", "post_id", Some("id,text,user_id"))
            .inner_join("users", "user_id", "id")
            .execute::<serde_json::Value>()
            .await;

        if let Err(e) = &result {
            println!("Join query failed: {:?}", e);
        }

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(
            data.first()
                .and_then(|v: &Value| v.get("title"))
                .and_then(Value::as_str),
            Some("First Post")
        );
        assert_eq!(
            data.first()
                .and_then(|v: &Value| v.get("comments"))
                .and_then(Value::as_array)
                .map(|a| a.len()),
            Some(2)
        );
    }

    #[tokio::test]
    async fn test_text_search() {
        let mock_server = MockServer::start().await;

        // 全文検索のモック
        Mock::given(method("GET"))
            .and(path("/rest/v1/articles"))
            .and(query_param("content", "fts(english).search terms"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                { "id": 1, "title": "Search Result", "content": "This is a search result" }
            ])))
            .mount(&mock_server)
            .await;

        let client = PostgrestClient::new(
            &mock_server.uri(),
            "fake-key",
            "articles",
            reqwest::Client::new(),
        );

        let result = client
            .text_search("content", "search terms", Some("english"))
            .execute::<serde_json::Value>()
            .await;

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(
            data.first()
                .and_then(|v: &Value| v.get("title"))
                .and_then(Value::as_str),
            Some("Search Result")
        );
    }

    #[tokio::test]
    async fn test_csv_export() {
        let mock_server = MockServer::start().await;

        // CSVエクスポートのモック
        Mock::given(method("GET"))
            .and(path("/rest/v1/users"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(
                        "id,name,email\n1,User 1,user1@example.com\n2,User 2,user2@example.com",
                    )
                    .append_header("Content-Type", "text/csv"),
            )
            .mount(&mock_server)
            .await;

        let client = PostgrestClient::new(
            &mock_server.uri(),
            "fake-key",
            "users",
            reqwest::Client::new(),
        );

        let result = client.export_csv().await;

        assert!(result.is_ok());
        let csv_data = result.unwrap();
        assert!(csv_data.contains("id,name,email"));
        assert!(csv_data.contains("User 1"));
        assert!(csv_data.contains("User 2"));
    }

    #[tokio::test]
    async fn test_transaction() {
        let mock_server = MockServer::start().await;
        println!("Mock server started at: {}", mock_server.uri());

        // BEGIN トランザクションのモック
        Mock::given(method("POST"))
            .and(path("/rpc/begin_transaction"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "transaction_id": "tx-12345"
            })))
            .mount(&mock_server)
            .await;
        println!("Begin transaction mock set up");

        // トランザクション内のINSERTのモック
        Mock::given(method("POST"))
            .and(path("/rest/v1/users"))
            .and(query_param("transaction", "tx-12345"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!([{
                "id": 1,
                "name": "テストユーザー"
            }])))
            .mount(&mock_server)
            .await;
        println!("Insert mock set up");

        // トランザクション内のSELECTのモック
        Mock::given(method("GET"))
            .and(path("/rest/v1/users"))
            .and(query_param("transaction", "tx-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
                "id": 1,
                "name": "テストユーザー"
            }])))
            .mount(&mock_server)
            .await;
        println!("Select mock set up");

        // COMMITのモック
        Mock::given(method("POST"))
            .and(path("/rpc/commit_transaction"))
            .and(body_json(json!({
                "transaction_id": "tx-12345"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true
            })))
            .mount(&mock_server)
            .await;
        println!("Commit mock set up");

        // テスト実行
        let client = PostgrestClient::new(
            &mock_server.uri(),
            "fake-key",
            "users",
            reqwest::Client::new(),
        );
        println!("Client created");

        // トランザクション開始
        let transaction = client
            .begin_transaction(
                Some(IsolationLevel::ReadCommitted),
                Some(TransactionMode::ReadWrite),
                Some(30),
            )
            .await;

        if let Err(e) = &transaction {
            println!("Transaction failed: {:?}", e);
        }

        assert!(transaction.is_ok());
        let transaction = transaction.unwrap();

        // トランザクション内で挿入
        let insert_result = transaction
            .from("users")
            .insert(json!({
                "name": "テストユーザー"
            }))
            .await;

        assert!(insert_result.is_ok());

        // トランザクション内でクエリ
        let query_result = transaction
            .from("users")
            .select("id, name")
            .execute::<serde_json::Value>()
            .await;

        assert!(query_result.is_ok());
        let users = query_result.unwrap();
        assert_eq!(
            users
                .first()
                .and_then(|v: &Value| v.get("name"))
                .and_then(Value::as_str),
            Some("テストユーザー")
        );

        // トランザクションをコミット
        let commit_result = transaction.commit().await;
        assert!(commit_result.is_ok());
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let mock_server = MockServer::start().await;

        // BEGIN トランザクションのモック
        Mock::given(method("POST"))
            .and(path("/rpc/begin_transaction"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "transaction_id": "tx-67890"
            })))
            .mount(&mock_server)
            .await;

        // ROLLBACKのモック
        Mock::given(method("POST"))
            .and(path("/rpc/rollback_transaction"))
            .and(body_json(json!({
                "transaction_id": "tx-67890"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true
            })))
            .mount(&mock_server)
            .await;

        // テスト実行
        let client = PostgrestClient::new(
            &mock_server.uri(),
            "fake-key",
            "users",
            reqwest::Client::new(),
        );

        // トランザクション開始
        let transaction = client.begin_transaction(None, None, None).await;

        assert!(transaction.is_ok());
        let transaction = transaction.unwrap();

        // トランザクションをロールバック
        let rollback_result = transaction.rollback().await;
        assert!(rollback_result.is_ok());
    }

    #[tokio::test]
    async fn test_transaction_savepoint() {
        let mock_server = MockServer::start().await;

        // BEGIN トランザクションのモック
        Mock::given(method("POST"))
            .and(path("/rpc/begin_transaction"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "transaction_id": "tx-savepoint"
            })))
            .mount(&mock_server)
            .await;

        // SAVEPOINTのモック
        Mock::given(method("POST"))
            .and(path("/rpc/create_savepoint"))
            .and(body_json(json!({
                "transaction_id": "tx-savepoint",
                "name": "sp1"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true
            })))
            .mount(&mock_server)
            .await;

        // ROLLBACK TO SAVEPOINTのモック
        Mock::given(method("POST"))
            .and(path("/rpc/rollback_to_savepoint"))
            .and(body_json(json!({
                "transaction_id": "tx-savepoint",
                "name": "sp1"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true
            })))
            .mount(&mock_server)
            .await;

        // COMMITのモック
        Mock::given(method("POST"))
            .and(path("/rpc/commit_transaction"))
            .and(body_json(json!({
                "transaction_id": "tx-savepoint"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true
            })))
            .mount(&mock_server)
            .await;

        // テスト実行
        let client = PostgrestClient::new(
            &mock_server.uri(),
            "fake-key",
            "users",
            reqwest::Client::new(),
        );

        // トランザクション開始
        let transaction = client.begin_transaction(None, None, None).await;

        assert!(transaction.is_ok());
        let transaction = transaction.unwrap();

        // セーブポイント作成
        let savepoint_result = transaction.savepoint("sp1").await;
        assert!(savepoint_result.is_ok());

        // セーブポイントにロールバック
        let rollback_to_savepoint_result = transaction.rollback_to_savepoint("sp1").await;
        assert!(rollback_to_savepoint_result.is_ok());

        // トランザクションをコミット
        let commit_result = transaction.commit().await;
        assert!(commit_result.is_ok());
    }

    #[tokio::test]
    async fn test_jsonb_filters() {
        let mock_server = MockServer::start().await;

        let contains_value = json!({ "key": "value" });
        let contained_by_value = json!(["a", "b"]);

        // contains のモック
        Mock::given(method("GET"))
            .and(path("/rest/v1/data"))
            .and(query_param("metadata", format!("cs.{}", contains_value)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id": 1}])))
            .mount(&mock_server)
            .await;

        // contained_by のモック
        Mock::given(method("GET"))
            .and(path("/rest/v1/data"))
            .and(query_param("tags", format!("cd.{}", contained_by_value)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id": 2}])))
            .mount(&mock_server)
            .await;

        let _base_client = PostgrestClient::new(
            &mock_server.uri(),
            "fake-key",
            "data",
            reqwest::Client::new(),
        );

        // contains テスト
        let result_contains = PostgrestClient::new(
            // Re-create or adjust structure if needed
            &mock_server.uri(),
            "fake-key",
            "data",
            reqwest::Client::new(), // Assuming new client instance is ok for test
        )
        .contains("metadata", &contains_value)
        .unwrap() // Result from contains
        .execute::<serde_json::Value>()
        .await;
        assert!(result_contains.is_ok());
        assert_eq!(result_contains.unwrap().len(), 1);

        // contained_by テスト
        let result_contained_by = PostgrestClient::new(
            // Re-create or adjust structure if needed
            &mock_server.uri(),
            "fake-key",
            "data",
            reqwest::Client::new(), // Assuming new client instance is ok for test
        )
        .contained_by("tags", &contained_by_value)
        .unwrap()
        .execute::<serde_json::Value>()
        .await;
        assert!(result_contained_by.is_ok());
        assert_eq!(result_contained_by.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_filter_on_related_table() {
        let mock_server = MockServer::start().await;

        // Related table filter のモック
        Mock::given(method("GET"))
            .and(path("/rest/v1/posts"))
            .and(query_param("author.name", "eq.Specific Author")) // authorテーブルのnameでフィルタ
            .and(query_param("select", "title,author!inner(name)")) // select句も設定
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                { "title": "Post by Specific Author", "author": { "name": "Specific Author" } }
            ])))
            .mount(&mock_server)
            .await;

        let client = PostgrestClient::new(
            &mock_server.uri(),
            "fake-key",
            "posts",
            reqwest::Client::new(),
        );

        let result = client
            .select("title,author!inner(name)") // joinを含めておく
            .eq("author.name", "Specific Author") // 関連テーブルのカラムを指定してフィルタ
            .execute::<serde_json::Value>()
            .await;

        if let Err(e) = &result {
            println!("Join query failed: {:?}", e);
        }

        assert!(result.is_ok(), "Request failed: {:?}", result.err());
        let data = result.unwrap();
        assert_eq!(data.len(), 1);
        let post = data
            .first()
            .expect("Post should exist in related table test");
        assert_eq!(
            post.get("title").and_then(Value::as_str),
            Some("Post by Specific Author")
        );
        let author_obj: Option<&Value> = post.get("author");
        let name_val = author_obj
            .and_then(|a: &Value| a.get("name"))
            .and_then(Value::as_str);
        assert_eq!(name_val, Some("Specific Author"));
    }
}
