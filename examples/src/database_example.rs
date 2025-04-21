use supabase_rust_gftd::Supabase;
use dotenv::dotenv;
use std::env;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid;

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
    
    println!("Starting database example");
    
    // First, sign up a test user for our examples
    let test_email = format!("test-db-{}@example.com", uuid::Uuid::new_v4());
    let test_password = "password123";
    
    let sign_up_result = supabase
        .auth()
        .sign_up(&test_email, test_password)
        .await?;
    
    let user_id = sign_up_result.user.id;
    println!("Created test user with ID: {}", user_id);
    
    // Get the PostgreSQL client for the "tasks" table
    let postgrest_client = supabase.from("tasks");
    
    // Example 1: Create a new task
    println!("\nExample 1: Creating a new task");
    
    let new_task = Task {
        id: None,
        title: "Supabase Rust 学習".to_string(),
        description: Some("Rustでのデータベース操作を学ぶ".to_string()),
        is_complete: false,
        created_at: None,
        user_id: user_id.clone(),
    };
    
    // APIに合わせて修正: executeメソッドは単独で呼び出す必要がある
    let insert_query = postgrest_client
        .insert(serde_json::json!(new_task));
    
    let insert_result = insert_query.execute().await?;
    
    println!("Task created: {:?}", insert_result);
    
    // Example 2: Select all tasks for the current user
    println!("\nExample 2: Selecting user's tasks");
    
    let select_query = postgrest_client
        .select("*")
        .eq("user_id", &user_id);
    
    let user_tasks: Vec<Value> = select_query.execute().await?;
    
    println!("Found {} tasks for user", user_tasks.len());
    for (i, task) in user_tasks.iter().enumerate() {
        println!("Task {}: {:?}", i + 1, task);
    }
    
    // Example 3: Update a task
    if let Some(task) = user_tasks.first() {
        println!("\nExample 3: Updating a task");
        
        if let Some(task_id) = task.get("id") {
            let update_query = postgrest_client
                .update(serde_json::json!({"is_complete": true}))
                .eq("id", task_id.to_string());
            
            let update_result = update_query.execute().await?;
            println!("Task updated: {:?}", update_result);
        }
    }
    
    // Example 4: Delete all tasks for this test user
    println!("\nExample 4: Cleaning up - deleting user's tasks");
    
    let delete_query = postgrest_client
        .delete()
        .eq("user_id", &user_id);
    
    let delete_result = delete_query.execute().await?;
    println!("Deleted tasks: {:?}", delete_result);
    
    println!("Database example completed");
    
    Ok(())
}