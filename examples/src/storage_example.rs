use bytes::Bytes;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::fs::File as StdFile;
use std::env;
use std::io::{Read, Write};
use supabase_rust_gftd::Supabase;
use supabase_rust_gftd::storage::{FileOptions, ListOptions};
use tempfile::NamedTempFile;

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
    mime_type: Option<String>,
    size: i64,
}

mod image_transform_examples {
    use std::env;
    use std::io::Write;
    use supabase_rust_gftd::storage::{FileOptions, ImageTransformOptions};
    use supabase_rust_gftd::Supabase;
    use tempfile::NamedTempFile;

    pub async fn run_image_transform_examples(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        // Supabaseの認証情報を環境変数から取得
        let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
        let supabase_key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");

        // バケット名とテスト画像のパスを設定
        let bucket_name = "test-images";

        // テスト用の一時画像ファイルを作成
        let mut temp_file = NamedTempFile::new()?;
        // ダミー画像データを書き込む (1x1 PNG)
        let dummy_png_data: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xD7, 0x63, 0x60, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        temp_file.write_all(dummy_png_data)?;
        let local_image_path = temp_file.path().to_path_buf();
        let upload_path = "test-transform-image.jpg";

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
        // アップロードオプションを追加して、正しいMIMEタイプを設定
        let upload_options = FileOptions::new().with_content_type("image/png");
        let upload_result = storage
            .from(bucket_name)
            .upload(
                upload_path,
                local_image_path.as_path(),
                Some(upload_options),
            )
            .await?;

        println!("画像をアップロードしました: {}", upload_result.name);

        // 画像変換オプションを作成
        let transform_options = vec![
            (
                "サムネイル (小)",
                ImageTransformOptions::new()
                    .with_width(100)
                    .with_height(100)
                    .with_resize("cover"),
            ),
            (
                "中サイズ (WebP)",
                ImageTransformOptions::new()
                    .with_width(300)
                    .with_height(200)
                    .with_resize("contain")
                    .with_format("webp"),
            ),
            (
                "大サイズ (低画質)",
                ImageTransformOptions::new()
                    .with_width(800)
                    .with_height(600)
                    .with_quality(50),
            ),
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

        // クリーンアップ
        println!("\nテスト画像を削除します...");
        storage.from(bucket_name).remove(vec![upload_path]).await?;

        println!("テスト画像を削除しました。");

        Ok(())
    }
}

/// S3互換APIの例を実行
async fn run_s3_compatible_example(
    supabase: &Supabase,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
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

    // 一時ファイルを作成してテキストを書き込む
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(text_content.as_bytes())?;
    let file_path = temp_file.path();

    println!("Uploading text file via S3 compatible API...");

    // 一時ファイルのパスから読み込んだデータをバイトに変換
    let mut file_data = Vec::new();
    let mut file = StdFile::open(file_path)?;
    file.read_to_end(&mut file_data)?;

    let text_bytes = Bytes::from(file_data);

    s3_client
        .put_object(
            "s3-test/test.txt",
            text_bytes.clone(),
            Some("text/plain".to_string()),
            Some({
                let mut metadata = std::collections::HashMap::new();
                metadata.insert(
                    "description".to_string(),
                    "Test file for S3 API".to_string(),
                );
                metadata
            }),
        )
        .await?;
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
        s3_client
            .put_object(
                &format!("s3-test/multiple/file{}.txt", i),
                bytes,
                Some("text/plain".to_string()),
                None,
            )
            .await?;
    }

    // オブジェクト一覧を取得
    println!("Listing objects...");
    let objects = s3_client
        .list_objects(Some("s3-test/multiple/"), None, None)
        .await?;
    println!("Objects in directory: {:?}", objects);

    // オブジェクトを削除
    println!("Deleting objects...");
    s3_client.delete_object("s3-test/test.txt").await?;
    println!("Object deleted");

    Ok(())
}

/// 基本的なストレージ操作の例を実行
async fn run_basic_storage_operations(
    supabase: &Supabase,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 基本的なストレージ操作の例 ===\n");

    // テストバケットの名前
    let bucket_name = "test-bucket";

    // ストレージクライアントを取得
    let storage = supabase.storage();

    // バケット操作
    println!("バケット一覧を取得中...");
    let buckets = storage.list_buckets().await?;
    println!("バケット一覧: {:?}", buckets);

    // テストバケットが存在するか確認
    let bucket_exists = buckets.iter().any(|b| b.name == bucket_name);

    if !bucket_exists {
        println!("バケット '{}' を作成します...", bucket_name);
        storage.create_bucket(bucket_name, true).await?;
        println!("バケットを作成しました: {}", bucket_name);
    } else {
        println!("バケット '{}' は既に存在します", bucket_name);
    }

    // テキストファイルのアップロード
    println!("\nテキストファイルをアップロードしています...");

    // 一時ファイルを作成
    let mut temp_file = NamedTempFile::new()?;
    let text_content = "This is a test file for storage operations.";
    temp_file.write_all(text_content.as_bytes())?;

    let upload_result = storage
        .from(bucket_name)
        .upload("test.txt", temp_file.path(), None)
        .await?;

    println!("ファイルをアップロードしました: {}", upload_result.name);

    // ファイル一覧の取得
    println!("\nファイル一覧を取得しています...");

    let list_options = ListOptions::new().limit(100);

    let files = storage
        .from(bucket_name)
        .list("", Some(list_options))
        .await?;

    println!("バケット内のファイル:");
    for file in &files {
        println!("  - {}: {} bytes", file.name, file.size);
    }

    // 公開URLの取得
    println!("\n公開URLを取得しています...");
    let public_url = storage.from(bucket_name).get_public_url("test.txt");

    println!("公開URL: {}", public_url);

    // 署名付きURLの取得
    println!("\n署名付きURLを取得しています...");
    let signed_url = storage
        .from(bucket_name)
        .create_signed_url("test.txt", 60)
        .await?;

    println!("署名付きURL (有効期限60秒): {}", signed_url);

    // ファイルのダウンロード
    println!("\nファイルをダウンロードしています...");
    let downloaded_data = storage.from(bucket_name).download("test.txt").await?;

    let downloaded_text = String::from_utf8_lossy(&downloaded_data);
    println!("ダウンロードしたテキスト: {}", downloaded_text);

    // ファイルの削除
    println!("\nファイルを削除しています...");
    storage.from(bucket_name).remove(vec!["test.txt"]).await?;

    println!("ファイルを削除しました");

    // バケットの削除（オプション）
    /*
    println!("\nバケットを削除しています...");
    storage.delete_bucket(bucket_name).await?;
    println!("バケットを削除しました: {}", bucket_name);
    */

    Ok(())
}

/// 大容量ファイルのアップロード例を実行
async fn run_large_file_upload(
    supabase: &Supabase,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 大容量ファイルのアップロード例 ===\n");

    // テストバケットの名前
    let bucket_name = "test-bucket";

    // ストレージクライアントを取得
    let storage = supabase.storage();

    // バケットが存在するか確認し、なければ作成
    let buckets = storage.list_buckets().await?;
    let bucket_exists = buckets.iter().any(|b| b.name == bucket_name);

    if !bucket_exists {
        println!("バケット '{}' を作成します...", bucket_name);
        storage.create_bucket(bucket_name, true).await?;
        println!("バケットを作成しました: {}", bucket_name);
    }

    // 大きなテストファイルを作成 (5MB)
    println!("テスト用の大容量ファイルを作成しています...");
    let file_size = 5 * 1024 * 1024; // 5MB

    // 一時ファイルを作成
    let mut temp_file = NamedTempFile::new()?;

    // ランダムデータを生成して書き込む
    let mut large_data = Vec::with_capacity(file_size);
    for i in 0..file_size {
        large_data.push((i % 256) as u8);
    }
    temp_file.write_all(&large_data)?;

    // マルチパートアップロードでファイルをアップロード
    println!("\nマルチパートアップロードを開始しています...");

    let file_path = temp_file.path();

    // アップロードオプションを設定
    let upload_options = FileOptions::new().with_content_type("application/octet-stream");

    // マルチパートアップロードを実行（大きなファイルのため）
    let upload_result = storage
        .from(bucket_name)
        .upload_large_file(
            "large-file.bin",
            file_path,
            1024 * 1024, // 1MBチャンク
            Some(upload_options),
        )
        .await?;

    println!(
        "大容量ファイルのアップロードが完了しました: {}",
        upload_result.name
    );

    // ファイルを削除（クリーンアップ）
    println!("\nファイルを削除しています...");
    storage
        .from(bucket_name)
        .remove(vec!["large-file.bin"])
        .await?;

    println!("ファイルを削除しました");

    Ok(())
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // 環境変数を.envファイルから読み込む
    dotenv().ok();

    // Supabaseの認証情報を環境変数から取得
    let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");

    println!("Using Supabase URL: {}", supabase_url);

    // Supabaseクライアントの初期化
    let supabase = Supabase::new(&supabase_url, &supabase_key);

    println!("==== Supabase Storage Examples ====");

    // S3ドキュメントを処理...
    match run_basic_storage_operations(&supabase).await {
        Ok(_) => println!("\n基本的なストレージ操作の例が完了しました"),
        Err(e) => println!(
            "\n基本的なストレージ操作の例でエラーが発生しました: {:?}",
            e
        ),
    }

    match run_large_file_upload(&supabase).await {
        Ok(_) => println!("\n大容量ファイルのアップロード例が完了しました"),
        Err(e) => println!(
            "\n大容量ファイルのアップロード例でエラーが発生しました: {:?}",
            e
        ),
    }

    // 画像変換の例を実行
    match image_transform_examples::run_image_transform_examples().await {
        Ok(_) => println!("\n画像変換機能の例が完了しました"),
        Err(e) => println!("\n画像変換機能の例でエラーが発生しました: {:?}", e),
    }

    // S3互換APIの例を実行
    match run_s3_compatible_example(&supabase).await {
        Ok(_) => println!("\nS3互換APIの例が完了しました"),
        Err(e) => println!("\nS3互換APIの例でエラーが発生しました: {:?}", e),
    }

    println!("\n全ての例が実行されました。");

    Ok(())
}
