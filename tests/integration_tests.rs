use dotenv::dotenv;
use serde_json::json;
use std::time::Duration;
use supabase_rust_gftd::storage::{FileOptions, ImageTransformOptions};
use supabase_rust_gftd::Supabase;
use tokio::fs;
use tokio::time::sleep;
use uuid::Uuid;

/// ストレージ、リアルタイム、PostgreSTの統合テスト
#[tokio::test]
async fn test_storage_realtime_postgrest_integration() {
    // 環境変数の読み込み
    dotenv().ok();

    // Supabaseクライアントの初期化
    let supabase_url = std::env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_key = std::env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");

    let supabase = Supabase::new(&supabase_url, &supabase_key);

    println!("======= 統合テスト開始 =======");

    // テスト用の一意のIDを生成
    let test_id = Uuid::new_v4().to_string();
    let test_email = format!("test-{}@example.com", test_id);

    // 1. 認証テスト: テストユーザーを作成
    println!("\n== 認証テスト ==");
    let auth_result = supabase
        .auth()
        .sign_up(&test_email, "test_password123")
        .await;

    match auth_result {
        Ok(auth_response) => {
            println!("ユーザー作成成功: {}", auth_response.user.id);
            let user_id = auth_response.user.id;
            let auth_token = auth_response.access_token;

            // 2. PostgreSTテスト: タスクテーブルの操作
            println!("\n== PostgreST テスト ==");

            {
                // タスクを作成
                let postgrest = supabase.from("tasks").with_auth(&auth_token).unwrap();
                let task_result = postgrest
                    .with_header("Prefer", "return=representation").unwrap()
                    .insert(json!({
                        "title": "統合テスト用タスク",
                        "description": "ストレージ、リアルタイム、PostgreSTの統合テスト",
                        "is_complete": false,
                        "user_id": user_id
                    }))
                    .await;

                match task_result {
                    Ok(task) => {
                        if let Some(inserted_task) = task.as_array().and_then(|arr| arr.first()) {
                            println!("タスク作成成功: {}", inserted_task["id"]);
                            let task_id = inserted_task["id"].as_i64().unwrap();

                            // 3. リアルタイムテスト: タスクの更新を監視
                            println!("\n== リアルタイム テスト ==");

                            // リアルタイムクライアントを取得
                            let realtime = supabase.realtime();

                            // タスクテーブルの変更を購読
                            let channel_result = realtime
                                .channel("tasks-channel")
                                .on(
                                    supabase_rust_gftd::realtime::DatabaseChanges::new("tasks")
                                        .event(supabase_rust_gftd::realtime::ChannelEvent::Update)
                                        .eq("id", task_id),
                                    move |payload| {
                                        println!("リアルタイム更新を受信: {:?}", payload);
                                    },
                                )
                                .subscribe()
                                .await;

                            match channel_result {
                                Ok(_) => {
                                    println!("リアルタイムチャンネル購読成功");

                                    // タスクを更新して変更を発生させる
                                    sleep(Duration::from_secs(1)).await; // サブスクリプションが完了するまで待機

                                    // 新しいPostgRESTクライアントを作成して使用
                                    let update_postgrest =
                                        supabase.from("tasks").with_auth(&auth_token).unwrap();
                                    let update_result = update_postgrest
                                        .eq("id", &task_id.to_string())
                                        .update(json!({
                                            "is_complete": true
                                        }))
                                        .await;

                                    match update_result {
                                        Ok(_) => {
                                            println!("タスク更新成功 - リアルタイム更新が送信されます")
                                        }
                                        Err(e) => println!("タスク更新エラー: {:?}", e),
                                    }

                                    // リアルタイム更新が処理される時間を確保
                                    sleep(Duration::from_secs(2)).await;
                                }
                                Err(e) => println!("リアルタイムチャンネル購読エラー: {:?}", e),
                            }

                            // 4. ストレージテスト: テスト用の画像ファイルをアップロード
                            println!("\n== ストレージ テスト ==");

                            // ストレージクライアントを取得
                            let storage = supabase.storage();

                            // テスト用バケットの存在確認または作成
                            let bucket_name = "integration-test";
                            let buckets = storage.list_buckets().await;

                            let bucket_exists = match &buckets {
                                Ok(buckets) => buckets.iter().any(|b| b.name == bucket_name),
                                Err(_) => false,
                            };

                            if !bucket_exists {
                                let create_result = storage.create_bucket(bucket_name, true).await;
                                match create_result {
                                    Ok(_) => println!("テスト用バケット作成成功"),
                                    Err(e) => println!("バケット作成エラー: {:?}", e),
                                }
                            }

                            // テスト用画像の作成 (1x1ピクセルのPNG)
                            let test_image_data: &[u8] = &[
                                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
                                0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
                                0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
                                0x0A, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0x60, 0x00, 0x00, 0x00,
                                0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, 0x00, 0x00, 0x00, 0x00, 0x49,
                                0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
                            ];

                            // テスト用の一時ファイルを作成
                            let temp_dir = tempfile::tempdir().unwrap();
                            let test_file_path = temp_dir.path().join("test_image.png");
                            fs::write(&test_file_path, test_image_data).await.unwrap();

                            // 画像をアップロード
                            let upload_options = FileOptions::new().with_content_type("image/png");
                            let upload_result = storage
                                .from(bucket_name)
                                .upload("test_image.png", &test_file_path, Some(upload_options))
                                .await;

                            match upload_result {
                                Ok(file) => {
                                    println!("ファイルアップロード成功: {}", file.name);

                                    // 画像変換のテスト
                                    let transform_options = ImageTransformOptions::new()
                                        .with_width(100)
                                        .with_height(100);

                                    let transform_result = storage
                                        .from(bucket_name)
                                        .transform_image("test_image.png", transform_options.clone())
                                        .await;

                                    match transform_result {
                                        Ok(bytes) => println!("画像変換成功: {} バイト", bytes.len()),
                                        Err(e) => println!("画像変換エラー: {:?}", e),
                                    }

                                    // 変換画像の公開URLを取得
                                    let public_url = storage
                                        .from(bucket_name)
                                        .get_public_transform_url("test_image.png", transform_options);

                                    println!("変換画像の公開URL: {}", public_url);

                                    // ファイルを削除
                                    let remove_result = storage
                                        .from(bucket_name)
                                        .remove(vec!["test_image.png"])
                                        .await;

                                    match remove_result {
                                        Ok(_) => println!("ファイル削除成功"),
                                        Err(e) => println!("ファイル削除エラー: {:?}", e),
                                    }
                                }
                                Err(e) => println!("ファイルアップロードエラー: {:?}", e),
                            }

                            // クリーンアップ: タスクを削除
                            // 新しいPostgRESTクライアントを作成して使用
                            let delete_postgrest =
                                supabase.from("tasks").with_auth(&auth_token).unwrap();
                            let delete_result = delete_postgrest
                                .eq("id", &task_id.to_string())
                                .delete()
                                .await;

                            match delete_result {
                                Ok(_) => println!("タスク削除成功"),
                                Err(e) => println!("タスク削除エラー: {:?}", e),
                            }
                        } else {
                            println!("タスク作成エラー: 挿入レスポンスが予期しない形式でした: {:?}", task);
                        }
                    }
                    Err(e) => println!("タスク作成エラー: {:?}", e),
                }
            }
        }
        Err(e) => println!("ユーザー作成エラー: {:?}", e),
    }

    println!("\n======= 統合テスト完了 =======");
}
