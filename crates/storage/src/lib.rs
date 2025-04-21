//! Supabase Storage client for Rust
//!
//! This crate provides storage functionality for Supabase,
//! allowing for uploading, downloading, and managing files.

use reqwest::Client;
use reqwest::multipart::{Form, Part};
use serde::{Serialize, Deserialize};
use thiserror::Error;
use std::collections::HashMap;
use url::Url;
use bytes::Bytes;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

/// エラー型
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("JSON serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl StorageError {
    pub fn new(message: String) -> Self {
        Self::StorageError(message)
    }
}

/// ファイルアップロードオプション
#[derive(Debug, Clone, Serialize, Default)]
pub struct FileOptions {
    pub cache_control: Option<String>,
    pub content_type: Option<String>,
    pub upsert: Option<bool>,
}

impl FileOptions {
    /// 新しいファイルオプションを作成
    pub fn new() -> Self {
        Self::default()
    }
    
    /// キャッシュコントロールを設定
    pub fn with_cache_control(mut self, cache_control: &str) -> Self {
        self.cache_control = Some(cache_control.to_string());
        self
    }
    
    /// コンテンツタイプを設定
    pub fn with_content_type(mut self, content_type: &str) -> Self {
        self.content_type = Some(content_type.to_string());
        self
    }
    
    /// アップサートを設定
    pub fn with_upsert(mut self, upsert: bool) -> Self {
        self.upsert = Some(upsert);
        self
    }
}

/// ファイル一覧取得オプション
#[derive(Debug, Clone, Serialize, Default)]
pub struct ListOptions {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub sort_by: Option<SortBy>,
    pub search: Option<String>,
}

impl ListOptions {
    /// 新しい一覧オプションを作成
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 取得上限を設定
    pub fn limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }
    
    /// オフセットを設定
    pub fn offset(mut self, offset: i32) -> Self {
        self.offset = Some(offset);
        self
    }
    
    /// ソート順を設定
    pub fn sort_by(mut self, column: &str, order: SortOrder) -> Self {
        self.sort_by = Some(SortBy {
            column: column.to_string(),
            order,
        });
        self
    }
    
    /// 検索キーワードを設定
    pub fn search(mut self, search: &str) -> Self {
        self.search = Some(search.to_string());
        self
    }
}

/// ソート設定
#[derive(Debug, Clone, Serialize)]
pub struct SortBy {
    pub column: String,
    pub order: SortOrder,
}

impl ToString for SortBy {
    fn to_string(&self) -> String {
        format!("{}:{:?}", self.column, self.order).to_lowercase()
    }
}

/// ソート順
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// ファイル情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileObject {
    pub name: String,
    pub bucket_id: String,
    pub owner: String,
    pub id: String,
    pub updated_at: String,
    pub created_at: String,
    pub last_accessed_at: String,
    pub metadata: Option<serde_json::Value>,
    pub mime_type: Option<String>,
    pub size: i64,
}

/// バケット情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bucket {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub public: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// チャンクアップロードの初期化結果
#[derive(Debug, Clone, Deserialize)]
pub struct InitiateMultipartUploadResponse {
    pub id: String,
    #[serde(rename = "uploadId")]
    pub upload_id: String,
    pub key: String,
    pub bucket: String,
}

/// アップロードされたチャンク情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadedPartInfo {
    #[serde(rename = "partNumber")]
    pub part_number: u32,
    pub etag: String,
}

/// チャンクアップロードの完了リクエスト
#[derive(Debug, Clone, Serialize)]
struct CompleteMultipartUploadRequest {
    #[serde(rename = "uploadId")]
    pub upload_id: String,
    pub parts: Vec<UploadedPartInfo>,
}

/// ストレージバケットクライアント
pub struct StorageBucketClient<'a> {
    parent: &'a StorageClient,
    bucket_id: String,
}

/// ストレージクライアント
pub struct StorageClient {
    base_url: String,
    api_key: String,
    http_client: Client,
}

impl StorageClient {
    /// 新しいストレージクライアントを作成
    pub fn new(base_url: &str, api_key: &str, http_client: Client) -> Self {
        Self {
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            http_client,
        }
    }
    
    /// バケットを指定
    pub fn from<'a>(&'a self, bucket_id: &str) -> StorageBucketClient<'a> {
        StorageBucketClient {
            parent: self,
            bucket_id: bucket_id.to_string(),
        }
    }
    
    /// バケット一覧を取得
    pub async fn list_buckets(&self) -> Result<Vec<Bucket>, StorageError> {
        let url = format!("{}/storage/v1/bucket", self.base_url);
        
        let response = self.http_client.get(&url)
            .header("apikey", &self.api_key)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(StorageError::ApiError(error_text));
        }
        
        let buckets = response.json::<Vec<Bucket>>().await?;
        
        Ok(buckets)
    }
    
    /// バケットを作成
    pub async fn create_bucket(&self, bucket_id: &str, is_public: bool) -> Result<Bucket, StorageError> {
        let url = format!("{}/storage/v1/bucket", self.base_url);
        
        let payload = serde_json::json!({
            "id": bucket_id,
            "name": bucket_id,
            "public": is_public
        });
        
        let response = self.http_client.post(&url)
            .header("apikey", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(StorageError::ApiError(error_text));
        }
        
        let bucket = response.json::<Bucket>().await?;
        
        Ok(bucket)
    }
    
    /// バケットを削除
    pub async fn delete_bucket(&self, bucket_id: &str) -> Result<(), StorageError> {
        let url = format!("{}/storage/v1/bucket/{}", self.base_url, bucket_id);
        
        let response = self.http_client.delete(&url)
            .header("apikey", &self.api_key)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(StorageError::ApiError(error_text));
        }
        
        Ok(())
    }
    
    /// バケット情報を更新
    pub async fn update_bucket(&self, bucket_id: &str, is_public: bool) -> Result<Bucket, StorageError> {
        let url = format!("{}/storage/v1/bucket/{}", self.base_url, bucket_id);
        
        let payload = serde_json::json!({
            "id": bucket_id,
            "public": is_public
        });
        
        let response = self.http_client.put(&url)
            .header("apikey", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(StorageError::ApiError(error_text));
        }
        
        let bucket = response.json::<Bucket>().await?;
        
        Ok(bucket)
    }
}

impl<'a> StorageBucketClient<'a> {
    /// ファイルをアップロード
    pub async fn upload(&self, path: &str, file_path: &Path, options: Option<FileOptions>) -> Result<FileObject, StorageError> {
        let mut url = Url::parse(&self.parent.base_url)?;
        url.set_path(&format!("/storage/v1/object/{}/{}", self.bucket_id, path));
        
        // オプションをURLクエリとして設定
        if let Some(opts) = &options {
            let mut query_pairs = url.query_pairs_mut();
            if let Some(cache_control) = &opts.cache_control {
                query_pairs.append_pair("cache_control", cache_control);
            }
            if let Some(upsert) = &opts.upsert {
                query_pairs.append_pair("upsert", &upsert.to_string());
            }
        }
        
        // ファイルの内容を読み込む
        let mut file = File::open(file_path).await?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).await?;
        
        // マルチパートフォームデータの作成
        let part = Part::bytes(contents)
            .file_name(file_path.file_name().unwrap().to_string_lossy().to_string());
            
        let form = Form::new().part("file", part);
        
        let response = self.parent.http_client
            .post(url)
            .header("apikey", &self.parent.api_key)
            .header("Authorization", format!("Bearer {}", &self.parent.api_key))
            .multipart(form)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(StorageError::ApiError(error_text));
        }
        
        let file_object = response.json::<FileObject>().await?;
        
        Ok(file_object)
    }
    
    /// ファイルをダウンロード
    pub async fn download(&self, path: &str) -> Result<Bytes, StorageError> {
        let mut url = Url::parse(&self.parent.base_url)?;
        url.set_path(&format!("/storage/v1/object/{}/{}", self.bucket_id, path));
        
        let response = self.parent.http_client
            .get(url)
            .header("apikey", &self.parent.api_key)
            .header("Authorization", format!("Bearer {}", &self.parent.api_key))
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(StorageError::ApiError(error_text));
        }
        
        let bytes = response.bytes().await?;
        
        Ok(bytes)
    }
    
    /// ファイル一覧を取得
    pub async fn list(&self, prefix: &str, options: Option<ListOptions>) -> Result<Vec<FileObject>, StorageError> {
        let mut url = Url::parse(&self.parent.base_url)?;
        url.set_path(&format!("/storage/v1/object/list/{}", self.bucket_id));
        
        // プレフィックスと検索オプションをクエリとして設定
        {
            let mut query_pairs = url.query_pairs_mut();
            query_pairs.append_pair("prefix", prefix);
            
            if let Some(opts) = &options {
                if let Some(limit) = opts.limit {
                    query_pairs.append_pair("limit", &limit.to_string());
                }
                if let Some(offset) = opts.offset {
                    query_pairs.append_pair("offset", &offset.to_string());
                }
                if let Some(sort_by) = &opts.sort_by {
                    query_pairs.append_pair("sortBy", &sort_by.to_string());
                }
                if let Some(search) = &opts.search {
                    query_pairs.append_pair("search", search);
                }
            }
        } // query_pairsのスコープはここで終了
        
        let response = self.parent.http_client
            .get(url)
            .header("apikey", &self.parent.api_key)
            .header("Authorization", format!("Bearer {}", &self.parent.api_key))
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(StorageError::ApiError(error_text));
        }
        
        let files = response.json::<Vec<FileObject>>().await?;
        
        Ok(files)
    }
    
    /// ファイルを削除
    pub async fn remove(&self, paths: Vec<&str>) -> Result<(), StorageError> {
        let url = format!("{}/storage/v1/object/{}", self.parent.base_url, self.bucket_id);
        
        let payload = serde_json::json!({
            "prefixes": paths
        });
        
        let response = self.parent.http_client.delete(&url)
            .header("apikey", &self.parent.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(StorageError::ApiError(error_text));
        }
        
        Ok(())
    }
    
    /// 公開URLを取得
    pub fn get_public_url(&self, path: &str) -> String {
        format!("{}/storage/v1/object/public/{}/{}", self.parent.base_url, self.bucket_id, path)
    }
    
    /// 署名付きURLを作成
    pub async fn create_signed_url(&self, path: &str, expires_in: i32) -> Result<String, StorageError> {
        let url = format!("{}/storage/v1/object/sign/{}/{}", self.parent.base_url, self.bucket_id, path);
        
        let payload = serde_json::json!({
            "expiresIn": expires_in
        });
        
        let response = self.parent.http_client.post(&url)
            .header("apikey", &self.parent.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(StorageError::ApiError(error_text));
        }
        
        #[derive(Deserialize)]
        struct SignedUrlResponse {
            signed_url: String,
        }
        
        let signed_url = response.json::<SignedUrlResponse>().await?;
        
        Ok(signed_url.signed_url)
    }

    /// マルチパートアップロードを初期化
    pub async fn initiate_multipart_upload(
        &self,
        path: &str,
        options: Option<FileOptions>
    ) -> Result<InitiateMultipartUploadResponse, StorageError> {
        let url = format!(
            "{}/storage/v1/upload/initiate",
            self.parent.base_url
        );
        
        let options = options.unwrap_or_default();
        
        let cache_control = options.cache_control.unwrap_or_else(|| "max-age=3600".to_string());
        let content_type = options.content_type.unwrap_or_else(|| "application/octet-stream".to_string());
        let upsert = options.upsert.unwrap_or(false);
        
        let payload = serde_json::json!({
            "bucket": self.bucket_id,
            "name": path,
            "cacheControl": cache_control,
            "contentType": content_type,
            "upsert": upsert,
        });
        
        let response = self.parent.http_client.post(&url)
            .header("apikey", &self.parent.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(StorageError::ApiError(error_text));
        }
        
        let initiate_response: InitiateMultipartUploadResponse = response.json().await?;
        
        Ok(initiate_response)
    }
    
    /// チャンクをアップロード
    pub async fn upload_part(
        &self,
        upload_id: &str,
        part_number: u32,
        data: Bytes
    ) -> Result<UploadedPartInfo, StorageError> {
        let url = format!(
            "{}/storage/v1/upload/part",
            self.parent.base_url
        );
        
        let body = reqwest::Body::from(data);
        
        let response = self.parent.http_client.post(&url)
            .header("apikey", &self.parent.api_key)
            .query(&[
                ("uploadId", upload_id),
                ("partNumber", &part_number.to_string()),
                ("bucket", &self.bucket_id),
            ])
            .body(body)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(StorageError::ApiError(error_text));
        }
        
        let etag = response.headers()
            .get("etag")
            .ok_or_else(|| StorageError::new("ETag header not found in response".to_string()))?
            .to_str()
            .map_err(|e| StorageError::new(format!("Invalid ETag header: {}", e)))?
            .to_string();
        
        let part_info = UploadedPartInfo {
            part_number,
            etag,
        };
        
        Ok(part_info)
    }
    
    /// マルチパートアップロードを完了
    pub async fn complete_multipart_upload(
        &self,
        upload_id: &str,
        path: &str,
        parts: Vec<UploadedPartInfo>
    ) -> Result<FileObject, StorageError> {
        let url = format!(
            "{}/storage/v1/upload/complete",
            self.parent.base_url
        );
        
        let payload = CompleteMultipartUploadRequest {
            upload_id: upload_id.to_string(),
            parts,
        };
        
        let response = self.parent.http_client.post(&url)
            .header("apikey", &self.parent.api_key)
            .header("Content-Type", "application/json")
            .query(&[
                ("bucket", &self.bucket_id),
                ("key", &path),
            ])
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(StorageError::ApiError(error_text));
        }
        
        let file_object: FileObject = response.json().await?;
        
        Ok(file_object)
    }
    
    /// マルチパートアップロードを中止
    pub async fn abort_multipart_upload(
        &self,
        upload_id: &str,
        path: &str
    ) -> Result<(), StorageError> {
        let url = format!(
            "{}/storage/v1/upload/abort",
            self.parent.base_url
        );
        
        let payload = serde_json::json!({
            "uploadId": upload_id,
            "bucket": self.bucket_id,
            "key": path,
        });
        
        let response = self.parent.http_client.post(&url)
            .header("apikey", &self.parent.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(StorageError::ApiError(error_text));
        }
        
        Ok(())
    }
    
    /// 大容量ファイルをチャンクでアップロード
    /// 
    /// このメソッドは大きなファイルを自動的にチャンクに分割してアップロードします。
    /// 各チャンクは非同期でアップロードされ、すべてのチャンクがアップロードされると
    /// 自動的にマルチパートアップロードを完了します。
    pub async fn upload_large_file(
        &self,
        path: &str,
        file_path: &Path,
        chunk_size: usize,
        options: Option<FileOptions>
    ) -> Result<FileObject, StorageError> {
        // ファイルを開く
        let mut file = File::open(file_path).await?;
        
        // ファイルサイズを取得
        let file_size = file.metadata().await?.len() as usize;
        
        // チャンク数を計算
        let chunk_count = (file_size + chunk_size - 1) / chunk_size;
        
        if chunk_count == 0 {
            return Err(StorageError::new("File is empty".to_string()));
        }
        
        // マルチパートアップロードを初期化
        let init_response = self.initiate_multipart_upload(path, options).await?;
        
        // 部分アップロード情報を保持するベクター
        let mut uploaded_parts = Vec::with_capacity(chunk_count);
        
        // バッファを準備
        let mut buffer = vec![0u8; chunk_size];
        
        // 各チャンクをアップロード
        for part_number in 1..=chunk_count as u32 {
            // バッファにデータを読み込む
            let n = file.read(&mut buffer).await?;
            
            if n == 0 {
                break;
            }
            
            // 実際に読み込んだサイズに合わせてバッファを調整
            let chunk_data = Bytes::from(buffer[0..n].to_vec());
            
            // チャンクをアップロード
            let part_info = self.upload_part(&init_response.upload_id, part_number, chunk_data).await?;
            
            // アップロードした部分情報を保存
            uploaded_parts.push(part_info);
        }
        
        // マルチパートアップロードを完了
        let file_object = self.complete_multipart_upload(
            &init_response.upload_id,
            path,
            uploaded_parts
        ).await?;
        
        Ok(file_object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_list_buckets() {
        // TODO: モック実装を用いたテスト
    }

    #[tokio::test]
    async fn test_multipart_upload() {
        // このテストは実際のAPIと通信するため、モックサーバーを使用するべきですが、
        // 簡略化のため省略しています。実際の実装ではモックサーバーを使用してください。
    }
}