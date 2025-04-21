use supabase_rust_gftd::prelude::*;
use supabase_rust_gftd::Supabase;
use supabase_rust_gftd::functions::FunctionOptions;
use dotenv::dotenv;
use std::env;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize)]
struct HelloRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct HelloResponse {
    message: String,
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
    
    println!("Starting Edge Functions example");
    
    // Access the Functions client
    let functions = supabase.functions();
    
    // This example assumes you have created an Edge Function named "hello-world"
    // that accepts a JSON payload with a "name" field and returns a JSON response
    // with a "message" field.
    
    // Example Edge Function in JavaScript might look like:
    //
    // Deno.serve(async (req) => {
    //   try {
    //     const { name } = await req.json();
    //     return new Response(
    //       JSON.stringify({
    //         message: `Hello, ${name || 'World'}!`,
    //       }),
    //       { headers: { 'Content-Type': 'application/json' } }
    //     );
    //   } catch (error) {
    //     return new Response(
    //       JSON.stringify({ error: error.message }),
    //       { status: 400, headers: { 'Content-Type': 'application/json' } }
    //     );
    //   }
    // });
    
    // Invoke the function without authentication
    println!("Invoking hello-world function without authentication");
    
    let request_data = HelloRequest {
        name: "Rust".to_string(),
    };
    
    let function_request = functions.create_request::<HelloResponse>("hello-world");
    
    match function_request
        .execute(Some(json!(request_data)), None)
        .await {
            Ok(response) => {
                println!("Function response: {:?}", response);
            },
            Err(e) => {
                println!("Error invoking function: {:?}", e);
                println!("Note: This example assumes you have created an Edge Function named 'hello-world'");
                println!("If the function doesn't exist, you'll need to create it in your Supabase dashboard.");
            }
        }
    
    // Now let's try with authentication
    // First, sign up a test user
    let test_email = format!("test-functions-{}@example.com", uuid::Uuid::new_v4());
    let test_password = "password123";
    
    let sign_up_result = supabase
        .auth()
        .sign_up(&test_email, test_password)
        .await?;
    
    let user_id = sign_up_result.user.id;
    println!("Created test user with ID: {}", user_id);
    
    // Sign in to get a session
    let sign_in_result = supabase
        .auth()
        .sign_in_with_password(&test_email, test_password)
        .await?;
    
    let access_token = sign_in_result.access_token;
    println!("Got access token for authenticated requests");
    
    // Invoke the function with authentication
    println!("Invoking hello-world function with authentication");
    
    let request_data = HelloRequest {
        name: "Authenticated Rust".to_string(),
    };
    
    let function_request = functions.create_request::<HelloResponse>("hello-world");
    let options = FunctionOptions {
        headers: Some(json!({
            "Authorization": format!("Bearer {}", access_token)
        })),
        ..Default::default()
    };
    
    match function_request
        .execute(Some(json!(request_data)), Some(options))
        .await {
            Ok(response) => {
                println!("Authenticated function response: {:?}", response);
            },
            Err(e) => {
                println!("Error invoking authenticated function: {:?}", e);
            }
        }
    
    // Example of invoking a function with custom headers
    println!("Invoking function with custom headers");
    
    let function_request = functions.create_request::<HelloResponse>("hello-world");
    let options = FunctionOptions {
        headers: Some(json!({
            "x-custom-header": "custom-value",
            "x-client-info": "supabase-rust-client"
        })),
        ..Default::default()
    };
    
    match function_request
        .execute(Some(json!(request_data)), Some(options))
        .await {
            Ok(response) => {
                println!("Function response with custom headers: {:?}", response);
            },
            Err(e) => {
                println!("Error invoking function with custom headers: {:?}", e);
            }
        }
    
    println!("Edge Functions example completed");
    
    Ok(())
}