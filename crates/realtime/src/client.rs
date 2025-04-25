use crate::channel::{Channel, ChannelBuilder}; // Added ChannelBuilder import
use crate::error::RealtimeError;
use crate::message::{RealtimeMessage, ChannelEvent}; // Added ChannelEvent import here
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::sleep;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use url::Url;
use log::{debug, error, info, trace, warn}; // Use log crate

/// 接続状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// RealtimeClient設定オプション
#[derive(Debug, Clone)]
pub struct RealtimeClientOptions {
    pub auto_reconnect: bool,
    pub max_reconnect_attempts: Option<u32>,
    pub reconnect_interval: u64,
    pub reconnect_backoff_factor: f64,
    pub max_reconnect_interval: u64,
    pub heartbeat_interval: u64,
}

impl Default for RealtimeClientOptions {
    fn default() -> Self {
        Self {
            auto_reconnect: true,
            max_reconnect_attempts: None, // Infinite attempts
            reconnect_interval: 1000,     // 1 second
            reconnect_backoff_factor: 1.5,
            max_reconnect_interval: 30000, // 30 seconds
            heartbeat_interval: 30000,     // 30 seconds
        }
    }
}

/// Realtimeクライアント本体
pub struct RealtimeClient {
    pub(crate) url: String,
    pub(crate) key: String,
    pub(crate) next_ref: AtomicU32,
    // Shared map of active channels (topic -> Channel)
    pub(crate) channels: Arc<RwLock<HashMap<String, Arc<Channel>>>>,
    // Shared sender for the WebSocket task
    pub(crate) socket: Arc<RwLock<Option<mpsc::Sender<Message>>>>,
    pub(crate) options: RealtimeClientOptions,
    state: Arc<RwLock<ConnectionState>>,
    reconnect_attempts: AtomicU32,
    // Wrap AtomicBool in Arc for sharing across tasks
    is_manually_closed: Arc<AtomicBool>,
    state_change: broadcast::Sender<ConnectionState>,
    // Make token field accessible within the crate
    pub(crate) access_token: Arc<RwLock<Option<String>>>,
}

impl RealtimeClient {
    /// デフォルトオプションで新しいクライアントを作成
    pub fn new(url: &str, key: &str) -> Self {
        Self::new_with_options(url, key, RealtimeClientOptions::default())
    }

    /// カスタムオプションで新しいクライアントを作成
    pub fn new_with_options(url: &str, key: &str, options: RealtimeClientOptions) -> Self {
        let (state_change_tx, _) = broadcast::channel(16); // Channel for state changes
        Self {
            url: url.to_string(),
            key: key.to_string(),
            next_ref: AtomicU32::new(1),
            channels: Arc::new(RwLock::new(HashMap::new())),
            socket: Arc::new(RwLock::new(None)),
            options,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            reconnect_attempts: AtomicU32::new(0),
            // Initialize the Arc<AtomicBool>
            is_manually_closed: Arc::new(AtomicBool::new(false)),
            state_change: state_change_tx,
            // Initialize token as None
            access_token: Arc::new(RwLock::new(None)),
        }
    }

    /// Method to set the authentication token
    pub async fn set_auth(&self, token: Option<String>) {
        info!("Setting auth token (is_some: {})", token.is_some());
        let mut current_token = self.access_token.write().await;
        *current_token = token;
        // TODO: Handle auth update while connected
    }

    /// 接続状態変更の通知を受け取るためのレシーバーを取得
    pub fn on_state_change(&self) -> broadcast::Receiver<ConnectionState> {
        self.state_change.subscribe()
    }

    /// 現在の接続状態を取得
    pub async fn get_connection_state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// 特定のトピックに対するチャンネルビルダーを作成
    pub fn channel(&self, topic: &str) -> ChannelBuilder {
        info!("Creating channel builder for topic: {}", topic);
        ChannelBuilder::new(self, topic)
    }

    /// 次のメッセージ参照番号を生成
    pub(crate) fn next_ref(&self) -> String {
        self.next_ref.fetch_add(1, Ordering::SeqCst).to_string()
    }

    /// 内部接続状態を設定し、変更を通知
    async fn set_connection_state(&self, state: ConnectionState) {
        let mut current_state = self.state.write().await;
        if *current_state != state {
            info!("Client state changing from {:?} to {:?}", *current_state, state);
            *current_state = state;
            // Ignore send error if no receivers are listening
            if let Err(e) = self.state_change.send(state) {
                warn!("Failed to broadcast state change {:?}: {}", state, e);
            }
        } else {
            trace!("Client state already {:?}, not changing.", state);
        }
    }

    /// WebSocket接続を開始および管理するタスク
    pub fn connect(
        &self,
    ) -> impl std::future::Future<Output = Result<(), RealtimeError>> + Send + 'static {
        info!("connect() called");
        // Clone necessary Arcs and fields for the async task
        let url = self.url.clone();
        let key = self.key.clone();
        let socket_arc = self.socket.clone();
        let state_arc = self.state.clone();
        let state_change_tx = self.state_change.clone();
        let _channels_arc = self.channels.clone();
        let options = self.options.clone();
        let is_manually_closed_arc = self.is_manually_closed.clone();
        let token_arc = self.access_token.clone(); // Clone token Arc

        async move {
            debug!("connect task started");
            is_manually_closed_arc.store(false, Ordering::SeqCst);
            debug!("Reset manual close flag");

            let token_guard = token_arc.read().await;
            let token_param = token_guard.as_ref().map(|t| format!("&token={}", t)).unwrap_or_default();
            debug!("Read token (present: {})", token_guard.is_some());
            drop(token_guard);

            let base_url = Url::parse(&url)?;
            debug!("Parsed base URL: {}", base_url);
            // Allow ws/wss schemes directly, map http/https
            match base_url.scheme() {
                 "http" | "ws" => { /* Ok, will use ws */ }
                 "https" | "wss" => { /* Ok, will use wss */ }
                 // Reject other schemes
                 s => return Err(RealtimeError::ConnectionError(format!("Unsupported URL scheme: {}", s))),
            };

            // Use the correct path /realtime/v1/websocket
            let _host = base_url.host_str().ok_or(RealtimeError::UrlParseError(url::ParseError::EmptyHost))?;
            let ws_url = format!(
                "{}?apikey={}{}",
                base_url.join("/realtime/v1/websocket?vsn=2.0.0") // Use join which preserves scheme/host/port
                    .map_err(RealtimeError::UrlParseError)?,
                key,
                token_param
            );

            info!("Attempting to connect to WebSocket: {}", ws_url);

            Self::set_connection_state_internal(
                state_arc.clone(),
                state_change_tx.clone(),
                ConnectionState::Connecting,
            )
            .await;

            let connect_result = connect_async(&ws_url).await; // Store result
            let ws_stream = match connect_result {
                Ok((stream, response)) => {
                    info!("WebSocket connection successful. Response: {:?}", response);
                    stream
                }
                Err(e) => {
                    error!("WebSocket connection failed: {}", e);
                    // Set state before returning error
                    Self::set_connection_state_internal(
                        state_arc.clone(),
                        state_change_tx.clone(),
                        ConnectionState::Disconnected,
                    )
                    .await;
                    return Err(RealtimeError::ConnectionError(format!("WebSocket connection failed: {}", e)));
                }
            };

            Self::set_connection_state_internal(
                state_arc.clone(),
                state_change_tx.clone(),
                ConnectionState::Connected,
            )
            .await;

            let (mut write, mut read) = ws_stream.split();
            debug!("WebSocket stream split into writer and reader");

            let (socket_tx, mut socket_rx) = mpsc::channel::<Message>(100);
            *socket_arc.write().await = Some(socket_tx.clone()); // Clone for writer task
            debug!("Internal MPSC channel created, sender stored");

            // --- WebSocket Writer Task ---
            let writer_socket_arc = socket_arc.clone();
            let writer_state_arc = state_arc.clone();
            let writer_state_change_tx = state_change_tx.clone();
            let writer_handle = tokio::spawn(async move {
                debug!("Writer task started");
                while let Some(message) = socket_rx.recv().await {
                    trace!("Writer task sending message: {:?}", message);
                    if let Err(e) = write.send(message).await {
                        error!("Writer task: WebSocket send error: {}. Closing connection.", e);
                        *writer_socket_arc.write().await = None;
                        Self::set_connection_state_internal(
                            writer_state_arc,
                            writer_state_change_tx,
                            ConnectionState::Disconnected,
                        )
                        .await;
                        socket_rx.close(); // Close the receiver side
                        break;
                    }
                }
                debug!("Writer task finished (sender dropped or error).");
            });

            // --- WebSocket Reader Task (and heartbeat/rejoin logic) ---
            let reader_socket_arc = socket_arc.clone();
            let reader_state_arc = state_arc.clone();
            let reader_state_change_tx = state_change_tx.clone();
            let heartbeat_interval = Duration::from_millis(options.heartbeat_interval);
            // Clone channels Arc for the reader task
            let reader_channels_arc = _channels_arc.clone();

            let reader_handle = tokio::spawn(async move {
                debug!("Reader task started");
                loop {
                    let socket_tx_ref = reader_socket_arc.read().await;
                    let current_socket_tx = if let Some(tx) = socket_tx_ref.as_ref() {
                        tx.clone()
                    } else {
                        warn!("Reader task: Socket sender gone, exiting.");
                        break;
                    };
                    drop(socket_tx_ref);

                    tokio::select! {
                        biased; // Prioritize reading incoming messages

                        // Read messages from WebSocket
                        msg_result = read.next() => {
                            match msg_result {
                                Some(Ok(msg)) => {
                                    trace!("Reader task received WS message: {:?}", msg);
                                    if let Message::Text(text) = &msg {
                                        match serde_json::from_str::<RealtimeMessage>(text) { // Parse as RealtimeMessage
                                            Ok(realtime_msg) => {
                                                debug!("Reader task parsed RealtimeMessage: topic='{}', event='{:?}', ref='{:?}'", 
                                                       realtime_msg.topic, realtime_msg.event, realtime_msg.message_ref);
                                                // Route based on topic
                                                let channels_guard = reader_channels_arc.read().await;
                                                if let Some(channel) = channels_guard.get(&realtime_msg.topic) {
                                                    let target_channel = channel.clone();
                                                    drop(channels_guard); // Release lock before await
                                                    trace!("Reader task dispatching message to channel {}", realtime_msg.topic);
                                                    // Spawn task for handling
                                                    tokio::spawn(async move {
                                                         // Pass the already parsed RealtimeMessage
                                                        target_channel.handle_message(realtime_msg).await;
                                                    });
                                                } else if realtime_msg.topic == "phoenix" {
                                                    // Handle special Phoenix replies if needed (e.g., join confirmation)
                                                    debug!("Reader task received Phoenix message: {:?}", realtime_msg);
                                                    // TODO: Potentially link replies back to join/leave calls via ref
                                                } else {
                                                    warn!("Reader task: Received message for unknown/unsubscribed topic: {}", realtime_msg.topic);
                                                }
                                            }
                                            Err(e) => {
                                                error!("Reader task: Failed to parse incoming text message as RealtimeMessage: {}. Raw: {}", e, text);
                                            }
                                        }
                                    } else if msg.is_close() {
                                         debug!("Reader task received Close frame: {:?}", msg);
                                         break; // Exit loop on Close frame
                                    } else {
                                        trace!("Reader task received non-text/non-close message: {:?}", msg);
                                    }
                                }
                                Some(Err(e)) => {
                                    error!("Reader task: WebSocket read error: {}", e);
                                    break; // Exit loop on read error
                                }
                                None => {
                                    debug!("Reader task: WebSocket stream closed by remote.");
                                    break; // Exit loop on stream close
                                }
                            }
                        }

                        // Send heartbeat periodically
                        _ = sleep(heartbeat_interval) => {
                             trace!("Reader task: Sending heartbeat");
                             // Revert to simple atomic ref for heartbeat within this task
                             let heartbeat_ref = AtomicU32::new(0).fetch_add(1, Ordering::SeqCst).to_string();
                             let heartbeat_msg = json!({
                                 "topic": "phoenix",
                                 "event": ChannelEvent::Heartbeat,
                                 "payload": {},
                                 "ref": heartbeat_ref
                             });
                             let ws_msg = Message::Text(heartbeat_msg.to_string());
                             if let Err(e) = current_socket_tx.send(ws_msg).await {
                                 error!("Reader task: Failed to send heartbeat: {}. Assuming connection lost.", e);
                                 break; // Exit loop if heartbeat send fails
                             }
                        }
                    }
                }
                debug!("Reader task finished loop.");
                // When reader exits, signal disconnect and clean up socket
                Self::set_connection_state_internal(reader_state_arc.clone(), reader_state_change_tx.clone(), ConnectionState::Disconnected).await;
                *reader_socket_arc.write().await = None;
            });

            // --- Wait for tasks or manual disconnect --- 
            // This part needs careful thought. Do we just return Ok(()) and let tasks run? 
            // Or wait for disconnect? Waiting here blocks the caller.
            // Let's return Ok(()) immediately and let the tasks run in the background.
            // The user can monitor state via on_state_change.
            debug!("connect task returning Ok, reader/writer tasks running in background");
            Ok(())

            // Example of waiting (might cause hangs if disconnect doesn't happen cleanly):
            // tokio::select! {
            //    _ = writer_handle => { warn!("Writer task exited unexpectedly."); },
            //    _ = reader_handle => { debug!("Reader task exited."); },
            // }
            // info!("Connect task finished after reader/writer tasks exited.");
            // Ok(())
        }
    }

    /// Helper for setting state (avoids async recursion issues)
    async fn set_connection_state_internal(
        state_arc: Arc<RwLock<ConnectionState>>,
        state_change_tx: broadcast::Sender<ConnectionState>,
        state: ConnectionState,
    ) {
        let mut current_state = state_arc.write().await;
        if *current_state != state {
             trace!("set_connection_state_internal changing from {:?} to {:?}", *current_state, state);
            *current_state = state;
            let _ = state_change_tx.send(state);
        }
    }

    /// 切断処理
    pub async fn disconnect(&self) -> Result<(), RealtimeError> {
        info!("disconnect() called");
        // Use the Arc<AtomicBool>
        self.is_manually_closed.store(true, Ordering::SeqCst);
        debug!("Set manual close flag");
        self.set_connection_state(ConnectionState::Disconnected)
            .await;

        // Close the socket sender channel
        let mut socket_guard = self.socket.write().await;
        if let Some(socket_tx) = socket_guard.take() {
            debug!("disconnect(): Dropping socket sender to signal tasks.");
            drop(socket_tx); // This signals the writer task to exit.
                             // Reader task should exit upon seeing sender drop or stream close.
            info!("WebSocket connection closed manually via disconnect().");
        } else {
            warn!("disconnect(): No active socket sender found, likely already disconnected.");
        }

        Ok(())
    }

    /// 再接続処理 (TODO: Implement backoff logic)
    #[allow(dead_code)]
    fn reconnect(&self) -> impl std::future::Future<Output = ()> + Send + 'static {
        let self_clone = self.clone(); // Clones the Arcs including is_manually_closed
        async move {
            let mut attempts = 0;
            let mut interval = self_clone.options.reconnect_interval;

            loop {
                // Use the cloned Arc<AtomicBool>
                if self_clone.is_manually_closed.load(Ordering::SeqCst) {
                    println!("Manual disconnect requested, stopping reconnect attempts.");
                    break;
                }

                if let Some(max_attempts) = self_clone.options.max_reconnect_attempts {
                    if attempts >= max_attempts {
                        println!("Max reconnect attempts ({}) reached.", max_attempts);
                        self_clone
                            .set_connection_state(ConnectionState::Disconnected)
                            .await;
                        break;
                    }
                }

                attempts += 1;
                self_clone
                    .reconnect_attempts
                    .store(attempts, Ordering::SeqCst);
                self_clone
                    .set_connection_state(ConnectionState::Reconnecting)
                    .await;
                println!("Attempting to reconnect... (Attempt #{})", attempts);

                sleep(Duration::from_millis(interval)).await;

                // Try connecting again
                // Need to call the connection logic, maybe refactor connect?
                match self_clone.connect().await {
                    Ok(_) => {
                        println!("Reconnection successful!");
                        self_clone.reconnect_attempts.store(0, Ordering::SeqCst); // Reset attempts
                                                                                  // TODO: Rejoin channels?
                        break; // Exit reconnect loop
                    }
                    Err(e) => {
                        eprintln!("Reconnect attempt #{} failed: {}", attempts, e);
                        // Increase interval with backoff
                        interval =
                            (interval as f64 * self_clone.options.reconnect_backoff_factor) as u64;
                        interval = interval.min(self_clone.options.max_reconnect_interval);
                    }
                }
            }
        }
    }

    /// Helper to send a raw JSON message through the WebSocket connection
    pub(crate) async fn send_message(&self, message: serde_json::Value) -> Result<(), RealtimeError> {
        trace!("Client attempting to send message: {}", message);
        let socket_guard = self.socket.read().await;
        if let Some(socket_tx) = socket_guard.as_ref() {
            let ws_message = Message::Text(message.to_string());
            socket_tx.send(ws_message).await.map_err(RealtimeError::from)
        } else {
            warn!("Cannot send message, client socket unavailable.");
            Err(RealtimeError::ConnectionError("Client socket unavailable".to_string()))
        }
    }
}

// Implement Clone manually to handle Arc fields correctly
impl Clone for RealtimeClient {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            key: self.key.clone(),
            next_ref: AtomicU32::new(self.next_ref.load(Ordering::SeqCst)), // Clone value
            channels: self.channels.clone(),
            socket: self.socket.clone(),
            options: self.options.clone(),
            state: self.state.clone(),
            reconnect_attempts: AtomicU32::new(self.reconnect_attempts.load(Ordering::SeqCst)), // Clone value
            // Clone the Arc<AtomicBool>
            is_manually_closed: self.is_manually_closed.clone(),
            state_change: self.state_change.clone(),
            // Clone the token Arc
            access_token: self.access_token.clone(),
        }
    }
}

// WebSocketメッセージ送信エラーからの変換
impl From<tokio::sync::mpsc::error::SendError<Message>> for RealtimeError {
    fn from(err: tokio::sync::mpsc::error::SendError<Message>) -> Self {
        RealtimeError::ConnectionError(format!("Failed to send message to socket task: {}", err))
    }
}
