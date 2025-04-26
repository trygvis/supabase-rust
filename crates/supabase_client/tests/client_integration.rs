// crates/supabase_client/tests/client_integration.rs

// Import the crate itself
use supabase_client_lib::client::SupabaseClientWrapper;
use supabase_client_lib::client::SupabaseConfig;
use supabase_client_lib::models::{AuthCredentials, Item};

// Import dev dependencies for mocking, etc.
use chrono::Utc;
use dotenv::dotenv;
use serde_json::json; // To create mock JSON bodies
use std::env;
use uuid::Uuid;
use wiremock::{
    matchers::{header, method, path, path_regex}, // Use path_regex
    Mock,
    MockServer,
    ResponseTemplate,
};

// Import Auth client directly for more controlled testing
use supabase_rust_auth::{Auth, AuthError, AuthOptions, Session, User as AuthUser};
use reqwest::Client as ReqwestClient;

// Helper function
async fn setup_mock_config(mock_server: &MockServer) -> SupabaseConfig {
    dotenv().ok();
    let mut url = mock_server.uri(); // Get base URI
    // Ensure no trailing slash for base URL passed to config
    if url.ends_with('/') {
        url.pop();
    }
    let anon_key = env::var("SUPABASE_ANON_KEY").unwrap_or_else(|_| "mock_anon_key".to_string());
    SupabaseConfig::new(&url, anon_key).unwrap()
}

#[tokio::test]
async fn test_authenticate_success() {
    let mock_server = MockServer::start().await;
    // Use the raw address string
    let base_url = format!("http://{}", mock_server.address());
    let anon_key = "mock_anon_key";

    // Create Auth client directly
    let http_client = ReqwestClient::new();
    let auth_client = Auth::new(
        &base_url,
        anon_key,
        http_client,
        AuthOptions::default(),
    );

    let credentials = AuthCredentials {
        email: "test@example.com".to_string(),
        password: "password".to_string(),
    };

    let mock_user_id_str = Uuid::new_v4().to_string();
    let mock_access_token = "mock_access_token_auth_success";
    let mock_created_at = Utc::now().to_rfc3339();
    let mock_updated_at = Utc::now().to_rfc3339();

    let mock_session_response = json!({
        "access_token": mock_access_token,
        "refresh_token": "mock_refresh_token",
        "expires_in": 3600,
        "token_type": "bearer",
        "user": {
            "id": mock_user_id_str,
            "email": Some(credentials.email.clone()),
            "phone": None::<String>,
            "app_metadata": {},
            "user_metadata": {},
            "created_at": mock_created_at,
            "updated_at": mock_updated_at,
        }
    });

    Mock::given(method("POST"))
        .and(path_regex(r"^/auth/v1/token(?:\?.*)?$"))
        .and(header("apikey", anon_key))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&mock_session_response))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Call authenticate directly
    let auth_result = auth_client
        .sign_in_with_password(&credentials.email, &credentials.password)
        .await;

    // Assert success and check session data
    assert!(auth_result.is_ok());
    let session = auth_result.unwrap();
    assert_eq!(session.access_token, mock_access_token);
    assert_eq!(session.user.id, mock_user_id_str);
    assert_eq!(session.user.email, Some(credentials.email));
    assert_eq!(session.user.created_at, mock_created_at);
}

#[tokio::test]
async fn test_authenticate_failure() {
    let mock_server = MockServer::start().await;
    // Use the raw address string for the Auth client
    let base_url = format!("http://{}", mock_server.address());
    let anon_key = "mock_anon_key";

    // Create Auth client directly for test
    let http_client = ReqwestClient::new();
    let auth_client = Auth::new(
        &base_url, // Use the raw base URL string
        anon_key,
        http_client,
        AuthOptions::default(),
    );

    let credentials = AuthCredentials {
        email: "wrong@example.com".to_string(),
        password: "wrong".to_string(),
    };

    let mock_error_response = json!({
        "error": "invalid_grant",
        "error_description": "Invalid email or password"
    });

    Mock::given(method("POST"))
        .and(path_regex(r"^/auth/v1/token(?:\?.*)?$"))
        .and(header("apikey", anon_key))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(401).set_body_json(&mock_error_response))
        .expect(1)
        .mount(&mock_server) // Mount on the server instance
        .await;

    // Call authenticate directly on the test auth_client
    let auth_result = auth_client
        .sign_in_with_password(&credentials.email, &credentials.password)
        .await;

    // Assert that the specific error occurred
    assert!(auth_result.is_err());
    match auth_result.err().unwrap() {
        AuthError::ApiError(_) => { /* Expected */ }
        e => panic!("Expected ApiError, got {:?}", e),
    }
}

#[tokio::test]
async fn test_fetch_items_authenticated() {
    let _mock_server = MockServer::start().await;
    let config = setup_mock_config(&_mock_server).await;
    let _client = SupabaseClientWrapper::new(config.clone()).unwrap();
    let _mock_access_token = "mock_access_token_fetch";

    // Simulate authentication by manually setting session if possible, or ignore for stub
    // let session = AuthSession { access_token: mock_access_token.to_string(), ... };
    // let mut session_guard = client.current_session.lock().await;
    // *session_guard = Some(session);
    // drop(session_guard);

    let mock_item_id = Uuid::new_v4();
    let mock_user_id = Uuid::new_v4(); // Example user id
    let mock_items = vec![Item {
        id: mock_item_id,
        user_id: mock_user_id,
        name: "Mock Item 1".to_string(),
        description: Some("Description for mock item".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }];
    let auth_header_value = format!("Bearer {}", _mock_access_token);

    Mock::given(method("GET"))
        .and(path("/rest/v1/items"))
        .and(header("Authorization", auth_header_value.as_str())) // Use .as_str()
        .and(header("apikey", config.anon_key.as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(&mock_items))
        .expect(1)
        .mount(&_mock_server)
        .await;

    // Call fetch_items (stubbed)
    let fetch_result = _client.fetch_items().await;
    assert!(fetch_result.is_ok());
}

#[tokio::test]
async fn test_fetch_items_unauthenticated() {
    let _mock_server = MockServer::start().await;
    let config = setup_mock_config(&_mock_server).await;
    let _client = SupabaseClientWrapper::new(config).unwrap();

    Mock::given(method("GET"))
        .and(path("/rest/v1/items"))
        .respond_with(ResponseTemplate::new(401))
        .expect(0) // Expect zero calls
        .mount(&_mock_server)
        .await;

    // Call fetch_items (stubbed)
    let fetch_result = _client.fetch_items().await;
    assert!(fetch_result.is_err());
}

#[tokio::test]
async fn test_integration_crud() {
    let _mock_server = MockServer::start().await;
    let config = setup_mock_config(&_mock_server).await;
    let _client = SupabaseClientWrapper::new(config.clone()).unwrap();
    let _mock_access_token = "mock_access_token_crud";

    // Simulate auth
    // ...

    let item_user_id = Uuid::new_v4();
    let item_to_create = Item {
        // Don't set ID if DB generates it
        id: Uuid::new_v4(), // Or provide if client generates
        user_id: item_user_id,
        name: "CRUD Test Item".to_string(),
        description: Some("Description".to_string()),
        created_at: Utc::now(), // Or use default
        updated_at: Utc::now(), // Or use default
    };
    // Response usually echoes the created item, possibly with DB-generated fields
    let created_item_response = vec![item_to_create.clone()];
    let auth_header_value = format!("Bearer {}", _mock_access_token);

    Mock::given(method("POST"))
        .and(path("/rest/v1/items"))
        .and(header("Authorization", auth_header_value.as_str())) // Use .as_str()
        .and(header("apikey", config.anon_key.as_str()))
        .and(header("Prefer", "return=representation")) // Common for inserts
        // Potentially add body matcher: .and(body_json(json!(item_to_create_without_generated_fields)))
        .respond_with(ResponseTemplate::new(201).set_body_json(&created_item_response))
        .expect(1)
        .mount(&_mock_server)
        .await;

    // ... Mocks for update, delete ...

    // Call create_item (stubbed)
    let create_result = _client.create_item(item_to_create).await;
    assert!(create_result.is_ok());
}
