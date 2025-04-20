use supabase_rust::prelude::*;
use dotenv::dotenv;
use std::env;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Define a struct for our data model
#[derive(Debug, Serialize, Deserialize)]
struct Task {
    id: Option<String>,
    title: String,
    description: Option<String>,
    is_complete: bool,
    created_at: Option<String>,
    user_id: Option<String>,
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
    
    println!("Starting Database example");
    
    // Auth - sign up a test user to use their ID for tasks
    let test_email = format!("test-{}@example.com", Uuid::new_v4());
    let test_password = "password123";
    
    let sign_up_result = supabase
        .auth()
        .sign_up(&test_email, test_password)
        .await?;
    
    let user_id = sign_up_result.user.unwrap().id;
    println!("Created test user with ID: {}", user_id);
    
    // Sign in the user to get authentication
    let sign_in_result = supabase
        .auth()
        .sign_in_with_email(&test_email, test_password)
        .await?;
    
    println!("Signed in as user: {}", sign_in_result.user.unwrap().email);
    
    // Initialize a database client
    let db = supabase.database();
    
    // Create a new task
    let new_task = Task {
        id: None,
        title: "Learn Supabase with Rust".to_string(),
        description: Some("Complete the database example".to_string()),
        is_complete: false,
        created_at: None,
        user_id: Some(user_id.clone()),
    };
    
    println!("Creating a new task");
    let insert_result = db
        .from("tasks")
        .insert(json!(new_task))
        .execute()
        .await?;
    
    let task_id = insert_result
        .json::<Vec<Task>>()
        .unwrap()
        .first()
        .unwrap()
        .id
        .as_ref()
        .unwrap()
        .to_string();
    
    println!("Created task with ID: {}", task_id);
    
    // Query the inserted task
    println!("Fetching the created task");
    let fetch_result = db
        .from("tasks")
        .select("*")
        .eq("id", &task_id)
        .execute()
        .await?;
    
    let task = fetch_result.json::<Vec<Task>>().unwrap().first().unwrap().clone();
    println!("Fetched task: {:?}", task);
    
    // Update the task
    println!("Updating the task");
    let update_result = db
        .from("tasks")
        .update(json!({
            "description": "Completed the database example",
            "is_complete": true
        }))
        .eq("id", &task_id)
        .execute()
        .await?;
    
    let updated_task = update_result.json::<Vec<Task>>().unwrap().first().unwrap().clone();
    println!("Updated task: {:?}", updated_task);
    
    // Query with filters
    println!("Querying tasks with filters");
    let filtered_result = db
        .from("tasks")
        .select("*")
        .eq("user_id", &user_id)
        .eq("is_complete", true)
        .execute()
        .await?;
    
    let tasks = filtered_result.json::<Vec<Task>>().unwrap();
    println!("Found {} completed tasks for user", tasks.len());
    
    // Delete the task
    println!("Deleting the task");
    let delete_result = db
        .from("tasks")
        .delete()
        .eq("id", &task_id)
        .execute()
        .await?;
    
    println!("Deleted task: {:?}", delete_result.status());
    
    // Clean up - remove the test user
    println!("Cleaning up - removing test user");
    // Note: This would typically require admin privileges or a server-side function
    
    println!("Database example completed");
    
    Ok(())
}