use supabase_rust::prelude::*;
use dotenv::dotenv;
use std::env;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct Task {
    id: Option<Uuid>,
    title: String,
    description: Option<String>,
    status: String,
    priority: i32,
    due_date: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    user_id: String,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            id: None,
            title: String::new(),
            description: None,
            status: "pending".to_string(),
            priority: 1,
            due_date: None,
            created_at: None,
            updated_at: None,
            user_id: String::new(),
        }
    }
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
    
    // First, sign up a test user for our example to test RLS policies
    let test_email = format!("test-db-{}@example.com", Uuid::new_v4());
    let test_password = "password123";
    
    let sign_up_result = supabase
        .auth()
        .sign_up(&test_email, test_password)
        .await?;
    
    let user_id = sign_up_result.user.id;
    println!("Created test user with ID: {}", user_id);
    
    // Example 1: Creating a task record using insert
    println!("\nExample 1: Creating a task record");
    
    let task = Task {
        title: "Complete Supabase Rust example".to_string(),
        description: Some("Write a comprehensive database example for the Supabase Rust library".to_string()),
        status: "in_progress".to_string(),
        priority: 2,
        due_date: Some("2023-12-31".to_string()),
        user_id: user_id.clone(),
        ..Default::default()
    };
    
    // Insert the task
    let created_task: Task = supabase
        .from("tasks")
        .insert(&task)
        .execute_one()
        .await?;
    
    println!("Created task with ID: {:?}", created_task.id);
    
    // Example 2: Creating multiple tasks with a batch insert
    println!("\nExample 2: Creating multiple tasks in a batch");
    
    let task1 = Task {
        title: "Research Supabase features".to_string(),
        description: Some("Look into all the features available in Supabase".to_string()),
        status: "pending".to_string(),
        priority: 1,
        user_id: user_id.clone(),
        ..Default::default()
    };
    
    let task2 = Task {
        title: "Test RLS policies".to_string(),
        description: Some("Ensure that row level security is working correctly".to_string()),
        status: "pending".to_string(),
        priority: 3,
        user_id: user_id.clone(),
        ..Default::default()
    };
    
    // Batch insert
    let created_tasks: Vec<Task> = supabase
        .from("tasks")
        .insert(&[task1, task2])
        .execute()
        .await?;
    
    println!("Created {} tasks in batch", created_tasks.len());
    for task in &created_tasks {
        println!("  - Task: {} (ID: {:?})", task.title, task.id);
    }
    
    // Example 3: Retrieving tasks with select
    println!("\nExample 3: Retrieving tasks with select");
    
    let tasks: Vec<Task> = supabase
        .from("tasks")
        .select("*")
        .eq("user_id", &user_id)
        .execute()
        .await?;
    
    println!("Retrieved {} tasks for user", tasks.len());
    for task in &tasks {
        println!(
            "  - Task: {} (Status: {}, Priority: {})",
            task.title, task.status, task.priority
        );
    }
    
    // Example 4: Retrieving a single task by ID
    println!("\nExample 4: Retrieving a single task by ID");
    
    if let Some(first_task_id) = tasks.first().and_then(|t| t.id) {
        let task: Task = supabase
            .from("tasks")
            .select("*")
            .eq("id", &first_task_id.to_string())
            .execute_one()
            .await?;
        
        println!(
            "Retrieved task: {} (Status: {}, Priority: {})",
            task.title, task.status, task.priority
        );
    }
    
    // Example 5: Updating a task
    println!("\nExample 5: Updating a task");
    
    if let Some(task_to_update) = tasks.first() {
        let task_id = task_to_update.id.expect("Task should have an ID");
        
        let updated_task: Task = supabase
            .from("tasks")
            .update(json!({
                "status": "completed",
                "updated_at": "now()"
            }))
            .eq("id", &task_id.to_string())
            .execute_one()
            .await?;
        
        println!(
            "Updated task '{}' status to '{}'",
            updated_task.title, updated_task.status
        );
    }
    
    // Example 6: Filtering and ordering tasks
    println!("\nExample 6: Filtering and ordering tasks");
    
    let filtered_tasks: Vec<Task> = supabase
        .from("tasks")
        .select("*")
        .eq("user_id", &user_id)
        .eq("status", "pending")
        .gt("priority", 0)
        .order("priority", Some(true)) // true = descending order
        .limit(10)
        .execute()
        .await?;
    
    println!(
        "Retrieved {} high priority pending tasks",
        filtered_tasks.len()
    );
    for task in &filtered_tasks {
        println!(
            "  - Task: {} (Priority: {}, Status: {})",
            task.title, task.priority, task.status
        );
    }
    
    // Example 7: Counting tasks
    println!("\nExample 7: Counting tasks");
    
    let count = supabase
        .from("tasks")
        .select("*")
        .eq("user_id", &user_id)
        .count(CountMethod::Exact)
        .execute_count()
        .await?;
    
    println!("Total task count for user: {}", count);
    
    // Example 8: Retrieving specific columns
    println!("\nExample 8: Retrieving specific columns");
    
    #[derive(Debug, Deserialize)]
    struct TaskSummary {
        id: Uuid,
        title: String,
        status: String,
    }
    
    let task_summaries: Vec<TaskSummary> = supabase
        .from("tasks")
        .select("id,title,status")
        .eq("user_id", &user_id)
        .execute()
        .await?;
    
    println!("Retrieved {} task summaries", task_summaries.len());
    for summary in &task_summaries {
        println!("  - Task: {} (Status: {})", summary.title, summary.status);
    }
    
    // Example 9: Using OR conditions
    println!("\nExample 9: Using OR conditions");
    
    let or_tasks: Vec<Task> = supabase
        .from("tasks")
        .select("*")
        .eq("user_id", &user_id)
        .or("status.eq.completed,status.eq.in_progress")
        .execute()
        .await?;
    
    println!(
        "Retrieved {} tasks that are either completed or in progress",
        or_tasks.len()
    );
    for task in &or_tasks {
        println!("  - Task: {} (Status: {})", task.title, task.status);
    }
    
    // Example 10: Using IS conditions (for NULL values)
    println!("\nExample 10: Finding tasks with no due date");
    
    let no_due_date_tasks: Vec<Task> = supabase
        .from("tasks")
        .select("*")
        .eq("user_id", &user_id)
        .is("due_date", "null")
        .execute()
        .await?;
    
    println!("Retrieved {} tasks with no due date", no_due_date_tasks.len());
    for task in &no_due_date_tasks {
        println!("  - Task: {}", task.title);
    }
    
    // Example 11: Text search
    println!("\nExample 11: Text search");
    
    let search_tasks: Vec<Task> = supabase
        .from("tasks")
        .select("*")
        .eq("user_id", &user_id)
        .ilike("title", "%supabase%")
        .execute()
        .await?;
    
    println!(
        "Retrieved {} tasks with 'supabase' in the title",
        search_tasks.len()
    );
    for task in &search_tasks {
        println!("  - Task: {}", task.title);
    }
    
    // Example 12: Deleting a task
    println!("\nExample 12: Deleting a task");
    
    if let Some(task_to_delete) = tasks.first() {
        let task_id = task_to_delete.id.expect("Task should have an ID");
        
        let deleted_task: Task = supabase
            .from("tasks")
            .delete()
            .eq("id", &task_id.to_string())
            .execute_one()
            .await?;
        
        println!("Deleted task: {}", deleted_task.title);
        
        // Verify deletion
        let remaining_tasks: Vec<Task> = supabase
            .from("tasks")
            .select("*")
            .eq("user_id", &user_id)
            .execute()
            .await?;
        
        println!(
            "Remaining tasks after deletion: {}",
            remaining_tasks.len()
        );
    }
    
    // Example 13: Using custom queries with RPC
    println!("\nExample 13: Using RPC for a custom function");
    
    // Note: This assumes you have created a function named 'get_task_count_by_status'
    // in your Supabase database that accepts a user_id and status parameter
    /*
    #[derive(Debug, Deserialize)]
    struct TaskCount {
        count: i64,
    }
    
    let task_count: TaskCount = supabase
        .rpc("get_task_count_by_status", json!({
            "p_user_id": user_id,
            "p_status": "pending"
        }))
        .execute_one()
        .await?;
    
    println!("Task count via RPC: {}", task_count.count);
    */
    
    // Example 14: Deleting all test data
    println!("\nExample 14: Cleaning up - deleting all test tasks");
    
    let _: Vec<Task> = supabase
        .from("tasks")
        .delete()
        .eq("user_id", &user_id)
        .execute()
        .await?;
    
    println!("Deleted all test tasks for user");
    
    println!("\nDatabase example completed");
    
    Ok(())
}