use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Once;
use supabase_rust_realtime::{
    ChannelEvent, DatabaseChanges, RealtimeClient, RealtimeClientOptions, RealtimeMessage,
};
use tokio::sync::mpsc;

// Ensure logger is initialized only once across all tests
static INIT_LOGGER: Once = Once::new();

fn setup_logger() {
    INIT_LOGGER.call_once(|| {
        pretty_env_logger::init();
    });
}

#[tokio::test]
async fn test_client_creation_default_options() {
    setup_logger();
    let _client = RealtimeClient::new("ws://localhost:4000/socket", "someapikey");
    // Assert default options if they are accessible or test behavior based on them
    // Cannot directly access private fields like url, key, options, access_token in integration tests.
    // Tests should rely on public API behavior.
}

#[tokio::test]
async fn test_client_creation_custom_options() {
    setup_logger();
    let options = RealtimeClientOptions {
        auto_reconnect: false,
        max_reconnect_attempts: Some(5),
        ..Default::default()
    };
    // We can't directly assert private fields `url`, `key`, `options` from an integration test.
    // We trust the constructor sets them correctly. We can test behavior related to options later
    // (e.g., auto_reconnect behavior).
    let _client = RealtimeClient::new_with_options(
        "wss://realtime.supabase.io/socket",
        "anotherkey",
        options.clone(),
    );
    // Example: Assert behavior based on options if possible
    // assert_eq!(_client.some_public_method_reflecting_options(), expected_value);
}

#[tokio::test]
async fn test_set_auth() {
    setup_logger();
    // We cannot directly access `access_token` to verify.
    // This test requires either making `access_token` pub(crate) and running as a unit test,
    // making it fully public (not recommended), or testing behavior that depends on the token
    // (e.g., attempting a connection with a valid/invalid token).
    // For now, we will assume set_auth works internally but cannot verify state directly here.
    let client = RealtimeClient::new("ws://localhost:1234/socket", "apikey");
    let token = "some_jwt_token".to_string();
    client.set_auth(Some(token.clone())).await; // Call the public method
    client.set_auth(None).await; // Call the public method
                                 // No direct assertion possible here in integration test.
}

#[tokio::test]
async fn test_channel_builder_creation() {
    setup_logger();
    let client = RealtimeClient::new("ws://localhost:5000/socket", "apikey");
    let topic = "public:mytable";

    // Create a channel builder
    let builder = client.channel(topic);

    // Verify the builder is created (further assertions might depend on ChannelBuilder's API)
    // For now, just ensure the call doesn't panic and we get a builder.
    // We could potentially check the topic if ChannelBuilder exposes it.
    // assert_eq!(builder.topic, topic); // Example if topic field were public

    // Prevent unused variable warning
    let _ = builder;
}

// Helper function to start a simple mock WebSocket server
async fn start_mock_server(
) -> Result<(std::net::SocketAddr, tokio::task::JoinHandle<()>), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let handle = tokio::spawn(async move {
        println!("[Mock Server] Listening on {}", addr);
        match listener.accept().await {
            Ok((stream, _peer_addr)) => {
                println!("[Mock Server] Accepted connection from {}", _peer_addr);
                match tokio_tungstenite::accept_async(stream).await {
                    Ok(mut ws_stream) => {
                        println!("[Mock Server] WebSocket handshake successful");
                        // Simple loop to handle messages (e.g., heartbeat, join)
                        while let Some(msg_res) = ws_stream.next().await {
                            match msg_res {
                                Ok(msg) => {
                                    println!("[Mock Server] Received: {:?}", msg);
                                    // Revert to simple generic reply for any text message
                                    if msg.is_text() {
                                        let text_content = msg.to_text().unwrap_or("");
                                        // Try to parse to get the ref and topic if possible
                                        let mut reply_ref = json!(null); // Default ref
                                        let mut reply_topic = "phoenix".to_string(); // Default topic
                                        match serde_json::from_str::<RealtimeMessage>(text_content)
                                        {
                                            Ok(parsed) => {
                                                println!("[Mock Server] Parsed message ref: {:?} topic: {} from {}", parsed.message_ref, parsed.topic, _peer_addr);
                                                reply_ref = parsed.message_ref;
                                                reply_topic = parsed.topic; // Use original topic
                                            }
                                            Err(e) => {
                                                // Log parse error but still send a basic reply
                                                println!("[Mock Server] Failed to parse message for ref/topic from {}: {}. Raw: {}", _peer_addr, e, text_content);
                                            }
                                        }

                                        let reply = tokio_tungstenite::tungstenite::Message::Text(
                                            json!({
                                                "event": ChannelEvent::PhoenixReply,
                                                "payload": {"status": "ok", "response": {}},
                                                "ref": reply_ref, // Use parsed ref if available
                                                "topic": reply_topic // Use original topic if parsed
                                            })
                                            .to_string(),
                                        );
                                        println!("[Mock Server] Sending generic reply: {}", reply);
                                        if ws_stream.send(reply).await.is_err() {
                                            eprintln!("[Mock Server] Error sending reply");
                                            break;
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("[Mock Server] Error receiving message: {}", e);
                                    break;
                                }
                            }
                        }
                        println!("[Mock Server] Client disconnected or error");
                    }
                    Err(e) => {
                        eprintln!("[Mock Server] WebSocket handshake error: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[Mock Server] Failed to accept connection: {}", e);
            }
        }
        println!("[Mock Server] Task finished");
    });

    Ok((addr, handle))
}

#[tokio::test]
async fn test_connect_disconnect() {
    setup_logger();
    // Start the mock server
    let (server_addr, server_handle) = start_mock_server()
        .await
        .expect("Failed to start mock server");
    let server_url = format!("ws://{}", server_addr); // Use ws scheme

    println!("Test connecting to: {}", server_url);

    let client = RealtimeClient::new(&server_url, "mock_api_key");

    // Subscribe to state changes
    let mut state_rx = client.on_state_change();

    // Expect Connecting state
    let connect_future = client.connect();
    assert_eq!(
        state_rx.recv().await.unwrap(),
        supabase_rust_realtime::ConnectionState::Connecting
    );

    // Expect Connected state shortly after connect future resolves
    // We might need a small timeout or wait for the Connected state explicitly
    tokio::time::timeout(std::time::Duration::from_secs(2), connect_future)
        .await
        .expect("Connect future timed out")
        .expect("Client connect failed");

    assert_eq!(
        state_rx.recv().await.unwrap(),
        supabase_rust_realtime::ConnectionState::Connected
    );

    // Disconnect
    client.disconnect().await.expect("Client disconnect failed");

    // Expect Disconnected state
    // Note: The state might transition immediately upon calling disconnect, or after the tasks clean up.
    // Check current state directly or wait for broadcast.
    assert_eq!(
        client.get_connection_state().await,
        supabase_rust_realtime::ConnectionState::Disconnected
    );
    // Optionally, try receiving from broadcast channel with a timeout
    match tokio::time::timeout(std::time::Duration::from_millis(100), state_rx.recv()).await {
        Ok(Ok(state)) => assert_eq!(state, supabase_rust_realtime::ConnectionState::Disconnected),
        Ok(Err(_)) => println!("State broadcast channel closed as expected after disconnect."),
        Err(_) => println!(
            "Did not receive Disconnected state broadcast within timeout (might be expected)."
        ),
    }

    // Ensure the server task finishes (optional, helps cleanup)
    // server_handle.abort(); // Or let it finish naturally if the client disconnects cleanly
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), server_handle).await;
    println!("Connect/disconnect test finished.");
}

#[tokio::test]
async fn test_join_channel() {
    setup_logger();
    // Start the mock server
    let (server_addr, _server_handle) = start_mock_server()
        .await
        .expect("Failed to start mock server");
    let server_url = format!("ws://{}", server_addr);
    let client = RealtimeClient::new(&server_url, "mock_api_key");

    // Connect the client
    client.connect().await.expect("Client connect failed");
    // Allow time for connection state update
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(
        client.get_connection_state().await,
        supabase_rust_realtime::ConnectionState::Connected
    );

    // Create and subscribe to a channel
    let topic = "public:users".to_string();
    let channel_builder = client.channel(&topic);
    // Use subscribe() instead of join()
    let subscribe_result = channel_builder.subscribe().await;

    // Assert subscription was successful (mock server sends phx_reply with status ok)
    assert!(
        subscribe_result.is_ok(),
        "Channel subscription failed: {:?}",
        subscribe_result.err()
    );

    // Keep subscriptions in scope until end of test to prevent premature unsubscribe
    let _subscriptions = subscribe_result.unwrap();

    // Optionally, assert channel state if accessible - Requires changes to Channel/Client API
    // assert_eq!(channel.state.read().await, ChannelState::Joined);

    // Disconnect
    client.disconnect().await.expect("Client disconnect failed");
}

#[tokio::test]
async fn test_receive_message() {
    setup_logger();
    // Start the mock server - modification needed to send a message *after* join reply
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind failed");
    let addr = listener.local_addr().expect("local_addr failed");
    let server_url = format!("ws://{}", addr);
    let topic_to_join = "public:messages".to_string();
    let topic_clone = topic_to_join.clone(); // Clone for server closure

    let server_handle = tokio::spawn(async move {
        println!("[Mock Server Msg] Listening on {}", addr);
        if let Ok((stream, _)) = listener.accept().await {
            println!("[Mock Server Msg] Accepted connection");
            if let Ok(mut ws_stream) = tokio_tungstenite::accept_async(stream).await {
                println!("[Mock Server Msg] WebSocket handshake successful");
                let mut joined = false;
                while let Some(Ok(msg)) = ws_stream.next().await {
                    if msg.is_text() {
                        let text_msg = msg.to_text().unwrap();
                        println!("[Mock Server Msg] Received: {}", text_msg);
                        if let Ok(parsed) = serde_json::from_str::<RealtimeMessage>(text_msg) {
                            let reply_event;
                            let reply_payload;
                            let reply_ref = parsed.message_ref.clone();
                            let reply_topic = parsed.topic.clone();

                            match parsed.event {
                                ChannelEvent::PhoenixJoin if parsed.topic == topic_clone => {
                                    println!("[Mock Server Msg] Join received for {}", topic_clone);
                                    reply_event = ChannelEvent::PhoenixReply;
                                    reply_payload = json!({"status": "ok", "response": {}});
                                    joined = true; // Mark as joined
                                }
                                ChannelEvent::Heartbeat => {
                                    println!("[Mock Server Msg] Heartbeat received");
                                    reply_event = ChannelEvent::PhoenixReply;
                                    reply_payload = json!({"status": "ok", "response": {}});
                                }
                                _ => continue, // Ignore others
                            }

                            let reply = tokio_tungstenite::tungstenite::Message::Text(
                                json!({
                                    "event": reply_event,
                                    "payload": reply_payload,
                                    "ref": reply_ref,
                                    "topic": reply_topic
                                })
                                .to_string(),
                            );
                            println!("[Mock Server Msg] Sending reply: {}", reply);
                            if ws_stream.send(reply).await.is_err() {
                                break;
                            }

                            // If joined, send a mock message after a short delay
                            if joined {
                                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                                let mock_db_message = json!({
                                    "topic": topic_clone,
                                    "event": ChannelEvent::PostgresChanges, // Use correct enum variant if available, else string
                                    "payload": {
                                        "type": "INSERT",
                                        "schema": "public",
                                        "table": "messages",
                                        "commit_timestamp": "2024-01-01T00:00:00Z",
                                        "data": {"id": 1, "text": "Hello Realtime!"}
                                    },
                                    "ref": null // Broadcasts usually have null ref
                                });
                                let ws_msg = tokio_tungstenite::tungstenite::Message::Text(
                                    mock_db_message.to_string(),
                                );
                                println!("[Mock Server Msg] Sending mock DB message: {}", ws_msg);
                                if ws_stream.send(ws_msg).await.is_err() {
                                    eprintln!("[Mock Server Msg] Failed to send mock DB message");
                                    break;
                                }
                                joined = false; // Reset to prevent sending multiple times in this simple server
                            }
                        }
                    }
                }
            }
            println!("[Mock Server Msg] Client disconnected or error");
        }
        println!("[Mock Server Msg] Task finished");
    });

    // Client side
    let client = RealtimeClient::new(&server_url, "mock_api_key");
    client.connect().await.expect("Client connect failed");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await; // Allow connection

    // Setup listener *before* subscribing
    let (tx, mut rx) = mpsc::channel::<serde_json::Value>(1); // Channel to receive messages
    let channel_builder = client.channel(&topic_to_join);

    let configured_channel = channel_builder.on(
        DatabaseChanges::new(&topic_to_join).event(ChannelEvent::PostgresChanges),
        move |payload| {
            // Listen for specific event
            println!("[Test Listener] Received PostgresChanges: {:?}", payload);
            let tx_clone = tx.clone();
            // payload has type Payload { data: serde_json::Value, ... }, send payload.data
            let payload_data = payload.data.clone(); // Clone payload data for the async block
                                                     // Spawn the async task, don't return it from the closure
            tokio::spawn(async move {
                if let Err(e) = tx_clone.send(payload_data).await {
                    eprintln!(
                        "[Test Listener] Failed to send payload to test channel: {}",
                        e
                    );
                }
            }); // Removed .await here - channel.on is not async
        },
    );

    // Subscribe to the channel (this sends the phx_join message)
    let subscribe_result = configured_channel.subscribe().await;
    assert!(
        subscribe_result.is_ok(),
        "Channel subscription failed: {:?}",
        subscribe_result.err()
    );
    let _subscriptions = subscribe_result.unwrap(); // Keep subscriptions in scope

    println!("[Test] Channel subscribed, waiting for message...");

    // Wait for the mock message from the server
    match tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await {
        Ok(Some(payload_data)) => {
            println!(
                "[Test] Received message payload via listener: {:?}",
                payload_data
            );
            // Assert properties of the received payload data
            assert_eq!(payload_data["type"], "INSERT");
            assert_eq!(payload_data["table"], "messages");
            assert_eq!(payload_data["data"]["text"], "Hello Realtime!");
        }
        Ok(None) => panic!("Listener channel closed unexpectedly"),
        Err(_) => panic!("Timed out waiting for message from mock server"),
    }

    // Disconnect
    client.disconnect().await.expect("Client disconnect failed");
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), server_handle).await; // Allow server task to finish
    println!("[Test] Receive message test finished.");
}

// TODO: Add tests for channel creation
// TODO: Add tests for message handling (requires mock server or integration setup)
// TODO: Add tests for state changes
// TODO: Add tests for authentication (set_auth)
