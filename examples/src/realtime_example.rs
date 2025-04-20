use supabase_rust::prelude::*;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    id: Option<i32>,
    content: String,
    user_id: Option<String>,
    created_at: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Get Supabase URL and key from environment variables
    let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");
    
    // Initialize the Supabase client
    let supabase = Supabase::new(&supabase_url, &supabase_key);
    
    println!("Starting Realtime example");
    
    // This example assumes you have a 'messages' table with the following structure:
    // create table messages (
    //   id serial primary key,
    //   content text not null,
    //   user_id uuid references auth.users(id),
    //   created_at timestamp with time zone default now()
    // );
    //
    // And that you've enabled realtime for this table in the Supabase dashboard
    
    // Create a message counter to track received messages
    let message_counter = Arc::new(Mutex::new(0));
    let message_counter_clone = message_counter.clone();
    
    // Set up the realtime client
    let realtime = supabase.realtime();
    
    // Connect to the realtime channel for the messages table
    let channel = realtime.channel("public:messages");
    
    // Subscribe to all inserts on the messages table
    channel
        .on(
            RealtimeListenType::PostgresChanges,
            RealtimeChannelOptions::new("public", "messages", Some(RealtimePostgresChangeEvent::Insert))
        )
        .subscribe(move |payload: RealtimeMessage| {
            let mut counter = message_counter_clone.blocking_lock();
            *counter += 1;
            
            if let Some(record) = payload.new {
                match serde_json::from_value::<Message>(record.clone()) {
                    Ok(message) => {
                        println!("Received new message: {:?}", message);
                    },
                    Err(e) => {
                        println!("Error deserializing message: {:?}", e);
                        println!("Raw payload: {:?}", record);
                    }
                }
            }
        })
        .await?;
    
    println!("Subscribed to realtime updates for the messages table");
    println!("Will listen for 30 seconds and simultaneously insert some test messages");
    
    // Use the Postgres client to insert some test messages
    let client = supabase.postgrest();
    
    // Insert messages in another task to not block the main thread
    tokio::spawn({
        let client = client.clone();
        async move {
            for i in 1..=5 {
                sleep(Duration::from_secs(3)).await;
                
                let message = Message {
                    id: None,
                    content: format!("Test message {}", i),
                    user_id: None,
                    created_at: None,
                };
                
                match client
                    .from("messages")
                    .insert(json!(message))
                    .execute_one::<Message>()
                    .await {
                        Ok(msg) => println!("Inserted message: {:?}", msg),
                        Err(e) => println!("Error inserting message: {:?}", e),
                    }
            }
        }
    });
    
    // Wait for 30 seconds to receive realtime updates
    sleep(Duration::from_secs(30)).await;
    
    // Unsubscribe from the channel
    channel.unsubscribe().await?;
    
    let final_count = *message_counter.lock().await;
    println!("Realtime example completed");
    println!("Received {} messages during the test", final_count);
    
    Ok(())
}