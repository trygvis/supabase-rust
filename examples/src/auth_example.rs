use supabase_rust::prelude::*;
use serde::{Deserialize, Serialize};
use std::env;
use dotenv::dotenv;
use uuid::Uuid;
use serde_json::json;

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
    
    if let Some(ref user_data) = user_response.user {
        println!("User created successfully: {}", user_data.id);
    } else {
        println!("User creation response received but user data is None");
    }
    
    // Sign in the user
    println!("Signing in the user");
    let sign_in_response = auth.sign_in(&user.email, &user.password).await?;
    println!("User signed in successfully");
    
    // Get the user's access token
    let access_token = match &sign_in_response.session {
        Some(session) => &session.access_token,
        None => {
            println!("No session in sign-in response");
            return Ok(());
        }
    };
    println!("Access token retrieved: {:.15}...", access_token);
    
    // Use the existing client with the session already stored
    let client = supabase.from("profiles");
    
    // Create a profile for the user
    println!("Creating user profile");
    
    let user_id = match &user_response.user {
        Some(user_data) => user_data.id.clone(),
        None => {
            println!("No user data available");
            return Ok(());
        }
    };
    
    let profile = Profile {
        id: None,
        user_id,
        username: format!("user_{}", unique_id.split('-').next().unwrap_or("default")),
        avatar_url: None,
    };
    
    // This assumes you have a 'profiles' table that's set up with RLS policies
    // that allow authenticated users to insert their own profile
    match client
        .insert(json!(profile))
        .execute::<Profile>()
        .await {
            Ok(created_profiles) => {
                if let Some(created_profile) = created_profiles.first() {
                    println!("Profile created successfully: {:?}", created_profile);
                } else {
                    println!("Profile creation response received but no profile data returned");
                }
            },
            Err(e) => {
                println!("Error creating profile: {:?}", e);
                println!("Note: This example assumes you have a 'profiles' table set up with appropriate RLS policies");
            }
        }
    
    // Get user data
    println!("Getting user data");
    match auth.get_user().await {
        Ok(user_data) => {
            println!("User data: {:?}", user_data);
        },
        Err(e) => {
            println!("Error getting user data: {:?}", e);
        }
    }
    
    // Sign out
    println!("Signing out");
    match auth.sign_out().await {
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