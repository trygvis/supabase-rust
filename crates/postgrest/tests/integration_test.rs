#![cfg(feature = "integration-tests")]

use chrono::{DateTime, Utc};
use dotenvy::dotenv;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use supabase_rust_postgrest::PostgrestClient;

// Structure to hold test configuration
struct TestConfig {
    url: String,
    key: String,
}

// Lazily load environment variables once
static CONFIG: Lazy<TestConfig> = Lazy::new(|| {
    dotenv().ok(); // Load .env file if present
    let url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set for integration tests");
    let key =
        env::var("SUPABASE_ANON_KEY").expect("SUPABASE_ANON_KEY must be set for integration tests");
    TestConfig { url, key }
});

fn create_test_client(table: &str) -> PostgrestClient {
    let http_client = Client::new();
    // Use the provided table name when creating the client
    PostgrestClient::new(&CONFIG.url, &CONFIG.key, table, http_client)
}

// Basic test to ensure we can connect and make a simple request
// Assumes a table named 'integration_test_items' exists
// You might need to create this table in your test Supabase project:
// CREATE TABLE integration_test_items (id SERIAL PRIMARY KEY, name TEXT);
#[tokio::test]
async fn test_connection_and_basic_select() {
    let client = create_test_client("integration_test_items");
    let table_name = "integration_test_items"; // Define a specific table for tests

    // Attempt to select from the test table (it might be empty)
    let result = client
        .select("*")
        .limit(1) // Just need to know if the request succeeds
        .execute::<serde_json::Value>()
        .await;

    // Check if the request was successful (even if it returns an empty vec)
    assert!(
        result.is_ok(),
        "Failed to connect and select from {}: {:?}",
        table_name,
        result.err()
    );

    println!(
        "Successfully connected to {} and performed a basic select.",
        CONFIG.url
    );
}

#[tokio::test]
async fn test_crud_operations() {
    let table_name = "integration_test_items";
    let client = create_test_client(table_name);
    // Use a unique name for each test run to avoid conflicts if cleanup fails
    let now: DateTime<Utc> = Utc::now();
    let item_name = format!("test_item_{}", now.timestamp_nanos_opt().unwrap_or(0));
    let initial_data = json!({ "value": 1 });
    let updated_data = json!({ "value": 2, "new_field": "added" });

    // --- 1. Insert ---
    let insert_payload = json!({ "name": item_name, "data": initial_data });
    let insert_result = client
        .insert(insert_payload)
        .await;

    assert!(
        insert_result.is_ok(),
        "Insert failed: {:?}",
        insert_result.err()
    );
    let inserted_value = insert_result.unwrap();
    let inserted_item = if inserted_value.is_array() {
        inserted_value.get(0)
    } else {
        Some(&inserted_value)
    }.expect("Insert should return an object or a single-element array");

    assert_eq!(inserted_item.get("name").and_then(Value::as_str), Some(item_name.as_str()));
    let inserted_id = inserted_item.get("id");
    assert!(inserted_id.map(|v| v.is_number()).unwrap_or(false), "Inserted item should have a numeric ID");
    let item_id = inserted_id.and_then(Value::as_i64).unwrap(); // Get ID for later use
    println!("Successfully inserted item with ID: {}", item_id);

    // --- 2. Select (with filter) ---
    let select_result = client
        .select("id, name, data")
        .eq("id", &item_id.to_string())
        .execute::<serde_json::Value>()
        .await;

    assert!(
        select_result.is_ok(),
        "Select failed: {:?}",
        select_result.err()
    );
    let selected_items = select_result.unwrap();
    assert_eq!(selected_items.len(), 1, "Should select exactly one item by ID");
    let selected_item = selected_items.get(0).expect("Selected item should exist");
    let selected_name = selected_item.as_object().expect("Selected item should be object").get("name");
    assert_eq!(selected_name.and_then(Value::as_str), Some(item_name.as_str()));
    let selected_data = selected_item.as_object().expect("Selected item should be object").get("data");
    assert_eq!(selected_data.cloned(), Some(initial_data.clone()));
    println!("Successfully selected item with ID: {}", item_id);

    // --- 3. Update ---
    let client = create_test_client(table_name); // Re-create client
    let update_payload = json!({ "data": updated_data });
    let update_result = client
        .eq("id", &item_id.to_string())
        .update(update_payload)
        .await;

    assert!(
        update_result.is_ok(),
        "Update failed: {:?}",
        update_result.err()
    );
    let updated_value = update_result.unwrap();
    let updated_item = if updated_value.is_array() {
        updated_value.get(0)
    } else {
        Some(&updated_value)
    }.expect("Update should return an object or a single-element array");

    assert_eq!(updated_item.get("data"), Some(&updated_data));
    println!("Successfully updated item with ID: {}", item_id);

    // --- 4. Delete ---
    let client = create_test_client(table_name); // Re-create client
    let delete_result = client
        .eq("id", &item_id.to_string())
        .delete()
        .await;

    assert!(
        delete_result.is_ok(),
        "Delete failed: {:?}",
        delete_result.err()
    );
    // Optionally check the returned deleted data
    // assert_eq!(delete_result.unwrap().len(), 1);
    println!("Successfully deleted item with ID: {}", item_id);

    // --- Verify Deletion ---
    let client = create_test_client(table_name); // Re-create client
    let verify_select_result = client
        .select("id")
        .eq("id", &item_id.to_string())
        .execute::<Value>()
        .await;

    assert!(
        verify_select_result.is_ok(),
        "Verification select failed: {:?}",
        verify_select_result.err()
    );
    assert!(
        verify_select_result.unwrap().is_empty(),
        "Item should be deleted"
    );
    println!(
        "Successfully verified deletion of item with ID: {}",
        item_id
    );
}

// TODO: Add more integration tests (filters, joins, rpc, transactions)
