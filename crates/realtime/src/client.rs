use crate::channel::Channel; // Assuming Channel is in channel.rs
use crate::error::RealtimeError;
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
        }
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
    pub fn channel(&self, topic: &str) -> crate::channel::ChannelBuilder {
        crate::channel::ChannelBuilder::new(self, topic)
    }

    /// 次のメッセージ参照番号を生成
    pub(crate) fn next_ref(&self) -> String {
        self.next_ref.fetch_add(1, Ordering::SeqCst).to_string()
    }

    /// 内部接続状態を設定し、変更を通知
    async fn set_connection_state(&self, state: ConnectionState) {
        let mut current_state = self.state.write().await;
        if *current_state != state {
            *current_state = state;
            // Ignore send error if no receivers are listening
            let _ = self.state_change.send(state);
        }
    }

    /// WebSocket接続を開始および管理するタスク
    pub fn connect(
        &self,
    ) -> impl std::future::Future<Output = Result<(), RealtimeError>> + Send + 'static {
        // Clone necessary Arcs and fields for the async task
        let url = self.url.clone();
        let key = self.key.clone();
        let socket_arc = self.socket.clone();
        let state_arc = self.state.clone();
        let state_change_tx = self.state_change.clone();
        let _channels_arc = self.channels.clone();
        let options = self.options.clone();
        let is_manually_closed_arc = self.is_manually_closed.clone();

        async move {
            // Reset manual close flag using the cloned Arc
            is_manually_closed_arc.store(false, Ordering::SeqCst);

            // Construct the WebSocket URL carefully
            let base_url = Url::parse(&url)?;
            let ws_scheme = match base_url.scheme() {
                 "http" => "ws",
                 "https" => "wss",
                 // Use ConnectionError for unsupported schemes
                 s => return Err(RealtimeError::ConnectionError(format!("Unsupported URL scheme: {}", s))),
            };

            // Use the correct path /realtime/v1/websocket
            let host = base_url.host_str().ok_or(RealtimeError::UrlParseError(url::ParseError::EmptyHost))?;
            let ws_url_str = if let Some(port) = base_url.port() {
                format!(
                    "{}://{}:{}/realtime/v1/websocket?apikey={}&vsn=1.0.0",
                    ws_scheme, host, port, key
                )
            } else {
                format!(
                    "{}://{}/realtime/v1/websocket?apikey={}&vsn=1.0.0",
                    ws_scheme, host, key
                )
            };

            let ws_url = Url::parse(&ws_url_str)?;

            Self::set_connection_state_internal(
                state_arc.clone(),
                state_change_tx.clone(),
                ConnectionState::Connecting,
            )
            .await;

            let (ws_stream, _) = connect_async(ws_url).await.map_err(|e| {
                RealtimeError::ConnectionError(format!("WebSocket connection failed: {}", e))
            })?;

            Self::set_connection_state_internal(
                state_arc.clone(),
                state_change_tx.clone(),
                ConnectionState::Connected,
            )
            .await;

            let (mut write, mut read) = ws_stream.split();

            // Create an MPSC channel for sending messages to the WebSocket writer task
            let (socket_tx, mut socket_rx) = mpsc::channel::<Message>(100);

            // Store the sender half in the shared state
            *socket_arc.write().await = Some(socket_tx);

            // --- WebSocket Writer Task ---
            let writer_socket_arc = socket_arc.clone();
            let writer_state_arc = state_arc.clone();
            let writer_state_change_tx = state_change_tx.clone();
            tokio::spawn(async move {
                while let Some(message) = socket_rx.recv().await {
                    if let Err(e) = write.send(message).await {
                        eprintln!("WebSocket send error: {}. Closing connection.", e);
                        *writer_socket_arc.write().await = None; // Clear sender on error
                        Self::set_connection_state_internal(
                            writer_state_arc,
                            writer_state_change_tx,
                            ConnectionState::Disconnected,
                        )
                        .await;
                        socket_rx.close();
                        break;
                    }
                }
                // Writer task normally ends when socket_rx channel is closed
                println!("WebSocket writer task finished.");
            });

            // --- WebSocket Reader Task (and heartbeat/rejoin logic) ---
            let reader_socket_arc = socket_arc.clone();
            let reader_state_arc = state_arc.clone();
            let reader_state_change_tx = state_change_tx.clone();
            let heartbeat_interval = Duration::from_millis(options.heartbeat_interval);

            loop {
                let socket_tx_ref = reader_socket_arc.read().await;
                let current_socket_tx = if let Some(tx) = socket_tx_ref.as_ref() {
                    tx.clone()
                } else {
                    // Socket was closed (likely by writer task error or disconnect)
                    println!("Socket sender gone, exiting reader task.");
                    break;
                };
                drop(socket_tx_ref); // Release read lock

                tokio::select! {
                    // Read messages from WebSocket
                    msg_result = read.next() => {
                        match msg_result {
                            Some(Ok(msg)) => {
                                // TODO: Process incoming message (phx_reply, events, presence)
                                println!("Received WS message: {:?}", msg);
                                // Example: Handle heartbeat replies
                                if let Message::Text(text) = &msg {
                                    if let Ok(json_msg) = serde_json::from_str::<serde_json::Value>(text) {
                                        if json_msg["event"].as_str() == Some("phx_reply") && json_msg["payload"]["status"].as_str() == Some("ok") {
                                            // Likely heartbeat response, do nothing specific for now
                                        } else {
                                             // TODO: Route other messages to relevant channel callbacks
                                        }
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                eprintln!("WebSocket read error: {}", e);
                                Self::set_connection_state_internal(reader_state_arc.clone(), reader_state_change_tx.clone(), ConnectionState::Disconnected).await;
                                *reader_socket_arc.write().await = None;
                                break; // Exit loop on read error
                            }
                            None => {
                                println!("WebSocket stream closed by remote.");
                                Self::set_connection_state_internal(reader_state_arc.clone(), reader_state_change_tx.clone(), ConnectionState::Disconnected).await;
                                *reader_socket_arc.write().await = None;
                                break; // Exit loop on stream close
                            }
                        }
                    }
                    // Send heartbeat periodically
                    _ = sleep(heartbeat_interval) => {
                         let heartbeat_ref = AtomicU32::new(0).fetch_add(1, Ordering::SeqCst).to_string(); // Simple ref for heartbeat
                         let heartbeat_msg = json!({
                             "topic": "phoenix",
                             "event": "heartbeat",
                             "payload": {},
                             "ref": heartbeat_ref
                         });
                         if let Err(e) = current_socket_tx.send(Message::Text(heartbeat_msg.to_string())).await {
                             eprintln!("Failed to send heartbeat: {}. Assuming connection lost.", e);
                             Self::set_connection_state_internal(reader_state_arc.clone(), reader_state_change_tx.clone(), ConnectionState::Disconnected).await;
                             *reader_socket_arc.write().await = None;
                             break; // Exit loop if heartbeat send fails
                         }
                    }
                }
            }

            // Connection closed, attempt reconnect if enabled and not manually closed
            if options.auto_reconnect && !is_manually_closed_arc.load(Ordering::SeqCst) {
                println!("Connection lost. Auto-reconnect is enabled but reconnect logic needs implementation.");
                // self.reconnect(); // This needs careful handling
            }

            Ok(())
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
            *current_state = state;
            let _ = state_change_tx.send(state);
        }
    }

    /// 切断処理
    pub async fn disconnect(&self) -> Result<(), RealtimeError> {
        // Use the Arc<AtomicBool>
        self.is_manually_closed.store(true, Ordering::SeqCst);
        self.set_connection_state(ConnectionState::Disconnected)
            .await;

        let mut socket_guard = self.socket.write().await;
        if let Some(socket_tx) = socket_guard.take() {
            // Close the sender channel, which will cause the writer task to exit
            // The reader task should exit due to stream closure or heartbeat failure.
            drop(socket_tx);
            println!("WebSocket connection closed manually.");
        }
        // Clear channels? Maybe not, allow re-connecting later?
        // self.channels.write().await.clear();

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
        }
    }
}

// WebSocketメッセージ送信エラーからの変換
impl From<tokio::sync::mpsc::error::SendError<Message>> for RealtimeError {
    fn from(err: tokio::sync::mpsc::error::SendError<Message>) -> Self {
        RealtimeError::ConnectionError(format!("Failed to send message to socket task: {}", err))
    }
}
