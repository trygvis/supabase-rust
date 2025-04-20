use supabase_rust::prelude::*;
use dotenv::dotenv;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Task {
    id: Option<i32>,
    title: String,
    description: Option<String>,
    is_complete: bool,
    created_at: Option<String>,
    user_id: String,
}

// The payload for Real-time messages
#[derive(Debug, Deserialize)]
struct RealtimePayload<T> {
    #[serde(rename = "type")]
    event_type: String,
    table: String,
    schema: String,
    record: Option<T>,
    old_record: Option<T>,
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
    
    println!("Starting Realtime example");
    
    // First, sign up a test user for our example
    let test_email = format!("test-realtime-{}@example.com", uuid::Uuid::new_v4());
    let test_password = "password123";
    
    let sign_up_result = supabase
        .auth()
        .sign_up(&test_email, test_password)
        .await?;
    
    let user_id = sign_up_result.user.id;
    println!("Created test user with ID: {}", user_id);
    
    // Store received messages in a thread-safe vector
    let messages = Arc::new(Mutex::new(Vec::new()));
    let messages_clone = messages.clone();
    
    // Setup realtime client to listen to the 'tasks' table
    let realtime = supabase.realtime();
    
    // Create channel for 'tasks' table changes
    let channel = realtime
        .channel("public:tasks")
        .on_insert(move |payload: HashMap<String, serde_json::Value>| {
            let messages = messages_clone.clone();
            async move {
                if let Ok(payload) = serde_json::from_value::<RealtimePayload<Task>>(json!(payload)) {
                    if let Some(record) = payload.record {
                        let mut messages = messages.lock().await;
                        messages.push(format!("INSERT: {}", record.title));
                        println!("Received INSERT event: {:?}", record);
                    }
                }
            }
        })
        .on_update(|payload: HashMap<String, serde_json::Value>| {
            async move {
                if let Ok(payload) = serde_json::from_value::<RealtimePayload<Task>>(json!(payload)) {
                    if let Some(record) = payload.record {
                        println!("Received UPDATE event: {:?}", record);
                    }
                }
            }
        })
        .on_delete(|payload: HashMap<String, serde_json::Value>| {
            async move {
                if let Ok(payload) = serde_json::from_value::<RealtimePayload<Task>>(json!(payload)) {
                    if let Some(old_record) = payload.old_record {
                        println!("Received DELETE event: {:?}", old_record);
                    }
                }
            }
        });
    
    // Subscribe to the channel
    let subscription = channel.subscribe().await?;
    println!("Subscribed to realtime changes on public:tasks");
    
    // Create a task through PostgREST
    let postgrest = supabase.from("tasks");
    
    // Insert tasks with some delay to observe realtime events
    for i in 1..5 {
        let task = Task {
            id: None,
            title: format!("Realtime Task {}", i),
            description: Some(format!("Description for realtime task {}", i)),
            is_complete: false,
            created_at: None,
            user_id: user_id.clone(),
        };
        
        println!("Inserting task: {}", task.title);
        postgrest
            .insert(json!(task))
            .execute()
            .await?;
        
        // Wait a bit to see the realtime event
        sleep(Duration::from_secs(1)).await;
    }
    
    // Update a task
    println!("\nUpdating tasks...");
    postgrest
        .update(json!({ "is_complete": true }))
        .like("title", "Realtime Task 1")
        .execute()
        .await?;
    
    sleep(Duration::from_secs(1)).await;
    
    // Delete a task
    println!("\nDeleting a task...");
    postgrest
        .delete()
        .like("title", "Realtime Task 2")
        .execute()
        .await?;
    
    // Wait a bit to receive all events
    sleep(Duration::from_secs(2)).await;
    
    // Check received messages
    let message_count = messages.lock().await.len();
    println!("\nReceived {} insert notifications", message_count);
    
    // Unsubscribe from realtime updates
    subscription.unsubscribe().await?;
    println!("Unsubscribed from realtime updates");
    
    // Clean up - delete all tasks for our test user
    postgrest
        .delete()
        .eq("user_id", &user_id)
        .execute()
        .await?;
    
    println!("Realtime example completed");
    
    Ok(())
}