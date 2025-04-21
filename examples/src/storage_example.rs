use supabase_rust_gftd::prelude::*;
use supabase_rust_gftd::Supabase;
use supabase_rust_gftd::storage::ImageTransformOptions;
use dotenv::dotenv;
use std::env;
use std::path::PathBuf;
use std::io::Cursor;
use serde::{Deserialize, Serialize};
use tokio::fs;
use mime;
use bytes::Bytes;
use serde_json::json;

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

mod image_transform_examples {
    use std::env;
    use std::path::PathBuf;
    use tokio::fs;
    use supabase_rust_gftd::prelude::*;
    use supabase_rust_gftd::Supabase;
    use supabase_rust_gftd::storage::ImageTransformOptions;
    
    pub async fn run_image_transform_examples() -> std::result::Result<(), Box<dyn std::error::Error>> {
        // Supabaseの認証情報を環境変数から取得
        let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
        let supabase_key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");
        
        // バケット名とテスト画像のパスを設定
        let bucket_name = "test-images";
        let local_image_path = PathBuf::from("./test-image.jpg");
        let upload_path = "test-transform-image.jpg";
        
        // テスト用の画像が存在するか確認
        if !local_image_path.exists() {
            println!("テスト用の画像が見つかりません: {:?}", local_image_path);
            println!("テスト画像を用意して再実行してください。");
            return Ok(());
        }
        
        // Supabaseクライアントの初期化
        let supabase = Supabase::new(&supabase_url, &supabase_key);
        let storage = supabase.storage();
        
        println!("\n=== 画像変換機能の例 ===\n");
        
        // バケットが存在するか確認し、なければ作成
        let buckets = storage.list_buckets().await?;
        let bucket_exists = buckets.iter().any(|b| b.name == bucket_name);
        
        if !bucket_exists {
            println!("バケット '{}' を作成します...", bucket_name);
            storage.create_bucket(bucket_name, true).await?;
            println!("バケットを作成しました。");
        }
        
        // 画像をアップロード
        println!("画像をアップロードしています...");
        let file_data = fs::read(&local_image_path).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        let upload_result = storage
            .from(bucket_name)
            .upload(upload_path, file_data, None)
            .await?;
            
        println!("画像をアップロードしました: {}", upload_result.name);
        
        // 画像変換オプションを作成
        let transform_options = vec![
            ("サムネイル (小)", ImageTransformOptions::new()
                .with_width(100)
                .with_height(100)
                .with_resize("cover")),
                
            ("中サイズ (WebP)", ImageTransformOptions::new()
                .with_width(300)
                .with_height(200)
                .with_resize("contain")
                .with_format("webp")),
                
            ("大サイズ (低画質)", ImageTransformOptions::new()
                .with_width(800)
                .with_height(600)
                .with_quality(50))
        ];
        
        // 各変換オプションで画像を変換
        println!("\n画像変換の例:");
        
        for (name, options) in transform_options {
            println!("\n{} の変換:", name);
            
            // 変換画像を取得
            let transformed_image = storage
                .from(bucket_name)
                .transform_image(upload_path, options.clone())
                .await?;
                
            println!("変換後の画像サイズ: {} バイト", transformed_image.len());
            
            // 変換画像の公開URLを取得
            let public_url = storage
                .from(bucket_name)
                .get_public_transform_url(upload_path, options.clone());
                
            println!("公開URL: {}", public_url);
            
            // 変換画像の署名付きURLを取得
            let signed_url = storage
                .from(bucket_name)
                .create_signed_transform_url(upload_path, options, 60)
                .await?;
                
            println!("署名付きURL (有効期限60秒): {}", signed_url);
        }
        
        // クリーンアップ（必要に応じて）
        println!("\nテスト画像を削除しますか？(y/n)");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        if input.trim().to_lowercase() == "y" {
            println!("テスト画像を削除します...");
            storage
                .from(bucket_name)
                .remove(vec![upload_path])
                .await?;
                
            println!("テスト画像を削除しました。");
        }
        
        Ok(())
    }
}

/// S3互換APIの例を実行
async fn run_s3_compatible_example(supabase: &Supabase) -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("\n=== S3互換APIの例 ===\n");
    
    // S3互換オプションを設定
    let s3_options = supabase_rust_gftd::storage::s3::S3Options {
        access_key_id: "your-access-key".to_string(), // 実際の環境では適切な値に置き換えてください
        secret_access_key: "your-secret-key".to_string(), // 実際の環境では適切な値に置き換えてください
        region: Some("auto".to_string()),
        ..Default::default()
    };
    
    // S3互換クライアントを取得
    let storage_client = supabase.storage();
    let bucket_client = storage_client.from("test-bucket");
    let s3_client = bucket_client.s3_compatible(s3_options);
    
    // テキストファイルをアップロード
    let text_content = "This is a test file uploaded via S3 compatible API";
    let text_bytes = Bytes::from(text_content.as_bytes().to_vec());
    
    println!("Uploading text file via S3 compatible API...");
    s3_client.put_object(
        "s3-test/test.txt", 
        text_bytes.clone(), 
        Some("text/plain".to_string()),
        Some({
            let mut metadata = std::collections::HashMap::new();
            metadata.insert("description".to_string(), "Test file for S3 API".to_string());
            metadata
        })
    ).await?;
    println!("Text file uploaded successfully");
    
    // ファイルをダウンロード
    println!("Downloading file via S3 compatible API...");
    let downloaded_data = s3_client.get_object("s3-test/test.txt").await?;
    let downloaded_text = String::from_utf8_lossy(&downloaded_data);
    println!("Downloaded content: {}", downloaded_text);
    
    // メタデータを取得
    println!("Retrieving file metadata...");
    let metadata = s3_client.head_object("s3-test/test.txt").await?;
    println!("File metadata: {:?}", metadata);
    
    // 複数のファイルをアップロード
    println!("Uploading multiple files...");
    for i in 1..4 {
        let content = format!("This is file number {}", i);
        let bytes = Bytes::from(content.into_bytes());
        s3_client.put_object(
            &format!("s3-test/multiple/file{}.txt", i),
            bytes,
            Some("text/plain".to_string()),
            None
        ).await?;
    }
    println!("Multiple files uploaded");
    
    // ファイルをコピー
    println!("Copying file...");
    s3_client.copy_object(
        "s3-test/test.txt",
        "s3-test/test-copy.txt"
    ).await?;
    println!("File copied successfully");
    
    // コピーされたファイルをダウンロードして確認
    let copied_data = s3_client.get_object("s3-test/test-copy.txt").await?;
    let copied_text = String::from_utf8_lossy(&copied_data);
    println!("Copied file content: {}", copied_text);
    
    // ファイルを削除
    println!("Would you like to delete the test files? (y/n)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    if input.trim().to_lowercase() == "y" {
        println!("Deleting test files...");
        let objects_to_delete = vec![
            "s3-test/test.txt", 
            "s3-test/test-copy.txt"
        ];
        
        for object in &objects_to_delete {
            s3_client.delete_object(object).await?;
        }
        
        println!("Test files deleted");
        
        // multiple ディレクトリのファイルも削除
        println!("Would you like to delete the multiple files too? (y/n)");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        if input.trim().to_lowercase() == "y" {
            for i in 1..4 {
                let path = format!("s3-test/multiple/file{}.txt", i);
                s3_client.delete_object(&path).await?;
            }
            println!("Multiple files deleted");
        }
    }
    
    Ok(())
}

/// 基本的なストレージ操作の例を実行
async fn run_basic_storage_operations(supabase: &Supabase) -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 基本的なストレージ操作の例 ===\n");
    
    // ストレージクライアントを取得
    let storage = supabase.storage();
    
    // 1. バケットの一覧を取得
    println!("バケットの一覧を取得しています...");
    let buckets = storage.list_buckets().await?;
    
    println!("バケット一覧:");
    for bucket in &buckets {
        println!("  - {}: {}", bucket.id, bucket.name);
    }
    
    // 2. 新しいバケットを作成
    let test_bucket_name = format!("test-bucket-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
    println!("\n新しいバケット '{}' を作成しています...", test_bucket_name);
    
    let new_bucket = storage.create_bucket(&test_bucket_name, false).await?;
    println!("バケットを作成しました: {}", new_bucket.name);
    
    // 3. テキストファイルをアップロード
    println!("\nテキストファイルをアップロードしています...");
    let text_content = "This is a test file uploaded from Rust client";
    
    let upload_result = storage
        .from(&test_bucket_name)
        .upload("test.txt", text_content.as_bytes().to_vec(), None)
        .await?;
        
    println!("ファイルをアップロードしました: {}", upload_result.name);
    
    // 4. ファイルの一覧を取得
    println!("\nファイルの一覧を取得しています...");
    let files = storage
        .from(&test_bucket_name)
        .list(Some(""), Some(100), Some("/"), Some("."))
        .await?;
        
    println!("ファイル一覧:");
    for file in &files {
        println!("  - {}: {} bytes", file.name, file.metadata.size);
    }
    
    // 5. ファイルの公開URLを取得
    println!("\nファイルの公開URLを取得しています...");
    let public_url = storage
        .from(&test_bucket_name)
        .get_public_url("test.txt");
        
    println!("公開URL: {}", public_url);
    
    // 6. 署名付きURLを作成
    println!("\n署名付きURLを作成しています...");
    let signed_url = storage
        .from(&test_bucket_name)
        .create_signed_url("test.txt", 60)
        .await?;
        
    println!("署名付きURL (有効期限60秒): {}", signed_url);
    
    // 7. ファイルをダウンロード
    println!("\nファイルをダウンロードしています...");
    let downloaded_data = storage
        .from(&test_bucket_name)
        .download("test.txt")
        .await?;
        
    let downloaded_text = String::from_utf8_lossy(&downloaded_data);
    println!("ダウンロードしたファイルの内容: {}", downloaded_text);
    
    // 8. クリーンアップ
    println!("\nクリーンアップを行いますか？(y/n)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    if input.trim().to_lowercase() == "y" {
        println!("ファイルを削除しています...");
        storage
            .from(&test_bucket_name)
            .remove(vec!["test.txt"])
            .await?;
            
        println!("バケットを削除しています...");
        storage.delete_bucket(&test_bucket_name).await?;
        
        println!("クリーンアップ完了");
    }
    
    Ok(())
}

/// 大きなファイルのアップロード例を実行
async fn run_large_file_upload(supabase: &Supabase) -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 大きなファイルのアップロード例 ===\n");
    
    // この例では実際の大きなファイルではなく、生成したデータを使用します
    println!("生成したデータを使った大きなファイルのアップロード例です。");
    
    // ストレージクライアントを取得
    let storage = supabase.storage();
    
    // テスト用バケットの名前
    let test_bucket_name = format!("large-file-test-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
    
    // バケットを作成
    println!("テスト用バケット '{}' を作成しています...", test_bucket_name);
    storage.create_bucket(&test_bucket_name, false).await?;
    
    // 1MBのランダムデータを生成
    println!("1MBのテストデータを生成しています...");
    let data_size = 1024 * 1024; // 1MB
    let mut large_data = Vec::with_capacity(data_size);
    
    for i in 0..data_size {
        large_data.push((i % 256) as u8);
    }
    
    // 大きなファイルをアップロード
    println!("大きなファイルをアップロードしています...");
    let start_time = std::time::Instant::now();
    
    let upload_result = storage
        .from(&test_bucket_name)
        .upload("large-file.bin", large_data, None)
        .await?;
        
    let duration = start_time.elapsed();
    println!("アップロード完了: {}", upload_result.name);
    println!("アップロード時間: {:.2}秒", duration.as_secs_f64());
    
    // クリーンアップ
    println!("\nクリーンアップを行いますか？(y/n)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    if input.trim().to_lowercase() == "y" {
        println!("ファイルを削除しています...");
        storage
            .from(&test_bucket_name)
            .remove(vec!["large-file.bin"])
            .await?;
            
        println!("バケットを削除しています...");
        storage.delete_bucket(&test_bucket_name).await?;
        
        println!("クリーンアップ完了");
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv().ok();
    
    // Get Supabase URL and key from environment variables
    let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");
    
    // Initialize the Supabase client
    let supabase = Supabase::new(&supabase_url, &supabase_key);
    
    println!("Starting storage example");
    
    // First, sign up a test user for our examples
    let test_email = format!("test-storage-{}@example.com", uuid::Uuid::new_v4());
    let test_password = "password123";
    
    let sign_up_result = supabase
        .auth()
        .sign_up(&test_email, test_password)
        .await?;
    
    println!("Created test user with ID: {}", sign_up_result.user.id);
    
    // 基本的なストレージ操作の例を実行
    if let Err(e) = run_basic_storage_operations(&supabase).await {
        println!("基本的なストレージ操作の例でエラーが発生しました: {}", e);
    }
    
    // S3互換APIの例を実行
    if let Err(e) = run_s3_compatible_example(&supabase).await {
        println!("S3互換APIの例でエラーが発生しました: {}", e);
    }
    
    // 大きなファイルのアップロード例を実行
    if let Err(e) = run_large_file_upload(&supabase).await {
        println!("大きなファイルのアップロード例でエラーが発生しました: {}", e);
    }
    
    // 画像変換の例を実行
    // 注：テスト画像のパスを適切に設定する必要があります
    if let Err(e) = image_transform_examples::run_image_transform_examples().await {
        println!("画像変換の例でエラーが発生しました: {}", e);
    }
    
    println!("Storage example completed");
    
    Ok(())
}