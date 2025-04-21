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
    
    println!("Using Supabase URL: {}", supabase_url);
    
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
    
    let user_id = sign_up_result.user.id.clone();
    let access_token = sign_up_result.access_token.clone();
    println!("Created test user with ID: {}", user_id);
    println!("Access token obtained for authentication");
    
    // Example 1: Create a new task
    println!("\nExample 1: Creating a new task");
    
    // JSONオブジェクトを使用してidフィールドを除外
    match supabase
        .from("tasks")
        .with_auth(&access_token)?
        .insert(serde_json::json!({
            "title": "Supabase Rust 学習",
            "description": "Rustでのデータベース操作を学ぶ",
            "is_complete": false,
            "user_id": user_id
        }))
        .await {
            Ok(result) => {
                println!("Task created: {:?}", result);
            },
            Err(e) => {
                println!("Error creating task: {:?}", e);
                println!("This could be due to missing 'tasks' table or insufficient permissions.");
                println!("Please check your Supabase project configuration.");
                return Ok(());
            }
        }
    
    // Example 2: Select all tasks for the current user
    println!("\nExample 2: Selecting user's tasks");
    
    match supabase
        .from("tasks")
        .with_auth(&access_token)?
        .select("*")
        .eq("user_id", &user_id)
        .execute::<Value>()
        .await {
            Ok(user_tasks) => {
                println!("Found {} tasks for user", user_tasks.len());
                for (i, task) in user_tasks.iter().enumerate() {
                    println!("Task {}: {:?}", i + 1, task);
                }
                
                // Example 3: Update a task
                if let Some(task) = user_tasks.first() {
                    println!("\nExample 3: Updating a task");
                    
                    if let Some(task_id) = task.get("id") {
                        let task_id_str = task_id.to_string();
                        match supabase
                            .from("tasks")
                            .with_auth(&access_token)?
                            .eq("id", &task_id_str)
                            .update(serde_json::json!({"is_complete": true}))
                            .await {
                                Ok(update_result) => {
                                    println!("Task updated: {:?}", update_result);
                                },
                                Err(e) => {
                                    println!("Error updating task: {:?}", e);
                                }
                            }
                    }
                }
            },
            Err(e) => {
                println!("Error selecting tasks: {:?}", e);
            }
        }
    
    // Example 4: Delete all tasks for this test user
    println!("\nExample 4: Cleaning up - deleting user's tasks");
    
    match supabase
        .from("tasks")
        .with_auth(&access_token)?
        .eq("user_id", &user_id)
        .delete()
        .await {
            Ok(delete_result) => {
                println!("Deleted tasks: {:?}", delete_result);
            },
            Err(e) => {
                println!("Error deleting tasks: {:?}", e);
            }
        }
    
    println!("Database example completed");
    
    Ok(())
}