//! Supabase Realtime client for Rust
//!
//! This crate provides realtime functionality for Supabase,
//! allowing for subscribing to database changes in real-time.

use std::sync::Arc;
use std::collections::HashMap;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use url::Url;
use futures_util::{StreamExt, SinkExt};
use tokio::sync::mpsc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::RwLock;

/// エラー型
#[derive(Error, Debug)]
pub enum RealtimeError {
    #[error("WebSocket error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),
    
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
    
    #[error("JSON serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Subscription error: {0}")]
    SubscriptionError(String),
    
    #[error("Channel error: {0}")]
    ChannelError(String),
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
}

impl RealtimeError {
    pub fn new(message: String) -> Self {
        Self::ChannelError(message)
    }
}

/// チャンネルイベント
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelEvent {
    Insert,
    Update,
    Delete,
    All,
}

/// データベース変更監視設定
#[derive(Debug, Clone, Serialize)]
pub struct DatabaseChanges {
    schema: String,
    table: String,
    events: Vec<ChannelEvent>,
}

impl DatabaseChanges {
    /// 新しいデータベース変更監視設定を作成
    pub fn new(table: &str) -> Self {
        Self {
            schema: "public".to_string(),
            table: table.to_string(),
            events: Vec::new(),
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
}

/// ブロードキャスト変更監視設定
#[derive(Debug, Clone, Serialize)]
pub struct BroadcastChanges {
    event: String,
}

impl BroadcastChanges {
    /// 新しいブロードキャスト変更監視設定を作成
    pub fn new(event: &str) -> Self {
        Self {
            event: event.to_string(),
        }
    }
}

/// メッセージペイロード
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    pub data: serde_json::Value,
    pub event_type: Option<String>,
    pub timestamp: Option<i64>,
}

/// サブスクリプション
pub struct Subscription {
    id: String,
    channel: Arc<Channel>,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        // サブスクリプションが破棄されたときに自動的に購読解除
        let channel = self.channel.clone();
        let id = self.id.clone();
        tokio::spawn(async move {
            let _ = channel.unsubscribe(&id).await;
        });
    }
}

struct Channel {
    topic: String,
    socket: Arc<RwLock<Option<mpsc::Sender<Message>>>>,
    callbacks: RwLock<HashMap<String, Box<dyn Fn(Payload) + Send + Sync>>>,
}

/// リアルタイムクライアント
pub struct RealtimeClient {
    url: String,
    key: String,
    next_ref: AtomicU32,
    channels: Arc<RwLock<HashMap<String, Arc<Channel>>>>,
    socket: Arc<RwLock<Option<mpsc::Sender<Message>>>>,
}

impl RealtimeClient {
    /// 新しいリアルタイムクライアントを作成
    pub fn new(url: &str, key: &str) -> Self {
        Self {
            url: url.to_string(),
            key: key.to_string(),
            next_ref: AtomicU32::new(0),
            channels: Arc::new(RwLock::new(HashMap::new())),
            socket: Arc::new(RwLock::new(None)),
        }
    }
    
    /// チャンネルを設定
    pub fn channel(&self, topic: &str) -> ChannelBuilder {
        ChannelBuilder {
            client: self,
            topic: topic.to_string(),
            callbacks: HashMap::new(),
        }
    }
    
    // 次のリファレンスIDを生成
    fn next_ref(&self) -> String {
        self.next_ref.fetch_add(1, Ordering::SeqCst).to_string()
    }
    
    // WebSocketに接続
    async fn connect(&self) -> Result<(), RealtimeError> {
        let ws_url = format!("{}/realtime/v1/websocket?apikey={}", self.url.replace("http", "ws"), self.key);
        let (ws_stream, _) = connect_async(ws_url).await?;
        let (mut write, read) = ws_stream.split();
        
        let (tx, mut rx) = mpsc::channel::<Message>(100);
        
        // 送信タスク
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = write.send(msg).await {
                    eprintln!("Error sending message: {}", e);
                    break;
                }
            }
        });
        
        // 受信タスク
        let channels = self.channels.clone();
        tokio::spawn(async move {
            read.for_each(|message| async {
                match message {
                    Ok(Message::Text(text)) => {
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                            // メッセージを処理
                            if let Some(topic) = value.get("topic").and_then(|t| t.as_str()) {
                                let read_guard = channels.read().await;
                                if let Some(channel) = read_guard.get(topic) {
                                    if let Some(payload) = value.get("payload") {
                                        let payload = Payload {
                                            data: payload.clone(),
                                            event_type: value.get("event").and_then(|e| e.as_str()).map(|s| s.to_string()),
                                            timestamp: value.get("timestamp").and_then(|t| t.as_i64()),
                                        };
                                        
                                        let callbacks = channel.callbacks.read().await;
                                        for callback in callbacks.values() {
                                            callback(payload.clone());
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Ok(Message::Close(_)) => {
                        // 接続が閉じられた
                    },
                    Err(e) => {
                        eprintln!("Error receiving message: {}", e);
                    },
                    _ => {}
                }
            }).await;
        });
        
        // ソケットを保存
        let mut socket_guard = self.socket.write().await;
        *socket_guard = Some(tx);
        
        Ok(())
    }
}

/// チャンネルビルダー
pub struct ChannelBuilder<'a> {
    client: &'a RealtimeClient,
    topic: String,
    callbacks: HashMap<String, Box<dyn Fn(Payload) + Send + Sync>>,
}

impl<'a> ChannelBuilder<'a> {
    /// データベース変更のハンドラを設定
    pub fn on<F>(mut self, _changes: DatabaseChanges, callback: F) -> Self
    where
        F: Fn(Payload) + Send + Sync + 'static,
    {
        let id = self.client.next_ref();
        self.callbacks.insert(id, Box::new(callback));
        self
    }
    
    /// ブロードキャストのハンドラを設定
    pub fn on_broadcast<F>(mut self, _changes: BroadcastChanges, callback: F) -> Self
    where
        F: Fn(Payload) + Send + Sync + 'static,
    {
        let id = self.client.next_ref();
        self.callbacks.insert(id, Box::new(callback));
        self
    }
    
    /// サブスクライブ
    pub async fn subscribe(self) -> Result<Subscription, RealtimeError> {
        // WebSocketが接続されていなければ接続
        let socket_guard = self.client.socket.read().await;
        if socket_guard.is_none() {
            drop(socket_guard);
            self.client.connect().await?;
        }
        
        let channel = Arc::new(Channel {
            topic: self.topic.clone(),
            socket: self.client.socket.clone(),
            callbacks: RwLock::new(self.callbacks),
        });
        
        // チャンネルを保存
        let mut channels_guard = self.client.channels.write().await;
        channels_guard.insert(self.topic.clone(), channel.clone());
        
        // サブスクリプションを送信
        let subscribe_message = serde_json::json!({
            "topic": self.topic,
            "event": "phx_join",
            "payload": {},
            "ref": self.client.next_ref(),
        });
        
        let socket_guard = self.client.socket.read().await;
        if let Some(tx) = &*socket_guard {
            tx.send(Message::Text(subscribe_message.to_string())).await
                .map_err(|_| RealtimeError::SubscriptionError("Failed to send subscription message".to_string()))?;
        } else {
            return Err(RealtimeError::ConnectionError("WebSocket not connected".to_string()));
        }
        
        let subscription_id = self.client.next_ref();
        Ok(Subscription {
            id: subscription_id,
            channel,
        })
    }
}

impl Channel {
    /// サブスクリプションを解除
    async fn unsubscribe(&self, id: &str) -> Result<(), RealtimeError> {
        // コールバックを削除
        let mut callbacks_guard = self.callbacks.write().await;
        callbacks_guard.remove(id);
        
        // すべてのコールバックが削除された場合、チャンネルを閉じる
        if callbacks_guard.is_empty() {
            drop(callbacks_guard);
            
            // チャンネルからの退出メッセージを送信
            let unsubscribe_message = serde_json::json!({
                "topic": self.topic,
                "event": "phx_leave",
                "payload": {},
                "ref": id,
            });
            
            let socket_guard = self.socket.read().await;
            if let Some(tx) = &*socket_guard {
                tx.send(Message::Text(unsubscribe_message.to_string())).await
                    .map_err(|_| RealtimeError::SubscriptionError("Failed to send unsubscription message".to_string()))?;
            } else {
                return Err(RealtimeError::ConnectionError("WebSocket not connected".to_string()));
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // モックのWebSocketサーバーを使用してテストを書く必要がある
}