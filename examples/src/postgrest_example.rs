use supabase_rust::prelude::*;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
struct Todo {
    id: Option<i32>,
    task: String,
    is_complete: bool,
    inserted_at: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Get Supabase URL and key from environment variables
    let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");
    
    // Initialize the Supabase client
    let supabase = Supabase::new(&supabase_url, &supabase_key);
    
    // Access the Postgrest client for database operations
    let client = supabase.postgrest();
    
    println!("Starting Postgrest example");
    
    // Create a new todo
    let new_todo = Todo {
        id: None,
        task: "Learn Rust and Supabase".to_string(),
        is_complete: false,
        inserted_at: None,
    };
    
    // Step 1: Insert a new record
    println!("\nInserting a new todo");
    let inserted_todo: Option<Todo> = client
        .from("todos")
        .insert(&new_todo)
        .options(InsertOptions::new().returning(ReturnType::Representation))
        .execute()
        .await?
        .json()
        .await?;
    
    if let Some(todo) = &inserted_todo {
        println!("Inserted todo: {:?}", todo);
    } else {
        println!("Failed to get inserted todo");
    }
    
    // Step 2: Select all todos
    println!("\nSelecting all todos");
    let todos: Vec<Todo> = client
        .from("todos")
        .select("*")
        .execute()
        .await?
        .json()
        .await?;
    
    println!("All todos:");
    for todo in &todos {
        println!("{:?}", todo);
    }
    
    // Step 3: Filter todos with conditions
    println!("\nFiltering incomplete todos");
    let incomplete_todos: Vec<Todo> = client
        .from("todos")
        .select("*")
        .eq("is_complete", "false")
        .execute()
        .await?
        .json()
        .await?;
    
    println!("Incomplete todos:");
    for todo in &incomplete_todos {
        println!("{:?}", todo);
    }
    
    // Step 4: Update a todo
    if let Some(todo) = &inserted_todo {
        println!("\nUpdating todo with id: {:?}", todo.id);
        let updated_todo: Option<Todo> = client
            .from("todos")
            .update(json!({
                "is_complete": true
            }))
            .eq("id", todo.id.unwrap().to_string())
            .options(UpdateOptions::new().returning(ReturnType::Representation))
            .execute()
            .await?
            .json()
            .await?;
        
        if let Some(todo) = updated_todo {
            println!("Updated todo: {:?}", todo);
        }
    }
    
    // Step 5: Count todos
    println!("\nCounting todos");
    let count: CountResult = client
        .from("todos")
        .select("*")
        .options(SelectOptions::new().count(CountType::Exact))
        .execute()
        .await?
        .count();
    
    println!("Total todo count: {}", count.count.unwrap_or(0));
    
    // Step 6: Demonstrate advanced query features
    println!("\nAdvanced query: filtering, ordering, limiting");
    let filtered_todos: Vec<Todo> = client
        .from("todos")
        .select("*")
        .order("inserted_at", Some(Order::Descending))
        .limit(5)
        .execute()
        .await?
        .json()
        .await?;
    
    println!("Recent todos (up to 5):");
    for todo in &filtered_todos {
        println!("{:?}", todo);
    }
    
    // Step 7: Delete the created todo
    if let Some(todo) = &inserted_todo {
        println!("\nDeleting todo with id: {:?}", todo.id);
        let deleted = client
            .from("todos")
            .delete()
            .eq("id", todo.id.unwrap().to_string())
            .execute()
            .await?;
        
        println!("Delete result: {:?}", deleted.status());
    }
    
    // Step 8: Demonstrate a join query (if you have related tables)
    println!("\nJoin query example (users and their todos)");
    #[derive(Debug, Deserialize)]
    struct UserWithTodos {
        id: i32,
        username: String,
        todos: Vec<Todo>,
    }
    
    // This assumes you have a users table related to todos
    // Note: This might need adjustment based on your actual schema
    let users_with_todos: Vec<UserWithTodos> = client
        .from("users")
        .select("id, username, todos(*)")
        .execute()
        .await?
        .json()
        .await?;
    
    println!("Users with their todos:");
    for user in &users_with_todos {
        println!("User: {} (ID: {})", user.username, user.id);
        for todo in &user.todos {
            println!("  - Todo: {:?}", todo);
        }
    }
    
    // Step 9: Full text search (if your Postgres has it enabled)
    println!("\nFull text search example");
    let search_results: Vec<Todo> = client
        .from("todos")
        .select("*")
        .textSearch("task", "learn", Some(TextSearchOptions::new()))
        .execute()
        .await?
        .json()
        .await?;
    
    println!("Search results for 'learn':");
    for todo in &search_results {
        println!("{:?}", todo);
    }
    
    // Step 10: Demonstrate using RPC (Remote Procedure Call)
    println!("\nRPC function call example");
    #[derive(Deserialize, Debug)]
    struct RpcResult {
        result: i32,
    }
    
    // This assumes you have a function named 'sum_two_numbers' in your database
    let rpc_result: Option<RpcResult> = client
        .rpc("sum_two_numbers", json!({"a": 3, "b": 4}))
        .execute()
        .await?
        .json()
        .await?;
    
    if let Some(result) = rpc_result {
        println!("RPC result: {}", result.result);
    } else {
        println!("RPC call returned no result");
    }
    
    println!("\nPostgrest example completed");
    
    Ok(())
}