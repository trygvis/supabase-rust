use supabase_rust_gftd::prelude::*;
use supabase_rust_gftd::Supabase;
use supabase_rust_gftd::postgrest::{IsolationLevel, TransactionMode};
use dotenv::dotenv;
use std::env;
use serde::{Deserialize, Serialize};
use serde_json::json;

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
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
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
        .execute()
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
    
    // Query with complex filters and ordering
    let filtered_tasks_json = postgrest
        .select("*")
        .or("title.eq.Task 1,title.eq.Task 2") // Title is either "Task 1" or "Task 2"
        .order("created_at", Some(true))      // Order by created_at ascending
        .limit(10)
        .execute()
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
            .await?;
    }
    
    // Query for a range of IDs
    let range_tasks_json = postgrest
        .select("*")
        .gte("id", "100")
        .lte("id", "102")
        .execute()
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
    let count_result = postgrest
        .select("count")
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
        Ok(profiles) => {
            println!("Found {} profiles in private schema", profiles.len());
        },
        Err(e) => {
            println!("Note: Custom schema example failed: {}", e);
            println!("This is expected if the 'private.profile' table doesn't exist in your database.");
        }
    }
    
    // Example 9: Transactions
    println!("\nExample 9: Using transactions");
    
    // Start a transaction
    let transaction = postgrest.transaction(TransactionMode::ReadWrite, Some(IsolationLevel::Serializable));
    
    // Create two tasks in a transaction
    let task1 = Task {
        id: None,
        title: "Transaction Task 1".to_string(),
        description: Some("Created in a transaction".to_string()),
        is_complete: false,
        created_at: None,
        user_id: user_id.clone(),
    };
    
    let task2 = Task {
        id: None,
        title: "Transaction Task 2".to_string(),
        description: Some("Created in the same transaction".to_string()),
        is_complete: false,
        created_at: None,
        user_id: user_id.clone(),
    };
    
    // Insert the first task
    let insert_result1 = transaction
        .from("tasks")
        .insert(json!(task1))
        .execute()
        .await?;
    
    // Get the ID of the first task
    let task1_id = insert_result1[0]["id"].as_i64().unwrap();
    println!("Inserted task 1 with ID: {}", task1_id);
    
    // Insert the second task
    let insert_result2 = transaction
        .from("tasks")
        .insert(json!(task2))
        .execute()
        .await?;
    
    // Get the ID of the second task
    let task2_id = insert_result2[0]["id"].as_i64().unwrap();
    println!("Inserted task 2 with ID: {}", task2_id);
    
    // Update the first task in the transaction
    let update_result = transaction
        .from("tasks")
        .update(json!({ "is_complete": true }))
        .eq("id", &task1_id.to_string())
        .execute()
        .await?;
    
    println!("Updated task 1 in transaction");
    
    // Commit the transaction
    transaction.commit().await?;
    println!("Transaction committed");
    
    // Check that both tasks are in the database
    let final_tasks_json = supabase
        .from("tasks")
        .select("*")
        .eq("title", "Transaction Task 1")
        .execute()
        .await?;
    
    // 手動で型変換する
    let final_tasks: Vec<Task> = final_tasks_json
        .iter()
        .map(|task_json| serde_json::from_value(task_json.clone()).unwrap())
        .collect();
    
    println!("Task 1 after transaction: is_complete = {}", final_tasks[0].is_complete);
    
    // Example 10: Rollback a transaction
    println!("\nExample 10: Transaction rollback");
    
    // Start another transaction
    let transaction2 = postgrest.transaction(TransactionMode::ReadWrite, None);
    
    // Create a task that will be rolled back
    let rollback_task = Task {
        id: None,
        title: "Task to be rolled back".to_string(),
        description: Some("This task should not be committed".to_string()),
        is_complete: false,
        created_at: None,
        user_id: user_id.clone(),
    };
    
    // Insert the task
    let _ = transaction2
        .from("tasks")
        .insert(json!(rollback_task))
        .execute()
        .await?;
    
    println!("Task inserted in transaction (not yet committed)");
    
    // Rollback the transaction
    transaction2.rollback().await?;
    println!("Transaction rolled back");
    
    // Verify that the task was not committed
    let rollback_check_json = supabase
        .from("tasks")
        .select("*")
        .eq("title", "Task to be rolled back")
        .execute()
        .await?;
    
    // 手動で型変換する
    let rollback_check: Vec<Task> = rollback_check_json
        .iter()
        .map(|task_json| serde_json::from_value(task_json.clone()).unwrap())
        .collect();
    
    println!("Tasks found after rollback: {} (should be 0)", rollback_check.len());
    assert_eq!(rollback_check.len(), 0, "Transaction rollback failed");
    
    // Clean up - delete all tasks for our test user
    println!("\nCleaning up - deleting all test tasks");
    
    let _ = supabase
        .from("tasks")
        .delete()
        .eq("user_id", &user_id)
        .execute()
        .await?;
    
    println!("All test tasks deleted");
    println!("PostgREST example completed");
    
    Ok(())
}