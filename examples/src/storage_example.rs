use bytes::Bytes;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File as StdFile;
use std::io::{Read, Write};
use supabase_rust_gftd::storage::{FileOptions, ListOptions};
use supabase_rust_gftd::Supabase;
use tempfile::NamedTempFile;
use uuid::Uuid;

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
            .with_auth(&dummy_token)?
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
                .with_auth(&dummy_token)?
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
                .with_auth(&dummy_token)?
                .create_signed_transform_url(upload_path, options, 60)
                .await?;

            println!("署名付きURL (有効期限60秒): {}", signed_url);
        }

        // クリーンアップ
        println!("\nテスト画像を削除します...");
        storage
            .from(bucket_name)
            .with_auth(&dummy_token)?
            .remove(vec![upload_path])
            .await?;

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
    storage: &supabase_rust_gftd::storage::StorageClient,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 基本的なストレージ操作の例 ===\n");

    // バケット名とテストファイルのパスを設定
    let bucket_name = "test-bucket";
    let upload_path = "test-file.txt";
    let local_file_path = "local-test-file.txt";

    // テスト用の一時ファイルを作成
    let mut file = StdFile::create(local_file_path)?;
    writeln!(file, "これはテストファイルです。")?;

    // バケット一覧を取得
    println!("バケット一覧を取得中...");
    let buckets = storage.list_buckets().await?;
    println!("利用可能なバケット:");
    for bucket in &buckets {
        println!("- {} (Public: {})", bucket.name, bucket.public);
    }

    // バケットが存在するか確認し、なければ作成
    let bucket_exists = buckets.iter().any(|b| b.name == bucket_name);
    if !bucket_exists {
        println!("バケット '{}' を作成します...", bucket_name);
        storage.create_bucket(bucket_name, true).await?;
        println!("バケットを作成しました。");
    }

    // ファイルをアップロード
    println!("ファイルをアップロードしています...");
    let upload_options = FileOptions::new().with_content_type("text/plain");
    let upload_result = storage
        .from(bucket_name)
        .with_auth(&dummy_token)?
        .upload(upload_path, local_file_path, Some(upload_options))
        .await?;
    println!("ファイルをアップロードしました: {}", upload_result.name);

    // バケット内のファイル一覧を取得
    println!("\nバケット '{}' のファイル一覧を取得中...", bucket_name);
    let list_options = ListOptions::new().with_limit(10).with_offset(0);
    let files = storage
        .from(bucket_name)
        .with_auth(&dummy_token)?
        .list(Some(""), Some(list_options))
        .await?;
    println!("ファイル一覧:");
    for file in &files {
        println!("- {} (サイズ: {} バイト)", file.name, file.metadata.size);
    }

    // ファイルの公開URLを取得
    let public_url = storage.from(bucket_name).get_public_url(upload_path);
    println!("\n公開URL: {}", public_url);

    // ファイルの署名付きURLを取得
    let signed_url = storage
        .from(bucket_name)
        .with_auth(&dummy_token)?
        .create_signed_url(upload_path, 60) // 60秒間有効なURL
        .await?;
    println!("署名付きURL (有効期限60秒): {}", signed_url);

    // ファイルをダウンロード
    println!("\nファイルをダウンロードしています...");
    let download_data = storage
        .from(bucket_name)
        .with_auth(&dummy_token)?
        .download(upload_path)
        .await?;
    println!(
        "ダウンロードしたファイルサイズ: {} バイト",
        download_data.len()
    );
    // ダウンロードした内容をファイルに保存（オプション）
    // let mut download_file = StdFile::create("downloaded-test-file.txt")?;\n    // download_file.write_all(&download_data)?;\n\n    // ファイルを移動
    let move_destination = "moved/test-file.txt";
    println!(
        "\nファイルを移動しています: {} -> {}",
        upload_path, move_destination
    );
    storage
        .from(bucket_name)
        .with_auth(&dummy_token)?
        .move_(upload_path, move_destination)
        .await?;
    println!("ファイルを移動しました。");

    // 移動後のファイルを確認
    let moved_files = storage
        .from(bucket_name)
        .with_auth(&dummy_token)?
        .list(Some("moved/"), None)
        .await?;
    println!("移動先のディレクトリの内容:");
    for file in moved_files {
        println!("- {}", file.name);
    }

    // ファイルを削除
    println!("\nファイルを削除しています...");
    let deleted_files = storage
        .from(bucket_name)
        .with_auth(&dummy_token)?
        .remove(vec![move_destination])
        .await?;
    println!("削除されたファイル: {}", deleted_files.len());

    // バケットを空にする
    // println!("\nバケット '{}' を空にしています...", bucket_name);
    // storage.empty_bucket(bucket_name).await?;
    // println!("バケットを空にしました。");

    // バケットを削除
    // println!("\nバケット '{}' を削除しています...", bucket_name);
    // storage.delete_bucket(bucket_name).await?;
    // println!("バケットを削除しました。");

    // ローカルテストファイルを削除
    std::fs::remove_file(local_file_path)?;

    Ok(())
}

/// 大容量ファイルのアップロード例を実行
async fn run_large_file_upload(
    storage: &supabase_rust_gftd::storage::StorageClient,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 大容量ファイルのアップロード例 ===\n");

    // テストバケットの名前
    let bucket_name = "test-bucket";

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
        .with_auth(&dummy_token)?
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
        .with_auth(&dummy_token)?
        .remove(vec!["large-file.bin"])
        .await?;

    println!("ファイルを削除しました");

    Ok(())
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // .env ファイルを読み込む
    dotenv().ok();

    // Supabaseの認証情報を環境変数から取得
    let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");

    println!("Using Supabase URL: {}", supabase_url);
    println!("==== Supabase Storage Examples ====");

    // Supabaseクライアントの初期化
    let supabase = Supabase::new(&supabase_url, &supabase_key);

    // !!!!! 仮定: 認証セッションを取得 !!!!!
    // 本来は auth_example.rs のように sign_in する必要があります
    // ここでは、環境変数からダミーのトークンを取得するか、
    // 適切な認証処理を実装する必要があります。
    let dummy_token =
        env::var("SUPABASE_DUMMY_TOKEN").unwrap_or_else(|_| "dummy-jwt-token".to_string());

    let storage = supabase.storage();

    // 各サンプル関数を呼び出す
    match run_basic_storage_operations(&storage).await {
        Ok(_) => println!("基本的なストレージ操作の例が正常に完了しました。"),
        Err(e) => eprintln!("基本的なストレージ操作の例でエラーが発生しました: {}", e),
    }

    match run_large_file_upload(&storage).await {
        Ok(_) => println!("大容量ファイルのアップロード例が正常に完了しました。"),
        Err(e) => eprintln!(
            "大容量ファイルのアップロード例でエラーが発生しました: {}",
            e
        ),
    }

    // Note: These examples still create their own unauthenticated clients internally.
    // To fix their auth errors, they need refactoring similar to the above.
    match image_transform_examples::run_image_transform_examples().await {
        Ok(_) => println!("画像変換機能の例が正常に完了しました。"),
        Err(e) => eprintln!("画像変換機能の例でエラーが発生しました: {}", e),
    }

    match run_s3_compatible_example(&supabase).await {
        Ok(_) => println!("S3互換APIの例が正常に完了しました。"),
        Err(e) => eprintln!("S3互換APIの例でエラーが発生しました: {}", e),
    }

    println!("\n全ての例が実行されました。");

    Ok(())
}
