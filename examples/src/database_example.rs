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
    
    // Example 1: Create a new task
    println!("\nExample 1: Creating a new task");
    
    // JSONオブジェクトを使用してidフィールドを除外
    let insert_result = supabase
        .from("tasks")
        .insert(serde_json::json!({
            "title": "Supabase Rust 学習",
            "description": "Rustでのデータベース操作を学ぶ",
            "is_complete": false,
            "user_id": user_id
        }))
        .await?;
    
    println!("Task created: {:?}", insert_result);
    
    // Example 2: Select all tasks for the current user
    println!("\nExample 2: Selecting user's tasks");
    
    let user_tasks: Vec<Value> = supabase
        .from("tasks")
        .select("*")
        .eq("user_id", &user_id)
        .execute()
        .await?;
    
    println!("Found {} tasks for user", user_tasks.len());
    for (i, task) in user_tasks.iter().enumerate() {
        println!("Task {}: {:?}", i + 1, task);
    }
    
    // Example 3: Update a task
    if let Some(task) = user_tasks.first() {
        println!("\nExample 3: Updating a task");
        
        if let Some(task_id) = task.get("id") {
            let task_id_str = task_id.to_string();
            let update_result = supabase
                .from("tasks")
                .eq("id", &task_id_str)
                .update(serde_json::json!({"is_complete": true}))
                .await?;
            
            println!("Task updated: {:?}", update_result);
        }
    }
    
    // Example 4: Delete all tasks for this test user
    println!("\nExample 4: Cleaning up - deleting user's tasks");
    
    let delete_result = supabase
        .from("tasks")
        .eq("user_id", &user_id)
        .delete()
        .await?;
    
    println!("Deleted tasks: {:?}", delete_result);
    
    println!("Database example completed");
    
    Ok(())
}