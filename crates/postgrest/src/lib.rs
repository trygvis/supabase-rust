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


use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

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

    #[error("Transaction error: {0}")]
    TransactionError(String),
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
    fn to_string(&self) -> &'static str {
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
    fn to_string(&self) -> &'static str {
        match self {
            TransactionMode::ReadWrite => "read write",
            TransactionMode::ReadOnly => "read only",
        }
    }
}

/// トランザクションの状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    
    /// 結合クエリ: 参照テーブルとの内部結合
    pub fn inner_join(mut self, foreign_table: &str, column: &str, foreign_column: &str) -> Self {
        // 選択列にリレーションを追加
        let current_select = self.query_params.get("select").cloned().unwrap_or_else(|| "*".to_string());
        let new_select = if current_select == "*" {
            format!("*,{}!inner({})", foreign_table, foreign_column)
        } else {
            format!("{},{},{}!inner({})", current_select, column, foreign_table, foreign_column)
        };
        
        self.query_params.insert("select".to_string(), new_select);
        self
    }
    
    /// 結合クエリ: 参照テーブルとの左外部結合
    pub fn left_join(mut self, foreign_table: &str, column: &str, foreign_column: &str) -> Self {
        // 選択列にリレーションを追加
        let current_select = self.query_params.get("select").cloned().unwrap_or_else(|| "*".to_string());
        let new_select = if current_select == "*" {
            format!("*,{}!left({})", foreign_table, foreign_column)
        } else {
            format!("{},{},{}!left({})", current_select, column, foreign_table, foreign_column)
        };
        
        self.query_params.insert("select".to_string(), new_select);
        self
    }
    
    /// 結合クエリ: 一対多関係の子テーブルを含める
    pub fn include(mut self, foreign_table: &str, foreign_column: &str, columns: Option<&str>) -> Self {
        // 選択列にリレーションを追加
        let current_select = self.query_params.get("select").cloned().unwrap_or_else(|| "*".to_string());
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
        let current_select = self.query_params.get("select").cloned().unwrap_or_else(|| "*".to_string());
        let new_select = if current_select == "*" {
            format!("*,{}!fk({})", foreign_table, foreign_column)
        } else {
            format!("{},{}!fk({})", current_select, foreign_table, foreign_column)
        };
        
        self.query_params.insert("select".to_string(), new_select);
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
    pub fn geo_distance(mut self, column: &str, lat: f64, lng: f64, distance: f64, unit: &str) -> Self {
        self.query_params.insert(
            column.to_string(),
            format!("st_dwithin.POINT({} {}).{}.{}", lng, lat, distance, unit)
        );
        self
    }
    
    /// グループ化
    pub fn group_by(mut self, columns: &str) -> Self {
        self.query_params.insert("group".to_string(), columns.to_string());
        self
    }
    
    /// 行数カウント
    pub fn count(mut self, exact: bool) -> Self {
        let count_method = if exact { "exact" } else { "planned" };
        self.query_params.insert("count".to_string(), count_method.to_string());
        self
    }
    
    /// RLS（行レベルセキュリティ）ポリシーを無視
    pub fn ignore_rls(mut self) -> Self {
        self.headers.insert(
            reqwest::header::HeaderName::from_static("x-supabase-admin-role"),
            reqwest::header::HeaderValue::from_static("service_role")
        );
        self
    }
    
    /// スキーマを指定（デフォルトのpublicスキーマではない場合）
    pub fn schema(mut self, schema_name: &str) -> Self {
        self.query_params.insert("schema".to_string(), schema_name.to_string());
        self
    }
    
    /// CSVとしてデータをエクスポート
    pub async fn export_csv(&self) -> Result<String, PostgrestError> {
        let mut url = self.build_url()?;
        
        // CSVフォーマットを指定
        if url.contains('?') {
            url.push_str("&");
        } else {
            url.push_str("?");
        }
        url.push_str("accept=text/csv");
        
        let mut headers = self.headers.clone();
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("text/csv")
        );
        
        let response = self.http_client.get(url)
            .headers(headers)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(PostgrestError::ApiError(error_text));
        }
        
        let csv_data = response.text().await?;
        
        Ok(csv_data)
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

    /// トランザクションを開始
    pub async fn begin_transaction(
        &self,
        isolation_level: Option<IsolationLevel>,
        transaction_mode: Option<TransactionMode>,
        timeout_seconds: Option<u64>
    ) -> Result<PostgrestTransaction, PostgrestError> {
        // トランザクションオプションを構築
        let isolation = isolation_level.unwrap_or(IsolationLevel::ReadCommitted);
        let mode = transaction_mode.unwrap_or(TransactionMode::ReadWrite);
        
        // トランザクション開始リクエストを構築
        let mut request_body = json!({
            "isolation_level": isolation.to_string(),
            "mode": mode.to_string(),
        });
        
        if let Some(timeout) = timeout_seconds {
            request_body["timeout_seconds"] = json!(timeout);
        }
        
        // トランザクション開始APIを呼び出し
        let transaction_url = format!("{}/rpc/begin_transaction", self.base_url);
        
        let response = self.http_client
            .post(&transaction_url)
            .headers(self.headers.clone())
            .json(&request_body)
            .send()
            .await
            .map_err(|e| PostgrestError::NetworkError(e))?;
            
        if !response.status().is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
                
            return Err(PostgrestError::TransactionError(format!(
                "Failed to begin transaction: {} (Status: {})",
                error_text,
                response.status()
            )));
        }
        
        #[derive(Debug, Deserialize)]
        struct TransactionResponse {
            transaction_id: String,
        }
        
        let response_data = response.json::<TransactionResponse>().await
            .map_err(|e| PostgrestError::SerializationError(e))?;
            
        // トランザクションオブジェクトを作成して返す
        Ok(PostgrestTransaction::new(
            &self.base_url,
            &self.api_key,
            self.http_client.clone(),
            self.headers.clone(),
            response_data.transaction_id
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
        transaction_id: String
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
            self.http_client.clone()
        );
        
        // トランザクションヘッダーを設定
        for (key, value) in self.headers.iter() {
            if let Some(key_str) = key.as_str() {
                if let Ok(value_str) = value.to_str() {
                    client = client.with_header(key_str, value_str).unwrap_or(client);
                }
            }
        }
        
        // トランザクションIDをクエリパラメータに追加
        client.query_params.insert("transaction".to_string(), self.transaction_id.clone());
        
        client
    }
    
    /// トランザクションをコミット
    pub async fn commit(&self) -> Result<(), PostgrestError> {
        // トランザクションがアクティブかチェック
        if !self.state.load(Ordering::SeqCst) {
            return Err(PostgrestError::TransactionError(
                "Cannot commit: transaction is no longer active".to_string()
            ));
        }
        
        // コミットAPIを呼び出し
        let commit_url = format!("{}/rpc/commit_transaction", self.base_url);
        
        let commit_body = json!({
            "transaction_id": self.transaction_id
        });
        
        let response = self.http_client
            .post(&commit_url)
            .headers(self.headers.clone())
            .json(&commit_body)
            .send()
            .await
            .map_err(|e| PostgrestError::NetworkError(e))?;
            
        if !response.status().is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
                
            return Err(PostgrestError::TransactionError(format!(
                "Failed to commit transaction: {} (Status: {})",
                error_text,
                response.status()
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
                "Cannot rollback: transaction is no longer active".to_string()
            ));
        }
        
        // ロールバックAPIを呼び出し
        let rollback_url = format!("{}/rpc/rollback_transaction", self.base_url);
        
        let rollback_body = json!({
            "transaction_id": self.transaction_id
        });
        
        let response = self.http_client
            .post(&rollback_url)
            .headers(self.headers.clone())
            .json(&rollback_body)
            .send()
            .await
            .map_err(|e| PostgrestError::NetworkError(e))?;
            
        if !response.status().is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
                
            return Err(PostgrestError::TransactionError(format!(
                "Failed to rollback transaction: {} (Status: {})",
                error_text,
                response.status()
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
                "Cannot create savepoint: transaction is no longer active".to_string()
            ));
        }
        
        // セーブポイント作成APIを呼び出し
        let savepoint_url = format!("{}/rpc/create_savepoint", self.base_url);
        
        let savepoint_body = json!({
            "transaction_id": self.transaction_id,
            "name": name
        });
        
        let response = self.http_client
            .post(&savepoint_url)
            .headers(self.headers.clone())
            .json(&savepoint_body)
            .send()
            .await
            .map_err(|e| PostgrestError::NetworkError(e))?;
            
        if !response.status().is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
                
            return Err(PostgrestError::TransactionError(format!(
                "Failed to create savepoint: {} (Status: {})",
                error_text,
                response.status()
            )));
        }
        
        Ok(())
    }
    
    /// セーブポイントにロールバック
    pub async fn rollback_to_savepoint(&self, name: &str) -> Result<(), PostgrestError> {
        // トランザクションがアクティブかチェック
        if !self.state.load(Ordering::SeqCst) {
            return Err(PostgrestError::TransactionError(
                "Cannot rollback to savepoint: transaction is no longer active".to_string()
            ));
        }
        
        // セーブポイントへのロールバックAPIを呼び出し
        let rollback_savepoint_url = format!("{}/rpc/rollback_to_savepoint", self.base_url);
        
        let rollback_savepoint_body = json!({
            "transaction_id": self.transaction_id,
            "name": name
        });
        
        let response = self.http_client
            .post(&rollback_savepoint_url)
            .headers(self.headers.clone())
            .json(&rollback_savepoint_body)
            .send()
            .await
            .map_err(|e| PostgrestError::NetworkError(e))?;
            
        if !response.status().is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
                
            return Err(PostgrestError::TransactionError(format!(
                "Failed to rollback to savepoint: {} (Status: {})",
                error_text,
                response.status()
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
            let _ = client.post(url)
                .headers(self.headers.clone())
                .json(&json!({ "transaction_id": self.transaction_id }))
                .send();
        }
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
    
    #[tokio::test]
    async fn test_join_queries() {
        let mock_server = MockServer::start().await;
        
        // 結合クエリの戻り値をモック
        Mock::given(method("GET"))
            .and(path("/rest/v1/posts"))
            .and(query_param("select", "id,title,content,comments(id,text,user_id),users!inner(id)"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!([
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
                ]))
            )
            .mount(&mock_server)
            .await;
        
        let client = PostgrestClient::new(
            &mock_server.uri(),
            "fake-key",
            "posts",
            reqwest::Client::new()
        );
        
        let result = client
            .select("id,title,content")
            .include("comments", "post_id", Some("id,text,user_id"))
            .inner_join("users", "user_id", "id")
            .execute::<serde_json::Value>()
            .await;
            
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0]["title"], "First Post");
        assert_eq!(data[0]["comments"].as_array().unwrap().len(), 2);
    }
    
    #[tokio::test]
    async fn test_text_search() {
        let mock_server = MockServer::start().await;
        
        // 全文検索のモック
        Mock::given(method("GET"))
            .and(path("/rest/v1/articles"))
            .and(query_param("content", "fts(english).search terms"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!([
                    { "id": 1, "title": "Search Result", "content": "This is a search result" }
                ]))
            )
            .mount(&mock_server)
            .await;
        
        let client = PostgrestClient::new(
            &mock_server.uri(),
            "fake-key",
            "articles",
            reqwest::Client::new()
        );
        
        let result = client
            .text_search("content", "search terms", Some("english"))
            .execute::<serde_json::Value>()
            .await;
            
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0]["title"], "Search Result");
    }
    
    #[tokio::test]
    async fn test_csv_export() {
        let mock_server = MockServer::start().await;
        
        // CSVエクスポートのモック
        Mock::given(method("GET"))
            .and(path("/rest/v1/users"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string("id,name,email\n1,User 1,user1@example.com\n2,User 2,user2@example.com")
                .append_header("Content-Type", "text/csv")
            )
            .mount(&mock_server)
            .await;
        
        let client = PostgrestClient::new(
            &mock_server.uri(),
            "fake-key",
            "users",
            reqwest::Client::new()
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
        
        // BEGIN トランザクションのモック
        Mock::given(method("POST"))
            .and(path("/rest/v1/rpc/begin_transaction"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!({
                    "transaction_id": "tx-12345"
                }))
            )
            .mount(&mock_server)
            .await;
        
        // トランザクション内のINSERTのモック
        Mock::given(method("POST"))
            .and(path("/rest/v1/users"))
            .and(header("x-postgresql-transaction-id", "tx-12345"))
            .respond_with(ResponseTemplate::new(201)
                .set_body_json(json!([{
                    "id": 1,
                    "name": "テストユーザー"
                }]))
            )
            .mount(&mock_server)
            .await;
        
        // トランザクション内のSELECTのモック
        Mock::given(method("GET"))
            .and(path("/rest/v1/users"))
            .and(header("x-postgresql-transaction-id", "tx-12345"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!([{
                    "id": 1,
                    "name": "テストユーザー"
                }]))
            )
            .mount(&mock_server)
            .await;
        
        // COMMITのモック
        Mock::given(method("POST"))
            .and(path("/rest/v1/rpc/commit_transaction"))
            .and(body_json(json!({
                "transaction_id": "tx-12345"
            })))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true
                }))
            )
            .mount(&mock_server)
            .await;
        
        // テスト実行
        let client = PostgrestClient::new(
            &mock_server.uri(),
            "fake-key",
            "users",
            reqwest::Client::new()
        );
        
        // トランザクション開始
        let transaction = client
            .begin_transaction(
                Some(IsolationLevel::ReadCommitted),
                Some(TransactionMode::ReadWrite),
                Some(30)
            )
            .await;
            
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
        assert_eq!(query_result.unwrap()[0]["name"], "テストユーザー");
        
        // トランザクションをコミット
        let commit_result = transaction.commit().await;
        assert!(commit_result.is_ok());
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let mock_server = MockServer::start().await;
        
        // BEGIN トランザクションのモック
        Mock::given(method("POST"))
            .and(path("/rest/v1/rpc/begin_transaction"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!({
                    "transaction_id": "tx-67890"
                }))
            )
            .mount(&mock_server)
            .await;
        
        // ROLLBACKのモック
        Mock::given(method("POST"))
            .and(path("/rest/v1/rpc/rollback_transaction"))
            .and(body_json(json!({
                "transaction_id": "tx-67890"
            })))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true
                }))
            )
            .mount(&mock_server)
            .await;
        
        // テスト実行
        let client = PostgrestClient::new(
            &mock_server.uri(),
            "fake-key",
            "users",
            reqwest::Client::new()
        );
        
        // トランザクション開始
        let transaction = client
            .begin_transaction(None, None, None)
            .await;
            
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
            .and(path("/rest/v1/rpc/begin_transaction"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!({
                    "transaction_id": "tx-savepoint"
                }))
            )
            .mount(&mock_server)
            .await;
        
        // SAVEPOINTのモック
        Mock::given(method("POST"))
            .and(path("/rest/v1/rpc/create_savepoint"))
            .and(body_json(json!({
                "transaction_id": "tx-savepoint",
                "savepoint_name": "sp1"
            })))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true
                }))
            )
            .mount(&mock_server)
            .await;
        
        // ROLLBACK TO SAVEPOINTのモック
        Mock::given(method("POST"))
            .and(path("/rest/v1/rpc/rollback_to_savepoint"))
            .and(body_json(json!({
                "transaction_id": "tx-savepoint",
                "savepoint_name": "sp1"
            })))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true
                }))
            )
            .mount(&mock_server)
            .await;
        
        // COMMITのモック
        Mock::given(method("POST"))
            .and(path("/rest/v1/rpc/commit_transaction"))
            .and(body_json(json!({
                "transaction_id": "tx-savepoint"
            })))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true
                }))
            )
            .mount(&mock_server)
            .await;
        
        // テスト実行
        let client = PostgrestClient::new(
            &mock_server.uri(),
            "fake-key",
            "users",
            reqwest::Client::new()
        );
        
        // トランザクション開始
        let transaction = client
            .begin_transaction(None, None, None)
            .await;
            
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
}