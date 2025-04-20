use supabase_rust::prelude::*;
use serde::{Deserialize, Serialize};
use std::env;
use dotenv::dotenv;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: Option<String>,
    email: String,
    #[serde(skip_serializing)]
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Profile {
    id: Option<String>,
    user_id: String,
    username: String,
    avatar_url: Option<String>,
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
    
    println!("Starting Auth example");
    
    // Generate a unique email for testing
    let unique_id = Uuid::new_v4().to_string();
    let test_email = format!("test-user-{}@example.com", unique_id);
    
    // Create a new user
    let auth = supabase.auth();
    
    // Sign up a new user
    println!("Signing up a new user with email: {}", test_email);
    let user = User {
        id: None,
        email: test_email.clone(),
        password: "securePassword123!".to_string(),
    };
    
    let user_response = auth.sign_up(&user.email, &user.password).await?;
    println!("User created successfully: {}", user_response.user.id);
    
    // Sign in the user
    println!("Signing in the user");
    let sign_in_response = auth.sign_in_with_password(&user.email, &user.password).await?;
    println!("User signed in successfully");
    
    // Get the user's access token
    let access_token = sign_in_response.session.access_token;
    println!("Access token retrieved: {:.15}...", access_token);
    
    // Create a new Supabase client with the user's session
    let authenticated_supabase = Supabase::new_with_session(&supabase_url, &supabase_key, &access_token);
    
    // Create a profile for the user
    println!("Creating user profile");
    let client = authenticated_supabase.postgrest();
    
    let profile = Profile {
        id: None,
        user_id: user_response.user.id.clone(),
        username: format!("user_{}", unique_id.split('-').next().unwrap_or("default")),
        avatar_url: None,
    };
    
    // This assumes you have a 'profiles' table that's set up with RLS policies
    // that allow authenticated users to insert their own profile
    match client
        .from("profiles")
        .insert(json!(profile))
        .execute_one::<Profile>()
        .await {
            Ok(created_profile) => {
                println!("Profile created successfully: {:?}", created_profile);
            },
            Err(e) => {
                println!("Error creating profile: {:?}", e);
                println!("Note: This example assumes you have a 'profiles' table set up with appropriate RLS policies");
            }
        }
    
    // Get user data
    println!("Getting user data");
    match auth.get_user(&access_token).await {
        Ok(user_data) => {
            println!("User data: {:?}", user_data);
        },
        Err(e) => {
            println!("Error getting user data: {:?}", e);
        }
    }
    
    // Sign out
    println!("Signing out");
    match auth.sign_out(&access_token).await {
        Ok(_) => {
            println!("User signed out successfully");
        },
        Err(e) => {
            println!("Error signing out: {:?}", e);
        }
    }
    
    // Clean up - Usually not needed in a real application,
    // but helpful for our example to avoid accumulating test users
    println!("Note: In a real application, you would normally not delete users programmatically.");
    println!("This is only done in this example to clean up the test user.");
    
    // To clean up the test user, you would typically use the Supabase dashboard
    // or a server-side admin API, which is not available through the client library
    
    println!("Auth example completed");
    
    Ok(())
}