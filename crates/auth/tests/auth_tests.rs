use serde_json::json;
use supabase_rust_auth::{AuthClient, SignInWithPasswordCredentials};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_sign_up() {
    // モックサーバーの起動
    let mock_server = MockServer::start().await;

    // モックレスポンスの設定
    Mock::given(method("POST"))
        .and(path("/auth/v1/signup"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "test_access_token",
            "token_type": "bearer",
            "expires_in": 3600,
            "refresh_token": "test_refresh_token",
            "user": {
                "id": "test_user_id",
                "email": "test@example.com",
                "role": "authenticated"
            }
        })))
        .mount(&mock_server)
        .await;

    // AuthClient の初期化
    let auth_client = AuthClient::new(mock_server.uri(), "test_anon_key".to_string(), None, None);

    // サインアップのテスト
    let result = auth_client.sign_up("test@example.com", "password123").await;

    assert!(result.is_ok());
    if let Ok(response) = result {
        assert_eq!(response.access_token, "test_access_token");
        assert_eq!(response.user.id, "test_user_id");
        assert_eq!(response.user.email, Some("test@example.com".to_string()));
    }
}

#[tokio::test]
async fn test_sign_in_with_password() {
    // モックサーバーの起動
    let mock_server = MockServer::start().await;

    // モックレスポンスの設定
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "test_access_token",
            "token_type": "bearer",
            "expires_in": 3600,
            "refresh_token": "test_refresh_token",
            "user": {
                "id": "test_user_id",
                "email": "test@example.com",
                "role": "authenticated"
            }
        })))
        .mount(&mock_server)
        .await;

    // AuthClient の初期化
    let auth_client = AuthClient::new(mock_server.uri(), "test_anon_key".to_string(), None, None);

    // サインインのテスト
    let creds = SignInWithPasswordCredentials {
        email: "test@example.com".to_string(),
        password: "password123".to_string(),
        ..Default::default()
    };

    let result = auth_client.sign_in_with_password(creds).await;

    assert!(result.is_ok());
    if let Ok(response) = result {
        assert_eq!(response.access_token, "test_access_token");
        assert_eq!(response.user.id, "test_user_id");
        assert_eq!(response.user.email, Some("test@example.com".to_string()));
    }
}

#[tokio::test]
async fn test_sign_out() {
    // モックサーバーの起動
    let mock_server = MockServer::start().await;

    // モックレスポンスの設定
    Mock::given(method("POST"))
        .and(path("/auth/v1/logout"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // AuthClient の初期化
    let auth_client = AuthClient::new(
        mock_server.uri(),
        "test_anon_key".to_string(),
        Some("test_access_token".to_string()),
        None,
    );

    // サインアウトのテスト
    let result = auth_client.sign_out().await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_refresh_session() {
    // モックサーバーの起動
    let mock_server = MockServer::start().await;

    // モックレスポンスの設定
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "new_access_token",
            "token_type": "bearer",
            "expires_in": 3600,
            "refresh_token": "new_refresh_token",
            "user": {
                "id": "test_user_id",
                "email": "test@example.com",
                "role": "authenticated"
            }
        })))
        .mount(&mock_server)
        .await;

    // AuthClient の初期化
    let auth_client = AuthClient::new(
        mock_server.uri(),
        "test_anon_key".to_string(),
        Some("old_access_token".to_string()),
        Some("old_refresh_token".to_string()),
    );

    // セッション更新のテスト
    let result = auth_client.refresh_session().await;

    assert!(result.is_ok());
    if let Ok(response) = result {
        assert_eq!(response.access_token, "new_access_token");
        assert_eq!(response.refresh_token, "new_refresh_token");
    }
}
