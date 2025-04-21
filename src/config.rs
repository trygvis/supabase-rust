//! Configuration options for the Supabase client

/// Client options for configuring the Supabase client
#[derive(Debug, Clone)]
pub struct ClientOptions {
    /// Auto refresh token when it's about to expire
    pub auto_refresh_token: bool,

    /// Persist session in memory
    pub persist_session: bool,

    /// Detect session from URL query parameters
    pub detect_session_in_url: bool,

    /// Headers to be sent with each request
    pub headers: std::collections::HashMap<String, String>,

    /// Global fetch options (for future compatibility)
    pub fetch_options: Option<serde_json::Value>,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            auto_refresh_token: true,
            persist_session: true,
            detect_session_in_url: true,
            headers: std::collections::HashMap::new(),
            fetch_options: None,
        }
    }
}

impl ClientOptions {
    /// Create a new ClientOptions with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to auto refresh token
    pub fn with_auto_refresh_token(mut self, auto_refresh_token: bool) -> Self {
        self.auto_refresh_token = auto_refresh_token;
        self
    }

    /// Set whether to persist session
    pub fn with_persist_session(mut self, persist_session: bool) -> Self {
        self.persist_session = persist_session;
        self
    }

    /// Set whether to detect session from URL
    pub fn with_detect_session_in_url(mut self, detect_session_in_url: bool) -> Self {
        self.detect_session_in_url = detect_session_in_url;
        self
    }

    /// Add a header to be sent with each request
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set fetch options
    pub fn with_fetch_options(mut self, fetch_options: serde_json::Value) -> Self {
        self.fetch_options = Some(fetch_options);
        self
    }
}

/// Supabase constants
#[allow(dead_code)]
pub struct Constants;

impl Constants {
    /// Default Supabase API version
    #[allow(dead_code)]
    pub const API_VERSION: &'static str = "v1";

    /// Default Supabase headers
    #[allow(dead_code)]
    pub const DEFAULT_HEADERS: &'static [(&'static str, &'static str)] =
        &[("X-Client-Info", "supabase-rust/0.1.0")];
}
