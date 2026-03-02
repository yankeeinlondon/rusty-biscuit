//! Per-API typed WebSocket client generation.
//!
//! Generates client structs, connect methods, send/request methods,
//! event stream adapters, and close/state accessors.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::plan::{
    CorrelationStrategy, EndpointPlan, LifecycleMessagePlan, WsAuthStrategy, WsRuntimePlan,
};

/// Generate the complete client module for a WS API.
pub fn generate_ws_client_module(plan: &WsRuntimePlan) -> TokenStream {
    let api_struct = generate_api_struct(plan);
    let endpoint_clients: Vec<TokenStream> = plan
        .endpoints
        .iter()
        .map(|ep| generate_endpoint_client(plan, ep))
        .collect();

    quote! {
        #api_struct
        #(#endpoint_clients)*
    }
}

fn ws_struct_name(api_name: &str, suffix: &str) -> proc_macro2::Ident {
    let base = api_name.strip_suffix("Ws").unwrap_or(api_name);
    format_ident!("{}Ws{}", base, suffix)
}

fn generate_headers_init(plan: &WsRuntimePlan) -> TokenStream {
    match &plan.auth {
        WsAuthStrategy::None => quote! { schematic_define::Headers::default() },
        WsAuthStrategy::HeaderBased {
            strategy,
            env_auth,
        }
        | WsAuthStrategy::MessageBased {
            header_strategy: strategy,
            env_auth,
            ..
        } => {
            let env_mapping_init = generate_env_mapping(strategy, env_auth);
            quote! {
                schematic_define::Headers::default()
                    .with_env_mapping(#env_mapping_init)
            }
        }
    }
}

fn generate_env_mapping(strategy: &schematic_define::AuthStrategy, env_auth: &[String]) -> TokenStream {
    match strategy {
        schematic_define::AuthStrategy::ApiKey { header } => {
            quote! {
                schematic_define::EnvMapping {
                    bearer_token: None,
                    basic_user: None,
                    basic_pass: None,
                    api_key: Some(schematic_define::ApiKeyEnv {
                        names: schematic_define::EnvList::new(vec![#(#env_auth.to_string()),*]),
                        header: #header.to_string(),
                    }),
                }
            }
        }
        schematic_define::AuthStrategy::BearerToken { .. } => {
            quote! {
                schematic_define::EnvMapping {
                    bearer_token: Some(schematic_define::EnvList::new(vec![#(#env_auth.to_string()),*])),
                    basic_user: None,
                    basic_pass: None,
                    api_key: None,
                }
            }
        }
        schematic_define::AuthStrategy::Basic => {
            quote! {
                schematic_define::EnvMapping {
                    bearer_token: None,
                    basic_user: Some(schematic_define::EnvList::new(vec![#(#env_auth.to_string()),*])),
                    basic_pass: None,
                    api_key: None,
                }
            }
        }
        _ => {
            quote! {
                schematic_define::EnvMapping {
                    bearer_token: None,
                    basic_user: None,
                    basic_pass: None,
                    api_key: None,
                }
            }
        }
    }
}

fn generate_api_struct(plan: &WsRuntimePlan) -> TokenStream {
    let struct_name = ws_struct_name(&plan.api_name, "");
    let base_url = &plan.base_url;
    let description = &plan.description;
    let headers_init = generate_headers_init(plan);
    let connect_methods: Vec<TokenStream> = plan
        .endpoints
        .iter()
        .map(|ep| generate_connect_method(plan, ep))
        .collect();

    quote! {
        #[doc = #description]
        pub struct #struct_name {
            base_url: String,
            /// Headers builder with environment variable support for credentials.
            pub headers: schematic_define::Headers,
        }

        impl #struct_name {
            /// Create a new client with the default base URL.
            #[must_use]
            pub fn new() -> Self {
                Self {
                    base_url: #base_url.to_string(),
                    headers: #headers_init,
                }
            }

            /// Create a new client with a custom base URL.
            #[must_use]
            pub fn with_base_url(base_url: impl Into<String>) -> Self {
                Self {
                    base_url: base_url.into(),
                    headers: #headers_init,
                }
            }

            /// Returns the configured base URL.
            #[must_use]
            pub fn base_url(&self) -> &str {
                &self.base_url
            }

            #(#connect_methods)*
        }

        impl Default for #struct_name {
            fn default() -> Self {
                Self::new()
            }
        }
    }
}

fn generate_connect_method(_plan: &WsRuntimePlan, ep: &EndpointPlan) -> TokenStream {
    let method_name = format_ident!("connect_{}", to_snake_case(&ep.id));
    let client_name = format_ident!("{}Client", ep.id);
    let path = &ep.path;
    let description = format!("Connect to the {} endpoint.", ep.id);

    quote! {
        #[doc = #description]
        pub async fn #method_name(
            &self,
            options: super::ws_shared::WsClientOptions,
        ) -> Result<#client_name, super::ws_shared::WsError> {
            let url = format!("{}{}", self.base_url, #path);
            let header_pairs = self.headers.clone().build()
                .map_err(|e| super::ws_shared::WsError::Protocol(e.to_string()))?;
            #client_name::connect(url, options, header_pairs).await
        }
    }
}

fn generate_endpoint_client(_plan: &WsRuntimePlan, ep: &EndpointPlan) -> TokenStream {
    let client_name = format_ident!("{}Client", ep.id);
    let has_correlation = matches!(ep.correlation, CorrelationStrategy::Correlated { .. });
    let lifecycle_helpers = generate_lifecycle_helpers(ep);

    let request_method = if let CorrelationStrategy::Correlated {
        ref request_id_field,
        ..
    } = ep.correlation
    {
        let req_field = request_id_field;
        quote! {
            /// Send a correlated request and wait for the response.
            pub async fn request(
                &self,
                message: serde_json::Value,
            ) -> Result<serde_json::Value, super::ws_shared::WsError> {
                use std::sync::atomic::Ordering;

                let id = self.transport.next_id.fetch_add(1, Ordering::Relaxed);
                let (tx, rx) = tokio::sync::oneshot::channel();

                {
                    let mut pending = self.transport.pending.lock().await;
                    if pending.len() >= self.max_pending {
                        return Err(super::ws_shared::WsError::BackpressureFull(self.max_pending));
                    }
                    pending.insert(id, tx);
                }

                let mut msg = message;
                if let serde_json::Value::Object(ref mut map) = msg {
                    map.insert(#req_field.to_string(), serde_json::Value::Number(id.into()));
                }

                let text = serde_json::to_string(&msg)?;
                self.transport
                    .writer_tx
                    .send(super::ws_shared::WriterCommand::SendText(text))
                    .await
                    .map_err(|_| super::ws_shared::WsError::Disconnected)?;

                let timeout = self.request_timeout;
                match tokio::time::timeout(timeout, rx).await {
                    Ok(Ok(response)) => Ok(response),
                    Ok(Err(_)) => Err(super::ws_shared::WsError::Disconnected),
                    Err(_) => {
                        self.transport.pending.lock().await.remove(&id);
                        Err(super::ws_shared::WsError::RequestTimeout(timeout.as_secs()))
                    }
                }
            }
        }
    } else {
        TokenStream::new()
    };

    let extra_fields = if has_correlation {
        quote! {
            request_timeout: std::time::Duration,
            max_pending: usize,
        }
    } else {
        TokenStream::new()
    };

    let constructor_extra_fields = if let CorrelationStrategy::Correlated {
        default_timeout_secs,
        ..
    } = ep.correlation
    {
        quote! {
            request_timeout: std::time::Duration::from_secs(#default_timeout_secs),
            max_pending: _options.max_pending_requests,
        }
    } else {
        TokenStream::new()
    };

    // Generate reader task message handling based on correlation
    let reader_message_handler = if let CorrelationStrategy::Correlated {
        ref response_id_field,
        ..
    } = ep.correlation
    {
        let resp_field = response_id_field;
        quote! {
            Ok(value) => {
                let mut handled = false;
                if let Some(req_id) = value.get(#resp_field).and_then(|v| v.as_u64()) {
                    let mut pending = pending_clone.lock().await;
                    if let Some(tx) = pending.remove(&req_id) {
                        let _ = tx.send(value.clone());
                        handled = true;
                    }
                }
                if !handled {
                    let _ = event_tx.send(Ok(value)).await;
                }
            }
        }
    } else {
        quote! {
            Ok(value) => {
                let _ = event_tx.send(Ok(value)).await;
            }
        }
    };

    let pending_clone_decl = if has_correlation {
        quote! { let pending_clone = pending.clone(); }
    } else {
        TokenStream::new()
    };

    let pending_cleanup = if has_correlation {
        quote! {
            let mut pending = pending_clone.lock().await;
            for (_, tx) in pending.drain() {
                drop(tx);
            }
        }
    } else {
        TokenStream::new()
    };

    let client_description = format!("Client for the {} endpoint.", ep.id);

    quote! {
        #[doc = #client_description]
        pub struct #client_name {
            transport: super::ws_shared::WsTransportHandle,
            event_rx: tokio::sync::mpsc::Receiver<Result<serde_json::Value, super::ws_shared::WsError>>,
            #extra_fields
        }

        impl #client_name {
            /// Connect to the endpoint.
            pub(crate) async fn connect(
                url: String,
                _options: super::ws_shared::WsClientOptions,
                header_pairs: Vec<(String, String)>,
            ) -> Result<Self, super::ws_shared::WsError> {
                use tokio_tungstenite::connect_async;
                use tokio_tungstenite::tungstenite::client::IntoClientRequest;

                let mut request = url.into_client_request()?;
                for (name, value) in &header_pairs {
                    if let (Ok(hdr_name), Ok(hdr_value)) = (
                        name.parse::<tokio_tungstenite::tungstenite::http::header::HeaderName>(),
                        value.parse::<tokio_tungstenite::tungstenite::http::header::HeaderValue>(),
                    ) {
                        request.headers_mut().insert(hdr_name, hdr_value);
                    }
                }

                let (ws_stream, _) = connect_async(request).await?;
                let (write, read) = futures_util::StreamExt::split(ws_stream);

                let (writer_tx, writer_rx) = tokio::sync::mpsc::channel(_options.outbound_capacity);
                let (event_tx, event_rx) = tokio::sync::mpsc::channel(_options.inbound_capacity);
                let (state_tx, state_rx) = tokio::sync::watch::channel(super::ws_shared::WsConnectionState::Ready);
                let next_id = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1));
                let pending: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<u64, tokio::sync::oneshot::Sender<serde_json::Value>>>> =
                    std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));

                #pending_clone_decl

                // Spawn reader task
                let reader_handle = tokio::spawn(async move {
                    use futures_util::StreamExt;
                    use tokio_tungstenite::tungstenite::Message;

                    let mut read = read;
                    while let Some(msg_result) = read.next().await {
                        match msg_result {
                            Ok(Message::Text(text)) => {
                                match serde_json::from_str::<serde_json::Value>(text.as_ref()) {
                                    #reader_message_handler
                                    Err(e) => {
                                        let _ = event_tx.send(Err(super::ws_shared::WsError::Serde(e))).await;
                                    }
                                }
                            }
                            Ok(Message::Close(_)) => {
                                let _ = state_tx.send(super::ws_shared::WsConnectionState::Closed);
                                break;
                            }
                            Ok(Message::Ping(_)) | Ok(Message::Pong(_)) | Ok(Message::Frame(_)) => {}
                            Ok(Message::Binary(_)) => {}
                            Err(e) => {
                                let _ = event_tx.send(Err(super::ws_shared::WsError::Transport(e))).await;
                                let _ = state_tx.send(super::ws_shared::WsConnectionState::Disconnected);
                                break;
                            }
                        }
                    }

                    #pending_cleanup
                });

                // Spawn writer task
                let writer_handle = tokio::spawn(async move {
                    use futures_util::SinkExt;
                    use tokio_tungstenite::tungstenite::Message;

                    let mut write = write;
                    let mut writer_rx = writer_rx;
                    while let Some(cmd) = writer_rx.recv().await {
                        let msg = match cmd {
                            super::ws_shared::WriterCommand::SendText(text) => Message::Text(text.into()),
                            super::ws_shared::WriterCommand::SendBinary(data) => Message::Binary(data.into()),
                            super::ws_shared::WriterCommand::Close => {
                                let _ = write.close().await;
                                break;
                            }
                        };
                        if write.send(msg).await.is_err() {
                            break;
                        }
                    }
                });

                let transport = super::ws_shared::WsTransportHandle {
                    writer_tx,
                    state_rx,
                    reader_handle,
                    writer_handle,
                    next_id,
                    pending,
                };

                Ok(Self {
                    transport,
                    event_rx,
                    #constructor_extra_fields
                })
            }

            /// Send a fire-and-forget message.
            pub async fn send(
                &self,
                message: serde_json::Value,
            ) -> Result<(), super::ws_shared::WsError> {
                self.send_typed(&message).await
            }

            /// Send a strongly-typed message payload.
            pub async fn send_typed<M: super::ws_shared::WsEncode + ?Sized>(
                &self,
                message: &M,
            ) -> Result<(), super::ws_shared::WsError> {
                let text = super::ws_shared::WsEncode::ws_encode(message)?;
                self.transport
                    .writer_tx
                    .send(super::ws_shared::WriterCommand::SendText(text))
                    .await
                    .map_err(|_| super::ws_shared::WsError::Disconnected)
            }

            #(#lifecycle_helpers)*

            /// Receive the next inbound event.
            pub async fn next_event(
                &mut self,
            ) -> Option<Result<serde_json::Value, super::ws_shared::WsError>> {
                self.event_rx.recv().await
            }

            /// Returns a stream of inbound events.
            pub fn events(self) -> tokio_stream::wrappers::ReceiverStream<Result<serde_json::Value, super::ws_shared::WsError>> {
                tokio_stream::wrappers::ReceiverStream::new(self.event_rx)
            }

            /// Initiate a graceful close.
            pub async fn close(&self) -> Result<(), super::ws_shared::WsError> {
                self.transport
                    .writer_tx
                    .send(super::ws_shared::WriterCommand::Close)
                    .await
                    .map_err(|_| super::ws_shared::WsError::Disconnected)
            }

            /// Returns the current connection state.
            pub fn state(&self) -> super::ws_shared::WsConnectionState {
                self.transport.state()
            }

            #request_method
        }
    }
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    let mut prev_was_lower_or_digit = false;
    let mut prev_was_upper = false;

    while let Some(c) = chars.next() {
        let is_upper = c.is_uppercase();
        let next_is_lower = chars.peek().is_some_and(|ch| ch.is_lowercase());

        if !result.is_empty() && is_upper && (prev_was_lower_or_digit || (prev_was_upper && next_is_lower)) {
            result.push('_');
        }
        result.extend(c.to_lowercase());

        prev_was_lower_or_digit = c.is_lowercase() || c.is_ascii_digit();
        prev_was_upper = is_upper;
    }
    result
}

fn parse_schema_type_tokens(schema_type: &str) -> TokenStream {
    match schema_type.parse::<TokenStream>() {
        Ok(tokens) => tokens,
        Err(_) => quote! { serde_json::Value },
    }
}

fn generate_lifecycle_helper(method: &LifecycleMessagePlan, kind: &str) -> TokenStream {
    let method_ident = format_ident!("send_{}", to_snake_case(&method.name));
    let payload_type = parse_schema_type_tokens(&method.schema_type);
    let doc = format!(
        "Send the `{}` {} lifecycle message.",
        method.name, kind
    );

    quote! {
        #[doc = #doc]
        pub async fn #method_ident(
            &self,
            message: #payload_type,
        ) -> Result<(), super::ws_shared::WsError> {
            self.send_typed(&message).await
        }
    }
}

fn generate_lifecycle_helpers(ep: &EndpointPlan) -> Vec<TokenStream> {
    let mut methods = Vec::new();

    if let Some(ref open) = ep.lifecycle_open {
        methods.push(generate_lifecycle_helper(open, "open"));
    }
    if let Some(ref close) = ep.lifecycle_close {
        methods.push(generate_lifecycle_helper(close, "close"));
    }
    if let Some(ref keepalive) = ep.lifecycle_keepalive {
        methods.push(generate_lifecycle_helper(keepalive, "keepalive"));
    }

    methods
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws_codegen::plan::{
        CorrelationStrategy, EndpointPlan, WsAuthStrategy, WsRuntimePlan,
    };
    use schematic_define::websocket::{FrameFormat, RequestIdType};

    fn test_plan() -> WsRuntimePlan {
        WsRuntimePlan {
            api_name: "TestApi".to_string(),
            description: "Test API".to_string(),
            base_url: "wss://test.com".to_string(),
            docs_url: None,
            frame_format: FrameFormat::JsonText,
            request_id_type: RequestIdType::U64,
            supports_reconnect: true,
            auth: WsAuthStrategy::None,
            endpoints: vec![EndpointPlan {
                id: "Echo".to_string(),
                path: "/echo".to_string(),
                description: "Echo endpoint".to_string(),
                role: super::super::plan::EndpointRole::Client,
                correlation: CorrelationStrategy::None,
                heartbeat: None,
                client_messages: vec!["Msg".to_string()],
                server_messages: vec!["Reply".to_string()],
                bidirectional_messages: vec![],
                lifecycle_open: None,
                lifecycle_close: None,
                lifecycle_keepalive: None,
                has_lifecycle_open: false,
                has_lifecycle_close: false,
                has_lifecycle_keepalive: false,
                connection_param_count: 0,
            }],
            diagnostics: vec![],
        }
    }

    #[test]
    fn generate_client_module_produces_tokens() {
        let plan = test_plan();
        let tokens = generate_ws_client_module(&plan);
        let code = tokens.to_string();
        assert!(code.contains("TestApiWs"));
        assert!(code.contains("EchoClient"));
        assert!(code.contains("connect_echo"));
    }

    #[test]
    fn correlated_endpoint_has_request_method() {
        let mut plan = test_plan();
        plan.endpoints[0].correlation = CorrelationStrategy::Correlated {
            request_id_field: "id".to_string(),
            response_id_field: "req_id".to_string(),
            default_timeout_secs: 30,
        };
        let tokens = generate_ws_client_module(&plan);
        let code = tokens.to_string();
        assert!(code.contains("request"));
    }
}
