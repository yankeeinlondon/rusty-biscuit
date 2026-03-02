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
    let (accept_expr, auth_helpers) = generate_accept_and_auth_helpers(plan);

    quote! {
        /// WebSocket host server.
        pub struct #struct_name;

        impl #struct_name {
            #auth_helpers

            /// Serve WebSocket connections on the given TCP listener.
            pub async fn serve<H: WsHandler + 'static>(
                listener: tokio::net::TcpListener,
                handler: std::sync::Arc<H>,
            ) -> Result<(), super::ws_shared::WsError> {
                loop {
                    let (stream, _) = listener.accept().await
                        .map_err(|e| super::ws_shared::WsError::Protocol(e.to_string()))?;
                    let handler = handler.clone();
                    tokio::spawn(async move {
                        if let Ok(ws_stream) = #accept_expr {
                            Self::handle_connection(ws_stream, handler).await;
                        }
                    });
                }
            }

            /// Serve WebSocket connections on the given address.
            pub async fn serve_addr<H: WsHandler + 'static>(
                addr: impl tokio::net::ToSocketAddrs,
                handler: std::sync::Arc<H>,
            ) -> Result<(), super::ws_shared::WsError> {
                let listener = tokio::net::TcpListener::bind(addr).await
                    .map_err(|e| super::ws_shared::WsError::Protocol(e.to_string()))?;
                Self::serve(listener, handler).await
            }

            async fn handle_connection<H: WsHandler>(
                ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
                handler: std::sync::Arc<H>,
            ) {
                use futures_util::{SinkExt, StreamExt};
                use tokio_tungstenite::tungstenite::Message;

                let (mut write, mut read) = ws_stream.split();
                while let Some(msg_result) = read.next().await {
                    match msg_result {
                        Ok(Message::Text(text)) => {
                            match serde_json::from_str::<serde_json::Value>(text.as_ref()) {
                                Ok(request) => {
                                    if let Some(response) = handler.handle_message(request).await {
                                        if let Ok(resp_text) = serde_json::to_string(&response) {
                                            let _ = write.send(Message::Text(resp_text.into())).await;
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
            }
        }
    }
}

fn generate_accept_and_auth_helpers(plan: &WsRuntimePlan) -> (TokenStream, TokenStream) {
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
            );
        }
    };

    match strategy {
        schematic_define::AuthStrategy::ApiKey { header } => {
            let header_name = header.clone();
            let env_names = env_auth.to_vec();

            (
                quote! {
                    tokio_tungstenite::accept_hdr_async(stream, Self::validate_upgrade_request)
                        .await
                },
                quote! {
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

                    fn validate_upgrade_request(
                        request: &tokio_tungstenite::tungstenite::handshake::server::Request,
                        response: tokio_tungstenite::tungstenite::handshake::server::Response,
                    ) -> Result<
                        tokio_tungstenite::tungstenite::handshake::server::Response,
                        tokio_tungstenite::tungstenite::handshake::server::ErrorResponse,
                    > {
                        let expected_values: Vec<String> = vec![#(#env_names.to_string()),*]
                            .into_iter()
                            .filter_map(|env| std::env::var(env).ok())
                            .filter(|value| !value.trim().is_empty())
                            .collect();

                        if expected_values.is_empty() {
                            return Err(Self::auth_error_response(
                                tokio_tungstenite::tungstenite::http::StatusCode::INTERNAL_SERVER_ERROR,
                                "WebSocket host auth is configured but no auth env vars are set",
                            ));
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
                                "Invalid authentication header",
                            ))
                        }
                    }
                },
            )
        }
        schematic_define::AuthStrategy::BearerToken { header } => {
            let header_name = header
                .clone()
                .unwrap_or_else(|| "Authorization".to_string());
            let env_names = env_auth.to_vec();

            (
                quote! {
                    tokio_tungstenite::accept_hdr_async(stream, Self::validate_upgrade_request)
                        .await
                },
                quote! {
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

                    fn validate_upgrade_request(
                        request: &tokio_tungstenite::tungstenite::handshake::server::Request,
                        response: tokio_tungstenite::tungstenite::handshake::server::Response,
                    ) -> Result<
                        tokio_tungstenite::tungstenite::handshake::server::Response,
                        tokio_tungstenite::tungstenite::handshake::server::ErrorResponse,
                    > {
                        let expected_values: Vec<String> = vec![#(#env_names.to_string()),*]
                            .into_iter()
                            .filter_map(|env| std::env::var(env).ok())
                            .filter(|value| !value.trim().is_empty())
                            .collect();

                        if expected_values.is_empty() {
                            return Err(Self::auth_error_response(
                                tokio_tungstenite::tungstenite::http::StatusCode::INTERNAL_SERVER_ERROR,
                                "WebSocket host auth is configured but no auth env vars are set",
                            ));
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
                            ))
                        }
                    }
                },
            )
        }
        _ => (
            quote! { tokio_tungstenite::accept_async(stream).await },
            TokenStream::new(),
        ),
    }
}

fn generate_handler_trait(
    _plan: &WsRuntimePlan,
    _endpoints: &[&EndpointPlan],
) -> TokenStream {
    quote! {
        /// Handler trait for WebSocket host connections.
        ///
        /// Implement this trait to handle incoming WebSocket messages.
        pub trait WsHandler: Send + Sync {
            /// Handle an incoming message, optionally returning a response.
            fn handle_message(
                &self,
                message: serde_json::Value,
            ) -> impl std::future::Future<Output = Option<serde_json::Value>> + Send;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws_codegen::plan::{
        CorrelationStrategy, WsAuthStrategy, WsRuntimePlan,
    };
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
                connection_param_count: 0,
            }],
            diagnostics: vec![],
        }
    }

    #[test]
    fn generate_host_for_bidirectional_endpoint() {
        let plan = test_plan_with_host();
        let tokens = generate_ws_host_module(&plan);
        let code = tokens.to_string();
        assert!(code.contains("IntegrationWsHost"), "Expected IntegrationWsHost in: {}", code);
        assert!(code.contains("WsHandler"));
    }

    #[test]
    fn host_with_header_auth_uses_accept_hdr_async() {
        let mut plan = test_plan_with_host();
        plan.auth = WsAuthStrategy::HeaderBased {
            strategy: schematic_define::AuthStrategy::ApiKey {
                header: "auth-token".to_string(),
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
