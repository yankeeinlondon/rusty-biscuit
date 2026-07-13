//! Server-role WebSocket host generation.
//!
//! Generates `XxxWsHost` structs and typed handler traits
//! for APIs where the generated code acts as the WebSocket server.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::plan::{EndpointPlan, EndpointRole, WsAuthStrategy, WsRuntimePlan};

/// Generate host runtime code for a WS API.
///
/// Only generates host code for endpoints with `Host` or `Both` role.
pub fn generate_ws_host_module(plan: &WsRuntimePlan) -> TokenStream {
    let host_endpoints: Vec<&EndpointPlan> = plan
        .endpoints
        .iter()
        .filter(|ep| matches!(ep.role, EndpointRole::Host | EndpointRole::Both))
        .collect();

    if host_endpoints.is_empty() {
        return TokenStream::new();
    }

    let host_struct = generate_host_struct(plan);
    let handler_trait = generate_handler_trait(plan, &host_endpoints);

    quote! {
        #host_struct
        #handler_trait
    }
}

fn generate_host_struct(plan: &WsRuntimePlan) -> TokenStream {
    let base = plan.api_name.strip_suffix("Ws").unwrap_or(&plan.api_name);
    let struct_name = format_ident!("{}WsHost", base);
    let (accept_expr, auth_helpers, emit_initial_authentication) =
        generate_accept_and_auth_helpers(plan);

    quote! {
        const HOST_OUTBOUND_CAPACITY: usize = 64;

        #[derive(Debug)]
        struct HostConnection {
            sender: tokio::sync::mpsc::Sender<serde_json::Value>,
            subscribed: bool,
        }

        #[derive(Debug)]
        struct HostState {
            next_connection_id: std::sync::atomic::AtomicU64,
            connections: tokio::sync::RwLock<std::collections::HashMap<u64, HostConnection>>,
        }

        impl Default for HostState {
            fn default() -> Self {
                Self {
                    next_connection_id: std::sync::atomic::AtomicU64::new(1),
                    connections: tokio::sync::RwLock::new(std::collections::HashMap::new()),
                }
            }
        }

        /// Shared connection registry and event broadcaster for integration hosts.
        #[derive(Clone, Debug, Default)]
        pub struct UnfoldedCircleEventHub {
            state: std::sync::Arc<HostState>,
        }

        impl UnfoldedCircleEventHub {
            /// Create a new empty hub.
            #[must_use]
            pub fn new() -> Self {
                Self::default()
            }

            async fn register_connection(&self) -> (u64, tokio::sync::mpsc::Receiver<serde_json::Value>) {
                use std::sync::atomic::Ordering;

                let connection_id = self
                    .state
                    .next_connection_id
                    .fetch_add(1, Ordering::Relaxed);
                let (sender, receiver) = tokio::sync::mpsc::channel(HOST_OUTBOUND_CAPACITY);

                self.state.connections.write().await.insert(
                    connection_id,
                    HostConnection {
                        sender,
                        subscribed: false,
                    },
                );

                (connection_id, receiver)
            }

            async fn connection_sender(
                &self,
                connection_id: u64,
            ) -> Option<tokio::sync::mpsc::Sender<serde_json::Value>> {
                self.state
                    .connections
                    .read()
                    .await
                    .get(&connection_id)
                    .map(|connection| connection.sender.clone())
            }

            async fn stale_connection_ids(
                &self,
                only_subscribed: bool,
                message: &serde_json::Value,
            ) -> (usize, Vec<u64>) {
                let senders: Vec<(u64, tokio::sync::mpsc::Sender<serde_json::Value>)> = self
                    .state
                    .connections
                    .read()
                    .await
                    .iter()
                    .filter(|(_, connection)| !only_subscribed || connection.subscribed)
                    .map(|(connection_id, connection)| (*connection_id, connection.sender.clone()))
                    .collect();

                let mut delivered = 0;
                let mut stale = Vec::new();

                for (connection_id, sender) in senders {
                    match sender.try_send(message.clone()) {
                        Ok(()) => delivered += 1,
                        Err(tokio::sync::mpsc::error::TrySendError::Full(_))
                        | Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                            stale.push(connection_id);
                        }
                    }
                }

                (delivered, stale)
            }

            /// Mark a connection as subscribed to unsolicited events.
            pub async fn mark_subscribed(&self, connection_id: u64) -> bool {
                if let Some(connection) = self.state.connections.write().await.get_mut(&connection_id) {
                    connection.subscribed = true;
                    true
                } else {
                    false
                }
            }

            /// Remove a connection from the registry.
            pub async fn remove_connection(&self, connection_id: u64) {
                self.state.connections.write().await.remove(&connection_id);
            }

            /// Send a message to a single connection.
            ///
            /// Slow or disconnected clients are removed to prevent stalled fan-out.
            ///
            /// ## Errors
            ///
            /// Returns [`super::ws_shared::WsError::Disconnected`] if the connection no
            /// longer exists or the outbound queue has closed. Returns
            /// [`super::ws_shared::WsError::BackpressureFull`] if the connection's
            /// outbound queue is full.
            pub async fn send_to_connection(
                &self,
                connection_id: u64,
                message: serde_json::Value,
            ) -> Result<(), super::ws_shared::WsError> {
                let Some(sender) = self.connection_sender(connection_id).await else {
                    return Err(super::ws_shared::WsError::Disconnected);
                };

                match sender.try_send(message) {
                    Ok(()) => Ok(()),
                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                        self.remove_connection(connection_id).await;
                        Err(super::ws_shared::WsError::BackpressureFull(
                            HOST_OUTBOUND_CAPACITY,
                        ))
                    }
                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                        self.remove_connection(connection_id).await;
                        Err(super::ws_shared::WsError::Disconnected)
                    }
                }
            }

            /// Broadcast a message to all currently connected clients.
            pub async fn broadcast(&self, message: serde_json::Value) -> usize {
                let (delivered, stale) = self.stale_connection_ids(false, &message).await;

                if !stale.is_empty() {
                    let mut connections = self.state.connections.write().await;
                    for connection_id in stale {
                        connections.remove(&connection_id);
                    }
                }

                delivered
            }

            /// Broadcast a message only to clients that previously subscribed.
            pub async fn broadcast_to_subscribers(&self, message: serde_json::Value) -> usize {
                let (delivered, stale) = self.stale_connection_ids(true, &message).await;

                if !stale.is_empty() {
                    let mut connections = self.state.connections.write().await;
                    for connection_id in stale {
                        connections.remove(&connection_id);
                    }
                }

                delivered
            }
        }

        /// Per-connection host context exposed to request handlers.
        #[derive(Clone, Debug)]
        pub struct WsConnectionContext {
            connection_id: u64,
            hub: UnfoldedCircleEventHub,
        }

        impl WsConnectionContext {
            pub fn new(connection_id: u64, hub: UnfoldedCircleEventHub) -> Self {
                Self { connection_id, hub }
            }

            /// Returns the unique connection id assigned by the host.
            #[must_use]
            pub fn connection_id(&self) -> u64 {
                self.connection_id
            }

            /// Returns a clone of the shared event hub.
            #[must_use]
            pub fn event_hub(&self) -> UnfoldedCircleEventHub {
                self.hub.clone()
            }

            /// Mark this connection as subscribed for unsolicited events.
            pub async fn mark_subscribed(&self) -> bool {
                self.hub.mark_subscribed(self.connection_id).await
            }

            /// Send a message directly to the current connection.
            ///
            /// ## Errors
            ///
            /// Propagates queueing errors from the underlying event hub.
            pub async fn send(
                &self,
                message: serde_json::Value,
            ) -> Result<(), super::ws_shared::WsError> {
                self.hub.send_to_connection(self.connection_id, message).await
            }
        }

        /// WebSocket host server.
        pub struct #struct_name;

        impl #struct_name {
            #auth_helpers

            /// Create a new shared event hub for integrations that need unsolicited
            /// event delivery.
            #[must_use]
            pub fn new_event_hub() -> UnfoldedCircleEventHub {
                UnfoldedCircleEventHub::new()
            }

            /// Serve WebSocket connections on the given TCP listener.
            pub async fn serve<H: WsHandler + 'static>(
                listener: tokio::net::TcpListener,
                handler: std::sync::Arc<H>,
            ) -> Result<(), super::ws_shared::WsError> {
                Self::serve_with_hub(listener, handler, UnfoldedCircleEventHub::new()).await
            }

            /// Serve WebSocket connections with a caller-provided event hub.
            pub async fn serve_with_hub<H: WsHandler + 'static>(
                listener: tokio::net::TcpListener,
                handler: std::sync::Arc<H>,
                hub: UnfoldedCircleEventHub,
            ) -> Result<(), super::ws_shared::WsError> {
                loop {
                    let (stream, _) = listener.accept().await
                        .map_err(|e| super::ws_shared::WsError::Protocol(e.to_string()))?;
                    let handler = handler.clone();
                    let hub = hub.clone();
                    tokio::spawn(async move {
                        if let Ok(ws_stream) = #accept_expr {
                            Self::handle_connection(ws_stream, handler, hub).await;
                        }
                    });
                }
            }

            /// Serve WebSocket connections on the given address.
            pub async fn serve_addr<H: WsHandler + 'static>(
                addr: impl tokio::net::ToSocketAddrs,
                handler: std::sync::Arc<H>,
            ) -> Result<(), super::ws_shared::WsError> {
                Self::serve_addr_with_hub(addr, handler, UnfoldedCircleEventHub::new()).await
            }

            /// Serve WebSocket connections on the given address with a caller-provided
            /// event hub.
            pub async fn serve_addr_with_hub<H: WsHandler + 'static>(
                addr: impl tokio::net::ToSocketAddrs,
                handler: std::sync::Arc<H>,
                hub: UnfoldedCircleEventHub,
            ) -> Result<(), super::ws_shared::WsError> {
                let listener = tokio::net::TcpListener::bind(addr).await
                    .map_err(|e| super::ws_shared::WsError::Protocol(e.to_string()))?;
                Self::serve_with_hub(listener, handler, hub).await
            }

            async fn handle_connection<H: WsHandler>(
                ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
                handler: std::sync::Arc<H>,
                hub: UnfoldedCircleEventHub,
            ) {
                use futures_util::{SinkExt, StreamExt};
                use tokio_tungstenite::tungstenite::Message;

                let (connection_id, mut outbound_rx) = hub.register_connection().await;
                let (mut write, mut read) = ws_stream.split();

                let writer = tokio::spawn(async move {
                    while let Some(message) = outbound_rx.recv().await {
                        let Ok(text) = serde_json::to_string(&message) else {
                            continue;
                        };

                        if write.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }

                    let _ = write.close().await;
                });

                if #emit_initial_authentication
                    && hub
                        .send_to_connection(connection_id, Self::authentication_success_response())
                        .await
                        .is_err()
                {
                    hub.remove_connection(connection_id).await;
                    let _ = writer.await;
                    return;
                }

                while let Some(msg_result) = read.next().await {
                    match msg_result {
                        Ok(Message::Text(text)) => {
                            match serde_json::from_str::<serde_json::Value>(text.as_ref()) {
                                Ok(request) => {
                                    let context = WsConnectionContext::new(connection_id, hub.clone());
                                    if let Some(response) = handler.handle_message(request, context).await {
                                        if hub.send_to_connection(connection_id, response).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                                Err(_) => {}
                            }
                        }
                        Ok(Message::Binary(data)) => {
                            match serde_json::from_slice::<serde_json::Value>(data.as_ref()) {
                                Ok(request) => {
                                    let context = WsConnectionContext::new(connection_id, hub.clone());
                                    if let Some(response) = handler.handle_message(request, context).await {
                                        if hub.send_to_connection(connection_id, response).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                                Err(_) => {}
                            }
                        }
                        Ok(Message::Close(_)) | Err(_) => break,
                        _ => {}
                    }
                }

                hub.remove_connection(connection_id).await;
                let _ = writer.await;
            }
        }
    }
}

fn generate_accept_and_auth_helpers(
    plan: &WsRuntimePlan,
) -> (TokenStream, TokenStream, TokenStream) {
    let (strategy, env_auth) = match &plan.auth {
        WsAuthStrategy::HeaderBased { strategy, env_auth } => (strategy, env_auth.as_slice()),
        WsAuthStrategy::MessageBased {
            header_strategy,
            env_auth,
            ..
        } => (header_strategy, env_auth.as_slice()),
        WsAuthStrategy::None => {
            return (
                quote! { tokio_tungstenite::accept_async(stream).await },
                TokenStream::new(),
                quote! { false },
            );
        }
    };

    match strategy {
        schematic_define::AuthStrategy::ApiKey { header, .. } => {
            let header_name = header.clone();
            let env_names = env_auth.to_vec();
            let common_helpers = super::shared::generate_host_common_auth_helpers(&env_names);
            let validate_fn = super::shared::generate_host_validate_raw_header(
                &header_name,
                "Invalid authentication header",
            );

            (
                quote! {
                    tokio_tungstenite::accept_hdr_async(stream, Self::validate_upgrade_request)
                        .await
                },
                quote! {
                    #common_helpers
                    #validate_fn
                },
                quote! { !Self::auth_is_required() },
            )
        }
        schematic_define::AuthStrategy::BearerToken { header } => {
            let header_name = header
                .clone()
                .unwrap_or_else(|| "Authorization".to_string());
            let env_names = env_auth.to_vec();
            let common_helpers = super::shared::generate_host_common_auth_helpers(&env_names);
            let validate_fn = super::shared::generate_host_validate_bearer_header(&header_name);

            (
                quote! {
                    tokio_tungstenite::accept_hdr_async(stream, Self::validate_upgrade_request)
                        .await
                },
                quote! {
                    #common_helpers
                    #validate_fn
                },
                quote! { !Self::auth_is_required() },
            )
        }
        _ => (
            quote! { tokio_tungstenite::accept_async(stream).await },
            TokenStream::new(),
            quote! { false },
        ),
    }
}

fn generate_handler_trait(_plan: &WsRuntimePlan, _endpoints: &[&EndpointPlan]) -> TokenStream {
    quote! {
            /// Handler trait for WebSocket host connections.
            ///
            /// Implement this trait to handle incoming WebSocket messages.
    pub trait WsHandler: Send + Sync {
        /// Handle an incoming message, optionally returning a response.
        fn handle_message(
            &self,
            message: serde_json::Value,
            context: WsConnectionContext,
        ) -> impl std::future::Future<Output = Option<serde_json::Value>> + Send;
    }
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws_codegen::plan::{CorrelationStrategy, WsAuthStrategy, WsRuntimePlan};
    use schematic_define::websocket::{FrameFormat, RequestIdType};

    fn test_plan_with_host() -> WsRuntimePlan {
        WsRuntimePlan {
            api_name: "Integration".to_string(),
            description: "Integration API".to_string(),
            base_url: "ws://localhost".to_string(),
            docs_url: None,
            frame_format: FrameFormat::JsonText,
            request_id_type: RequestIdType::U64,
            supports_reconnect: false,
            auth: WsAuthStrategy::None,
            endpoints: vec![EndpointPlan {
                id: "Intg".to_string(),
                path: "/intg".to_string(),
                description: "Integration channel".to_string(),
                role: EndpointRole::Both,
                correlation: CorrelationStrategy::None,
                heartbeat: None,
                client_messages: vec![],
                server_messages: vec![],
                bidirectional_messages: vec!["Msg".to_string()],
                lifecycle_open: None,
                lifecycle_close: None,
                lifecycle_keepalive: None,
                has_lifecycle_open: false,
                has_lifecycle_close: false,
                has_lifecycle_keepalive: false,
                path_params: vec![],
                query_params: vec![],
            }],
            diagnostics: vec![],
        }
    }

    #[test]
    fn generate_host_for_bidirectional_endpoint() {
        let plan = test_plan_with_host();
        let tokens = generate_ws_host_module(&plan);
        let code = tokens.to_string();
        assert!(
            code.contains("IntegrationWsHost"),
            "Expected IntegrationWsHost in: {}",
            code
        );
        assert!(code.contains("WsHandler"));
    }

    #[test]
    fn host_with_header_auth_uses_accept_hdr_async() {
        let mut plan = test_plan_with_host();
        plan.auth = WsAuthStrategy::HeaderBased {
            strategy: schematic_define::AuthStrategy::ApiKey {
                header: "auth-token".to_string(),
                value_prefix: None,
            },
            env_auth: vec!["UCR_INTEGRATION_TOKEN".to_string()],
        };

        let tokens = generate_ws_host_module(&plan);
        let code = tokens.to_string();
        assert!(code.contains("accept_hdr_async"));
        assert!(code.contains("validate_upgrade_request"));
    }

    #[test]
    fn no_host_code_for_client_only() {
        let mut plan = test_plan_with_host();
        plan.endpoints[0].role = EndpointRole::Client;
        let tokens = generate_ws_host_module(&plan);
        assert!(tokens.is_empty());
    }
}
