use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Once;
use supabase_rust_realtime::{
    ChannelEvent, DatabaseChanges, RealtimeClient, RealtimeClientOptions, RealtimeMessage
};
use tokio::sync::mpsc;
// Add tracing imports
use std::collections::VecDeque; // For storing received messages
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::Mutex; // For thread-safe access to received messages
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use tracing_subscriber::{fmt, EnvFilter};

// Ensure logger is initialized only once across all tests
static INIT_LOGGER: Once = Once::new();

fn setup_logger() {
    INIT_LOGGER.call_once(|| {
        // Initialize tracing subscriber
        fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_test_writer()
            .init();
        // pretty_env_logger::init(); // Keep or remove based on preference, tracing is now primary
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
#[instrument]
async fn start_mock_server() -> Result<
    (
        std::net::SocketAddr,
        tokio::task::JoinHandle<()>,
        Arc<Mutex<VecDeque<RealtimeMessage>>>,
    ),
    Box<dyn std::error::Error>,
> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    info!(address = %addr, "Mock server binding successful");

    // Store received messages for assertion
    let received_messages = Arc::new(Mutex::new(VecDeque::new()));
    let received_messages_clone = received_messages.clone();

    let handle = tokio::spawn(async move {
        let server_span = span!(Level::INFO, "mock_server", %addr);
        let _enter = server_span.enter(); // Enter the span

        info!("Listening for connections");
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                info!(%peer_addr, "Accepted connection");
                let connection_span = span!(Level::INFO, "connection", %peer_addr);
                let _conn_enter = connection_span.enter();

                // Extract token from URL for auth test
                let _token = {
                    let token_val: Option<String> = None;
                    if let Ok(_req) =
                        tokio_tungstenite::connect_async(format!("ws://{}", addr)).await
                    {
                        // This is a bit hacky, ideally the request URI is accessible
                        // during accept_async, but it's not directly available.
                        // We simulate getting the request URI here.
                        // In a real scenario, use a proper HTTP server framework (e.g., warp, axum)
                        // that provides access to request details during handshake.
                        // For now, we assume the test passes the token correctly.
                        // We can't easily verify the *exact* token here without more complex setup.
                        // Let's focus on verifying JOIN messages for now.
                    }
                    token_val
                };
                // if let Some(t) = token { info!(token = %t, "Client connected with token"); }

                match tokio_tungstenite::accept_async(stream).await {
                    Ok(mut ws_stream) => {
                        info!("WebSocket handshake successful");
                        // Simple loop to handle messages (e.g., heartbeat, join)
                        while let Some(msg_res) = ws_stream.next().await {
                            match msg_res {
                                Ok(msg) => {
                                    trace!(message = ?msg, "Received message");

                                    if msg.is_text() {
                                        let text_content = msg.to_text().unwrap_or("");
                                        trace!(content = %text_content, "Received text message");

                                        match serde_json::from_str::<RealtimeMessage>(text_content)
                                        {
                                            Ok(parsed) => {
                                                debug!(?parsed, "Parsed message from client");

                                                // Store received message for test assertions
                                                received_messages_clone
                                                    .lock()
                                                    .await
                                                    .push_back(parsed.clone());

                                                let reply_ref = parsed.message_ref.clone();
                                                let reply_topic = parsed.topic.clone();
                                                let reply_event = ChannelEvent::PhoenixReply;
                                                let reply_payload =
                                                    json!({ "status": "ok", "response": {} });

                                                // Specific handling for join
                                                if parsed.event == ChannelEvent::PhoenixJoin {
                                                    info!(topic = %parsed.topic, "Received Join request");
                                                    // reply_payload = json!({ "status": "ok", "response": {"some_join_info":"value"} });
                                                }

                                                let reply = Message::Text(
                                                    json!({
                                                        "event": reply_event,
                                                        "payload": reply_payload,
                                                        "ref": reply_ref,
                                                        "topic": reply_topic
                                                    })
                                                    .to_string(),
                                                );

                                                debug!(reply = %reply.to_text().unwrap_or("[non-text]"), "Sending reply");
                                                if let Err(e) = ws_stream.send(reply).await {
                                                    error!(error = %e, "Error sending reply");
                                                    break;
                                                }
                                            }
                                            Err(e) => {
                                                warn!(error = %e, raw_message = %text_content, "Failed to parse message, ignoring");
                                            }
                                        }
                                    } else if msg.is_ping() {
                                        trace!("Received Ping, sending Pong");
                                        if let Err(e) =
                                            ws_stream.send(Message::Pong(msg.into_data())).await
                                        {
                                            error!(error = %e, "Error sending Pong");
                                            break;
                                        }
                                    } else if msg.is_close() {
                                        info!("Received Close frame, closing connection");
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!(error = %e, "Error receiving message");
                                    break;
                                }
                            }
                        }
                        info!("Client disconnected or error, ending connection loop");
                    }
                    Err(e) => {
                        error!(error = %e, "WebSocket handshake error");
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to accept connection");
            }
        }
        info!("Mock server task finished");
    });

    Ok((addr, handle, received_messages))
}

#[tokio::test]
#[instrument]
async fn test_connect_disconnect() {
    setup_logger();
    // Start the mock server
    let (server_addr, server_handle, _) = start_mock_server() // Ignore received_messages queue
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
    info!("Waiting for connect() future to complete...");
    tokio::time::timeout(std::time::Duration::from_secs(5), connect_future) // Increased timeout slightly
        .await
        .expect("Connect future timed out")
        .expect("Client connect failed");
    info!("connect() future completed");

    // Increased timeout for receiving Connected state
    info!("Waiting for Connected state...");
    match tokio::time::timeout(Duration::from_secs(5), state_rx.recv()).await {
        Ok(Ok(state)) => {
            info!(?state, "Received state change");
            assert_eq!(state, supabase_rust_realtime::ConnectionState::Connected);
        }
        Ok(Err(RecvError::Closed)) => panic!("State change channel closed unexpectedly"),
        Ok(Err(RecvError::Lagged(_))) => panic!("State change receiver lagged behind"),
        Err(_) => panic!("Timed out waiting for Connected state"),
    }

    // Disconnect
    info!("Calling disconnect()...");
    client.disconnect().await.expect("Client disconnect failed");
    info!("disconnect() call completed");

    // Expect Disconnected state
    // Wait briefly for state propagation
    tokio::time::sleep(Duration::from_millis(100)).await;
    info!("Checking final connection state...");
    assert_eq!(
        client.get_connection_state().await,
        supabase_rust_realtime::ConnectionState::Disconnected
    );
    info!("test_connect_disconnect finished successfully");

    // Ensure the server task finishes (optional, helps cleanup)
    // server_handle.abort(); // Or let it finish naturally if the client disconnects cleanly
    let _ = tokio::time::timeout(Duration::from_secs(1), server_handle).await;
    println!("Connect/disconnect test finished.");
}

#[tokio::test]
#[instrument]
async fn test_set_auth_connect() {
    setup_logger();
    let (server_addr, server_handle, _) = start_mock_server()
        .await
        .expect("Failed to start mock server");
    let server_url = format!("ws://{}", server_addr);
    let test_token = "test-jwt-token-123".to_string();

    let client = RealtimeClient::new(&server_url, "mock_api_key");

    // Set auth *before* connecting
    client.set_auth(Some(test_token.clone())).await;
    // info!(token = ?client.access_token.read().await, "Auth token set"); // Remove access to private field

    // Connect and verify state
    let mut state_rx = client.on_state_change();
    let connect_future = client.connect();
    assert_eq!(
        state_rx.recv().await.unwrap(),
        supabase_rust_realtime::ConnectionState::Connecting
    );
    tokio::time::timeout(Duration::from_secs(5), connect_future)
        .await
        .expect("Connect future timed out")
        .expect("Client connect failed");
    match tokio::time::timeout(Duration::from_secs(5), state_rx.recv()).await {
        Ok(Ok(state)) => assert_eq!(state, supabase_rust_realtime::ConnectionState::Connected),
        Ok(Err(RecvError::Closed)) => panic!("State channel closed unexpectedly during connect"),
        Ok(Err(RecvError::Lagged(_))) => panic!("State receiver lagged during connect"),
        Err(_) => panic!("Timed out waiting for Connected state after auth set"),
    }

    // TODO: Enhance mock server to capture the connection URL with the token
    // and assert it here. Currently, we only assert connection success.
    info!("Connected successfully after setting auth token.");

    client.disconnect().await.expect("Disconnect failed");
    let _ = tokio::time::timeout(Duration::from_secs(1), server_handle).await;
    println!("Set auth connect test finished.");
}

#[tokio::test]
#[instrument]
async fn test_join_channel_success() {
    setup_logger();
    let (server_addr, server_handle, received_messages) = start_mock_server()
        .await
        .expect("Failed to start mock server");
    let server_url = format!("ws://{}", server_addr);
    let client = RealtimeClient::new(&server_url, "mock_api_key");
    let topic = "public:test_join".to_string();

    // Connect the client
    client.connect().await.expect("Client connect failed");
    tokio::time::sleep(Duration::from_millis(100)).await; // Allow connection
    assert_eq!(
        client.get_connection_state().await,
        supabase_rust_realtime::ConnectionState::Connected
    );

    // Subscribe to the channel
    let channel_builder = client.channel(&topic);
    let subscribe_result = channel_builder.subscribe().await;

    assert!(
        subscribe_result.is_ok(),
        "Channel subscription failed: {:?}",
        subscribe_result.err()
    );
    let _subscriptions = subscribe_result.unwrap(); // Keep subscriptions in scope

    // Allow time for join message to be sent and reply received
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Assert that the mock server received the join message
    let received = received_messages.lock().await;
    let join_msg = received
        .iter()
        .find(|msg| msg.topic == topic && msg.event == ChannelEvent::PhoenixJoin);
    assert!(
        join_msg.is_some(),
        "Mock server did not receive join message for topic {}",
        topic
    );
    info!(message = ?join_msg.unwrap(), "Mock server received join message");
    drop(received); // Release lock

    // Assert client channel state (requires access or testing behavior)
    // This part is tricky as channel state is internal.
    // We rely on subscribe() returning Ok for now.
    // Ideally, Channel would expose state or a way to wait for Joined.
    /* Remove access to private field
    let channels = client.channels.read().await;
    let channel_arc = channels
        .get(&topic)
        .expect("Channel not found in client map");
    */
    // Direct state access removed, rely on subscribe success and received msg assertion
    // assert_eq!(*channel_arc.state.read().await, ChannelState::Joined);
    info!("Channel join test successful (join message sent and received by mock)");

    client.disconnect().await.expect("Disconnect failed");
    let _ = tokio::time::timeout(Duration::from_secs(1), server_handle).await;
    println!("Join channel success test finished.");
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
