use dotenv::dotenv;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use supabase_rust_gftd::postgrest::{IsolationLevel, PostgrestClient, SortOrder, TransactionMode};
use supabase_rust_gftd::Supabase;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Task {
    id: Option<i32>,
    title: String,
    description: Option<String>,
    is_complete: bool,
    created_at: Option<String>,
    user_id: String,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv().ok();

    // Get Supabase URL and key from environment variables
    let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");

    println!("Using Supabase URL: {}", supabase_url);

    // Initialize the Supabase client
    let supabase_client = Supabase::new(&supabase_url, &supabase_key);

    println!("Starting PostgREST advanced example");

    // First, sign up a test user for our examples
    let test_email = format!("test-postgrest-{}@example.com", uuid::Uuid::new_v4());
    let test_password = "password123";

    let sign_up_result = supabase_client.auth().sign_up(&test_email, test_password).await?;

    let user_id = sign_up_result.user.id.clone();
    let access_token = sign_up_result.access_token.clone();
    println!("Created test user with ID: {}", user_id);

    // Get the PostgREST client and attach the authorization token
    let postgrest = supabase_client.from("tasks").with_auth(&access_token)?;

    // Example 1: Basic operations
    println!("Example 1: Basic operations");

    // PostgreStを初期化 - 実際のURLとキーを使用
    let base_url = &supabase_url;
    let api_key = &supabase_key;
    let http_client = Client::new();

    // PostgreStクライアントを作成
    let supabase = PostgrestClient::new(base_url, api_key, "tasks", http_client.clone());

    // 新しいユーザーIDを生成
    let user_id = uuid::Uuid::new_v4().to_string();
    println!("Using user_id: {}", user_id);

    // Example 2: INSERT and SELECT - タスクを作成して取得
    println!("\nExample 2: INSERT and SELECT");

    // タスクを作成
    for i in 1..6 {
        let task = Task {
            id: None,
            title: format!("Task {}", i),
            description: Some(format!("Description for task {}", i)),
            is_complete: i % 2 == 0, // 偶数番号のタスクは完了済み
            created_at: None,
            user_id: user_id.clone(),
        };

        // INSERTリクエストを構築して実行
        let client = PostgrestClient::new(base_url, api_key, "tasks", http_client.clone());

        client.insert(json!(task)).await?;
    }

    println!("Created 5 tasks");

    // 未完了のタスクを取得
    let client = PostgrestClient::new(base_url, api_key, "tasks", http_client.clone());

    let incomplete_tasks_json = client
        .select("*")
        .eq("user_id", &user_id)
        .eq("is_complete", "false")
        .execute::<serde_json::Value>()
        .await?;

    // 手動で型変換する
    let incomplete_tasks: Vec<Task> = incomplete_tasks_json
        .iter()
        .map(|task_json| serde_json::from_value(task_json.clone()).unwrap())
        .collect();

    println!("Found {} incomplete tasks:", incomplete_tasks.len());
    for task in &incomplete_tasks {
        println!("  - {}", task.title);
    }

    // Example 3: Complex filters and order
    println!("\nExample 3: Complex filters and order");

    // Query with filters and ordering (OR条件の代わりにin_listを使用)
    let client = PostgrestClient::new(base_url, api_key, "tasks", http_client.clone());

    let filtered_tasks_json = client
        .select("*")
        .in_list("title", &["Task 1", "Task 2"]) // Title is either "Task 1" or "Task 2"
        .order("created_at", SortOrder::Ascending) // Order by created_at ascending
        .limit(10)
        .execute::<serde_json::Value>()
        .await?;

    // 手動で型変換する
    let filtered_tasks: Vec<Task> = filtered_tasks_json
        .iter()
        .map(|task_json| serde_json::from_value(task_json.clone()).unwrap())
        .collect();

    println!("Filtered tasks (Task 1 or Task 2, sorted by created_at):");
    for task in &filtered_tasks {
        println!("  - {} (is_complete: {})", task.title, task.is_complete);
    }

    // Example 4: UPDATE with filters
    println!("\nExample 4: UPDATE with filters");

    // Update all tasks to be complete
    let client = PostgrestClient::new(base_url, api_key, "tasks", http_client.clone());

    let update_result = client
        .eq("user_id", &user_id)
        .eq("is_complete", "false")
        .update(json!({ "is_complete": true }))
        .await?;

    // jsonの配列としてカウント
    println!(
        "Updated {} tasks to be complete",
        update_result.as_array().unwrap_or(&vec![]).len()
    );

    // Example 5: Using range queries
    println!("\nExample 5: Using range queries");

    // Insert tasks with explicit IDs for range example
    for i in 100..105 {
        let task = Task {
            id: Some(i),
            title: format!("Range Task {}", i),
            description: Some(format!("Description for range task {}", i)),
            is_complete: false,
            created_at: None,
            user_id: user_id.clone(),
        };

        let client = PostgrestClient::new(base_url, api_key, "tasks", http_client.clone());
        client.insert(json!(task)).await?;
    }

    // Query for a range of IDs
    let client = PostgrestClient::new(base_url, api_key, "tasks", http_client.clone());

    let range_tasks_json = client
        .select("*")
        .gte("id", "100")
        .lte("id", "102")
        .execute::<serde_json::Value>()
        .await?;

    // 手動で型変換する
    let range_tasks: Vec<Task> = range_tasks_json
        .iter()
        .map(|task_json| serde_json::from_value(task_json.clone()).unwrap())
        .collect();

    println!("Tasks with IDs between 100 and 102:");
    for task in &range_tasks {
        println!("  - ID: {:?}, Title: {}", task.id, task.title);
    }

    // Example 6: Using COUNT
    println!("\nExample 6: Using COUNT");

    // Count the total number of tasks
    // count関数を使用して直接カウント結果を取得
    let client = PostgrestClient::new(base_url, api_key, "tasks", http_client.clone());

    let count_result = client
        .select("count")
        .count(true)
        .execute::<serde_json::Value>()
        .await?;

    let count = count_result[0]
        .as_object()
        .unwrap()
        .get("count")
        .unwrap()
        .as_i64()
        .unwrap();
    println!("Total number of tasks: {}", count);

    // Example 7: DELETE with filters
    println!("\nExample 7: DELETE with filters");

    // Delete tasks that match specific criteria
    let client = PostgrestClient::new(base_url, api_key, "tasks", http_client.clone());

    let delete_result = client
        .eq("user_id", &user_id)
        .gte("id", "100")
        .lte("id", "102")
        .delete()
        .await?;

    // jsonの配列としてカウント
    println!(
        "Deleted {} tasks",
        delete_result.as_array().unwrap_or(&vec![]).len()
    );

    // Example 8: Transaction with savepoints (begin_transaction方式)
    println!("\nExample 8: Transaction with savepoints");

    // Start a transaction with explicit options
    let client = PostgrestClient::new(base_url, api_key, "tasks", http_client.clone());

    let transaction = client
        .begin_transaction(
            Some(IsolationLevel::ReadCommitted),
            Some(TransactionMode::ReadWrite),
            Some(30), // timeout in seconds
        )
        .await?;

    println!("Transaction started with isolation level: ReadCommitted, mode: ReadWrite");

    // Create a task in the transaction
    let transaction_task = Task {
        id: None,
        title: "Transaction Task".to_string(),
        description: Some("Created in a transaction".to_string()),
        is_complete: false,
        created_at: None,
        user_id: user_id.clone(),
    };

    // For each transaction operation, get a new client for that operation
    let tasks_in_transaction = transaction.from("tasks");

    // Insert the task
    let tx_insert_result = tasks_in_transaction.insert(json!(transaction_task)).await?;

    let tx_task_id = tx_insert_result[0]["id"].as_i64().unwrap();
    println!("Created task in transaction with ID: {}", tx_task_id);

    // Create a savepoint after inserting
    transaction.savepoint("after_insert").await?;
    println!("Created savepoint 'after_insert'");

    // Update the task - get a fresh client
    let tasks_in_transaction_update1 = transaction.from("tasks");
    let tx_update_result = tasks_in_transaction_update1
        .eq("id", &tx_task_id.to_string())
        .update(json!({ "description": "Updated in transaction" }))
        .await?;

    println!(
        "Updated task in transaction: {}",
        tx_update_result[0]["description"]
    );

    // Create another savepoint after updating
    transaction.savepoint("after_update").await?;
    println!("Created savepoint 'after_update'");

    // Update the task again - get a fresh client
    let tasks_in_transaction_update2 = transaction.from("tasks");
    let tx_update_result2 = tasks_in_transaction_update2
        .eq("id", &tx_task_id.to_string())
        .update(json!({ "description": "This update will be rolled back" }))
        .await?;

    println!(
        "Updated task again in transaction: {}",
        tx_update_result2[0]["description"]
    );

    // Now roll back to the previous savepoint
    transaction.rollback_to_savepoint("after_update").await?;
    println!("Rolled back to savepoint 'after_update'");

    // Commit the transaction
    transaction.commit().await?;
    println!("Transaction committed");

    // Example 9: Select the task created in transaction
    println!("\nExample 9: Verify task created in transaction");

    let client = PostgrestClient::new(base_url, api_key, "tasks", http_client.clone());

    let tx_tasks_json = client
        .select("*")
        .eq("title", "Transaction Task")
        .execute::<serde_json::Value>()
        .await?;

    let tx_tasks: Vec<Task> = tx_tasks_json
        .iter()
        .map(|task_json| serde_json::from_value(task_json.clone()).unwrap())
        .collect();

    println!("Tasks created in transaction:");
    for task in &tx_tasks {
        println!(
            "  - Title: {}, Description: {}",
            task.title,
            task.description.as_ref().unwrap()
        );
        // Description should be "Updated in transaction" not "This update will be rolled back"
        assert_eq!(task.description.as_ref().unwrap(), "Updated in transaction");
    }

    // Example 10: Transaction with rollback
    println!("\nExample 10: Transaction with rollback");

    // Start another transaction
    let transaction2 = client
        .begin_transaction(
            Some(IsolationLevel::ReadCommitted),
            Some(TransactionMode::ReadWrite),
            None,
        )
        .await?;

    // Create a task that will be rolled back
    let rollback_task = Task {
        id: None,
        title: "Rollback Task".to_string(),
        description: Some("This task should be rolled back".to_string()),
        is_complete: false,
        created_at: None,
        user_id: user_id.clone(),
    };

    // Insert the task
    let tasks_in_transaction2 = transaction2.from("tasks");
    let roll_insert_result = tasks_in_transaction2.insert(json!(rollback_task)).await?;

    let roll_task_id = roll_insert_result[0]["id"].as_i64().unwrap();
    println!(
        "Created task in transaction2 (will be rolled back) with ID: {}",
        roll_task_id
    );

    // Rollback the transaction
    transaction2.rollback().await?;
    println!("Transaction2 rolled back");

    // Example 11: Verify task was not created after rollback
    println!("\nExample 11: Verify task was not created after rollback");

    let rollback_tasks_json = client
        .select("*")
        .eq("title", "Rollback Task")
        .execute::<serde_json::Value>()
        .await?;

    println!(
        "Tasks with title 'Rollback Task' (should be 0): {}",
        rollback_tasks_json.len()
    );
    assert_eq!(
        rollback_tasks_json.len(),
        0,
        "Rollback wasn't successful, found tasks that should have been rolled back"
    );

    // Example 10: final cleanup
    println!("\nExample 10: final cleanup");
    
    let final_postgrest = PostgrestClient::new(base_url, api_key, "tasks", http_client.clone());
    let _ = final_postgrest
        .eq("user_id", &user_id)
        .delete()
        .await?;

    println!("Cleaned up all test data");

    println!("PostgREST example completed");

    Ok(())
}
