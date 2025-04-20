//! Storage operations for file uploads and downloads

mod types;

use reqwest::{Client, multipart};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::error::Error;
use crate::fetch::Fetch;

pub use types::*;

/// Client for Supabase Storage
pub struct StorageClient {
    /// The base URL for the Supabase project
    url: String,
    
    /// The anonymous API key for the Supabase project
    key: String,
    
    /// HTTP client used for requests
    client: Client,
}

/// Client for a specific storage bucket
pub struct BucketClient<'a> {
    /// Reference to the storage client
    storage: &'a StorageClient,
    
    /// The bucket ID
    bucket_id: String,
}

impl StorageClient {
    /// Create a new StorageClient
    pub(crate) fn new(url: &str, key: &str, client: Client) -> Self {
        Self {
            url: url.to_string(),
            key: key.to_string(),
            client,
        }
    }
    
    /// Get the base URL for storage operations
    fn get_url(&self, path: &str) -> String {
        format!("{}/storage/v1{}", self.url, path)
    }
    
    /// Get a client for a specific bucket
    pub fn from(&self, bucket_id: &str) -> BucketClient {
        BucketClient {
            storage: self,
            bucket_id: bucket_id.to_string(),
        }
    }
    
    /// Create a new bucket
    pub async fn create_bucket(&self, id: &str, options: BucketOptions) -> Result<Bucket, Error> {
        let url = self.get_url("/bucket");
        
        let mut body = HashMap::new();
        body.insert("id".to_string(), id.to_string());
        body.insert("name".to_string(), id.to_string());
        body.insert("public".to_string(), options.public.to_string());
        
        let bucket = Fetch::post(&self.client, &url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .json(&body)?
            .execute::<Bucket>()
            .await?;
        
        Ok(bucket)
    }
    
    /// Get a bucket by ID
    pub async fn get_bucket(&self, id: &str) -> Result<Bucket, Error> {
        let url = self.get_url(&format!("/bucket/{}", id));
        
        let bucket = Fetch::get(&self.client, &url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .execute::<Bucket>()
            .await?;
        
        Ok(bucket)
    }
    
    /// Get all buckets
    pub async fn list_buckets(&self) -> Result<Vec<Bucket>, Error> {
        let url = self.get_url("/bucket");
        
        let buckets = Fetch::get(&self.client, &url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .execute::<Vec<Bucket>>()
            .await?;
        
        Ok(buckets)
    }
    
    /// Update a bucket
    pub async fn update_bucket(&self, id: &str, options: BucketOptions) -> Result<Bucket, Error> {
        let url = self.get_url(&format!("/bucket/{}", id));
        
        let mut body = HashMap::new();
        body.insert("public".to_string(), options.public.to_string());
        
        let bucket = Fetch::put(&self.client, &url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .json(&body)?
            .execute::<Bucket>()
            .await?;
        
        Ok(bucket)
    }
    
    /// Delete a bucket
    pub async fn delete_bucket(&self, id: &str) -> Result<(), Error> {
        let url = self.get_url(&format!("/bucket/{}", id));
        
        Fetch::delete(&self.client, &url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .execute_raw()
            .await?;
        
        Ok(())
    }
    
    /// Empty a bucket
    pub async fn empty_bucket(&self, id: &str) -> Result<(), Error> {
        let url = self.get_url(&format!("/bucket/{}/empty", id));
        
        Fetch::post(&self.client, &url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .execute_raw()
            .await?;
        
        Ok(())
    }
}

impl<'a> BucketClient<'a> {
    /// Upload a file to the bucket
    pub async fn upload(&self, path: &str, file_data: Vec<u8>, options: FileOptions) -> Result<FileObject, Error> {
        let url = self.storage.get_url(&format!("/object/{}/{}", self.bucket_id, path));
        
        let form = multipart::Form::new().part(
            "file",
            multipart::Part::bytes(file_data)
                .file_name(Path::new(path).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "file".to_string()))
        );
        
        let response = self.storage.client.post(&url)
            .header("apikey", &self.storage.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .header("Cache-Control", options.cache_control.unwrap_or_else(|| "3600".to_string()))
            .header("x-upsert", options.upsert.to_string())
            .multipart(form)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(Error::storage(format!("Upload failed with status {}: {}", status, text)));
        }
        
        let file_object = response.json::<FileObject>().await?;
        Ok(file_object)
    }
    
    /// Download a file from the bucket
    pub async fn download(&self, path: &str) -> Result<Vec<u8>, Error> {
        let url = self.storage.get_url(&format!("/object/{}/{}", self.bucket_id, path));
        
        let response = Fetch::get(&self.storage.client, &url)
            .header("apikey", &self.storage.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .execute_raw()
            .await?;
        
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
    
    /// List files in the bucket
    pub async fn list(&self, path: Option<&str>, options: ListOptions) -> Result<Vec<FileObject>, Error> {
        let mut url = self.storage.get_url(&format!("/object/list/{}", self.bucket_id));
        
        // Convert options to query parameters
        let mut params = HashMap::new();
        if let Some(path) = path {
            params.insert("prefix".to_string(), path.to_string());
        }
        if let Some(limit) = options.limit {
            params.insert("limit".to_string(), limit.to_string());
        }
        if let Some(offset) = options.offset {
            params.insert("offset".to_string(), offset.to_string());
        }
        if let Some(sort_by) = options.sort_by {
            params.insert("sortBy".to_string(), sort_by.to_string());
        }
        
        let files = Fetch::get(&self.storage.client, &url)
            .header("apikey", &self.storage.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .query(params)
            .execute::<Vec<FileObject>>()
            .await?;
        
        Ok(files)
    }
    
    /// Delete files in the bucket
    pub async fn delete(&self, paths: &[&str]) -> Result<(), Error> {
        let url = self.storage.get_url(&format!("/object/{}", self.bucket_id));
        
        let body = serde_json::json!({
            "prefixes": paths
        });
        
        Fetch::delete(&self.storage.client, &url)
            .header("apikey", &self.storage.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .json(&body)?
            .execute_raw()
            .await?;
        
        Ok(())
    }
    
    /// Move a file within the bucket
    pub async fn move_(&self, from_path: &str, to_path: &str) -> Result<FileObject, Error> {
        let url = self.storage.get_url(&format!("/object/move"));
        
        let body = serde_json::json!({
            "bucketId": self.bucket_id,
            "sourceKey": from_path,
            "destinationKey": to_path,
        });
        
        let file_object = Fetch::post(&self.storage.client, &url)
            .header("apikey", &self.storage.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .json(&body)?
            .execute::<FileObject>()
            .await?;
        
        Ok(file_object)
    }
    
    /// Create a signed URL to access a file
    pub async fn create_signed_url(&self, path: &str, expires_in: i64) -> Result<SignedUrlResponse, Error> {
        let url = self.storage.get_url(&format!("/object/sign/{}/{}", self.bucket_id, path));
        
        let body = serde_json::json!({
            "expiresIn": expires_in
        });
        
        let signed_url = Fetch::post(&self.storage.client, &url)
            .header("apikey", &self.storage.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .json(&body)?
            .execute::<SignedUrlResponse>()
            .await?;
        
        Ok(signed_url)
    }
    
    /// Get the public URL for a file
    pub fn get_public_url(&self, path: &str) -> String {
        format!("{}/storage/v1/object/public/{}/{}", self.storage.url, self.bucket_id, path)
    }
}