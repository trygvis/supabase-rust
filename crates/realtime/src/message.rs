use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
            Self::All => write!(f, "*"), // Use '*' as per Phoenix/Realtime protocol
        }
    }
}

/// メッセージペイロード
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    // Consider making fields private and using accessors if necessary
    pub data: serde_json::Value,
    #[serde(rename = "type")] // Map 'type' field in JSON
    pub event_type: Option<String>,
    pub timestamp: Option<String>, // Timestamps often come as strings
}

/// プレゼンス変更情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceChange {
    pub joins: HashMap<String, serde_json::Value>,
    pub leaves: HashMap<String, serde_json::Value>,
}

/// プレゼンス状態全体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceState {
    // Consider wrapping HashMap for better type safety/abstraction
    pub state: HashMap<String, serde_json::Value>,
}

impl PresenceState {
    pub fn new() -> Self {
        Self { state: HashMap::new() }
    }

    /// Apply presence diff to update the state
    pub fn sync(&mut self, presence_diff: &PresenceChange) {
        for (key, value) in &presence_diff.joins {
            self.state.insert(key.clone(), value.clone());
        }
        for key in presence_diff.leaves.keys() {
            self.state.remove(key);
        }
    }

    /// List current presence state as key-value pairs
    pub fn list(&self) -> Vec<(String, serde_json::Value)> {
        self.state.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Get presence info for a specific key
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }
}

impl Default for PresenceState {
    fn default() -> Self {
        Self::new()
    }
} 