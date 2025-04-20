//! Authentication and user management for Supabase

mod types;
mod session;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use url::Url;

use crate::config::ClientOptions;
use crate::error::Error;
use crate::fetch::Fetch;

pub use types::*;
pub use session::*;

/// Client for Supabase Authentication
pub struct Auth {
    /// The base URL for the Supabase project
    url: String,
    
    /// The anonymous API key for the Supabase project
    key: String,
    
    /// HTTP client used for requests
    client: Client,
    
    /// The current session
    session: Arc<Mutex<Option<Session>>>,
    
    /// Client options
    options: ClientOptions,
}

impl Auth {
    /// Create a new Auth client
    pub(crate) fn new(
        url: &str, 
        key: &str, 
        client: Client, 
        options: ClientOptions,
    ) -> Self {
        Self {
            url: url.to_string(),
            key: key.to_string(),
            client,
            session: Arc::new(Mutex::new(None)),
            options,
        }
    }
    
    fn get_auth_url(&self, path: &str) -> String {
        format!("{}/auth/v1{}", self.url, path)
    }
    
    /// Sign up a new user with email and password
    pub async fn sign_up(&self, email: &str, password: &str) -> Result<AuthResponse, Error> {
        let url = self.get_auth_url("/signup");
        
        let mut body = HashMap::new();
        body.insert("email".to_string(), email.to_string());
        body.insert("password".to_string(), password.to_string());
        
        let result = Fetch::post(&self.client, &url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .json(&body)?
            .execute::<AuthResponse>()
            .await?;
        
        // Store session if one was returned
        if let Some(ref session) = result.session {
            let mut current_session = self.session.lock().unwrap();
            *current_session = Some(session.clone());
        }
        
        Ok(result)
    }
    
    /// Sign in a user with email and password
    pub async fn sign_in(&self, email: &str, password: &str) -> Result<AuthResponse, Error> {
        let url = self.get_auth_url("/token?grant_type=password");
        
        let mut body = HashMap::new();
        body.insert("email".to_string(), email.to_string());
        body.insert("password".to_string(), password.to_string());
        
        let result = Fetch::post(&self.client, &url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .json(&body)?
            .execute::<AuthResponse>()
            .await?;
        
        // Store session if one was returned
        if let Some(ref session) = result.session {
            let mut current_session = self.session.lock().unwrap();
            *current_session = Some(session.clone());
        }
        
        Ok(result)
    }
    
    /// Sign out the current user
    pub async fn sign_out(&self) -> Result<(), Error> {
        let url = self.get_auth_url("/logout");
        
        let token = {
            let current_session = self.session.lock().unwrap();
            match *current_session {
                Some(ref session) => session.access_token.clone(),
                None => return Err(Error::auth("Not logged in")),
            }
        };
        
        Fetch::post(&self.client, &url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .bearer_auth(&token)
            .execute_raw()
            .await?;
        
        // Clear the session
        let mut current_session = self.session.lock().unwrap();
        *current_session = None;
        
        Ok(())
    }
    
    /// Reset a user's password
    pub async fn reset_password_for_email(&self, email: &str) -> Result<(), Error> {
        let url = self.get_auth_url("/recover");
        
        let mut body = HashMap::new();
        body.insert("email".to_string(), email.to_string());
        
        Fetch::post(&self.client, &url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .json(&body)?
            .execute_raw()
            .await?;
        
        Ok(())
    }
    
    /// Get the user data for the currently authenticated user
    pub async fn get_user(&self) -> Result<User, Error> {
        let url = self.get_auth_url("/user");
        
        let token = {
            let current_session = self.session.lock().unwrap();
            match *current_session {
                Some(ref session) => session.access_token.clone(),
                None => return Err(Error::auth("Not logged in")),
            }
        };
        
        let user = Fetch::get(&self.client, &url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .bearer_auth(&token)
            .execute::<User>()
            .await?;
        
        Ok(user)
    }
    
    /// Update the user data
    pub async fn update(&self, attributes: UserAttributes) -> Result<User, Error> {
        let url = self.get_auth_url("/user");
        
        let token = {
            let current_session = self.session.lock().unwrap();
            match *current_session {
                Some(ref session) => session.access_token.clone(),
                None => return Err(Error::auth("Not logged in")),
            }
        };
        
        let user = Fetch::put(&self.client, &url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .bearer_auth(&token)
            .json(&attributes)?
            .execute::<User>()
            .await?;
        
        Ok(user)
    }
    
    /// Get the current session
    pub fn get_session(&self) -> Option<Session> {
        let current_session = self.session.lock().unwrap();
        current_session.clone()
    }
    
    /// Set the session
    pub fn set_session(&self, session: Session) {
        let mut current_session = self.session.lock().unwrap();
        *current_session = Some(session);
    }
}

/// User attributes that can be updated
#[derive(Debug, Serialize, Deserialize)]
pub struct UserAttributes {
    /// Email address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    
    /// Password
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    
    /// Phone number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    
    /// User metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}