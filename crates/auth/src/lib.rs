//! Supabase Auth client for Rust
//!
//! This crate provides authentication functionality for Supabase,
//! including sign up, sign in, session management, and user operations.

use std::sync::Arc;
use std::sync::RwLock;
use reqwest::Client;
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// エラー型
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("JSON serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Missing session")]
    MissingSession,
    
    #[error("Invalid token: {0}")]
    InvalidToken(String),
}

/// ユーザー情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub app_metadata: serde_json::Value,
    pub user_metadata: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

/// セッション情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
    pub user: User,
}

/// サインイン認証情報
#[derive(Debug, Serialize)]
pub struct SignInCredentials {
    pub email: String,
    pub password: String,
}

/// クライアントオプション
#[derive(Debug, Clone)]
pub struct AuthOptions {
    pub auto_refresh_token: bool,
    pub persist_session: bool,
    pub detect_session_in_url: bool,
}

impl Default for AuthOptions {
    fn default() -> Self {
        Self {
            auto_refresh_token: true,
            persist_session: true,
            detect_session_in_url: true,
        }
    }
}

/// Auth クライアント
pub struct Auth {
    url: String,
    key: String,
    http_client: Client,
    options: AuthOptions,
    current_session: Arc<RwLock<Option<Session>>>,
}

impl Auth {
    /// 新しい Auth クライアントを作成
    pub fn new(url: &str, key: &str, http_client: Client, options: AuthOptions) -> Self {
        Self {
            url: url.to_string(),
            key: key.to_string(),
            http_client,
            options,
            current_session: Arc::new(RwLock::new(None)),
        }
    }
    
    /// ユーザー登録
    pub async fn sign_up(&self, email: &str, password: &str) -> Result<Session, AuthError> {
        let url = format!("{}/auth/v1/signup", self.url);
        
        let payload = serde_json::json!({
            "email": email,
            "password": password,
        });
        
        let response = self.http_client.post(&url)
            .header("apikey", &self.key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AuthError::ApiError(error_text));
        }
        
        let session: Session = response.json().await?;
        
        // セッションを保存
        if self.options.persist_session {
            let mut write_guard = self.current_session.write().unwrap();
            *write_guard = Some(session.clone());
        }
        
        Ok(session)
    }
    
    /// メール・パスワードでログイン
    pub async fn sign_in_with_password(&self, email: &str, password: &str) -> Result<Session, AuthError> {
        let url = format!("{}/auth/v1/token?grant_type=password", self.url);
        
        let payload = serde_json::json!({
            "email": email,
            "password": password,
        });
        
        let response = self.http_client.post(&url)
            .header("apikey", &self.key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AuthError::ApiError(error_text));
        }
        
        let session: Session = response.json().await?;
        
        // セッションを保存
        if self.options.persist_session {
            let mut write_guard = self.current_session.write().unwrap();
            *write_guard = Some(session.clone());
        }
        
        Ok(session)
    }
    
    /// 現在のセッションを取得
    pub fn get_session(&self) -> Option<Session> {
        let read_guard = self.current_session.read().unwrap();
        read_guard.clone()
    }
    
    /// 現在のユーザーを取得
    pub async fn get_user(&self) -> Result<User, AuthError> {
        let session = self.get_session().ok_or(AuthError::MissingSession)?;
        
        let url = format!("{}/auth/v1/user", self.url);
        
        let response = self.http_client.get(&url)
            .header("apikey", &self.key)
            .header("Authorization", format!("Bearer {}", session.access_token))
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AuthError::ApiError(error_text));
        }
        
        let user: User = response.json().await?;
        
        Ok(user)
    }
    
    /// セッションをリフレッシュ
    pub async fn refresh_session(&self) -> Result<Session, AuthError> {
        let session = self.get_session().ok_or(AuthError::MissingSession)?;
        
        let url = format!("{}/auth/v1/token?grant_type=refresh_token", self.url);
        
        let payload = serde_json::json!({
            "refresh_token": session.refresh_token,
        });
        
        let response = self.http_client.post(&url)
            .header("apikey", &self.key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AuthError::ApiError(error_text));
        }
        
        let new_session: Session = response.json().await?;
        
        // セッションを更新
        if self.options.persist_session {
            let mut write_guard = self.current_session.write().unwrap();
            *write_guard = Some(new_session.clone());
        }
        
        Ok(new_session)
    }
    
    /// サインアウト
    pub async fn sign_out(&self) -> Result<(), AuthError> {
        let session = self.get_session().ok_or(AuthError::MissingSession)?;
        
        let url = format!("{}/auth/v1/logout", self.url);
        
        let response = self.http_client.post(&url)
            .header("apikey", &self.key)
            .header("Authorization", format!("Bearer {}", session.access_token))
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AuthError::ApiError(error_text));
        }
        
        // セッションをクリア
        let mut write_guard = self.current_session.write().unwrap();
        *write_guard = None;
        
        Ok(())
    }
    
    /// パスワードリセットメールの送信
    pub async fn reset_password_for_email(&self, email: &str) -> Result<(), AuthError> {
        let url = format!("{}/auth/v1/recover", self.url);
        
        let payload = serde_json::json!({
            "email": email,
        });
        
        let response = self.http_client.post(&url)
            .header("apikey", &self.key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AuthError::ApiError(error_text));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};
    
    #[tokio::test]
    async fn test_sign_up() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/auth/v1/signup"))
            .respond_with(ResponseTemplate::new(200).json(serde_json::json!({
                "access_token": "test_access_token",
                "refresh_token": "test_refresh_token",
                "expires_in": 3600,
                "token_type": "bearer",
                "user": {
                    "id": "test_user_id",
                    "email": "test@example.com",
                    "phone": null,
                    "app_metadata": {},
                    "user_metadata": {},
                    "created_at": "2021-01-01T00:00:00Z",
                    "updated_at": "2021-01-01T00:00:00Z"
                }
            })))
            .mount(&mock_server)
            .await;
        
        let http_client = Client::new();
        let auth = Auth::new(
            &mock_server.uri(),
            "test_key",
            http_client,
            AuthOptions::default(),
        );
        
        let result = auth.sign_up("test@example.com", "password123").await;
        
        assert!(result.is_ok());
        let session = result.unwrap();
        assert_eq!(session.access_token, "test_access_token");
        assert_eq!(session.user.email, Some("test@example.com".to_string()));
    }
}