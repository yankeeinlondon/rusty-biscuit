//! Server-role WebSocket host generation.
//!
//! Generates the `XxxWsHost` server, its `{ApiName}EventHub` connection
//! registry, and the `WsHandler` trait for endpoints with a `Host` or `Both`
//! role. Host input is the endpoint's client-message enum and host output its
//! server-message enum; nothing here assumes a particular protocol.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::websocket::FrameFormat;

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

    let host_struct = generate_host_struct(plan, &host_endpoints);
    let handler_trait = generate_handler_trait(&host_endpoints);

    quote! {
        #host_struct
        #handler_trait
    }
}

/// The PascalCase API name the client and host structs use.
fn api_base(plan: &WsRuntimePlan) -> &str {
    plan.api_name.strip_suffix("Ws").unwrap_or(&plan.api_name)
}

fn to_snake_case(name: &str) -> String {
    let mut out = String::new();
    for (index, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && index > 0 {
            out.push('_');
        }
        out.extend(ch.to_lowercase());
    }
    out
}

/// The handler's method names for one endpoint: `handle_message` and
/// `on_authenticated` for a single-endpoint host, endpoint-specific names
/// otherwise so each method's message types stay unambiguous.
fn handler_methods(ep: &EndpointPlan, single: bool) -> (proc_macro2::Ident, proc_macro2::Ident) {
    if single {
        (format_ident!("handle_message"), format_ident!("on_authenticated"))
    } else {
        let snake = to_snake_case(&ep.id);
        (
            format_ident!("handle_{}_message", snake),
            format_ident!("on_{}_authenticated", snake),
        )
    }
}

fn generate_host_struct(plan: &WsRuntimePlan, endpoints: &[&EndpointPlan]) -> TokenStream {
    let base = api_base(plan);
    let struct_name = format_ident!("{}WsHost", base);
    let hub_name = format_ident!("{}EventHub", base);
    let (credential_check, auth_helpers) = generate_credential_check(plan);
    let single = endpoints.len() == 1;

    let encode_frame = match plan.frame_format {
        FrameFormat::JsonText => quote! {
            serde_json::to_string(message).map(|text| tokio_tungstenite::tungstenite::Message::Text(text.into()))
        },
        FrameFormat::JsonBinary | FrameFormat::RawBinary | FrameFormat::Mixed => quote! {
            serde_json::to_vec(message).map(|bytes| tokio_tungstenite::tungstenite::Message::Binary(bytes.into()))
        },
    };

    let route_variants = endpoints.iter().map(|ep| format_ident!("{}", ep.id));
    let route_ids = endpoints.iter().map(|ep| {
        let variant = format_ident!("{}", ep.id);
        let id = &ep.id;
        quote! { Self::#variant => #id, }
    });
    let route_body = if single {
        let variant = format_ident!("{}", endpoints[0].id);
        quote! {
            let _ = path;
            Some(HostRoute::#variant)
        }
    } else {
        let checks = endpoints.iter().map(|ep| {
            let variant = format_ident!("{}", ep.id);
            let template = &ep.path;
            quote! {
                if path_matches(#template, path) {
                    return Some(HostRoute::#variant);
                }
            }
        });
        quote! {
            #(#checks)*
            None
        }
    };
    let outbound_impls = endpoints.iter().map(|ep| {
        let message = format_ident!("{}ServerMessage", ep.id);
        let id = &ep.id;
        quote! {
            impl HostOutbound for #message {
                const ENDPOINT: &'static str = #id;
            }
        }
    });
    let hook_arms = endpoints.iter().map(|ep| {
        let variant = format_ident!("{}", ep.id);
        let (_, hook) = handler_methods(ep, single);
        quote! {
            HostRoute::#variant => handler
                .#hook(context.clone())
                .await
                .map(|message| encode_frame(&message)),
        }
    });
    let dispatch_arms = endpoints.iter().map(|ep| {
        let variant = format_ident!("{}", ep.id);
        let client_message = format_ident!("{}ClientMessage", ep.id);
        let (handle, _) = handler_methods(ep, single);
        quote! {
            HostRoute::#variant => match decoded.and_then(#client_message::decode) {
                Ok(message) => match handler.#handle(message, context.clone()).await {
                    Some(reply) => hub.send_to_connection(connection_id, reply).await,
                    None => Ok(()),
                },
                Err(error) => {
                    handler.on_error(error, context.clone()).await;
                    Ok(())
                }
            },
        }
    });
    let path_matcher = if single {
        TokenStream::new()
    } else {
        quote! {
            /// Whether request path `path` fits endpoint path `template`, a
            /// `{name}` segment matching any one non-empty segment.
            fn path_matches(template: &str, path: &str) -> bool {
                let template: Vec<&str> = template.split('/').collect();
                let path: Vec<&str> = path.split('/').collect();
                template.len() == path.len()
                    && template.iter().zip(&path).all(|(expected, actual)| {
                        (expected.starts_with('{') && expected.ends_with('}') && !actual.is_empty())
                            || expected == actual
                    })
            }
        }
    };
    let hub_doc = format!("Connection registry and event fan-out for [`{struct_name}`].");

    quote! {
        const HOST_OUTBOUND_CAPACITY: usize = 64;

        /// Encodes one outbound message as this API's frame format.
        fn encode_frame<M: serde::Serialize + ?Sized>(
            message: &M,
        ) -> Result<tokio_tungstenite::tungstenite::Message, super::ws_shared::WsError> {
            Ok(#encode_frame?)
        }

        /// The host endpoint a connection was upgraded on.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum HostRoute {
            #(#route_variants,)*
        }

        impl HostRoute {
            fn id(self) -> &'static str {
                match self {
                    #(#route_ids)*
                }
            }
        }

        /// The endpoint serving request path `path`.
        fn route(path: &str) -> Option<HostRoute> {
            #route_body
        }

        #path_matcher

        /// A server message one endpoint of this host sends; implemented by
        /// each host endpoint's server-message enum.
        pub trait HostOutbound: serde::Serialize {
            /// The endpoint whose connections receive this message.
            const ENDPOINT: &'static str;
        }

        #(#outbound_impls)*

        #[derive(Debug)]
        struct HostConnection {
            endpoint: &'static str,
            sender: tokio::sync::mpsc::Sender<tokio_tungstenite::tungstenite::Message>,
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

        #[doc = #hub_doc]
        ///
        /// Typed sends reach only connections of the message's endpoint;
        /// the `_value` methods send a JSON value to any connection. A
        /// connection whose outbound queue is full or closed is removed.
        #[derive(Clone, Debug, Default)]
        pub struct #hub_name {
            state: std::sync::Arc<HostState>,
        }

        impl #hub_name {
            /// Create a new empty hub.
            #[must_use]
            pub fn new() -> Self {
                Self::default()
            }

            async fn register_connection(
                &self,
                endpoint: &'static str,
            ) -> (u64, tokio::sync::mpsc::Receiver<tokio_tungstenite::tungstenite::Message>) {
                use std::sync::atomic::Ordering;

                let connection_id = self
                    .state
                    .next_connection_id
                    .fetch_add(1, Ordering::Relaxed);
                let (sender, receiver) = tokio::sync::mpsc::channel(HOST_OUTBOUND_CAPACITY);

                self.state.connections.write().await.insert(
                    connection_id,
                    HostConnection {
                        endpoint,
                        sender,
                        subscribed: false,
                    },
                );

                (connection_id, receiver)
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

            /// Send a message to a single connection of the message's endpoint.
            ///
            /// ## Errors
            ///
            /// [`super::ws_shared::WsError::Disconnected`] if the connection no
            /// longer exists or its queue has closed,
            /// [`super::ws_shared::WsError::BackpressureFull`] if its queue is
            /// full, [`super::ws_shared::WsError::Protocol`] if it belongs to
            /// another endpoint, and [`super::ws_shared::WsError::Serde`] if the
            /// message does not serialize.
            pub async fn send_to_connection<M: HostOutbound>(
                &self,
                connection_id: u64,
                message: M,
            ) -> Result<(), super::ws_shared::WsError> {
                let frame = encode_frame(&message)?;
                self.enqueue(connection_id, Some(M::ENDPOINT), frame).await
            }

            /// Send a JSON value to a single connection, bypassing the
            /// declared messages.
            ///
            /// ## Errors
            ///
            /// As [`Self::send_to_connection`], without the endpoint check.
            pub async fn send_value_to_connection(
                &self,
                connection_id: u64,
                message: serde_json::Value,
            ) -> Result<(), super::ws_shared::WsError> {
                let frame = encode_frame(&message)?;
                self.enqueue(connection_id, None, frame).await
            }

            async fn enqueue(
                &self,
                connection_id: u64,
                endpoint: Option<&'static str>,
                frame: tokio_tungstenite::tungstenite::Message,
            ) -> Result<(), super::ws_shared::WsError> {
                let sender = {
                    let connections = self.state.connections.read().await;
                    let Some(connection) = connections.get(&connection_id) else {
                        return Err(super::ws_shared::WsError::Disconnected);
                    };
                    if let Some(endpoint) = endpoint
                        && connection.endpoint != endpoint
                    {
                        return Err(super::ws_shared::WsError::Protocol(format!(
                            "connection {connection_id} is a `{}` connection, not `{endpoint}`",
                            connection.endpoint
                        )));
                    }
                    connection.sender.clone()
                };

                match sender.try_send(frame) {
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

            /// Broadcast a message to every connection of its endpoint,
            /// returning how many received it.
            ///
            /// ## Errors
            ///
            /// [`super::ws_shared::WsError::Serde`] if the message does not
            /// serialize; nothing is sent.
            pub async fn broadcast<M: HostOutbound>(
                &self,
                message: M,
            ) -> Result<usize, super::ws_shared::WsError> {
                let frame = encode_frame(&message)?;
                Ok(self.fan_out(Some(M::ENDPOINT), false, frame).await)
            }

            /// [`Self::broadcast`] to subscribed connections only.
            ///
            /// ## Errors
            ///
            /// As [`Self::broadcast`].
            pub async fn broadcast_to_subscribers<M: HostOutbound>(
                &self,
                message: M,
            ) -> Result<usize, super::ws_shared::WsError> {
                let frame = encode_frame(&message)?;
                Ok(self.fan_out(Some(M::ENDPOINT), true, frame).await)
            }

            /// Broadcast a JSON value to every connection.
            pub async fn broadcast_value(&self, message: serde_json::Value) -> usize {
                match encode_frame(&message) {
                    Ok(frame) => self.fan_out(None, false, frame).await,
                    Err(_) => 0,
                }
            }

            /// Broadcast a JSON value to every subscribed connection.
            pub async fn broadcast_value_to_subscribers(&self, message: serde_json::Value) -> usize {
                match encode_frame(&message) {
                    Ok(frame) => self.fan_out(None, true, frame).await,
                    Err(_) => 0,
                }
            }

            async fn fan_out(
                &self,
                endpoint: Option<&'static str>,
                only_subscribed: bool,
                frame: tokio_tungstenite::tungstenite::Message,
            ) -> usize {
                let senders: Vec<(u64, tokio::sync::mpsc::Sender<tokio_tungstenite::tungstenite::Message>)> = self
                    .state
                    .connections
                    .read()
                    .await
                    .iter()
                    .filter(|(_, connection)| !only_subscribed || connection.subscribed)
                    .filter(|(_, connection)| endpoint.is_none_or(|endpoint| connection.endpoint == endpoint))
                    .map(|(connection_id, connection)| (*connection_id, connection.sender.clone()))
                    .collect();

                let mut delivered = 0;
                let mut stale = Vec::new();
                for (connection_id, sender) in senders {
                    match sender.try_send(frame.clone()) {
                        Ok(()) => delivered += 1,
                        Err(_) => stale.push(connection_id),
                    }
                }

                if !stale.is_empty() {
                    let mut connections = self.state.connections.write().await;
                    for connection_id in stale {
                        connections.remove(&connection_id);
                    }
                }

                delivered
            }
        }

        /// Per-connection host context exposed to handlers.
        #[derive(Clone, Debug)]
        pub struct WsConnectionContext {
            connection_id: u64,
            credential_verified: bool,
            hub: #hub_name,
        }

        impl WsConnectionContext {
            /// A context for `connection_id` whose upgrade checked no credential.
            pub fn new(connection_id: u64, hub: #hub_name) -> Self {
                Self { connection_id, credential_verified: false, hub }
            }

            /// Returns the unique connection id assigned by the host.
            #[must_use]
            pub fn connection_id(&self) -> u64 {
                self.connection_id
            }

            /// Whether the upgrade presented a credential the host checked
            /// and accepted. `false` when the host has no credential
            /// configured and accepted the upgrade without one.
            #[must_use]
            pub fn credential_verified(&self) -> bool {
                self.credential_verified
            }

            /// Returns a clone of the shared event hub.
            #[must_use]
            pub fn event_hub(&self) -> #hub_name {
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
            /// As the hub's `send_to_connection`.
            pub async fn send<M: HostOutbound>(&self, message: M) -> Result<(), super::ws_shared::WsError> {
                self.hub.send_to_connection(self.connection_id, message).await
            }

            /// Send a JSON value directly to the current connection,
            /// bypassing the declared messages.
            ///
            /// ## Errors
            ///
            /// As the hub's `send_value_to_connection`.
            pub async fn send_value(&self, message: serde_json::Value) -> Result<(), super::ws_shared::WsError> {
                self.hub.send_value_to_connection(self.connection_id, message).await
            }
        }

        /// WebSocket host server.
        pub struct #struct_name;

        impl #struct_name {
            #auth_helpers

            fn upgrade_error_response(
                status: tokio_tungstenite::tungstenite::http::StatusCode,
                message: &str,
            ) -> tokio_tungstenite::tungstenite::handshake::server::ErrorResponse {
                tokio_tungstenite::tungstenite::http::Response::builder()
                    .status(status)
                    .body(Some(message.to_string()))
                    .unwrap_or_else(|_| {
                        tokio_tungstenite::tungstenite::http::Response::new(Some(message.to_string()))
                    })
            }

            /// Create a new shared event hub for hosts that send unsolicited
            /// events.
            #[must_use]
            pub fn new_event_hub() -> #hub_name {
                #hub_name::new()
            }

            /// Serve WebSocket connections on the given TCP listener.
            pub async fn serve<H: WsHandler + 'static>(
                listener: tokio::net::TcpListener,
                handler: std::sync::Arc<H>,
            ) -> Result<(), super::ws_shared::WsError> {
                Self::serve_with_hub(listener, handler, #hub_name::new()).await
            }

            /// Serve WebSocket connections with a caller-provided event hub.
            pub async fn serve_with_hub<H: WsHandler + 'static>(
                listener: tokio::net::TcpListener,
                handler: std::sync::Arc<H>,
                hub: #hub_name,
            ) -> Result<(), super::ws_shared::WsError> {
                loop {
                    let (stream, _) = listener.accept().await
                        .map_err(|e| super::ws_shared::WsError::Protocol(e.to_string()))?;
                    let handler = handler.clone();
                    let hub = hub.clone();
                    tokio::spawn(async move {
                        let mut accepted = None;
                        let upgrade = tokio_tungstenite::accept_hdr_async(
                            stream,
                            |request: &tokio_tungstenite::tungstenite::handshake::server::Request,
                             response: tokio_tungstenite::tungstenite::handshake::server::Response| {
                                let Some(endpoint) = route(request.uri().path()) else {
                                    return Err(Self::upgrade_error_response(
                                        tokio_tungstenite::tungstenite::http::StatusCode::NOT_FOUND,
                                        "Unknown WebSocket endpoint",
                                    ));
                                };
                                let verified = #credential_check;
                                accepted = Some((endpoint, verified));
                                Ok(response)
                            },
                        )
                        .await;
                        if let (Ok(ws_stream), Some((endpoint, verified))) = (upgrade, accepted) {
                            Self::handle_connection(ws_stream, endpoint, verified, handler, hub).await;
                        }
                    });
                }
            }

            /// Serve WebSocket connections on the given address.
            pub async fn serve_addr<H: WsHandler + 'static>(
                addr: impl tokio::net::ToSocketAddrs,
                handler: std::sync::Arc<H>,
            ) -> Result<(), super::ws_shared::WsError> {
                Self::serve_addr_with_hub(addr, handler, #hub_name::new()).await
            }

            /// Serve WebSocket connections on the given address with a caller-provided
            /// event hub.
            pub async fn serve_addr_with_hub<H: WsHandler + 'static>(
                addr: impl tokio::net::ToSocketAddrs,
                handler: std::sync::Arc<H>,
                hub: #hub_name,
            ) -> Result<(), super::ws_shared::WsError> {
                let listener = tokio::net::TcpListener::bind(addr).await
                    .map_err(|e| super::ws_shared::WsError::Protocol(e.to_string()))?;
                Self::serve_with_hub(listener, handler, hub).await
            }

            /// Serves one upgraded connection: the post-authentication hook's
            /// message is written first, then inbound messages are decoded
            /// and dispatched until the peer leaves.
            async fn handle_connection<H: WsHandler>(
                ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
                endpoint: HostRoute,
                credential_verified: bool,
                handler: std::sync::Arc<H>,
                hub: #hub_name,
            ) {
                use futures_util::{SinkExt, StreamExt};
                use tokio_tungstenite::tungstenite::Message;

                let (connection_id, mut outbound_rx) = hub.register_connection(endpoint.id()).await;
                let context = WsConnectionContext {
                    connection_id,
                    credential_verified,
                    hub: hub.clone(),
                };
                let (mut write, mut read) = ws_stream.split();

                // Written directly, before the writer task starts, so it
                // precedes every queued or replied message.
                let greeting = match endpoint {
                    #(#hook_arms)*
                };
                if let Some(frame) = greeting {
                    let written = match frame {
                        Ok(frame) => write.send(frame).await.map_err(super::ws_shared::WsError::from),
                        Err(error) => Err(error),
                    };
                    if let Err(error) = written {
                        hub.remove_connection(connection_id).await;
                        let _ = write.close().await;
                        handler.on_error(error, context).await;
                        return;
                    }
                }

                let writer = tokio::spawn(async move {
                    while let Some(frame) = outbound_rx.recv().await {
                        if write.send(frame).await.is_err() {
                            break;
                        }
                    }

                    let _ = write.close().await;
                });

                while let Some(msg_result) = read.next().await {
                    let raw = match msg_result {
                        Ok(Message::Text(text)) => super::ws_shared::WsRawFrame::Text(text.to_string()),
                        Ok(Message::Binary(data)) => super::ws_shared::WsRawFrame::Binary(data.to_vec()),
                        Ok(Message::Close(_)) | Err(_) => break,
                        Ok(_) => continue,
                    };
                    let decoded = super::ws_shared::WsInboundFrame::parse(raw);
                    let replied = match endpoint {
                        #(#dispatch_arms)*
                    };
                    if replied.is_err() {
                        break;
                    }
                }

                hub.remove_connection(connection_id).await;
                let _ = writer.await;
            }
        }
    }
}

/// The expression deciding an upgrade's credential (`bool`, early-returning
/// the rejection from the routing callback) and the host's auth helpers.
fn generate_credential_check(plan: &WsRuntimePlan) -> (TokenStream, TokenStream) {
    let (strategy, env_auth) = match &plan.auth {
        WsAuthStrategy::HeaderBased { strategy, env_auth } => (strategy, env_auth.as_slice()),
        WsAuthStrategy::MessageBased {
            header_strategy,
            env_auth,
            ..
        } => (header_strategy, env_auth.as_slice()),
        WsAuthStrategy::None => return (quote! { false }, TokenStream::new()),
    };

    let validate_fn = match strategy {
        schematic_define::AuthStrategy::ApiKey { header, .. } => {
            super::shared::generate_host_validate_raw_header(header, "Invalid authentication header")
        }
        schematic_define::AuthStrategy::BearerToken { header } => {
            let header_name = header.clone().unwrap_or_else(|| "Authorization".to_string());
            super::shared::generate_host_validate_bearer_header(&header_name)
        }
        _ => return (quote! { false }, TokenStream::new()),
    };
    let common_helpers = super::shared::generate_host_common_auth_helpers(env_auth);
    (
        quote! { Self::check_upgrade_credential(request)? },
        quote! {
            #common_helpers
            #validate_fn
        },
    )
}

fn generate_handler_trait(endpoints: &[&EndpointPlan]) -> TokenStream {
    let single = endpoints.len() == 1;
    let methods = endpoints.iter().map(|ep| {
        let (handle, hook) = handler_methods(ep, single);
        let client_message = format_ident!("{}ClientMessage", ep.id);
        let server_message = format_ident!("{}ServerMessage", ep.id);
        let handle_doc = format!(
            "Handles one decoded message from a `{}` connection, optionally returning a reply.",
            ep.id
        );
        let hook_doc = format!(
            "Runs once per `{}` connection after its upgrade is accepted (and its credential, \
             if the host checks one, verified) and before any message is read. A returned \
             message is written first; if that write fails the connection is closed and \
             the error goes to [`WsHandler::on_error`]. The default sends nothing.",
            ep.id
        );
        quote! {
            #[doc = #handle_doc]
            fn #handle(
                &self,
                message: #client_message,
                context: WsConnectionContext,
            ) -> impl std::future::Future<Output = Option<#server_message>> + Send;

            #[doc = #hook_doc]
            fn #hook(
                &self,
                context: WsConnectionContext,
            ) -> impl std::future::Future<Output = Option<#server_message>> + Send {
                let _ = context;
                async { None }
            }
        }
    });

    quote! {
        /// Handler trait for WebSocket host connections.
        ///
        /// Messages arrive decoded as the endpoint's client-message enum;
        /// replies are its server-message enum.
        pub trait WsHandler: Send + Sync {
            #(#methods)*

            /// Receives a connection's errors: an inbound frame that did not
            /// decode (the connection stays open), or a failed
            /// post-authentication write (the connection is closed). The
            /// default ignores them.
            fn on_error(
                &self,
                error: super::ws_shared::WsError,
                context: WsConnectionContext,
            ) -> impl std::future::Future<Output = ()> + Send {
                let _ = (error, context);
                async {}
            }
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
            handshake_auth: Default::default(),
            headers: vec![],
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
                messages: vec![],
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
        assert!(code.contains("check_upgrade_credential"));
    }

    #[test]
    fn no_host_code_for_client_only() {
        let mut plan = test_plan_with_host();
        plan.endpoints[0].role = EndpointRole::Client;
        let tokens = generate_ws_host_module(&plan);
        assert!(tokens.is_empty());
    }
}
