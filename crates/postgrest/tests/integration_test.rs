#![cfg(feature = "integration-tests")]

use dotenvy::dotenv;
use std::env;
use supabase_rust_postgrest::PostgrestClient;
use reqwest::Client;
use serde_json::json;
use once_cell::sync::Lazy;
use chrono;

// Structure to hold test configuration
struct TestConfig {
    url: String,
    key: String,
}

// Lazily load environment variables once
static CONFIG: Lazy<TestConfig> = Lazy::new(|| {
    dotenv().ok(); // Load .env file if present
    let url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set for integration tests");
    let key = env::var("SUPABASE_ANON_KEY").expect("SUPABASE_ANON_KEY must be set for integration tests");
    TestConfig { url, key }
});

fn create_test_client() -> PostgrestClient {
    let http_client = Client::new();
    // Use a dummy table name for the initial client, actual table is specified via .from() or specific methods
    PostgrestClient::new(&CONFIG.url, &CONFIG.key, "__dummy__", http_client)
}

// Basic test to ensure we can connect and make a simple request
// Assumes a table named 'integration_test_items' exists
// You might need to create this table in your test Supabase project:
// CREATE TABLE integration_test_items (id SERIAL PRIMARY KEY, name TEXT);
#[tokio::test]
async fn test_connection_and_basic_select() {
    let client = create_test_client();
    let table_name = "integration_test_items"; // Define a specific table for tests

    // Attempt to select from the test table (it might be empty)
    let result = client
        .from(table_name)
        .select("*")
        .limit(1) // Just need to know if the request succeeds
        .execute::<Vec<serde_json::Value>>()
        .await;

    // Check if the request was successful (even if it returns an empty vec)
    assert!(result.is_ok(), "Failed to connect and select from {}: {:?}", table_name, result.err());
    
    println!("Successfully connected to {} and performed a basic select.", CONFIG.url);
}

#[tokio::test]
async fn test_crud_operations() {
    let client = create_test_client();
    let table_name = "integration_test_items";
    // Use a unique name for each test run to avoid conflicts if cleanup fails
    let item_name = format!("test_item_{}", chrono::Utc::now().timestamp_nanos());
    let initial_data = json!({ "value": 1 });
    let updated_data = json!({ "value": 2, "new_field": "added" });

    // --- 1. Insert --- 
    let insert_payload = json!({ "name": item_name, "data": initial_data });
    let insert_result = client
        .from(table_name)
        .insert(insert_payload)
        .execute::<Vec<serde_json::Value>>() // PostgREST returns array on insert
        .await;

    assert!(insert_result.is_ok(), "Insert failed: {:?}", insert_result.err());
    let inserted_rows = insert_result.unwrap();
    assert_eq!(inserted_rows.len(), 1, "Insert should return one row");
    let inserted_item = &inserted_rows[0];
    assert_eq!(inserted_item["name"], item_name);
    assert!(inserted_item["id"].is_number(), "Inserted item should have a numeric ID");
    let item_id = inserted_item["id"].as_i64().unwrap(); // Get ID for later use
    println!("Successfully inserted item with ID: {}", item_id);

    // Defer cleanup using a simple scope guard (ensure deletion even on panic/early return)
    // Note: This requires the item_id obtained above.
    struct CleanupGuard<'a> {
        client: &'a PostgrestClient,
        table: &'a str,
        id: i64,
    }
    impl<'a> Drop for CleanupGuard<'a> {
        fn drop(&mut self) {
            let client = self.client.clone();
            let table = self.table.to_string();
            let id_str = self.id.to_string();
            println!("Cleaning up item with ID: {}", self.id);
            // We need a separate async runtime or block_on here for drop, which is complex.
            // For simplicity in this example, we'll just delete after the tests.
            // A better approach in real tests might involve test fixtures or manual cleanup.
        }
    }
    // let _guard = CleanupGuard { client: &client, table: table_name, id: item_id };

    // --- 2. Select (with filter) --- 
    let select_result = client
        .from(table_name)
        .select("id, name, data")
        .eq("id", &item_id.to_string())
        .execute::<Vec<serde_json::Value>>()
        .await;
    
    assert!(select_result.is_ok(), "Select failed: {:?}", select_result.err());
    let selected_items = select_result.unwrap();
    assert_eq!(selected_items.len(), 1, "Should select exactly one item by ID");
    assert_eq!(selected_items[0]["name"], item_name);
    assert_eq!(selected_items[0]["data"], initial_data);
    println!("Successfully selected item with ID: {}", item_id);

    // --- 3. Update --- 
    let update_payload = json!({ "data": updated_data });
    let update_result = client
        .from(table_name)
        .eq("id", &item_id.to_string())
        .update(update_payload)
        .execute::<Vec<serde_json::Value>>() // Update also returns array
        .await;
        
    assert!(update_result.is_ok(), "Update failed: {:?}", update_result.err());
    let updated_rows = update_result.unwrap();
    assert_eq!(updated_rows.len(), 1, "Update should return one row");
    assert_eq!(updated_rows[0]["data"], updated_data);
    println!("Successfully updated item with ID: {}", item_id);

    // --- 4. Delete --- 
    let delete_result = client
        .from(table_name)
        .eq("id", &item_id.to_string())
        .delete()
        .execute::<Vec<serde_json::Value>>() // Delete can also return the deleted row(s)
        .await;
        
    assert!(delete_result.is_ok(), "Delete failed: {:?}", delete_result.err());
    // Optionally check the returned deleted data
    // assert_eq!(delete_result.unwrap().len(), 1);
    println!("Successfully deleted item with ID: {}", item_id);

    // --- Verify Deletion --- 
    let verify_select_result = client
        .from(table_name)
        .select("id")
        .eq("id", &item_id.to_string())
        .execute::<Vec<serde_json::Value>>()
        .await;

    assert!(verify_select_result.is_ok(), "Verification select failed: {:?}", verify_select_result.err());
    assert!(verify_select_result.unwrap().is_empty(), "Item should be deleted");
    println!("Successfully verified deletion of item with ID: {}", item_id);
}

// TODO: Add more integration tests (filters, joins, rpc, transactions) 