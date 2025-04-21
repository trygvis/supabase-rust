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
use futures_util::{StreamExt, SinkExt};
use tokio::sync::mpsc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use tokio::sync::{broadcast, RwLock};
use std::time::Duration;
use tokio::time::sleep;
use url::Url;
use serde_json::{json, Value};

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

impl std::fmt::Display for ChannelEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Insert => write!(f, "INSERT"),
            Self::Update => write!(f, "UPDATE"),
            Self::Delete => write!(f, "DELETE"),
            Self::All => write!(f, "ALL"),
        }
    }
}

/// データベース変更に対するフィルター条件
#[derive(Debug, Clone, Serialize)]
pub struct DatabaseFilter {
    /// フィルター対象のカラム名
    pub column: String,
    /// 比較演算子
    pub operator: FilterOperator,
    /// 比較する値
    pub value: serde_json::Value,
}

/// フィルター演算子
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum FilterOperator {
    /// 等しい
    Eq,
    /// 等しくない
    Neq,
    /// より大きい
    Gt,
    /// より大きいか等しい
    Gte,
    /// より小さい
    Lt,
    /// より小さいか等しい
    Lte,
    /// 含む
    In,
    /// 含まない
    NotIn,
    /// 近い値（配列内の値に対して）
    ContainedBy,
    /// 含む（配列が対象の値を含む）
    Contains,
    /// 完全に含む（配列が対象の配列のすべての要素を含む）
    ContainedByArray,
    /// LIKE演算子（ワイルドカード検索）
    Like,
    /// ILIKE演算子（大文字小文字を区別しないワイルドカード検索）
    ILike,
}

impl ToString for FilterOperator {
    fn to_string(&self) -> String {
        match self {
            FilterOperator::Eq => "eq".to_string(),
            FilterOperator::Neq => "neq".to_string(),
            FilterOperator::Gt => "gt".to_string(),
            FilterOperator::Gte => "gte".to_string(),
            FilterOperator::Lt => "lt".to_string(),
            FilterOperator::Lte => "lte".to_string(),
            FilterOperator::In => "in".to_string(),
            FilterOperator::NotIn => "not.in".to_string(),
            FilterOperator::ContainedBy => "contained_by".to_string(),
            FilterOperator::Contains => "contains".to_string(),
            FilterOperator::ContainedByArray => "contained_by_array".to_string(),
            FilterOperator::Like => "like".to_string(),
            FilterOperator::ILike => "ilike".to_string(),
        }
    }
}

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
        if self.filter.is_none() {
            self.filter = Some(vec![filter]);
        } else {
            self.filter.as_mut().unwrap().push(filter);
        }
        self
    }

    /// eq演算子による簡便なフィルター追加メソッド
    pub fn eq<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::Eq,
            value: value.into(),
        })
    }

    /// neq演算子による簡便なフィルター追加メソッド
    pub fn neq<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::Neq,
            value: value.into(),
        })
    }

    /// gt演算子による簡便なフィルター追加メソッド
    pub fn gt<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::Gt,
            value: value.into(),
        })
    }

    /// gte演算子による簡便なフィルター追加メソッド
    pub fn gte<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::Gte,
            value: value.into(),
        })
    }

    /// lt演算子による簡便なフィルター追加メソッド
    pub fn lt<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::Lt,
            value: value.into(),
        })
    }

    /// lte演算子による簡便なフィルター追加メソッド
    pub fn lte<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::Lte,
            value: value.into(),
        })
    }

    /// in演算子による簡便なフィルター追加メソッド
    pub fn in_values<T: Into<serde_json::Value>>(self, column: &str, values: Vec<T>) -> Self {
        let json_values: Vec<serde_json::Value> = values.into_iter().map(|v| v.into()).collect();
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::In,
            value: serde_json::Value::Array(json_values),
        })
    }

    /// contains演算子による簡便なフィルター追加メソッド
    pub fn contains<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::Contains,
            value: value.into(),
        })
    }

    /// like演算子による簡便なフィルター追加メソッド
    pub fn like(self, column: &str, pattern: &str) -> Self {
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::Like,
            value: serde_json::Value::String(pattern.to_string()),
        })
    }

    /// ilike演算子による簡便なフィルター追加メソッド
    pub fn ilike(self, column: &str, pattern: &str) -> Self {
        self.filter(DatabaseFilter {
            column: column.to_string(),
            operator: FilterOperator::ILike,
            value: serde_json::Value::String(pattern.to_string()),
        })
    }

    // to_channel_configメソッドを更新
    fn to_channel_config(&self) -> serde_json::Value {
        let mut events_str = String::new();
        
        // イベントリストを文字列に変換
        for (i, event) in self.events.iter().enumerate() {
            if i > 0 {
                events_str.push(',');
            }
            events_str.push_str(&event.to_string());
        }
        
        // イベントが指定されていない場合は全イベント('*')を使用
        if events_str.is_empty() {
            events_str = "*".to_string();
        }
        
        let mut config = serde_json::json!({
            "schema": self.schema,
            "table": self.table,
            "event": events_str,
        });
        
        // フィルター条件があれば追加
        if let Some(filters) = &self.filter {
            let mut filter_obj = serde_json::Map::new();
            
            for filter in filters {
                let filter_key = format!("{}:{}", filter.column, filter.operator.to_string());
                filter_obj.insert(filter_key, filter.value.clone());
            }
            
            if !filter_obj.is_empty() {
                config["filter"] = serde_json::Value::Object(filter_obj);
            }
        }
        
        config
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

/// Presenceの変更イベント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceChange {
    pub joins: HashMap<String, serde_json::Value>,
    pub leaves: HashMap<String, serde_json::Value>,
}

/// Presenceの状態
#[derive(Debug, Clone, Default)]
pub struct PresenceState {
    pub state: HashMap<String, serde_json::Value>,
}

impl PresenceState {
    /// 新しいPresence状態を作成
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
        }
    }
    
    /// ユーザー状態を同期
    pub fn sync(&mut self, presence_diff: &PresenceChange) {
        // 退出ユーザーを削除
        for key in presence_diff.leaves.keys() {
            self.state.remove(key);
        }
        
        // 参加ユーザーを追加
        for (key, value) in &presence_diff.joins {
            self.state.insert(key.clone(), value.clone());
        }
    }
    
    /// 現在のユーザー一覧を取得
    pub fn list(&self) -> Vec<(String, serde_json::Value)> {
        self.state.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
    
    /// 特定のユーザーの状態を取得
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }
}

/// Presence変更設定
#[derive(Debug, Clone, Serialize)]
pub struct PresenceChanges {
    event: String,
}

impl PresenceChanges {
    /// 新しいPresence変更設定を作成
    pub fn new() -> Self {
        Self {
            event: "presence_state".to_string(),
        }
    }
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

/// 接続状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// リアルタイムクライアント設定
#[derive(Debug, Clone)]
pub struct RealtimeClientOptions {
    /// 自動再接続を有効にするかどうか
    pub auto_reconnect: bool,
    /// 最大再接続試行回数（Noneの場合は無限に試行）
    pub max_reconnect_attempts: Option<u32>,
    /// 再接続間隔（ミリ秒）
    pub reconnect_interval: u64,
    /// 再接続間隔の増加係数
    pub reconnect_backoff_factor: f64,
    /// 最大再接続間隔（ミリ秒）
    pub max_reconnect_interval: u64,
    /// ハートビート間隔（ミリ秒）
    pub heartbeat_interval: u64,
}

impl Default for RealtimeClientOptions {
    fn default() -> Self {
        Self {
            auto_reconnect: true,
            max_reconnect_attempts: Some(20),
            reconnect_interval: 1000,
            reconnect_backoff_factor: 1.5,
            max_reconnect_interval: 60000,
            heartbeat_interval: 30000,
        }
    }
}

/// リアルタイムクライアント
pub struct RealtimeClient {
    url: String,
    key: String,
    next_ref: AtomicU32,
    channels: Arc<RwLock<HashMap<String, Arc<Channel>>>>,
    socket: Arc<RwLock<Option<mpsc::Sender<Message>>>>,
    options: RealtimeClientOptions,
    state: Arc<RwLock<ConnectionState>>,
    reconnect_attempts: AtomicU32,
    is_manually_closed: AtomicBool,
    state_change: broadcast::Sender<ConnectionState>,
}

impl RealtimeClient {
    /// 新しいリアルタイムクライアントを作成
    pub fn new(url: &str, key: &str) -> Self {
        let (state_sender, _) = broadcast::channel(100);
        
        Self {
            url: url.to_string(),
            key: key.to_string(),
            next_ref: AtomicU32::new(0),
            channels: Arc::new(RwLock::new(HashMap::new())),
            socket: Arc::new(RwLock::new(None)),
            options: RealtimeClientOptions::default(),
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            reconnect_attempts: AtomicU32::new(0),
            is_manually_closed: AtomicBool::new(false),
            state_change: state_sender,
        }
    }
    
    /// カスタム設定でリアルタイムクライアントを作成
    pub fn new_with_options(url: &str, key: &str, options: RealtimeClientOptions) -> Self {
        let (state_sender, _) = broadcast::channel(100);
        
        Self {
            url: url.to_string(),
            key: key.to_string(),
            next_ref: AtomicU32::new(0),
            channels: Arc::new(RwLock::new(HashMap::new())),
            socket: Arc::new(RwLock::new(None)),
            options,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            reconnect_attempts: AtomicU32::new(0),
            is_manually_closed: AtomicBool::new(false),
            state_change: state_sender,
        }
    }
    
    /// 接続状態変更の監視用レシーバーを取得
    pub fn on_state_change(&self) -> broadcast::Receiver<ConnectionState> {
        self.state_change.subscribe()
    }
    
    /// 現在の接続状態を取得
    pub async fn get_connection_state(&self) -> ConnectionState {
        *self.state.read().await
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
    
    // 接続状態を変更
    async fn set_connection_state(&self, state: ConnectionState) {
        let mut state_guard = self.state.write().await;
        let old_state = *state_guard;
        *state_guard = state;
        
        if old_state != state {
            let _ = self.state_change.send(state);
        }
    }
    
    // WebSocketに接続
    fn connect(&self) -> impl std::future::Future<Output = Result<(), RealtimeError>> + Send + 'static {
        let client_clone = self.clone();
        async move {
            if client_clone.get_connection_state().await == ConnectionState::Connected {
                return Ok(());
            }
            
            client_clone.set_connection_state(ConnectionState::Connecting).await;
            
            // WebSocket URL構築
            let mut url = Url::parse(&client_clone.url)?;
            url.query_pairs_mut().append_pair("apikey", &client_clone.key);
            url.query_pairs_mut().append_pair("vsn", "1.0.0");
            
            // WebSocket接続
            let (ws_stream, _) = connect_async(url).await?;
            let (mut write, read) = ws_stream.split();
            
            // メッセージ送受信用のチャネル
            let (tx, mut rx) = mpsc::channel::<Message>(32);
            
            // クローンしておく
            let client_state = client_clone.state.clone();
            let auto_reconnect = client_clone.options.auto_reconnect;
            let manual_close = client_clone.is_manually_closed.load(Ordering::SeqCst);
            let reconnect_fn = client_clone.clone();
            
            // 受信メッセージ処理タスク
            tokio::task::spawn(async move {
                read.for_each(|message| async {
                    match message {
                        Ok(msg) => {
                            match msg {
                                Message::Text(text) => {
                                    // Jsonデコード
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                        // トピックとイベント取得
                                        let topic = json.get("topic").and_then(|v| v.as_str()).unwrap_or_default();
                                        let event = json.get("event").and_then(|v| v.as_str()).unwrap_or_default();
                                        let payload = json.get("payload").cloned().unwrap_or(serde_json::json!({}));
                                        
                                        // 接続確認応答
                                        if topic == "phoenix" && event == "phx_reply" {
                                            let status = payload.get("status").and_then(|v| v.as_str()).unwrap_or_default();
                                            
                                            if status == "ok" {
                                                let mut state_guard = client_state.write().await;
                                                *state_guard = ConnectionState::Connected;
                                                
                                                // リセット
                                                reconnect_fn.reconnect_attempts.store(0, Ordering::SeqCst);
                                                
                                                // 接続状態の変更を通知
                                                let _ = reconnect_fn.state_change.send(ConnectionState::Connected);
                                            }
                                        }
                                        // チャネルメッセージ
                                        else if let Some(payload_data) = payload.get("data") {
                                            let decoded_payload = Payload {
                                                data: payload_data.clone(),
                                                event_type: payload.get("type").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                                timestamp: payload.get("timestamp").and_then(|v| v.as_i64()),
                                            };
                                            
                                            if let Ok(channels_guard) = reconnect_fn.channels.try_read() {
                                                if let Some(channel) = channels_guard.get(topic) {
                                                    if let Ok(callbacks_guard) = channel.callbacks.try_read() {
                                                        for callback in callbacks_guard.values() {
                                                            callback(decoded_payload.clone());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Message::Close(_) => {
                                    // WebSocket接続が正常に閉じられた
                                    if !manual_close && auto_reconnect {
                                        let mut state_guard = client_state.write().await;
                                        *state_guard = ConnectionState::Reconnecting;
                                        
                                        // 再接続を実行
                                        let reconnect_client = reconnect_fn.clone();
                                        reconnect_client.reconnect().await;
                                    } else {
                                        let mut state_guard = client_state.write().await;
                                        *state_guard = ConnectionState::Disconnected;
                                    }
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            eprintln!("WebSocket error: {}", e);
                            
                            // エラー発生時、手動で閉じられていない場合は再接続
                            if !manual_close && auto_reconnect {
                                let mut state_guard = client_state.write().await;
                                *state_guard = ConnectionState::Reconnecting;
                                
                                // 再接続を実行
                                let reconnect_client = reconnect_fn.clone();
                                reconnect_client.reconnect().await;
                            } else {
                                let mut state_guard = client_state.write().await;
                                *state_guard = ConnectionState::Disconnected;
                            }
                        }
                    }
                }).await;
            });
            
            // 送信タスク
            tokio::task::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    if let Err(e) = write.send(msg).await {
                        eprintln!("Error sending message: {}", e);
                        
                        // エラー発生時に再接続
                        if auto_reconnect && !manual_close {
                            // 再接続を試みる
                            break;
                        }
                    }
                }
            });
            
            // ソケットを保存
            let mut socket_guard = client_clone.socket.write().await;
            *socket_guard = Some(tx.clone());
            
            // ハートビート送信タスク
            let socket_clone = tx.clone();
            let heartbeat_interval = client_clone.options.heartbeat_interval;
            let is_manually_closed = Arc::new(AtomicBool::new(client_clone.is_manually_closed.load(Ordering::SeqCst)));
            
            tokio::task::spawn(async move {
                loop {
                    sleep(Duration::from_millis(heartbeat_interval)).await;
                    
                    if is_manually_closed.load(Ordering::SeqCst) {
                        break;
                    }
                    
                    // ハートビートメッセージを送信
                    let heartbeat_msg = serde_json::json!({
                        "topic": "phoenix",
                        "event": "heartbeat",
                        "payload": {},
                        "ref": null
                    });
                    
                    if let Err(_) = socket_clone.send(Message::Text(heartbeat_msg.to_string())).await {
                        break;
                    }
                }
            });
            
            Ok(())
        }
    }
    
    /// 手動で接続を閉じる
    pub async fn disconnect(&self) -> Result<(), RealtimeError> {
        self.is_manually_closed.store(true, Ordering::SeqCst);
        
        let mut socket_guard = self.socket.write().await;
        if let Some(tx) = socket_guard.take() {
            // WebSocketのクローズメッセージを送信
            let close_msg = Message::Close(None);
            let _ = tx.send(close_msg).await;
        }
        
        self.set_connection_state(ConnectionState::Disconnected).await;
        
        Ok(())
    }
    
    // 再接続処理
    fn reconnect(&self) -> impl std::future::Future<Output = ()> + Send + 'static {
        let client_clone = self.clone();
        async move {
            if !client_clone.options.auto_reconnect || client_clone.is_manually_closed.load(Ordering::SeqCst) {
                return;
            }
            
            client_clone.set_connection_state(ConnectionState::Reconnecting).await;
            
            // 最大再接続試行回数をチェック
            let current_attempt = client_clone.reconnect_attempts.fetch_add(1, Ordering::SeqCst) + 1;
            if let Some(max) = client_clone.options.max_reconnect_attempts {
                if current_attempt > max {
                    client_clone.set_connection_state(ConnectionState::Disconnected).await;
                    return;
                }
            }
            
            // 現在の再接続間隔を計算
            let base_interval = client_clone.options.reconnect_interval as f64;
            let factor = client_clone.options.reconnect_backoff_factor.powi(current_attempt as i32 - 1);
            let interval = (base_interval * factor).min(client_clone.options.max_reconnect_interval as f64) as u64;
            
            // 指定時間待機
            sleep(Duration::from_millis(interval)).await;
            
            // 再接続を試みる
            let _ = client_clone.connect().await;
            
            // 再接続に成功したら既存のサブスクリプションを再登録
            if client_clone.get_connection_state().await == ConnectionState::Connected {
                let channels_guard = client_clone.channels.read().await;
                
                for (topic, _channel) in channels_guard.iter() {
                    let join_msg = serde_json::json!({
                        "topic": topic,
                        "event": "phx_join",
                        "payload": {},
                        "ref": client_clone.next_ref()
                    });
                    
                    let socket_guard = client_clone.socket.read().await;
                    if let Some(tx) = &*socket_guard {
                        let _ = tx.send(Message::Text(join_msg.to_string())).await;
                    }
                }
            }
        }
    }
}

// Clone実装を追加
impl Clone for RealtimeClient {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            key: self.key.clone(),
            next_ref: AtomicU32::new(self.next_ref.load(Ordering::SeqCst)),
            channels: self.channels.clone(),
            socket: self.socket.clone(),
            options: self.options.clone(),
            state: self.state.clone(),
            reconnect_attempts: AtomicU32::new(self.reconnect_attempts.load(Ordering::SeqCst)),
            is_manually_closed: AtomicBool::new(self.is_manually_closed.load(Ordering::SeqCst)),
            state_change: self.state_change.clone(),
        }
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
    pub fn on<F>(mut self, changes: DatabaseChanges, callback: F) -> Self
    where
        F: Fn(Payload) + Send + Sync + 'static,
    {
        let topic_key = serde_json::to_string(&changes).unwrap_or_default();
        self.callbacks.insert(topic_key, Box::new(callback));
        self
    }
    
    /// ブロードキャストメッセージに対するハンドラを登録
    pub fn on_broadcast<F>(mut self, changes: BroadcastChanges, callback: F) -> Self
    where
        F: Fn(Payload) + Send + Sync + 'static,
    {
        let topic_key = format!("broadcast:{}", changes.event);
        self.callbacks.insert(topic_key, Box::new(callback));
        self
    }
    
    /// プレゼンス変更に対するハンドラを登録
    pub fn on_presence<F>(mut self, callback: F) -> Self
    where
        F: Fn(PresenceChange) + Send + Sync + 'static,
    {
        // プレゼンスハンドラは特別なキーで保存
        let presence_callback = move |payload: Payload| {
            if let Ok(presence_diff) = serde_json::from_value::<PresenceChange>(payload.data.clone()) {
                callback(presence_diff);
            }
        };
        
        self.callbacks.insert("presence".to_string(), Box::new(presence_callback));
        self
    }
    
    /// チャンネルを購読
    pub async fn subscribe(self) -> Result<Subscription, RealtimeError> {
        // クライアントの接続状態を確認
        let state = self.client.get_connection_state().await;
        match state {
            ConnectionState::Disconnected | ConnectionState::Reconnecting => {
                // 自動再接続が有効ならば接続開始
                if self.client.options.auto_reconnect {
                    let connect_future = self.client.connect();
                    tokio::spawn(connect_future);
                    
                    // 接続が確立されるまで少し待機
                    for _ in 0..10 {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        let new_state = self.client.get_connection_state().await;
                        if matches!(new_state, ConnectionState::Connected) {
                            break;
                        }
                    }
                } else {
                    return Err(RealtimeError::ConnectionError(
                        "Client is disconnected and auto-reconnect is disabled".to_string()
                    ));
                }
            }
            ConnectionState::Connecting => {
                // 接続中なので少し待機
                for _ in 0..20 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    let new_state = self.client.get_connection_state().await;
                    if matches!(new_state, ConnectionState::Connected) {
                        break;
                    }
                }
                
                let final_state = self.client.get_connection_state().await;
                if !matches!(final_state, ConnectionState::Connected) {
                    return Err(RealtimeError::ConnectionError(
                        "Failed to connect to realtime server within timeout".to_string()
                    ));
                }
            }
            ConnectionState::Connected => {
                // 既に接続済み、そのまま続行
            }
        }
        
        // 既存のチャンネルを確認
        let channels = self.client.channels.read().await;
        if let Some(channel) = channels.get(&self.topic) {
            // 既存のチャンネルにコールバックを追加
            let mut callbacks = channel.callbacks.write().await;
            for (key, callback) in self.callbacks {
                callbacks.insert(key, callback);
            }
            
            return Ok(Subscription {
                id: self.client.next_ref(),
                channel: channel.clone(),
            });
        }
        
        // 新しいチャンネルを作成
        let channel = Arc::new(Channel {
            topic: self.topic.clone(),
            socket: self.client.socket.clone(),
            callbacks: RwLock::new(self.callbacks),
        });
        
        // チャンネル登録メッセージを送信
        let socket_guard = self.client.socket.read().await;
        if let Some(socket) = &*socket_guard {
            let ref_id = self.client.next_ref();
            let join_payload = json!({
                "event": "phx_join",
                "topic": self.topic,
                "payload": {},
                "ref": ref_id
            });
            
            let message = Message::Text(serde_json::to_string(&join_payload)
                .map_err(|e| RealtimeError::SerializationError(e))?);
                
            socket.send(message).await
                .map_err(|e| RealtimeError::SubscriptionError(format!("Failed to send join message: {}", e)))?;
                
            // チャンネルをマップに追加
            drop(socket_guard);
            let mut channels = self.client.channels.write().await;
            channels.insert(self.topic.clone(), channel.clone());
        } else {
            return Err(RealtimeError::ConnectionError("WebSocket connection not available".to_string()));
        }
        
        Ok(Subscription {
            id: self.client.next_ref(),
            channel: channel,
        })
    }
    
    /// このチャンネルに対してPresenceを初期化
    pub async fn track_presence(
        &self,
        user_id: &str,
        user_data: serde_json::Value
    ) -> Result<(), RealtimeError> {
        let socket_guard = self.client.socket.read().await;
        if let Some(tx) = &*socket_guard {
            let presence_msg = serde_json::json!({
                "topic": self.topic,
                "event": "presence",
                "payload": {
                    "user_id": user_id,
                    "user_data": user_data
                },
                "ref": self.client.next_ref()
            });
            
            tx.send(Message::Text(presence_msg.to_string())).await
                .map_err(|_| RealtimeError::ChannelError("Failed to send presence message".to_string()))?;
            
            Ok(())
        } else {
            Err(RealtimeError::ConnectionError("Socket not connected".to_string()))
        }
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

impl From<tokio::sync::mpsc::error::SendError<Message>> for RealtimeError {
    fn from(err: tokio::sync::mpsc::error::SendError<Message>) -> Self {
        RealtimeError::ChannelError(format!("Failed to send message: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_reconnection() {
        // このテストは実際のWebSocketサーバーとの通信を必要とするため、
        // ここでは単純にクライアントを作成して接続・切断できることを確認するだけにします。
        // 本格的なテストには、モックされたWebSocketサーバーを使用してください。
        
        let client = super::RealtimeClient::new("https://example.supabase.co", "test-key");
        
        // ステータス変更の購読
        let mut status_receiver = client.on_state_change();
        
        // 自動再接続をテスト
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            while let Ok(state) = status_receiver.recv().await {
                println!("Connection state changed: {:?}", state);
                
                if state == super::ConnectionState::Connected {
                    // 接続成功を確認
                    break;
                }
            }
        });
    }
}