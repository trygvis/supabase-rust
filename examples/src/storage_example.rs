use supabase_rust::prelude::*;
use dotenv::dotenv;
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use uuid::Uuid;

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
    
    // Define bucket name - this bucket should already exist in your Supabase project
    let bucket_name = "test-files";
    
    // Create a storage client
    let storage = supabase.storage();
    
    // Check if the bucket exists, create if it doesn't
    match storage.get_bucket(bucket_name).await {
        Ok(_) => {
            println!("Bucket '{}' already exists", bucket_name);
        },
        Err(_) => {
            println!("Creating bucket '{}'", bucket_name);
            match storage.create_bucket(bucket_name, Some(json!({
                "public": true,
                "file_size_limit": 1024 * 1024 * 2 // 2MB
            }))).await {
                Ok(_) => println!("Bucket created successfully"),
                Err(e) => {
                    println!("Error creating bucket: {:?}", e);
                    return Err(e);
                }
            }
        }
    }
    
    // List buckets
    match storage.list_buckets().await {
        Ok(buckets) => {
            println!("Available buckets:");
            for bucket in buckets {
                println!("  - {}", bucket.name);
            }
        },
        Err(e) => println!("Error listing buckets: {:?}", e)
    }
    
    // Create a test file
    let file_name = format!("test-file-{}.txt", Uuid::new_v4());
    let file_content = "This is a test file uploaded via Rust Supabase client";
    let temp_path = format!("/tmp/{}", file_name);
    
    // Write content to a temp file
    let mut file = File::create(&temp_path).expect("Failed to create temp file");
    file.write_all(file_content.as_bytes()).expect("Failed to write to temp file");
    
    // Upload the file
    println!("Uploading file '{}'", file_name);
    match storage.upload_file(bucket_name, &file_name, &temp_path).await {
        Ok(_) => println!("File uploaded successfully"),
        Err(e) => {
            println!("Error uploading file: {:?}", e);
            return Err(e);
        }
    }
    
    // List files in bucket
    match storage.list_files(bucket_name).await {
        Ok(files) => {
            println!("Files in bucket '{}':", bucket_name);
            for file in files {
                println!("  - {} ({})", file.name, file.id);
            }
        },
        Err(e) => println!("Error listing files: {:?}", e)
    }
    
    // Get public URL for the file
    match storage.get_public_url(bucket_name, &file_name) {
        Ok(url) => println!("Public URL: {}", url),
        Err(e) => println!("Error getting public URL: {:?}", e)
    }
    
    // Download the file
    let download_path = format!("/tmp/downloaded-{}", file_name);
    println!("Downloading file to '{}'", download_path);
    match storage.download_file(bucket_name, &file_name, &download_path).await {
        Ok(_) => {
            println!("File downloaded successfully");
            
            // Verify the content
            let mut file = File::open(&download_path).expect("Failed to open downloaded file");
            let mut content = String::new();
            file.read_to_string(&mut content).expect("Failed to read downloaded file");
            
            assert_eq!(content, file_content, "Downloaded content doesn't match original");
            println!("Downloaded content verified successfully");
        },
        Err(e) => println!("Error downloading file: {:?}", e)
    }
    
    // Clean up - remove the uploaded file
    println!("Removing file '{}'", file_name);
    match storage.remove_file(bucket_name, &[&file_name]).await {
        Ok(_) => println!("File removed successfully"),
        Err(e) => println!("Error removing file: {:?}", e)
    }
    
    // Clean up temporary files
    std::fs::remove_file(temp_path).ok();
    std::fs::remove_file(download_path).ok();
    
    println!("Storage example completed");
    
    Ok(())
}