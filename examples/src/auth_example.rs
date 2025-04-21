use supabase_rust_gftd::Supabase;
use dotenv::dotenv;
use std::env;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv().ok();
    
    // Get Supabase URL and key from environment variables
    let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");
    
    // Initialize the Supabase client
    let supabase = Supabase::new(&supabase_url, &supabase_key);
    
    println!("Starting Auth example");
    
    // Generate a unique email for testing
    let unique_id = Uuid::new_v4().to_string();
    let test_email = format!("test-user-{}@example.com", unique_id);
    let test_password = "securePassword123!";
    
    // Create a new user
    let auth = supabase.auth();
    
    // Sign up a new user
    println!("Signing up a new user with email: {}", test_email);
    
    let user_response = auth.sign_up(&test_email, test_password).await?;
    
    println!("User sign up response: {:?}", user_response);
    
    println!("Auth example completed");
    
    Ok(())
}