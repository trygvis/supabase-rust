use crate::client::{ConnectionState, RealtimeClient}; // Assuming RealtimeClient will be in client.rs
use crate::error::RealtimeError;
use crate::filters::{DatabaseFilter, FilterOperator};
use crate::message::{ChannelEvent, Payload, PresenceChange};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
// use tokio::sync::mpsc; // Unused import after commenting out `socket` field
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;
use tokio::time::{timeout, Duration}; // Add timeout import

/// データベース変更監視設定
#[derive(Debug, Clone, Serialize)]
pub struct DatabaseChanges {
    schema: String,
    table: String,
    events: Vec<ChannelEvent>,
    filter: Option<Vec<DatabaseFilter>>,
}

impl DatabaseChanges {
    /// 新しいデータベース変更監視設定を作成
    pub fn new(table: &str) -> Self {
        Self {
            schema: "public".to_string(),
            table: table.to_string(),
            events: Vec::new(),
            filter: None,
        }
    }

    /// スキーマを設定
    pub fn schema(mut self, schema: &str) -> Self {
        self.schema = schema.to_string();
        self
    }

    /// イベントを追加
    pub fn event(mut self, event: ChannelEvent) -> Self {
        if !self.events.contains(&event) {
            self.events.push(event);
        }
        self
    }

    /// フィルター条件を追加
    pub fn filter(mut self, filter: DatabaseFilter) -> Self {
        self.filter.get_or_insert_with(Vec::new).push(filter);
        self
    }

    // --- Filter convenience methods ---

    pub fn eq<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::Eq,
            value: value.into(),
        })
    }

    pub fn neq<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::Neq,
            value: value.into(),
        })
    }

    pub fn gt<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::Gt,
            value: value.into(),
        })
    }

    pub fn gte<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::Gte,
            value: value.into(),
        })
    }

    pub fn lt<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::Lt,
            value: value.into(),
        })
    }

    pub fn lte<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::Lte,
            value: value.into(),
        })
    }

    pub fn in_values<T: Into<serde_json::Value>>(self, column: &str, values: Vec<T>) -> Self {
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::In,
            value: values
                .into_iter()
                .map(|v| v.into())
                .collect::<Vec<_>>()
                .into(),
        })
    }

    // Add other filter methods (like, ilike, contains) if needed

    // --- Internal methods ---

    /// Convert config to JSON for the websocket message
    pub(crate) fn to_channel_config(&self) -> serde_json::Value {
        let events_str: Vec<String> = self.events.iter().map(|e| e.to_string()).collect();

        let mut config = json!({
            "schema": self.schema,
            "table": self.table,
            "events": events_str
        });

        if let Some(filters) = &self.filter {
            let filters_json: Vec<serde_json::Value> = filters
                .iter()
                .map(|f| {
                    json!({
                        "column": f.column,
                        "filter": f.operator.to_string(),
                        "value": f.value
                    })
                })
                .collect();
            // Assuming Realtime expects filters like: `column=eq.value` in URL query format
            // This needs clarification based on actual protocol.
            // For now, adding as a structured object, might need adjustment.
            config["filter"] = json!(filters_json);
        }

        json!({
            "type": "postgres_changes",
            "payload": {
                "config": config
            }
        })
    }
}

/// ブロードキャストイベント監視設定
#[derive(Debug, Clone, Serialize)]
pub struct BroadcastChanges {
    event: String, // Specific event name to listen for
}

impl BroadcastChanges {
    pub fn new(event: &str) -> Self {
        Self {
            event: event.to_string(),
        }
    }

    #[allow(dead_code)] // Mark as allowed since it might be useful later
    pub(crate) fn get_event_name(&self) -> &str {
        &self.event
    }
}

/// プレゼンスイベント監視設定 (シンプルなマーカー型)
#[derive(Debug, Clone, Default, Serialize)]
pub struct PresenceChanges;

impl PresenceChanges {
    pub fn new() -> Self {
        // Self::default() // Clippy: default_constructed_unit_structs
        PresenceChanges // Create directly
    }
}

/// アクティブなチャンネル購読を表す
pub struct Subscription {
    id: String, // Internal subscription identifier
    channel: Arc<Channel>,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let id_clone = self.id.clone();
        let channel_clone = self.channel.clone();
        tokio::spawn(async move {
            if let Err(e) = channel_clone.unsubscribe(&id_clone).await {
                // TODO: Log unsubscribe error properly
                eprintln!("Error unsubscribing from channel: {}", e);
            }
        });
    }
}

type CallbackFn = Box<dyn Fn(Payload) + Send + Sync>;
type PresenceCallbackFn = Box<dyn Fn(PresenceChange) + Send + Sync>;

/// 内部チャンネル表現
pub(crate) struct Channel {
    topic: String,
    // Use Weak reference to client to avoid cycles if Channel holds client ref?
    // For now, assume socket sender is passed down.
    // socket: Arc<RwLock<Option<mpsc::Sender<Message>>>>, // Clippy: dead_code - Seems unused within Channel methods
    callbacks: Arc<RwLock<HashMap<String, CallbackFn>>>,
    presence_callbacks: Arc<RwLock<Vec<PresenceCallbackFn>>>,
    // Add presence state if managed per-channel
    // presence_state: Arc<RwLock<PresenceState>>,
}

impl Channel {
    async fn unsubscribe(&self, id: &str) -> Result<(), RealtimeError> {
        // Remove callback
        self.callbacks.write().await.remove(id);
        // TODO: Unsubscribe presence if needed

        // Send unsubscribe message if this was the last callback? Requires tracking.
        // For simplicity, assume client handles full channel leave when all subscriptions drop.
        println!(
            "Subscription {} dropped. Channel {} might need explicit leave.",
            id, self.topic
        );
        Ok(())
    }

    // New method to handle incoming messages for this channel
    pub(crate) async fn handle_message(&self, message: serde_json::Value) {
        // Parse as generic Value first to access top-level fields like 'event'
        match serde_json::from_value::<serde_json::Value>(message.clone()) {
            Ok(json_msg) => {
                let event_field = json_msg.get("event").and_then(|v| v.as_str());
                let topic = json_msg.get("topic").and_then(|v| v.as_str()); // May not always be present
                let payload_data = json_msg.get("payload"); // This is the nested payload
                // 'type' field is within the nested payload for db changes, but top-level for others?
                // Let's extract it from payload_data if possible, or top level otherwise.
                let event_type = payload_data.and_then(|p| p.get("type")).and_then(|v| v.as_str())
                                 .or_else(|| json_msg.get("type").and_then(|v| v.as_str()));
                let timestamp = json_msg.get("timestamp").and_then(|v| v.as_str());

                println!(
                    "Handling message for topic '{}', event '{:?}'",
                    topic.unwrap_or(&self.topic),
                    event_field
                );

                // Route based on the 'event' field
                match event_field {
                    // --- Database Changes --- uses event "postgres_changes"
                    Some("postgres_changes") => {
                        // Construct the Payload struct expected by the callback
                        let payload_for_callback = Payload {
                            data: payload_data.cloned().unwrap_or(serde_json::Value::Null),
                            event_type: event_type.map(String::from),
                            timestamp: timestamp.map(String::from),
                        };
                        let callbacks = self.callbacks.read().await;
                        println!(
                            "Calling {} DB change callbacks for topic '{}'",
                            callbacks.len(),
                            self.topic
                        );
                        for (id, callback) in callbacks.iter() {
                            // TODO: Filter callback based on original subscription criteria
                            println!("  -> Calling DB callback ID: {}", id);
                            (callback)(payload_for_callback.clone());
                        }
                    }

                    // --- Broadcast --- uses event "broadcast"
                    Some("broadcast") => {
                        let payload_for_callback = Payload {
                            data: payload_data.cloned().unwrap_or(serde_json::Value::Null),
                            event_type: event_type.map(String::from),
                            timestamp: timestamp.map(String::from),
                        };
                        let callbacks = self.callbacks.read().await;
                        println!(
                            "Calling {} broadcast callbacks for topic '{}'",
                            callbacks.len(),
                            self.topic
                        );
                        for (id, callback) in callbacks.iter() {
                             // TODO: Filter based on the original BroadcastChanges config
                            println!("  -> Calling Broadcast callback ID: {}", id);
                            (callback)(payload_for_callback.clone());
                        }
                    }

                    // --- Presence --- uses events "presence_diff" or "presence_state"
                    Some("presence_diff") | Some("presence_state") => {
                        if let Some(data) = payload_data {
                            match serde_json::from_value::<PresenceChange>(data.clone()) {
                                Ok(presence_change) => {
                                    let presence_callbacks = self.presence_callbacks.read().await;
                                    println!(
                                        "Calling {} presence callbacks for topic '{}'",
                                        presence_callbacks.len(),
                                        self.topic
                                    );
                                    for callback in presence_callbacks.iter() {
                                        (callback)(presence_change.clone());
                                    }
                                }
                                Err(e) => {
                                    eprintln!(
                                        "Failed to parse presence payload for topic '{}': {}. Payload: {:?}",
                                        self.topic,
                                        e,
                                        data // Log the nested payload data
                                    );
                                }
                            }
                        } else {
                            eprintln!(
                                "Presence event '{:?}' received without payload data for topic '{}'",
                                event_field,
                                self.topic
                            );
                        }
                    }

                    // --- Other Phoenix Events ---
                    Some("phx_close") => {
                        println!("Channel '{}' received phx_close", self.topic);
                        // TODO: Maybe clear callbacks or notify client?
                    }
                    Some("phx_error") => {
                        eprintln!(
                            "Channel '{}' received phx_error: {:?}",
                            self.topic,
                            payload_data // Log the nested payload
                        );
                        // TODO: Maybe trigger reconnect or notify user?
                    }

                    // --- Unknown Event ---
                    Some(unknown_event) => {
                        println!(
                            "Unhandled event '{}' on channel '{}': {:?}",
                            unknown_event,
                            self.topic,
                            json_msg // Log the whole message
                        );
                    }
                    None => {
                        println!("Received message without event field: {:?}", json_msg);
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "Failed to parse incoming message for topic '{}' into JSON Value: {}. Message: {:?}",
                    self.topic,
                    e,
                    message // Log original potentially non-JSON value
                );
            }
        }
    }
}

/// チャンネル作成と購読設定のためのビルダー
pub struct ChannelBuilder<'a> {
    client: &'a RealtimeClient,
    topic: String,
    db_callbacks: HashMap<String, (DatabaseChanges, CallbackFn)>,
    broadcast_callbacks: HashMap<String, (BroadcastChanges, CallbackFn)>,
    presence_callbacks: Vec<PresenceCallbackFn>,
}

impl<'a> ChannelBuilder<'a> {
    pub(crate) fn new(client: &'a RealtimeClient, topic: &str) -> Self {
        Self {
            client,
            topic: topic.to_string(),
            db_callbacks: HashMap::new(),
            broadcast_callbacks: HashMap::new(),
            presence_callbacks: Vec::new(),
        }
    }

    /// データベース変更イベントのコールバックを登録
    pub fn on<F>(mut self, changes: DatabaseChanges, callback: F) -> Self
    where
        F: Fn(Payload) + Send + Sync + 'static,
    {
        // Use a unique identifier for the subscription
        let id = uuid::Uuid::new_v4().to_string();
        self.db_callbacks.insert(id, (changes, Box::new(callback)));
        self
    }

    /// ブロードキャストイベントのコールバックを登録
    pub fn on_broadcast<F>(mut self, changes: BroadcastChanges, callback: F) -> Self
    where
        F: Fn(Payload) + Send + Sync + 'static,
    {
        let id = uuid::Uuid::new_v4().to_string();
        self.broadcast_callbacks
            .insert(id, (changes, Box::new(callback)));
        self
    }

    /// プレゼンス変更イベントのコールバックを登録
    pub fn on_presence<F>(mut self, callback: F) -> Self
    where
        F: Fn(PresenceChange) + Send + Sync + 'static,
    {
        self.presence_callbacks.push(Box::new(callback));
        self
    }

    /// チャンネルへの接続と購読を開始
    pub async fn subscribe(self) -> Result<Vec<Subscription>, RealtimeError> {
        // --- START: Improved Connection Handling ---
        let mut rx = self.client.on_state_change(); // Get state change receiver

        // Check current state first
        let initial_state = self.client.get_connection_state().await;
        if initial_state != ConnectionState::Connected {
            println!(
                "Client not connected (state: {:?}). Ensuring connection attempt...",
                initial_state
            );
            // Initiate connection if not already connecting/connected
            // Spawning it prevents blocking the subscribe call.
            let connect_future = self.client.connect();
             // TODO: Add handling for immediate connect errors. For now, spawn and wait for state.
             tokio::spawn(async move {
                 if let Err(e) = connect_future.await {
                     eprintln!("Background connect task failed: {}", e);
                 }
             });

            println!("Waiting for client to connect...");
            // Wait for the Connected state notification with a timeout, skipping Connecting state
            let wait_result = timeout(Duration::from_secs(10), async {
                loop {
                    match rx.recv().await {
                        Ok(ConnectionState::Connected) => break Ok(()), // Success
                        Ok(ConnectionState::Connecting) => continue, // Ignore Connecting, wait further
                        Ok(other_state) => break Err(RealtimeError::ConnectionError(format!(
                            "Connection attempt resulted in unexpected state: {:?}",
                            other_state
                        ))),
                        Err(_) => break Err(RealtimeError::ConnectionError(
                            "State change receiver error while waiting for connection.".to_string(),
                        )),
                    }
                }
            }).await;

            match wait_result {
                 Ok(Ok(_)) => {
                    println!("Client connected successfully.");
                 }
                 Ok(Err(e)) => {
                    // Inner error (unexpected state or recv error)
                    return Err(e);
                 }
                 Err(_) => {
                    // Timeout occurred
                    let current_state = self.client.get_connection_state().await;
                     return Err(RealtimeError::ConnectionError(format!(
                        "Timeout waiting for connection. Current state: {:?}", current_state
                    )));
                 }
            }
        }
        // --- END: Improved Connection Handling ---

        // 2. Get or create the channel representation (Existing code)
        let mut channels = self.client.channels.write().await;
        let channel = channels
            .entry(self.topic.clone())
            .or_insert_with(|| {
                Arc::new(Channel {
                    topic: self.topic.clone(),
                    callbacks: Arc::new(RwLock::new(HashMap::new())),
                    presence_callbacks: Arc::new(RwLock::new(Vec::new())),
                })
            })
            .clone();

        // 3. Register callbacks and prepare JOIN message payload (Existing code)
        let mut payload_config = json!({});
        let mut subscriptions = Vec::new();

        for (id, (changes, callback)) in self.db_callbacks {
            channel.callbacks.write().await.insert(id.clone(), callback);
            payload_config["postgres_changes"] =
                serde_json::Value::Array(vec![changes.to_channel_config()]);
            subscriptions.push(Subscription {
                id,
                channel: channel.clone(),
            });
        }

        for (id, (changes, callback)) in self.broadcast_callbacks {
            channel.callbacks.write().await.insert(id.clone(), callback);
            payload_config["broadcast"] = json!({ "event": changes.get_event_name() });
            subscriptions.push(Subscription {
                id,
                channel: channel.clone(),
            });
        }

         if !self.presence_callbacks.is_empty() {
            let mut presence_cbs = channel.presence_callbacks.write().await;
            for cb in self.presence_callbacks {
                presence_cbs.push(cb);
            }
            payload_config["presence"] = json!({ "key": "" });
        }


        // 4. Send JOIN message via the client's socket (Existing code)
        let socket_guard = self.client.socket.read().await;
        if let Some(socket_tx) = socket_guard.as_ref() {
            let join_ref = self.client.next_ref();
            let message = json!({
                "topic": self.topic,
                "event": "phx_join",
                "payload": payload_config,
                "ref": join_ref,
            });
            let ws_message = Message::Text(message.to_string());

            socket_tx
                .send(ws_message)
                .await
                .map_err(|e| RealtimeError::ConnectionError(format!("Failed to send JOIN message: {}", e)))?;

            Ok(subscriptions)
        } else {
            Err(RealtimeError::ConnectionError(
                "Connection established but socket sender not found.".to_string(),
            ))
        }
    }

    // Method to track presence - might belong on RealtimeClient or Channel directly?
    pub async fn track_presence(
        &self,
        _user_id: &str,
        _user_data: serde_json::Value,
    ) -> Result<(), RealtimeError> {
        // TODO: Implement sending presence track message
        Err(RealtimeError::ChannelError(
            "track_presence not implemented".to_string(),
        ))
    }
}
