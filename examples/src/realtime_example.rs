use supabase_rust_gftd::prelude::*;
use supabase_rust_gftd::{Supabase, DatabaseFilter, FilterOperator};
use supabase_rust_gftd::realtime::{ChannelEvent, DatabaseChanges};
use dotenv::dotenv;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::io;
use serde_json::json;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Task {
    id: Option<i32>,
    title: String,
    description: Option<String>,
    is_complete: bool,
    created_at: Option<String>,
    user_id: String,
}

// The payload for Real-time messages
#[derive(Debug, Deserialize)]
struct RealtimePayload<T> {
    #[serde(rename = "type")]
    event_type: String,
    table: String,
    schema: String,
    record: Option<T>,
    old_record: Option<T>,
}

/// 高度なフィルタリング機能の例
async fn run_advanced_filter_example(supabase: &Supabase, user_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 高度なリアルタイムフィルタリングの例 ===\n");
    
    // リアルタイムクライアントを取得
    let realtime = supabase.realtime();
    
    // メッセージ受信カウンターを作成
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();
    
    // 高度なフィルターを使用してチャンネル作成
    // 完了済みタスクだけを監視する例
    println!("完了済みタスク（is_complete = true）のみを監視するチャンネルを作成しています...");
    
    let channel = realtime
        .channel("filtered-tasks")
        .on(
            DatabaseChanges::new("tasks")
                .event(ChannelEvent::Insert)
                .event(ChannelEvent::Update)
                // is_completeがtrueのレコードだけを対象にする
                .eq("is_complete", true),
            move |payload: HashMap<String, serde_json::Value>| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    if let Ok(payload) = serde_json::from_value::<RealtimePayload<Task>>(json!(payload)) {
                        if let Some(record) = payload.record {
                            println!("フィルター付きチャンネルで受信: 完了済みタスク「{}」", record.title);
                            // タスクは必ず完了済み (is_complete = true) のはず
                            assert!(record.is_complete);
                        }
                    }
                }
            },
        )
        .subscribe()
        .await?;
    
    println!("フィルター付きチャンネルを作成しました");
    
    // 複数のタスクを作成し、一部だけを完了済みにする
    let postgrest = supabase.from("tasks");
    
    // 5つのタスクを作成（最初は全て未完了）
    for i in 1..6 {
        let task = Task {
            id: None,
            title: format!("フィルタリングテスト用タスク {}", i),
            description: Some(format!("フィルター機能テスト用 {}", i)),
            is_complete: false,
            created_at: None,
            user_id: user_id.to_string(),
        };
        
        println!("タスクを作成: {}", task.title);
        let result = postgrest
            .insert(json!(task))
            .execute()
            .await?;
        
        // タスクIDを取得
        let task_id = result[0]["id"].as_i64().unwrap();
        
        // 偶数番目のタスクを完了済みに更新（フィルターに合致）
        if i % 2 == 0 {
            println!("タスク {} を完了済みに更新（フィルターに合致）", i);
            postgrest
                .update(json!({ "is_complete": true }))
                .eq("id", task_id)
                .execute()
                .await?;
        }
        
        // 少し待機してイベントを処理させる
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    
    // 少し待機してすべてのイベントを処理させる
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // 複合条件を使用したフィルタリングを追加で試す
    println!("\n複合条件を使ったフィルタリングテスト:");
    
    // カウンターをリセット
    let counter2 = Arc::new(AtomicU32::new(0));
    let counter2_clone = counter2.clone();
    
    // タイトルに「3」または「5」を含み、かつ未完了のタスクに一致するフィルター
    let channel2 = realtime
        .channel("complex-filter")
        .on(
            DatabaseChanges::new("tasks")
                .event(ChannelEvent::Insert)
                .event(ChannelEvent::Update)
                // タイトルが「3」を含むか「5」を含む
                .filter(DatabaseFilter {
                    column: "title".to_string(),
                    operator: FilterOperator::Like,
                    value: serde_json::Value::String("%3%".to_string()),
                })
                .filter(DatabaseFilter {
                    column: "title".to_string(), 
                    operator: FilterOperator::Like,
                    value: serde_json::Value::String("%5%".to_string()),
                })
                // かつ未完了のタスク
                .eq("is_complete", false),
            move |payload: HashMap<String, serde_json::Value>| {
                let counter = counter2_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    if let Ok(payload) = serde_json::from_value::<RealtimePayload<Task>>(json!(payload)) {
                        if let Some(record) = payload.record {
                            println!("複合フィルターで受信: 「{}」", record.title);
                            // タスクは必ず未完了 (is_complete = false) のはず
                            assert!(!record.is_complete);
                            // タスクのタイトルには「3」または「5」が含まれるはず
                            assert!(record.title.contains('3') || record.title.contains('5'));
                        }
                    }
                }
            },
        )
        .subscribe()
        .await?;
    
    println!("複合フィルター付きチャンネルを作成しました");
    
    // タスク3と5を更新
    for &i in &[3, 5] {
        let task_title = format!("フィルタリングテスト用タスク {}", i);
        println!("タスク {} の説明を更新（複合フィルターに合致）", i);
        
        let update_result = postgrest
            .update(json!({ 
                "description": format!("複合フィルターテスト用に更新 {}", i)
            }))
            .like("title", &task_title)
            .execute()
            .await?;
        
        // 少し待機
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    
    // 少し待機してすべてのイベントを処理させる
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // 結果を確認
    let received_count = counter.load(Ordering::SeqCst);
    println!("\n検証: 完了済みタスクフィルターで受信したイベント数: {}", received_count);
    // 偶数番号のタスク2つが完了済みに更新されたので、2件のイベントを受信しているはず
    assert!(received_count >= 2);
    
    let received_count2 = counter2.load(Ordering::SeqCst);
    println!("検証: 複合フィルターで受信したイベント数: {}", received_count2);
    // タスク3と5の更新で2件のイベントを受信しているはず
    assert_eq!(received_count2, 2);
    
    // チャンネルの購読を解除
    channel.unsubscribe().await?;
    channel2.unsubscribe().await?;
    
    println!("\n高度なリアルタイムフィルタリングの例が完了しました");
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv().ok();
    
    // Get Supabase URL and key from environment variables
    let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");
    
    // Initialize the Supabase client
    let supabase = Supabase::new(&supabase_url, &supabase_key);
    
    println!("Starting Realtime example");
    
    // First, sign up a test user for our example
    let test_email = format!("test-realtime-{}@example.com", uuid::Uuid::new_v4());
    let test_password = "password123";
    
    let sign_up_result = supabase
        .auth()
        .sign_up(&test_email, test_password)
        .await?;
    
    let user_id = sign_up_result.user.id;
    println!("Created test user with ID: {}", user_id);
    
    // Store received messages in a thread-safe vector
    let messages = Arc::new(Mutex::new(Vec::new()));
    let messages_clone = messages.clone();
    
    // Setup realtime client to listen to the 'tasks' table
    let realtime = supabase.realtime();
    
    // Create channel for 'tasks' table changes
    let channel = realtime
        .channel("public:tasks")
        .on_insert(move |payload: HashMap<String, serde_json::Value>| {
            let messages = messages_clone.clone();
            async move {
                if let Ok(payload) = serde_json::from_value::<RealtimePayload<Task>>(json!(payload)) {
                    if let Some(record) = payload.record {
                        let mut messages = messages.lock().await;
                        messages.push(format!("INSERT: {}", record.title));
                        println!("Received INSERT event: {:?}", record);
                    }
                }
            }
        })
        .on_update(|payload: HashMap<String, serde_json::Value>| {
            async move {
                if let Ok(payload) = serde_json::from_value::<RealtimePayload<Task>>(json!(payload)) {
                    if let Some(record) = payload.record {
                        println!("Received UPDATE event: {:?}", record);
                    }
                }
            }
        })
        .on_delete(|payload: HashMap<String, serde_json::Value>| {
            async move {
                if let Ok(payload) = serde_json::from_value::<RealtimePayload<Task>>(json!(payload)) {
                    if let Some(old_record) = payload.old_record {
                        println!("Received DELETE event: {:?}", old_record);
                    }
                }
            }
        });
    
    // Subscribe to the channel
    let subscription = channel.subscribe().await?;
    println!("Subscribed to realtime changes on public:tasks");
    
    // Create a task through PostgREST
    let postgrest = supabase.from("tasks");
    
    // Insert tasks with some delay to observe realtime events
    for i in 1..5 {
        let task = Task {
            id: None,
            title: format!("Realtime Task {}", i),
            description: Some(format!("Description for realtime task {}", i)),
            is_complete: false,
            created_at: None,
            user_id: user_id.clone(),
        };
        
        println!("Inserting task: {}", task.title);
        postgrest
            .insert(json!(task))
            .execute()
            .await?;
        
        // Wait a bit to see the realtime event
        sleep(Duration::from_secs(1)).await;
    }
    
    // Update a task
    println!("\nUpdating tasks...");
    postgrest
        .update(json!({ "is_complete": true }))
        .like("title", "Realtime Task 1")
        .execute()
        .await?;
    
    sleep(Duration::from_secs(1)).await;
    
    // Delete a task
    println!("\nDeleting a task...");
    postgrest
        .delete()
        .like("title", "Realtime Task 2")
        .execute()
        .await?;
    
    // Wait a bit to receive all events
    sleep(Duration::from_secs(2)).await;
    
    // Check received messages
    let message_count = messages.lock().await.len();
    println!("\nReceived {} insert notifications", message_count);
    
    // Unsubscribe from realtime updates
    subscription.unsubscribe().await?;
    println!("Unsubscribed from realtime updates");
    
    // Clean up - delete all tasks for our test user
    postgrest
        .delete()
        .eq("user_id", &user_id)
        .execute()
        .await?;
    
    println!("Realtime example completed");
    
    // 高度なフィルタリング機能のテストを実行
    println!("\n高度なリアルタイムフィルタリング機能をテストしますか？(y/n)");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() == "y" {
        run_advanced_filter_example(&supabase, &user_id).await?;
    }
    
    Ok(())
}