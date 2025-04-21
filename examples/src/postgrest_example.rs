use supabase_rust::prelude::*;
use supabase_rust::postgrest::{IsolationLevel, TransactionMode};
use dotenv::dotenv;
use std::env;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Task {
    id: Option<i32>,
    title: String,
    description: Option<String>,
    is_complete: bool,
    created_at: Option<String>,
    user_id: String,
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
    
    println!("Starting PostgREST advanced example");
    
    // First, sign up a test user for our examples
    let test_email = format!("test-postgrest-{}@example.com", uuid::Uuid::new_v4());
    let test_password = "password123";
    
    let sign_up_result = supabase
        .auth()
        .sign_up(&test_email, test_password)
        .await?;
    
    let user_id = sign_up_result.user.id;
    println!("Created test user with ID: {}", user_id);
    
    // Get the PostgREST client
    let postgrest = supabase.from("tasks");
    
    // Example 1: Basic INSERT with RLS
    println!("\nExample 1: Basic INSERT with RLS");
    
    // Create a new task
    let task = Task {
        id: None,
        title: "Learn Supabase Rust".to_string(),
        description: Some("Master the Supabase Rust client".to_string()),
        is_complete: false,
        created_at: None,
        user_id: user_id.clone(),
    };
    
    // Insert task
    let insert_result = postgrest
        .insert(json!(task))
        .execute()
        .await?;
    
    let inserted_task: Task = serde_json::from_value(insert_result[0].clone())?;
    println!("Inserted task: {:?}", inserted_task);
    
    // Example 2: SELECT with filters
    println!("\nExample 2: SELECT with filters");
    
    // Insert a few more tasks for our examples
    for i in 1..4 {
        let task = Task {
            id: None,
            title: format!("Task {}", i),
            description: Some(format!("Description for task {}", i)),
            is_complete: i % 2 == 0, // Every second task is complete
            created_at: None,
            user_id: user_id.clone(),
        };
        
        postgrest
            .insert(json!(task))
            .execute()
            .await?;
    }
    
    // Query for incomplete tasks
    let incomplete_tasks: Vec<Task> = postgrest
        .select("*")
        .eq("is_complete", "false")
        .eq("user_id", &user_id)
        .execute_typed()
        .await?;
    
    println!("Found {} incomplete tasks:", incomplete_tasks.len());
    for task in &incomplete_tasks {
        println!("  - {}", task.title);
    }
    
    // Example 3: Complex filters and order
    println!("\nExample 3: Complex filters and order");
    
    // Query with complex filters and ordering
    let filtered_tasks: Vec<Task> = postgrest
        .select("*")
        .or("title.eq.Task 1,title.eq.Task 2") // Title is either "Task 1" or "Task 2"
        .order("created_at", Some(true))      // Order by created_at ascending
        .limit(10)
        .execute_typed()
        .await?;
    
    println!("Filtered tasks (Task 1 or Task 2, sorted by created_at):");
    for task in &filtered_tasks {
        println!("  - {} (is_complete: {})", task.title, task.is_complete);
    }
    
    // Example 4: UPDATE with filters
    println!("\nExample 4: UPDATE with filters");
    
    // Update all tasks to be complete
    let update_result = postgrest
        .update(json!({ "is_complete": true }))
        .eq("user_id", &user_id)
        .eq("is_complete", "false")
        .execute()
        .await?;
    
    println!("Updated {} tasks to be complete", update_result.len());
    
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
        
        postgrest
            .insert(json!(task))
            .execute()
            .await?;
    }
    
    // Query for a range of IDs
    let range_tasks: Vec<Task> = postgrest
        .select("*")
        .gte("id", "100")
        .lte("id", "102")
        .execute_typed()
        .await?;
    
    println!("Tasks with IDs between 100 and 102:");
    for task in &range_tasks {
        println!("  - ID: {:?}, Title: {}", task.id, task.title);
    }
    
    // Example 6: Using COUNT
    println!("\nExample 6: Using COUNT");
    
    // Count the total number of tasks
    let count_result = postgrest
        .select("count", Some(true))
        .execute()
        .await?;
    
    let count = count_result[0].as_object().unwrap().get("count").unwrap().as_i64().unwrap();
    println!("Total number of tasks: {}", count);
    
    // Example 7: DELETE with filters
    println!("\nExample 7: DELETE with filters");
    
    // Delete all tasks for our test user
    let delete_result = postgrest
        .delete()
        .eq("user_id", &user_id)
        .execute()
        .await?;
    
    println!("Deleted {} tasks", delete_result.len());
    
    // Example 8: Using custom schema
    println!("\nExample 8: Using custom schema");
    
    // Access a table in a custom schema
    let custom_schema_result = supabase
        .from("profile")
        .select("*")
        .schema("private") // Use a different schema than public
        .limit(5)
        .execute()
        .await;
    
    match custom_schema_result {
        Ok(data) => {
            println!("Data from custom schema (private.profile): {} records", data.len());
        },
        Err(e) => {
            println!("Error accessing custom schema: {:?}", e);
            println!("Note: This is expected if the private.profile table doesn't exist.");
        }
    }
    
    println!("PostgREST advanced example completed");
    
    // Example 9: Using Transactions
    println!("\nExample 9: Using Transactions");
    
    // トランザクションを開始
    let transaction = supabase
        .from("tasks")
        .begin_transaction(
            Some(IsolationLevel::ReadCommitted),
            Some(TransactionMode::ReadWrite),
            Some(30) // 30秒のタイムアウト
        )
        .await?;
    
    println!("Transaction started");
    
    // トランザクション内で複数の操作を実行
    
    // 1. 新しいタスクを作成
    let task1 = Task {
        id: None,
        title: "Transaction Task 1".to_string(),
        description: Some("Created in transaction".to_string()),
        is_complete: false,
        created_at: None,
        user_id: user_id.clone(),
    };
    
    let insert_result1 = transaction
        .from("tasks")
        .insert(json!(task1))
        .execute()
        .await?;
    
    let task1_id = insert_result1[0]["id"].as_i64().unwrap();
    println!("Created Task 1 with ID: {}", task1_id);
    
    // 2. 別のタスクを作成
    let task2 = Task {
        id: None,
        title: "Transaction Task 2".to_string(),
        description: Some("Also created in transaction".to_string()),
        is_complete: false,
        created_at: None,
        user_id: user_id.clone(),
    };
    
    let insert_result2 = transaction
        .from("tasks")
        .insert(json!(task2))
        .execute()
        .await?;
    
    let task2_id = insert_result2[0]["id"].as_i64().unwrap();
    println!("Created Task 2 with ID: {}", task2_id);
    
    // 3. セーブポイントを作成
    transaction.savepoint("after_inserts").await?;
    println!("Created savepoint 'after_inserts'");
    
    // 4. 最初のタスクを更新
    let update_result = transaction
        .from("tasks")
        .update(json!({ "is_complete": true }))
        .eq("id", &task1_id.to_string())
        .execute()
        .await?;
    
    println!("Updated Task 1 to be complete");
    
    // 5. セーブポイントに戻る（更新を取り消す）
    transaction.rollback_to_savepoint("after_inserts").await?;
    println!("Rolled back to savepoint 'after_inserts' (update was cancelled)");
    
    // 6. トランザクションをコミット
    transaction.commit().await?;
    println!("Transaction committed");
    
    // トランザクション後にタスクを確認
    let final_tasks: Vec<Task> = supabase
        .from("tasks")
        .select("*")
        .eq("title", "Transaction Task 1")
        .execute_typed()
        .await?;
    
    if !final_tasks.is_empty() {
        println!(
            "Verified Task 1 after transaction: is_complete = {}", 
            final_tasks[0].is_complete
        );
        // is_complete は false のはず (セーブポイントにロールバックしたため)
        assert!(!final_tasks[0].is_complete);
    }
    
    // Example 10: トランザクションのロールバック
    println!("\nExample 10: Transaction Rollback");
    
    // 新しいトランザクションを開始
    let transaction2 = supabase
        .from("tasks")
        .begin_transaction(None, None, None)
        .await?;
    
    println!("Second transaction started");
    
    // トランザクション内でタスクを作成
    let rollback_task = Task {
        id: None,
        title: "Task to be rolled back".to_string(),
        description: Some("This task should not be saved".to_string()),
        is_complete: false,
        created_at: None,
        user_id: user_id.clone(),
    };
    
    let _ = transaction2
        .from("tasks")
        .insert(json!(rollback_task))
        .execute()
        .await?;
    
    println!("Created a task in second transaction");
    
    // トランザクションをロールバック
    transaction2.rollback().await?;
    println!("Second transaction rolled back");
    
    // ロールバックされたタスクが存在しないことを確認
    let rollback_check: Vec<Task> = supabase
        .from("tasks")
        .select("*")
        .eq("title", "Task to be rolled back")
        .execute_typed()
        .await?;
    
    println!(
        "Tasks with title 'Task to be rolled back': {}", 
        rollback_check.len()
    );
    assert!(rollback_check.is_empty());
    
    // 最後に作成したすべてのタスクを削除
    let _ = supabase
        .from("tasks")
        .delete()
        .eq("user_id", &user_id)
        .execute()
        .await?;
    
    println!("Cleaned up all tasks");
    println!("Transaction examples completed");
    
    Ok(())
}