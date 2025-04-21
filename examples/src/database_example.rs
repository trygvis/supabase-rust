use supabase_rust_gftd::Supabase;
use dotenv::dotenv;
use std::env;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    
    // Example 1: Select all tasks
    println!("\nExample 1: Selecting tasks");
    
    let select_result: Vec<Value> = postgrest_client
        .select("*")
        .execute()
        .await?;
    
    println!("Found {} tasks", select_result.len());
    println!("Select result: {:?}", select_result);
    
    println!("Database example completed");
    
    Ok(())
}