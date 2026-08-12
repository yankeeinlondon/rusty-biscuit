//! Shared WS runtime types generator.
//!
//! Generates `ws_shared.rs` containing `WsError`, `WsClientOptions`,
//! `ReconnectPolicy`, `WsConnectionState`, `WsTransportHandle`,
//! and `WsEncode`/`WsDecode` traits.

use proc_macro2::TokenStream;
use quote::quote;

/// Generate the complete `ws_shared` module token stream.
pub fn generate_ws_shared_module() -> TokenStream {
    let error_type = generate_ws_error();
    let client_options = generate_ws_client_options();
    let reconnect_policy = generate_reconnect_policy();
    let connection_state = generate_ws_connection_state();
    let transport_handle = generate_ws_transport_handle();
    let codec_traits = generate_codec_traits();
    let writer_command = generate_writer_command();

    quote! {
        //! Shared WebSocket runtime types.
        //!
        //! This module provides the core runtime infrastructure used by all
        //! generated WebSocket client and host modules.

        // The `WsError::Transport` variant wraps `tokio_tungstenite::tungstenite::Error`,
        // which is intentionally large. Boxing it would change the public
        // signature of every generated codec trait, so we suppress the lint here.
        #![allow(clippy::result_large_err)]

        use std::collections::HashMap;
        use std::sync::Arc;
        use std::sync::atomic::AtomicU64;

        use tokio::sync::{mpsc, oneshot, watch, Mutex};
        use tokio::task::JoinHandle;

        #error_type
        #client_options
        #reconnect_policy
        #connection_state
        #writer_command
        #transport_handle
        #codec_traits
    }
}

fn generate_ws_error() -> TokenStream {
    quote! {
        /// Errors that can occur during WebSocket operations.
        #[derive(Debug, thiserror::Error)]
        pub enum WsError {
            /// Transport-level error from tungstenite.
            #[error("WebSocket transport error: {0}")]
            Transport(#[from] tokio_tungstenite::tungstenite::Error),

            /// JSON serialization/deserialization error.
            #[error("Serialization error: {0}")]
            Serde(#[from] serde_json::Error),

            /// Authentication failed.
            #[error("Authentication failed: {0}")]
            AuthRejected(String),

            /// Authentication timed out.
            #[error("Authentication timed out")]
            AuthTimeout,

            /// WebSocket handshake timed out.
            #[error("Handshake timed out after {0}s")]
            HandshakeTimeout(u64),

            /// Request timed out waiting for a correlated response.
            #[error("Request timed out after {0}s")]
            RequestTimeout(u64),

            /// Connection was disconnected.
            #[error("Connection disconnected")]
            Disconnected,

            /// Connection is shutting down.
            #[error("Connection is shutting down")]
            Shutdown,

            /// Send queue is full (backpressure).
            #[error("Send queue full (capacity: {0})")]
            BackpressureFull(usize),

            /// Protocol error (unexpected message format).
            #[error("Protocol error: {0}")]
            Protocol(String),

            /// Correlation error (unmatched response).
            #[error("Correlation error: {0}")]
            Correlation(String),

            /// A connection header could not be represented as an HTTP header.
            ///
            /// Returned instead of silently dropping the header, so a malformed
            /// auth header surfaces here rather than as an opaque auth rejection.
            #[error("Invalid connection header `{name}`: {reason}")]
            InvalidHeader {
                /// The offending header name.
                name: String,
                /// Why the name or value was rejected.
                reason: String,
            },
        }
    }
}

fn generate_ws_client_options() -> TokenStream {
    quote! {
        /// Options for configuring a WebSocket client connection.
        #[derive(Debug, Clone)]
        pub struct WsClientOptions {
            /// Timeout for the WebSocket handshake.
            pub handshake_timeout: std::time::Duration,
            /// Timeout for receiving any message.
            pub receive_timeout: Option<std::time::Duration>,
            /// Default timeout for correlated request-response calls.
            pub request_timeout: std::time::Duration,
            /// Maximum number of pending correlated requests.
            pub max_pending_requests: usize,
            /// Capacity of the outbound message channel.
            pub outbound_capacity: usize,
            /// Capacity of the inbound event channel.
            pub inbound_capacity: usize,
            /// Optional reconnect policy.
            pub reconnect: Option<ReconnectPolicy>,
            /// Whether to disable Nagle's algorithm (TCP_NODELAY).
            pub disable_nagle: bool,
            /// Optional websocket transport configuration.
            pub websocket_config: Option<tokio_tungstenite::tungstenite::protocol::WebSocketConfig>,
        }

        impl Default for WsClientOptions {
            fn default() -> Self {
                Self {
                    handshake_timeout: std::time::Duration::from_secs(10),
                    receive_timeout: None,
                    request_timeout: std::time::Duration::from_secs(30),
                    max_pending_requests: 256,
                    outbound_capacity: 64,
                    inbound_capacity: 256,
                    reconnect: None,
                    disable_nagle: false,
                    websocket_config: None,
                }
            }
        }

        impl WsClientOptions {
            /// Create a fluent options builder.
            #[must_use]
            pub fn builder() -> WsClientOptionsBuilder {
                WsClientOptionsBuilder {
                    options: Self::default(),
                }
            }
        }

        /// Fluent builder for [`WsClientOptions`].
        #[derive(Debug, Clone)]
        pub struct WsClientOptionsBuilder {
            options: WsClientOptions,
        }

        impl WsClientOptionsBuilder {
            /// Set handshake timeout.
            #[must_use]
            pub fn handshake_timeout(mut self, timeout: std::time::Duration) -> Self {
                self.options.handshake_timeout = timeout;
                self
            }

            /// Set receive timeout.
            #[must_use]
            pub fn receive_timeout(mut self, timeout: Option<std::time::Duration>) -> Self {
                self.options.receive_timeout = timeout;
                self
            }

            /// Set request timeout.
            #[must_use]
            pub fn request_timeout(mut self, timeout: std::time::Duration) -> Self {
                self.options.request_timeout = timeout;
                self
            }

            /// Set maximum number of pending requests.
            #[must_use]
            pub fn max_pending_requests(mut self, max_pending: usize) -> Self {
                self.options.max_pending_requests = max_pending;
                self
            }

            /// Set outbound queue capacity.
            #[must_use]
            pub fn outbound_capacity(mut self, capacity: usize) -> Self {
                self.options.outbound_capacity = capacity;
                self
            }

            /// Set inbound queue capacity.
            #[must_use]
            pub fn inbound_capacity(mut self, capacity: usize) -> Self {
                self.options.inbound_capacity = capacity;
                self
            }

            /// Set reconnect policy.
            #[must_use]
            pub fn reconnect(mut self, policy: Option<ReconnectPolicy>) -> Self {
                self.options.reconnect = policy;
                self
            }

            /// Set disable_nagle flag.
            #[must_use]
            pub fn disable_nagle(mut self, disable: bool) -> Self {
                self.options.disable_nagle = disable;
                self
            }

            /// Set websocket transport configuration.
            #[must_use]
            pub fn websocket_config(
                mut self,
                config: Option<tokio_tungstenite::tungstenite::protocol::WebSocketConfig>,
            ) -> Self {
                self.options.websocket_config = config;
                self
            }

            /// Build options.
            #[must_use]
            pub fn build(self) -> WsClientOptions {
                self.options
            }
        }
    }
}

fn generate_reconnect_policy() -> TokenStream {
    quote! {
        /// Policy for automatic reconnection.
        #[derive(Debug, Clone)]
        pub struct ReconnectPolicy {
            /// Initial backoff duration.
            pub initial_backoff: std::time::Duration,
            /// Maximum backoff duration.
            pub max_backoff: std::time::Duration,
            /// Backoff multiplier.
            pub multiplier: f64,
            /// Random jitter ratio (0.0 to 1.0).
            pub jitter: f64,
            /// Maximum number of reconnect attempts (None = unlimited).
            pub max_attempts: Option<u32>,
        }

        impl Default for ReconnectPolicy {
            fn default() -> Self {
                Self {
                    initial_backoff: std::time::Duration::from_millis(500),
                    max_backoff: std::time::Duration::from_secs(30),
                    multiplier: 2.0,
                    jitter: 0.25,
                    max_attempts: None,
                }
            }
        }
    }
}

fn generate_ws_connection_state() -> TokenStream {
    quote! {
        /// State of a WebSocket connection.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum WsConnectionState {
            /// Not connected.
            Disconnected,
            /// Establishing connection.
            Connecting,
            /// Performing authentication.
            Authenticating,
            /// Connected and ready for messages.
            Ready,
            /// Graceful close in progress.
            Closing,
            /// Connection closed.
            Closed,
        }

        impl std::fmt::Display for WsConnectionState {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::Disconnected => write!(f, "disconnected"),
                    Self::Connecting => write!(f, "connecting"),
                    Self::Authenticating => write!(f, "authenticating"),
                    Self::Ready => write!(f, "ready"),
                    Self::Closing => write!(f, "closing"),
                    Self::Closed => write!(f, "closed"),
                }
            }
        }
    }
}

fn generate_writer_command() -> TokenStream {
    quote! {
        /// Internal command sent to the writer task.
        #[derive(Debug)]
        #[allow(dead_code)]
        pub(crate) enum WriterCommand {
            /// Send a text message.
            SendText(String),
            /// Send a binary message.
            SendBinary(Vec<u8>),
            /// Initiate graceful close.
            Close,
        }
    }
}

fn generate_ws_transport_handle() -> TokenStream {
    quote! {
        /// Handle to an active WebSocket connection's transport tasks.
        ///
        /// Provides channels for sending messages and observing connection state.
        /// Dropping this handle signals the transport tasks to shut down.
        pub struct WsTransportHandle {
            /// Channel for sending commands to the writer task.
            pub(crate) writer_tx: mpsc::Sender<WriterCommand>,
            /// Connection state watcher.
            pub(crate) state_rx: watch::Receiver<WsConnectionState>,
            /// Supervisor task join handle.
            pub(crate) supervisor_handle: JoinHandle<()>,
            /// Monotonic request ID generator.
            pub(crate) next_id: Arc<AtomicU64>,
            /// Pending correlated requests awaiting responses.
            pub(crate) pending: Arc<Mutex<HashMap<u64, oneshot::Sender<serde_json::Value>>>>,
        }

        impl WsTransportHandle {
            /// Returns the current connection state.
            pub fn state(&self) -> WsConnectionState {
                *self.state_rx.borrow()
            }

            /// Returns a cloned watch receiver for connection state changes.
            pub fn state_watcher(&self) -> watch::Receiver<WsConnectionState> {
                self.state_rx.clone()
            }
        }

        impl Drop for WsTransportHandle {
            fn drop(&mut self) {
                self.supervisor_handle.abort();
            }
        }

        /// Compute reconnect delay with capped exponential backoff and jitter.
        pub(crate) fn reconnect_delay(policy: &ReconnectPolicy, attempt: u32) -> std::time::Duration {
            let multiplier = policy.multiplier.max(1.0);
            let exp = multiplier.powi(i32::try_from(attempt).unwrap_or(i32::MAX));
            let base_secs = (policy.initial_backoff.as_secs_f64() * exp)
                .min(policy.max_backoff.as_secs_f64())
                .max(0.0);
            let jitter_ratio = policy.jitter.clamp(0.0, 1.0);

            if jitter_ratio <= f64::EPSILON {
                return std::time::Duration::from_secs_f64(base_secs);
            }

            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.subsec_nanos());
            let normalized = f64::from(nanos) / 1_000_000_000.0;
            let span = base_secs * jitter_ratio;
            let offset = (normalized * 2.0 - 1.0) * span;
            std::time::Duration::from_secs_f64((base_secs + offset).max(0.0))
        }
    }
}

fn generate_codec_traits() -> TokenStream {
    quote! {
        /// Trait for types that can be encoded to a WebSocket frame.
        pub trait WsEncode {
            /// Encode this value to a WebSocket text frame payload.
            fn ws_encode(&self) -> Result<String, WsError>;
        }

        /// Trait for types that can be decoded from a WebSocket frame.
        pub trait WsDecode: Sized {
            /// Decode from a WebSocket text frame payload.
            fn ws_decode(text: &str) -> Result<Self, WsError>;
        }

        // Blanket implementation for serde types
        impl<T: serde::Serialize> WsEncode for T {
            fn ws_encode(&self) -> Result<String, WsError> {
                serde_json::to_string(self).map_err(WsError::Serde)
            }
        }

        // Blanket implementation for serde types
        impl<T: serde::de::DeserializeOwned> WsDecode for T {
            fn ws_decode(text: &str) -> Result<Self, WsError> {
                serde_json::from_str(text).map_err(WsError::Serde)
            }
        }
    }
}

/// Generates the common host auth helper functions shared by ApiKey and BearerToken strategies.
///
/// Produces: `configured_auth_tokens`, `auth_is_required`, `authentication_success_response`,
/// `auth_error_response`. Callers add their own `validate_upgrade_request` with strategy-specific
/// token extraction logic.
pub fn generate_host_common_auth_helpers(env_names: &[String]) -> TokenStream {
    quote! {
        fn configured_auth_tokens() -> Vec<String> {
            vec![#(#env_names.to_string()),*]
                .into_iter()
                .filter_map(|env| std::env::var(env).ok())
                .filter(|value| !value.trim().is_empty())
                .collect()
        }

        fn auth_is_required() -> bool {
            !Self::configured_auth_tokens().is_empty()
        }

        fn authentication_success_response() -> serde_json::Value {
            serde_json::json!({
                "kind": "resp",
                "req_id": 0,
                "msg": "authentication",
                "code": 200,
                "msg_data": {}
            })
        }

        fn auth_error_response(
            status: tokio_tungstenite::tungstenite::http::StatusCode,
            message: &str,
        ) -> tokio_tungstenite::tungstenite::handshake::server::ErrorResponse {
            tokio_tungstenite::tungstenite::http::Response::builder()
                .status(status)
                .body(Some(message.to_string()))
                .unwrap_or_else(|_| {
                    tokio_tungstenite::tungstenite::http::Response::new(Some("Unauthorized".to_string()))
                })
        }
    }
}

/// Generates `validate_upgrade_request` for header-based auth that compares the raw header value.
pub fn generate_host_validate_raw_header(header_name: &str, invalid_msg: &str) -> TokenStream {
    quote! {
        fn validate_upgrade_request(
            request: &tokio_tungstenite::tungstenite::handshake::server::Request,
            response: tokio_tungstenite::tungstenite::handshake::server::Response,
        ) -> Result<
            tokio_tungstenite::tungstenite::handshake::server::Response,
            tokio_tungstenite::tungstenite::handshake::server::ErrorResponse,
        > {
            let expected_values = Self::configured_auth_tokens();

            if expected_values.is_empty() {
                return Ok(response);
            }

            let Some(received) = request
                .headers()
                .get(#header_name)
                .and_then(|value| value.to_str().ok())
            else {
                return Err(Self::auth_error_response(
                    tokio_tungstenite::tungstenite::http::StatusCode::UNAUTHORIZED,
                    "Missing authentication header",
                ));
            };

            if expected_values.iter().any(|expected| expected == received) {
                Ok(response)
            } else {
                Err(Self::auth_error_response(
                    tokio_tungstenite::tungstenite::http::StatusCode::UNAUTHORIZED,
                    #invalid_msg,
                ))
            }
        }
    }
}

/// Generates `validate_upgrade_request` for Bearer token auth that strips the "Bearer " prefix.
pub fn generate_host_validate_bearer_header(header_name: &str) -> TokenStream {
    quote! {
        fn validate_upgrade_request(
            request: &tokio_tungstenite::tungstenite::handshake::server::Request,
            response: tokio_tungstenite::tungstenite::handshake::server::Response,
        ) -> Result<
            tokio_tungstenite::tungstenite::handshake::server::Response,
            tokio_tungstenite::tungstenite::handshake::server::ErrorResponse,
        > {
            let expected_values = Self::configured_auth_tokens();

            if expected_values.is_empty() {
                return Ok(response);
            }

            let Some(received) = request
                .headers()
                .get(#header_name)
                .and_then(|value| value.to_str().ok())
            else {
                return Err(Self::auth_error_response(
                    tokio_tungstenite::tungstenite::http::StatusCode::UNAUTHORIZED,
                    "Missing authentication header",
                ));
            };

            let token = received.strip_prefix("Bearer ").unwrap_or(received);
            if expected_values.iter().any(|expected| expected == token) {
                Ok(response)
            } else {
                Err(Self::auth_error_response(
                    tokio_tungstenite::tungstenite::http::StatusCode::UNAUTHORIZED,
                    "Invalid bearer token",
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_ws_shared_produces_tokens() {
        let tokens = generate_ws_shared_module();
        let code = tokens.to_string();
        assert!(code.contains("WsError"));
        assert!(code.contains("WsClientOptions"));
        assert!(code.contains("WsClientOptionsBuilder"));
        assert!(code.contains("ReconnectPolicy"));
        assert!(code.contains("WsConnectionState"));
        assert!(code.contains("WsTransportHandle"));
        assert!(code.contains("reconnect_delay"));
        assert!(code.contains("WsEncode"));
        assert!(code.contains("WsDecode"));
    }

    #[test]
    fn generated_ws_shared_parses_as_valid_rust() {
        let tokens = generate_ws_shared_module();
        let file = syn::parse2::<syn::File>(tokens);
        assert!(
            file.is_ok(),
            "Generated ws_shared must parse: {:?}",
            file.err()
        );
    }

    #[test]
    fn host_common_auth_helpers_produces_tokens() {
        let env_names = vec!["API_KEY".to_string()];
        let tokens = generate_host_common_auth_helpers(&env_names);
        let code = tokens.to_string();
        assert!(code.contains("configured_auth_tokens"));
        assert!(code.contains("auth_is_required"));
        assert!(code.contains("authentication_success_response"));
        assert!(code.contains("auth_error_response"));
        assert!(code.contains("API_KEY"));
    }

    #[test]
    fn host_validate_raw_header_produces_tokens() {
        let tokens = generate_host_validate_raw_header("X-API-Key", "Invalid API key");
        let code = tokens.to_string();
        assert!(code.contains("validate_upgrade_request"));
        assert!(code.contains("X-API-Key"));
        assert!(code.contains("Invalid API key"));
    }

    #[test]
    fn host_validate_bearer_header_produces_tokens() {
        let tokens = generate_host_validate_bearer_header("Authorization");
        let code = tokens.to_string();
        assert!(code.contains("validate_upgrade_request"));
        assert!(code.contains("Authorization"));
        assert!(code.contains("Bearer"));
    }
}
