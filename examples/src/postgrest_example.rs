use supabase_rust_gftd::Supabase;
use supabase_rust_gftd::postgrest::{IsolationLevel, SortOrder, TransactionMode, PostgrestClient};
use dotenv::dotenv;
use std::env;
use serde::{Deserialize, Serialize};
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

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv().ok();
    
    // Get Supabase URL and key from environment variables
    let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");
    
    println!("Using Supabase URL: {}", supabase_url);
    
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
    
    let user_id = sign_up_result.user.id.clone();
    let access_token = sign_up_result.access_token.clone();
    println!("Created test user with ID: {}", user_id);
    
    // Get the PostgREST client and attach the authorization token
    let postgrest = supabase
        .from("tasks")
        .with_auth(&access_token)?;
    
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
            .await?;
    }
    
    // Query for incomplete tasks
    let incomplete_tasks_json = postgrest
        .select("*")
        .eq("is_complete", "false")
        .eq("user_id", &user_id)
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
    let filtered_tasks_json = postgrest
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
    let update_result = postgrest
        .eq("user_id", &user_id)
        .eq("is_complete", "false")
        .update(json!({ "is_complete": true }))
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
            .await?;
    }
    
    // Query for a range of IDs
    let range_tasks_json = postgrest
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
    let count_result = postgrest
        .select("count")
        .count(true)
        .execute::<serde_json::Value>()
        .await?;
    
    let count = count_result[0].as_object().unwrap().get("count").unwrap().as_i64().unwrap();
    println!("Total number of tasks: {}", count);
    
    // Example 7: DELETE with filters
    println!("\nExample 7: DELETE with filters");
    
    // Delete tasks that match specific criteria
    let delete_result = postgrest
        .eq("user_id", &user_id)
        .gte("id", "100")
        .lte("id", "102")
        .delete()
        .await?;
    
    println!("Deleted {} tasks", delete_result.len());
    
    // Example 8: Transaction with savepoints (begin_transaction方式)
    println!("\nExample 8: Transaction with savepoints");
    
    // Start a transaction
    let transaction = postgrest.begin_transaction(
        Some(IsolationLevel::ReadCommitted),
        Some(TransactionMode::ReadWrite),
        Some(30) // timeout in seconds
    ).await?;
    
    println!("Transaction started");
    
    // Create a task in the transaction
    let transaction_task = Task {
        id: None,
        title: "Transaction Task".to_string(),
        description: Some("Created in a transaction".to_string()),
        is_complete: false,
        created_at: None,
        user_id: user_id.clone(),
    };
    
    let tasks_in_transaction = transaction.from("tasks");
    
    // Insert the task
    let tx_insert_result = tasks_in_transaction
        .insert(json!(transaction_task))
        .await?;
    
    let tx_task_id = tx_insert_result[0]["id"].as_i64().unwrap();
    println!("Created task in transaction with ID: {}", tx_task_id);
    
    // Create a savepoint
    transaction.savepoint("after_insert").await?;
    println!("Created savepoint 'after_insert'");
    
    // Update the task
    let tx_update_result = tasks_in_transaction
        .eq("id", &tx_task_id.to_string())
        .update(json!({ "description": "Updated in transaction" }))
        .await?;
    
    println!("Updated task in transaction: {}", tx_update_result[0]["description"]);
    
    // Commit the transaction
    transaction.commit().await?;
    println!("Transaction committed");
    
    // Example 9: Select the task created in transaction
    println!("\nExample 9: Verify task created in transaction");
    
    let tx_tasks_json = postgrest
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
        println!("  - Title: {}, Description: {}", task.title, task.description.as_ref().unwrap());
    }
    
    // Example 10: Transaction with rollback
    println!("\nExample 10: Transaction with rollback");
    
    // Start another transaction
    let transaction2 = postgrest.begin_transaction(
        Some(IsolationLevel::ReadCommitted),
        Some(TransactionMode::ReadWrite),
        None
    ).await?;
    
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
    let _ = tasks_in_transaction2
        .insert(json!(rollback_task))
        .await?;
    
    println!("Created task in transaction2 (will be rolled back)");
    
    // Rollback the transaction
    transaction2.rollback().await?;
    println!("Transaction2 rolled back");
    
    // Example 11: Verify task was not created after rollback
    println!("\nExample 11: Verify task was not created after rollback");
    
    let rollback_tasks_json = postgrest
        .select("*")
        .eq("title", "Rollback Task")
        .execute::<serde_json::Value>()
        .await?;
    
    println!("Tasks with title 'Rollback Task' (should be 0): {}", rollback_tasks_json.len());
    
    // Example 12: Cleanup - delete all tasks for our test user
    println!("\nExample 12: Cleanup - delete all tasks for our test user");
    
    let _ = supabase
        .from("tasks")
        .with_auth(&access_token)?
        .eq("user_id", &user_id)
        .delete()
        .await?;
    
    println!("Deleted all tasks for test user");
    
    println!("PostgREST example completed");
    
    Ok(())
}