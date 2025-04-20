use supabase_rust::prelude::*;
use dotenv::dotenv;
use std::env;
use std::io::Cursor;
use serde::{Deserialize, Serialize};
use tokio::fs;
use mime;

#[derive(Debug, Serialize, Deserialize)]
struct FileObject {
    name: String,
    bucket_id: String,
    owner: Option<String>,
    id: String,
    updated_at: Option<String>,
    created_at: String,
    last_accessed_at: Option<String>,
    metadata: serde_json::Value,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Load environment variables from .env file
    dotenv().ok();
    
    // Get Supabase URL and key from environment variables
    let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");
    
    // Initialize the Supabase client
    let supabase = Supabase::new(&supabase_url, &supabase_key);
    
    println!("Starting Storage example");
    
    // First, sign up a test user for our example to test RLS policies
    let test_email = format!("test-storage-{}@example.com", uuid::Uuid::new_v4());
    let test_password = "password123";
    
    let sign_up_result = supabase
        .auth()
        .sign_up(&test_email, test_password)
        .await?;
    
    let user_id = sign_up_result.user.id;
    println!("Created test user with ID: {}", user_id);
    
    // Example 1: List existing buckets
    println!("\nExample 1: Listing storage buckets");
    let buckets = supabase
        .storage()
        .list_buckets()
        .await?;
    
    println!("Available buckets: {}", buckets.len());
    for bucket in &buckets {
        println!("  - {} (public: {})", bucket.name, bucket.public);
    }
    
    // Example 2: Create a new bucket if it doesn't exist
    let test_bucket_name = "test-rust-bucket";
    
    // Check if our test bucket already exists
    let bucket_exists = buckets.iter().any(|bucket| bucket.name == test_bucket_name);
    
    if !bucket_exists {
        println!("\nExample 2: Creating new bucket '{}'", test_bucket_name);
        supabase
            .storage()
            .create_bucket(test_bucket_name, BucketOptions {
                public: true,
                file_size_limit: Some(10 * 1024 * 1024), // 10MB
                allowed_mime_types: Some(vec!["image/png".to_string(), "image/jpeg".to_string(), "text/plain".to_string()]),
                ..Default::default()
            })
            .await?;
        println!("Bucket created successfully");
    } else {
        println!("\nExample 2: Bucket '{}' already exists", test_bucket_name);
    }
    
    // Example 3: Upload a file from memory
    println!("\nExample 3: Uploading a text file from memory");
    let file_name = "hello.txt";
    let file_content = "Hello from Supabase Rust client!";
    let file_data = Cursor::new(file_content);
    
    supabase
        .storage()
        .from(test_bucket_name)
        .upload(file_name, file_data, file_content.len() as u64, mime::TEXT_PLAIN)
        .await?;
    
    println!("Uploaded '{}' successfully", file_name);
    
    // Example 4: Upload a file with folder path
    println!("\nExample 4: Uploading a file to a nested folder");
    let folder_path = "documents/reports/";
    let nested_file_name = format!("{}report.txt", folder_path);
    let report_content = "This is a monthly report for Supabase Rust usage.";
    let report_data = Cursor::new(report_content);
    
    supabase
        .storage()
        .from(test_bucket_name)
        .upload(
            &nested_file_name,
            report_data,
            report_content.len() as u64,
            mime::TEXT_PLAIN,
        )
        .await?;
    
    println!("Uploaded '{}' successfully", nested_file_name);
    
    // Example 5: List files in a bucket
    println!("\nExample 5: Listing files in the bucket");
    let files = supabase
        .storage()
        .from(test_bucket_name)
        .list(Some(""), None)
        .await?;
    
    println!("Files in root of bucket:");
    for file in &files {
        println!("  - {} ({})", file.name, file.id);
    }
    
    // Example 6: List files in a folder
    println!("\nExample 6: Listing files in a specific folder");
    let folder_files = supabase
        .storage()
        .from(test_bucket_name)
        .list(Some("documents/"), None)
        .await?;
    
    println!("Files in 'documents/' folder:");
    for file in &folder_files {
        println!("  - {} ({})", file.name, file.id);
    }
    
    // Example 7: Get a file's public URL
    println!("\nExample 7: Getting public URL for a file");
    let public_url = supabase
        .storage()
        .from(test_bucket_name)
        .get_public_url(file_name);
    
    println!("Public URL for '{}': {}", file_name, public_url);
    
    // Example 8: Generate a signed URL with expiration
    println!("\nExample 8: Generating a signed URL");
    let signed_url = supabase
        .storage()
        .from(test_bucket_name)
        .create_signed_url(file_name, 60) // 60 seconds expiration
        .await?;
    
    println!("Signed URL for '{}' (expires in 60s): {}", file_name, signed_url);
    
    // Example 9: Download a file
    println!("\nExample 9: Downloading a file");
    let downloaded_data = supabase
        .storage()
        .from(test_bucket_name)
        .download(file_name)
        .await?;
    
    let downloaded_content = String::from_utf8_lossy(&downloaded_data);
    println!(
        "Downloaded '{}' (size: {} bytes):\n{}",
        file_name,
        downloaded_data.len(),
        downloaded_content
    );
    
    // Example 10: Move (rename) a file
    println!("\nExample 10: Moving/renaming a file");
    let new_file_name = "hello_renamed.txt";
    supabase
        .storage()
        .from(test_bucket_name)
        .move_file(file_name, new_file_name)
        .await?;
    
    println!("Moved '{}' to '{}'", file_name, new_file_name);
    
    // Example 11: Copy a file
    println!("\nExample 11: Copying a file");
    let copy_file_name = "hello_copy.txt";
    supabase
        .storage()
        .from(test_bucket_name)
        .copy(new_file_name, copy_file_name)
        .await?;
    
    println!("Copied '{}' to '{}'", new_file_name, copy_file_name);
    
    // Example 12: Remove a file
    println!("\nExample 12: Removing a file");
    supabase
        .storage()
        .from(test_bucket_name)
        .remove(&[copy_file_name])
        .await?;
    
    println!("Removed '{}' successfully", copy_file_name);
    
    // Example 13: Upload a local file
    println!("\nExample 13: Creating and uploading a local file");
    
    // Create a temporary file
    let temp_file_path = "temp_test_file.txt";
    let temp_file_content = "This is a temporary file for testing Supabase storage.";
    fs::write(temp_file_path, temp_file_content).await?;
    
    // Upload the file
    supabase
        .storage()
        .from(test_bucket_name)
        .upload_local_file(temp_file_path, "uploaded_local_file.txt", None)
        .await?;
    
    println!("Uploaded local file as 'uploaded_local_file.txt'");
    
    // Clean up temporary file
    fs::remove_file(temp_file_path).await?;
    
    // Example 14: Clean up all files
    println!("\nExample 14: Cleaning up all files in the bucket");
    let all_files = supabase
        .storage()
        .from(test_bucket_name)
        .list(None, None)
        .await?;
    
    let file_paths: Vec<String> = all_files.iter().map(|file| file.name.clone()).collect();
    
    if !file_paths.is_empty() {
        supabase
            .storage()
            .from(test_bucket_name)
            .remove(&file_paths)
            .await?;
        
        println!("Removed {} files from bucket", file_paths.len());
    } else {
        println!("No files to remove");
    }
    
    // Example 15: Delete the bucket (optional)
    // Uncomment to delete the bucket:
    /*
    println!("\nExample 15: Deleting the bucket");
    supabase
        .storage()
        .delete_bucket(test_bucket_name)
        .await?;
    
    println!("Deleted bucket '{}'", test_bucket_name);
    */
    
    println!("\nStorage example completed");
    
    Ok(())
}