//! Types for storage operations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A storage bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bucket {
    /// The bucket ID
    pub id: String,
    
    /// The bucket name
    pub name: String,
    
    /// Owner user ID
    pub owner: Option<String>,
    
    /// Creation timestamp
    pub created_at: Option<String>,
    
    /// Update timestamp
    pub updated_at: Option<String>,
    
    /// Whether the bucket is public
    pub public: Option<bool>,
    
    /// Bucket file size limit in bytes
    #[serde(rename = "file_size_limit")]
    pub file_size_limit: Option<i64>,
    
    /// Allowed MIME types
    #[serde(rename = "allowed_mime_types")]
    pub allowed_mime_types: Option<Vec<String>>,
}

/// Options for creating or updating a bucket
#[derive(Debug, Clone)]
pub struct BucketOptions {
    /// Whether the bucket is public
    pub public: bool,
    
    /// Bucket file size limit in bytes
    pub file_size_limit: Option<i64>,
    
    /// Allowed MIME types
    pub allowed_mime_types: Option<Vec<String>>,
}

impl Default for BucketOptions {
    fn default() -> Self {
        Self {
            public: false,
            file_size_limit: None,
            allowed_mime_types: None,
        }
    }
}

/// A file in a storage bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileObject {
    /// The file name
    pub name: String,
    
    /// The bucket ID
    #[serde(rename = "bucket_id")]
    pub bucket_id: Option<String>,
    
    /// Owner user ID
    pub owner: Option<String>,
    
    /// The file ID
    pub id: Option<String>,
    
    /// The file size in bytes
    #[serde(rename = "size")]
    pub size: Option<i64>,
    
    /// Creation timestamp
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    
    /// Update timestamp
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
    
    /// Last accessed timestamp
    #[serde(rename = "last_accessed_at")]
    pub last_accessed_at: Option<String>,
    
    /// File metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    
    /// MIME type
    #[serde(rename = "mime_type")]
    pub mime_type: Option<String>,
}

/// Options for uploading a file
#[derive(Debug, Clone)]
pub struct FileOptions {
    /// Cache control header
    pub cache_control: Option<String>,
    
    /// Content type header
    pub content_type: Option<String>,
    
    /// Whether to upsert the file
    pub upsert: bool,
}

impl Default for FileOptions {
    fn default() -> Self {
        Self {
            cache_control: None,
            content_type: None,
            upsert: false,
        }
    }
}

/// Options for listing files
#[derive(Debug, Clone)]
pub struct ListOptions {
    /// Maximum number of files to return
    pub limit: Option<i32>,
    
    /// Offset for pagination
    pub offset: Option<i32>,
    
    /// Field to sort by
    pub sort_by: Option<String>,
    
    /// Sort direction
    pub sort_order: Option<SortOrder>,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            limit: None,
            offset: None,
            sort_by: None,
            sort_order: None,
        }
    }
}

/// Sort order for listing files
#[derive(Debug, Clone)]
pub enum SortOrder {
    /// Ascending order
    Asc,
    
    /// Descending order
    Desc,
}

impl ToString for SortOrder {
    fn to_string(&self) -> String {
        match self {
            SortOrder::Asc => "asc".to_string(),
            SortOrder::Desc => "desc".to_string(),
        }
    }
}

/// Response for a signed URL request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedUrlResponse {
    /// The signed URL
    #[serde(rename = "signedURL")]
    pub signed_url: String,
    
    /// The path to the file
    pub path: Option<String>,
    
    /// Any error that occurred
    pub error: Option<String>,
}

/// Response for a signed URL upload request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedUploadResponse {
    /// The upload path
    pub path: String,
    
    /// The upload token
    pub token: String,
    
    /// The signed URL for uploading
    pub url: String,
}