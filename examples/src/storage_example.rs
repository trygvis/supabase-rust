use supabase_rust::prelude::*;
use dotenv::dotenv;
use std::env;
use std::io::Cursor;
use serde::{Deserialize, Serialize};
use tokio::fs;
use mime;
use std::path::PathBuf;
use supabase_rust::storage::ImageTransformOptions;

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
    use std::fs;
    use std::path::PathBuf;
    use supabase_rust::prelude::*;
    use supabase_rust::storage::ImageTransformOptions;
    
    pub async fn run_image_transform_examples() -> Result<(), Box<dyn std::error::Error>> {
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
        let file_data = fs::read(&local_image_path)?;
        
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
        std::io::stdin().read_line(&mut input)?;
        
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    println!("=== Supabase Storage Examples ===");
    
    // Basic storage operations
    run_basic_storage_operations().await?;
    
    // Large file upload example
    run_large_file_upload().await?;
    
    // 画像変換機能の例を実行
    println!("\n画像変換機能の例を実行しますか？(y/n)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() == "y" {
        image_transform_examples::run_image_transform_examples().await?;
    }
    
    Ok(())
}