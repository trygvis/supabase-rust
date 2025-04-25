use thiserror::Error;

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
    // Consider if this helper is still needed or if direct construction is clearer
    pub fn new(message: String) -> Self {
        Self::ChannelError(message)
    }
}

// Add conversion from SendError if needed, otherwise keep it in lib.rs/client.rs where Message is defined
// impl From<tokio::sync::mpsc::error::SendError<Message>> for RealtimeError {
//     fn from(err: tokio::sync::mpsc::error::SendError<Message>) -> Self {
//         RealtimeError::ConnectionError(format!("Failed to send message to socket task: {}", err))
//     }
// } 