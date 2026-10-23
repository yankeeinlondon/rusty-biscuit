//! Per-API typed WebSocket client generation.
//!
//! Generates client structs, connect methods, send/request methods,
//! event stream adapters, and close/state accessors.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::websocket::{FrameFormat, MessageDirection, MessageSchema, RequestIdType};

use super::plan::{
    CorrelatedPair, CorrelationStrategy, EndpointPlan, EndpointRole, LifecycleMessagePlan,
    QueryParamPlan, WsQueryForm, WsRuntimePlan, WsScalar, in_direction, inbound_directions,
};
use crate::codegen::api_struct::auth::{Placement, policy_tokens};

/// Generate the complete client module for a WS API.
///
/// Every endpoint gets its message enums; the API struct and endpoint
/// clients are emitted for endpoints with a client role.
pub fn generate_ws_client_module(plan: &WsRuntimePlan) -> TokenStream {
    let message_enums: Vec<TokenStream> = plan.endpoints.iter().map(generate_message_enums).collect();
    let client_endpoints: Vec<&EndpointPlan> = plan
        .endpoints
        .iter()
        .filter(|ep| matches!(ep.role, EndpointRole::Client | EndpointRole::Both))
        .collect();
    if client_endpoints.is_empty() {
        return quote! { #(#message_enums)* };
    }
    let api_struct = generate_api_struct(plan, &client_endpoints);
    let endpoint_clients: Vec<TokenStream> = client_endpoints
        .iter()
        .map(|ep| generate_endpoint_client(plan, ep))
        .collect();

    quote! {
        #(#message_enums)*
        #api_struct
        #(#endpoint_clients)*
    }
}

/// `{Endpoint}ServerMessage` and `{Endpoint}ClientMessage`.
fn generate_message_enums(ep: &EndpointPlan) -> TokenStream {
    let receives = inbound_directions(ep.role);
    let receives = |direction: MessageDirection| receives.iter().any(|(d, _)| *d == direction);
    let client_sends = matches!(ep.role, EndpointRole::Client | EndpointRole::Both);
    let host_sends = matches!(ep.role, EndpointRole::Host | EndpointRole::Both);
    let server = generate_message_enum(
        ep,
        MessageDirection::Server,
        host_sends,
        receives(MessageDirection::Server),
    );
    let client = generate_message_enum(
        ep,
        MessageDirection::Client,
        client_sends,
        receives(MessageDirection::Client),
    );
    quote! { #server #client }
}

/// The enum of one direction. It serializes as the payload alone where the
/// endpoint sends it, and decodes by declared selectors where the endpoint
/// receives it, so a payload needs only the serde capability its direction
/// uses. `Debug` is the only derive.
fn generate_message_enum(
    ep: &EndpointPlan,
    direction: MessageDirection,
    sends: bool,
    receives: bool,
) -> TokenStream {
    let (suffix, sender) = match direction {
        MessageDirection::Client => ("ClientMessage", "client"),
        _ => ("ServerMessage", "server"),
    };
    let name = format_ident!("{}{}", ep.id, suffix);
    let messages: Vec<&MessageSchema> = ep
        .messages
        .iter()
        .filter(|message| in_direction(message, direction))
        .collect();

    let variants = messages.iter().map(|message| {
        let variant = format_ident!("{}", message.name);
        let payload = parse_schema_type_tokens(&message.schema.full_path());
        let doc = message
            .description
            .clone()
            .unwrap_or_else(|| format!("The `{}` message.", message.name));
        quote! {
            #[doc = #doc]
            #variant(#payload),
        }
    });
    let doc = format!(
        "Messages the {} endpoint's {sender} sends. A value serializes as its payload alone.",
        ep.id
    );

    let serialize = if sends {
        let arms = messages.iter().map(|message| {
            let variant = format_ident!("{}", message.name);
            quote! { Self::#variant(payload) => serde::Serialize::serialize(payload, serializer), }
        });
        let body = if messages.is_empty() {
            // Uninhabited: no value exists to serialize.
            quote! { let _ = serializer; match *self {} }
        } else {
            quote! { match self { #(#arms)* } }
        };
        quote! {
            impl serde::Serialize for #name {
                fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    #body
                }
            }
        }
    } else {
        TokenStream::new()
    };

    let decode = if receives {
        generate_decode(&name, &messages)
    } else {
        TokenStream::new()
    };

    quote! {
        #[doc = #doc]
        #[derive(Debug)]
        pub enum #name {
            #(#variants)*
        }

        #serialize
        #decode
    }
}

/// `selected_message` and `decode`: selectors are tried most specific
/// first (validation allows two matching selector sets only when one
/// strictly contains the other), then the fallback. A direction's only
/// message, declared without selectors, takes every frame.
fn generate_decode(name: &proc_macro2::Ident, messages: &[&MessageSchema]) -> TokenStream {
    let decode_doc = quote! {
        /// Decodes an inbound frame as the message its declared selectors
        /// name, or the fallback when none matches.
        ///
        /// ## Errors
        ///
        /// [`super::ws_shared::WsError::Decode`], carrying the frame as
        /// received, when no message is selected or the selected
        /// message's payload does not decode.
    };
    if messages.is_empty() {
        return quote! {
            impl #name {
                #decode_doc
                pub fn decode(
                    frame: super::ws_shared::WsInboundFrame,
                ) -> Result<Self, super::ws_shared::WsError> {
                    Err(frame.unmatched())
                }
            }
        };
    }

    let mut selected: Vec<&MessageSchema> = messages
        .iter()
        .copied()
        .filter(|message| !message.fallback && message.selectors.as_ref().is_some_and(|s| !s.is_empty()))
        .collect();
    selected.sort_by_key(|message| std::cmp::Reverse(message.selectors.as_ref().map_or(0, Vec::len)));
    let catch_all = messages.iter().find(|message| message.fallback).or_else(|| {
        (messages.len() == 1 && messages[0].selectors.as_ref().is_none_or(Vec::is_empty))
            .then(|| &messages[0])
    });

    // A sole unselected message takes every frame without reading it.
    let unread = selected.is_empty().then(|| quote! { let _ = frame; });
    let checks = selected.iter().map(|message| {
        let message_name = &message.name;
        let pairs = message.selectors.iter().flatten().map(|selector| {
            let (field, value) = (&selector.field, &selector.value);
            quote! { (#field, #value) }
        });
        quote! {
            if frame.matches(&[#(#pairs),*]) {
                return Some(#message_name);
            }
        }
    });
    let otherwise = match catch_all {
        Some(message) => {
            let message_name = &message.name;
            quote! { Some(#message_name) }
        }
        None => quote! { None },
    };
    let arms = messages.iter().map(|message| {
        let variant = format_ident!("{}", message.name);
        let message_name = &message.name;
        quote! { Some(#message_name) => frame.decode_payload(#message_name).map(Self::#variant), }
    });

    quote! {
        impl #name {
            /// The name of the message `frame` selects, without decoding
            /// its payload; `None` when it matches no message and the
            /// direction has no fallback.
            #[must_use]
            pub fn selected_message(
                frame: &super::ws_shared::WsInboundFrame,
            ) -> Option<&'static str> {
                #unread
                #(#checks)*
                #otherwise
            }

            #decode_doc
            pub fn decode(
                frame: super::ws_shared::WsInboundFrame,
            ) -> Result<Self, super::ws_shared::WsError> {
                match Self::selected_message(&frame) {
                    #(#arms)*
                    _ => Err(frame.unmatched()),
                }
            }
        }
    }
}

fn ws_struct_name(api_name: &str, suffix: &str) -> proc_macro2::Ident {
    let base = api_name.strip_suffix("Ws").unwrap_or(api_name);
    format_ident!("{}Ws{}", base, suffix)
}

fn param_struct_name(endpoint_id: &str) -> proc_macro2::Ident {
    format_ident!("{}ConnectionParams", endpoint_id)
}

fn sanitize_ident(name: &str) -> String {
    let mut out = String::new();
    let mut chars = name.chars();

    if let Some(first) = chars.next() {
        if first.is_ascii_alphabetic() || first == '_' {
            out.push(first.to_ascii_lowercase());
        } else {
            out.push('_');
            if first.is_ascii_digit() {
                out.push(first);
            }
        }
    }

    for ch in chars {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }

    if out.is_empty() {
        "_param".to_string()
    } else {
        out
    }
}

fn param_ident(name: &str) -> proc_macro2::Ident {
    format_ident!("{}", sanitize_ident(name))
}

fn ws_scalar_tokens(scalar: WsScalar) -> TokenStream {
    match scalar {
        WsScalar::String => quote! { String },
        WsScalar::Integer => quote! { i64 },
        WsScalar::Number => quote! { f64 },
        WsScalar::Boolean => quote! { bool },
    }
}

/// The field type of a query parameter: its scalar, or a `Vec` of it for an
/// array form.
fn query_value_tokens(param: &QueryParamPlan) -> TokenStream {
    let scalar = ws_scalar_tokens(param.scalar);
    match param.form {
        WsQueryForm::Single => scalar,
        WsQueryForm::Exploded | WsQueryForm::Delimited(_) => quote! { Vec<#scalar> },
    }
}

/// Statements pushing `(name, encoded value)` pairs for `value`, a reference
/// to the field's value type. Values are percent-encoded here; a delimiter
/// is already in its wire form.
fn query_push_tokens(param: &QueryParamPlan) -> TokenStream {
    let name = &param.name;
    match param.form {
        WsQueryForm::Single => quote! {
            query_pairs.push((#name, urlencoding::encode(&value.to_string()).into_owned()));
        },
        WsQueryForm::Exploded => quote! {
            if value.is_empty() {
                query_pairs.push((#name, String::new()));
            }
            for element in value {
                query_pairs.push((#name, urlencoding::encode(&element.to_string()).into_owned()));
            }
        },
        WsQueryForm::Delimited(delimiter) => quote! {
            let joined = value
                .iter()
                .map(|element| urlencoding::encode(&element.to_string()).into_owned())
                .collect::<Vec<_>>()
                .join(#delimiter);
            query_pairs.push((#name, joined));
        },
    }
}

fn generate_connection_param_struct(ep: &EndpointPlan) -> TokenStream {
    if ep.query_params.is_empty() {
        return TokenStream::new();
    }

    let struct_name = param_struct_name(&ep.id);
    let all_optional = ep.query_params.iter().all(|p| !p.required);
    let derives = if all_optional {
        quote! { #[derive(Debug, Clone, Default)] }
    } else {
        quote! { #[derive(Debug, Clone)] }
    };

    let fields: Vec<TokenStream> = ep
        .query_params
        .iter()
        .map(|param| {
            let ident = param_ident(&param.name);
            let base_ty = query_value_tokens(param);
            let ty = if param.required {
                quote! { #base_ty }
            } else {
                quote! { Option<#base_ty> }
            };
            let doc = param
                .description
                .as_deref()
                .unwrap_or("Connection query parameter.");
            quote! {
                #[doc = #doc]
                pub #ident: #ty,
            }
        })
        .collect();

    let doc = format!(
        "Typed query parameters for the {} websocket endpoint.",
        ep.id
    );
    quote! {
        #[doc = #doc]
        #derives
        pub struct #struct_name {
            #(#fields)*
        }
    }
}

/// The headers a credential placement writes: a caller header of one of
/// these names would be a second credential source, so connect refuses it.
fn credential_header_names(plan: &WsRuntimePlan) -> Vec<String> {
    let config = &plan.handshake_auth;
    let mut names: Vec<String> = Vec::new();
    for placement in config.accepted.iter().chain(config.env.iter().map(|env| &env.placement)) {
        let name = match placement {
            Placement::Bearer { header } => header.clone().unwrap_or_else(|| "Authorization".to_string()),
            Placement::ApiKeyHeader { header, .. } => header.clone(),
            Placement::Basic => "Authorization".to_string(),
            Placement::ApiKeyQuery { .. } | Placement::ApiKeyCookie { .. } => continue,
        };
        if !names.iter().any(|known| known.eq_ignore_ascii_case(&name)) {
            names.push(name);
        }
    }
    names
}

/// `api_key`, when the API accepts a single-secret credential: the first
/// accepted placement that is not Basic decides whether the key is sent as
/// an access token or an API key.
fn generate_api_key_method(plan: &WsRuntimePlan) -> TokenStream {
    let Some(placement) = plan
        .handshake_auth
        .accepted
        .iter()
        .find(|placement| !matches!(placement, Placement::Basic))
    else {
        return TokenStream::new();
    };
    let credential = match placement {
        Placement::Bearer { .. } => quote! { schematic_runtime::Credential::access_token(key) },
        _ => quote! { schematic_runtime::Credential::api_key(key) },
    };
    quote! {
        /// Sets the client-level credential to `key`, of the kind the API's
        /// first single-secret method takes.
        #[must_use]
        pub fn api_key(self, key: impl Into<String>) -> Self {
            self.credentials(#credential)
        }
    }
}

fn generate_api_struct(plan: &WsRuntimePlan, endpoints: &[&EndpointPlan]) -> TokenStream {
    let struct_name = ws_struct_name(&plan.api_name, "");
    let base_url = &plan.base_url;
    let description = &plan.description;
    let auth_config = policy_tokens(&plan.handshake_auth);
    let constant_headers = plan.headers.iter().map(|(name, value)| {
        quote! { (#name.to_string(), #value.to_string()) }
    });
    let api_key_method = generate_api_key_method(plan);
    let connection_param_structs: Vec<TokenStream> = endpoints
        .iter()
        .map(|ep| generate_connection_param_struct(ep))
        .filter(|tokens| !tokens.is_empty())
        .collect();
    let connect_methods: Vec<TokenStream> = endpoints
        .iter()
        .map(|ep| generate_connect_method(plan, ep))
        .collect();

    quote! {
        #(#connection_param_structs)*

        #[doc = #description]
        ///
        /// Each `connect_*` selects one handshake credential, in this order:
        /// the options' `credential`, the client's [`Self::credentials`], the
        /// first non-empty variable of the definition's environment mapping,
        /// none. Reconnects reuse that selection.
        pub struct #struct_name {
            base_url: String,
            headers: Vec<(String, String)>,
            credential: Option<schematic_runtime::Credential>,
        }

        impl #struct_name {
            /// Create a new client with the default base URL.
            #[must_use]
            pub fn new() -> Self {
                Self::with_base_url(#base_url)
            }

            /// Create a new client with a custom base URL.
            #[must_use]
            pub fn with_base_url(base_url: impl Into<String>) -> Self {
                Self {
                    base_url: base_url.into(),
                    headers: vec![#(#constant_headers),*],
                    credential: None,
                }
            }

            /// How the handshake accepts credentials, translated from the
            /// definition's authentication policy as for a REST client.
            #[must_use]
            pub fn auth_config() -> schematic_runtime::AuthConfig {
                #auth_config
            }

            /// Returns the configured base URL.
            #[must_use]
            pub fn base_url(&self) -> &str {
                &self.base_url
            }

            /// Sets the client-level credential, used by every connect whose
            /// options carry none.
            #[must_use]
            pub fn credentials(mut self, credential: schematic_runtime::Credential) -> Self {
                self.credential = Some(credential);
                self
            }

            #api_key_method

            /// Sends `name: value` with every handshake, replacing a constant
            /// header of the definition with the same name.
            ///
            /// A header the WebSocket upgrade manages, or one that carries
            /// the API's credential, is refused when connecting.
            #[must_use]
            pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
                self.headers.push((name.into(), value.into()));
                self
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

fn generate_connect_method(plan: &WsRuntimePlan, ep: &EndpointPlan) -> TokenStream {
    let method_name = format_ident!("connect_{}", to_snake_case(&ep.id));
    let client_name = format_ident!("{}Client", ep.id);
    let path = &ep.path;
    let params_struct_name = param_struct_name(&ep.id);
    let credential_headers = credential_header_names(plan);

    let mut method_args: Vec<TokenStream> = ep
        .path_params
        .iter()
        .map(|param| {
            let ident = param_ident(&param.name);
            match param.scalar {
                WsScalar::String => quote! { #ident: impl Into<String> },
                scalar => {
                    let ty = ws_scalar_tokens(scalar);
                    quote! { #ident: #ty }
                }
            }
        })
        .collect();

    if !ep.query_params.is_empty() {
        method_args.push(quote! { params: #params_struct_name });
    }
    method_args.push(quote! { options: super::ws_shared::WsClientOptions });

    let path_substitutions: Vec<TokenStream> = ep
        .path_params
        .iter()
        .map(|param| {
            let ident = param_ident(&param.name);
            let placeholder = format!("{{{}}}", param.name);
            let value_ident = format_ident!("{}_value", ident);
            let value_expr = match param.scalar {
                WsScalar::String => quote! { #ident.into() },
                _ => quote! { #ident.to_string() },
            };

            quote! {
                let #value_ident = #value_expr;
                path = path.replace(#placeholder, urlencoding::encode(&#value_ident).as_ref());
            }
        })
        .collect();

    let query_collectors: Vec<TokenStream> = ep
        .query_params
        .iter()
        .map(|param| {
            let ident = param_ident(&param.name);
            let push = query_push_tokens(param);
            if param.required {
                quote! {
                    {
                        let value = &params.#ident;
                        #push
                    }
                }
            } else {
                quote! {
                    if let Some(value) = params.#ident.as_ref() {
                        #push
                    }
                }
            }
        })
        .collect();

    let query_assembly = if ep.query_params.is_empty() {
        TokenStream::new()
    } else {
        quote! {
            let mut query_pairs: Vec<(&str, String)> = Vec::new();
            #(#query_collectors)*
            if !query_pairs.is_empty() {
                // Stable: repeated values keep their order.
                query_pairs.sort_by(|a, b| a.0.cmp(b.0));
                let query = query_pairs
                    .into_iter()
                    .map(|(k, v)| format!("{}={}", urlencoding::encode(k), v))
                    .collect::<Vec<_>>()
                    .join("&");
                path.push('?');
                path.push_str(&query);
            }
        }
    };
    let path_binding = if ep.path_params.is_empty() && ep.query_params.is_empty() {
        quote! { let path = #path.to_string(); }
    } else {
        quote! { let mut path = #path.to_string(); }
    };

    let description = format!(
        "Connect to the {} endpoint.\n\n\
         ## Errors\n\n\
         [`super::ws_shared::WsError::InvalidCredential`] when the selected credential is of a \
         kind the API does not accept or a header, query value, or cookie of the request \
         already carries it; [`super::ws_shared::WsError::InvalidHeader`] for a header the \
         upgrade manages; [`super::ws_shared::WsError::HandshakeRejected`] when the server \
         answers the upgrade with another status, including a redirect, which is never \
         followed.",
        ep.id
    );

    quote! {
        #[doc = #description]
        pub async fn #method_name(
            &self,
            #(#method_args),*
        ) -> Result<#client_name, super::ws_shared::WsError> {
            #path_binding
            #(#path_substitutions)*

            // Checked before the query is added, so the error never shows a query value.
            if path.contains('{') {
                return Err(super::ws_shared::WsError::Protocol(
                    format!("unresolved path placeholder in '{}'", path),
                ));
            }
            #query_assembly

            let auth = Self::auth_config();
            let credential = auth
                .select_handshake(options.credential.as_ref(), self.credential.as_ref())
                .map_err(|error| super::ws_shared::WsError::InvalidCredential(error.to_string()))?;
            let handshake = super::ws_shared::WsHandshake::new(
                &format!("{}{}", self.base_url, path),
                &self.headers,
                &[#(#credential_headers),*],
                &credential,
                auth.secret_names(),
            )?;
            #client_name::connect(handshake, options).await
        }
    }
}

/// The typed request method of one `responds_with` pair: it sends the
/// request and returns the payload of its declared response.
fn generate_request_method(pair: &CorrelatedPair) -> TokenStream {
    let method = format_ident!("request_{}", to_snake_case(&pair.request));
    let request_type = parse_schema_type_tokens(&pair.request_schema);
    let response_type = parse_schema_type_tokens(&pair.response_schema);
    let response = &pair.response;
    let doc = format!(
        "Sends `{}` and returns the payload of the `{}` that answers it.\n\n\
         The request id is allocated by the client and written over any id the \
         payload carries. Only a frame selected as `{}` with the same id completes the \
         request; another message carrying that id stays an ordinary event.\n\n\
         ## Errors\n\n\
         [`super::ws_shared::WsError::RequestTimeout`] when no answer arrives in the \
         request timeout, [`super::ws_shared::WsError::Disconnected`] when the connection \
         is lost first (the request is never re-sent), \
         [`super::ws_shared::WsError::BackpressureFull`] at the pending-request limit, and \
         [`super::ws_shared::WsError::Decode`] when the answer does not decode.",
        pair.request, pair.response, pair.response
    );
    quote! {
        #[doc = #doc]
        pub async fn #method(
            &self,
            message: #request_type,
        ) -> Result<#response_type, super::ws_shared::WsError> {
            self.correlate(&message, Some(#response))
                .await?
                .decode_payload(#response)
        }
    }
}

/// `request_value`, the private `correlate` both request forms share, and
/// one typed method per declared pair.
fn generate_correlation_methods(ep: &EndpointPlan) -> TokenStream {
    let CorrelationStrategy::Correlated {
        request_id_field,
        id_type,
        pairs,
        ..
    } = &ep.correlation
    else {
        return TokenStream::new();
    };
    let string_ids = matches!(id_type, RequestIdType::String);
    let typed = pairs.iter().map(generate_request_method);
    quote! {
        #(#typed)*

        /// Sends `message` with a client-allocated request id and returns
        /// the first inbound frame carrying that id, as a JSON value,
        /// whatever message it is.
        ///
        /// ## Errors
        ///
        /// As the typed request methods; also
        /// [`super::ws_shared::WsError::Protocol`] when `message` is not a
        /// JSON object.
        pub async fn request_value(
            &self,
            message: serde_json::Value,
        ) -> Result<serde_json::Value, super::ws_shared::WsError> {
            self.correlate(&message, None)
                .await
                .map(super::ws_shared::WsInboundFrame::into_value)
        }

        /// Sends `message` with the next request id and waits for the frame
        /// that answers it: one selected as `response`, or any frame with
        /// the id when `response` is `None`. The pending entry is removed on
        /// every exit, including cancellation.
        async fn correlate<M: serde::Serialize + ?Sized>(
            &self,
            message: &M,
            response: Option<&'static str>,
        ) -> Result<super::ws_shared::WsInboundFrame, super::ws_shared::WsError> {
            use std::sync::atomic::Ordering;

            let mut value = serde_json::to_value(message)?;
            let Some(object) = value.as_object_mut() else {
                return Err(super::ws_shared::WsError::Protocol(
                    "a correlated request must serialize as a JSON object".to_string(),
                ));
            };
            // Ids come from one counter per connect, never reset by a redial.
            let id = self.transport.next_id.fetch_add(1, Ordering::Relaxed);
            let (key, wire_id) = super::ws_shared::request_id(id, #string_ids);
            object.insert(#request_id_field.to_string(), wire_id);

            let (reply, answer) = tokio::sync::oneshot::channel();
            let _pending = super::ws_shared::PendingGuard::insert(
                &self.transport.pending,
                key,
                response,
                reply,
                self.max_pending,
            )?;
            self.send_payload(&value).await?;

            let timeout = self.request_timeout;
            match tokio::time::timeout(timeout, answer).await {
                Ok(Ok(frame)) => Ok(frame),
                Ok(Err(_)) => Err(super::ws_shared::WsError::Disconnected),
                Err(_) => Err(super::ws_shared::WsError::RequestTimeout(timeout.as_secs())),
            }
        }
    }
}

fn generate_endpoint_client(plan: &WsRuntimePlan, ep: &EndpointPlan) -> TokenStream {
    let client_name = format_ident!("{}Client", ep.id);
    let has_correlation = matches!(ep.correlation, CorrelationStrategy::Correlated { .. });
    let lifecycle_helpers = generate_lifecycle_helpers(ep);
    let correlation_methods = generate_correlation_methods(ep);

    let encode_command = match plan.frame_format {
        FrameFormat::JsonText => quote! {
            super::ws_shared::WriterCommand::SendText(serde_json::to_string(message)?)
        },
        FrameFormat::JsonBinary | FrameFormat::RawBinary | FrameFormat::Mixed => quote! {
            super::ws_shared::WriterCommand::SendBinary(serde_json::to_vec(message)?)
        },
    };
    let client_message = format_ident!("{}ClientMessage", ep.id);
    let server_message = format_ident!("{}ServerMessage", ep.id);

    let constructor_extra_fields = if has_correlation {
        quote! {
            request_timeout: options.request_timeout,
            max_pending: options.max_pending_requests,
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

    let reader_message_handler = if let CorrelationStrategy::Correlated {
        ref response_id_field,
        id_type,
        ..
    } = ep.correlation
    {
        let string_ids = matches!(id_type, RequestIdType::String);
        quote! {
            Ok(frame) => {
                if let Some(frame) = super::ws_shared::answer_pending(
                    &pending_for_supervisor,
                    frame,
                    #response_id_field,
                    #string_ids,
                    #server_message::selected_message,
                ) {
                    let _ = event_tx.send(Ok(frame)).await;
                }
            }
        }
    } else {
        quote! {
            Ok(frame) => {
                let _ = event_tx.send(Ok(frame)).await;
            }
        }
    };

    let pending_cleanup = if has_correlation {
        quote! {
            // Dropping each reply wakes its caller with `Disconnected`.
            super::ws_shared::fail_pending(&pending_for_supervisor);
        }
    } else {
        TokenStream::new()
    };
    let pending_for_supervisor_decl = if has_correlation {
        quote! { let pending_for_supervisor = pending.clone(); }
    } else {
        TokenStream::new()
    };

    let client_docs = endpoint_client_docs(ep);

    quote! {
        #(#[doc = #client_docs])*
        pub struct #client_name {
            transport: super::ws_shared::WsTransportHandle,
            event_rx: tokio::sync::mpsc::Receiver<
                Result<super::ws_shared::WsInboundFrame, super::ws_shared::WsError>,
            >,
            auth_method: schematic_runtime::AuthMethod,
            #extra_fields
        }

        impl #client_name {
            /// Connect to the endpoint.
            pub(crate) async fn connect(
                handshake: super::ws_shared::WsHandshake,
                options: super::ws_shared::WsClientOptions,
            ) -> Result<Self, super::ws_shared::WsError> {
                let ws_stream = handshake.dial(&options).await?;
                let auth_method = handshake.auth_method().clone();

                let _receive_timeout = options.receive_timeout;
                // TODO: Enforce receive_timeout as an idle inbound timeout in the supervisor loop.

                let (writer_tx, mut writer_rx) = tokio::sync::mpsc::channel(options.outbound_capacity);
                let (event_tx, event_rx) = tokio::sync::mpsc::channel(options.inbound_capacity);
                let (state_tx, state_rx) = tokio::sync::watch::channel(super::ws_shared::WsConnectionState::Connecting);
                let next_id = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1));
                let pending = super::ws_shared::PendingRequests::default();

                let options_for_supervisor = options.clone();
                #pending_for_supervisor_decl

                let supervisor_handle = tokio::spawn(async move {
                    use futures_util::{SinkExt, StreamExt};
                    use tokio_tungstenite::tungstenite::Message;

                    let mut active_stream = Some(ws_stream);
                    let reconnect_policy = options_for_supervisor.reconnect.clone();
                    let mut reconnect_attempt: u32 = 0;
                    let mut manual_close = false;

                    loop {
                        let stream = if let Some(stream) = active_stream.take() {
                            reconnect_attempt = 0;
                            stream
                        } else {
                            let Some(policy) = reconnect_policy.as_ref() else {
                                let _ = state_tx.send(super::ws_shared::WsConnectionState::Closed);
                                break;
                            };

                            if let Some(max_attempts) = policy.max_attempts
                                && reconnect_attempt >= max_attempts
                            {
                                let _ = state_tx.send(super::ws_shared::WsConnectionState::Closed);
                                break;
                            }

                            let _ = state_tx.send(super::ws_shared::WsConnectionState::Connecting);
                            let delay = super::ws_shared::reconnect_delay(policy, reconnect_attempt);
                            tokio::time::sleep(delay).await;

                            // The credential selected by the explicit connect is reused.
                            match handshake.dial(&options_for_supervisor).await {
                                Ok(stream) => {
                                    reconnect_attempt = reconnect_attempt.saturating_add(1);
                                    stream
                                }
                                Err(err) => {
                                    // A rejected credential or a redirect would be refused again.
                                    let terminal = err.is_terminal_handshake_rejection();
                                    let _ = event_tx.send(Err(err)).await;
                                    if terminal {
                                        let _ = state_tx.send(super::ws_shared::WsConnectionState::Closed);
                                        break;
                                    }
                                    reconnect_attempt = reconnect_attempt.saturating_add(1);
                                    continue;
                                }
                            }
                        };

                        let (mut write, mut read) = stream.split();
                        let _ = state_tx.send(super::ws_shared::WsConnectionState::Ready);

                        loop {
                            tokio::select! {
                                cmd = writer_rx.recv() => {
                                    match cmd {
                                        Some(super::ws_shared::WriterCommand::SendText(text)) => {
                                            if write.send(Message::Text(text.into())).await.is_err() {
                                                break;
                                            }
                                        }
                                        Some(super::ws_shared::WriterCommand::SendBinary(data)) => {
                                            if write.send(Message::Binary(data.into())).await.is_err() {
                                                break;
                                            }
                                        }
                                        Some(super::ws_shared::WriterCommand::Close) => {
                                            manual_close = true;
                                            let _ = state_tx.send(super::ws_shared::WsConnectionState::Closing);
                                            let _ = write.close().await;
                                            break;
                                        }
                                        None => {
                                            manual_close = true;
                                            let _ = state_tx.send(super::ws_shared::WsConnectionState::Closing);
                                            let _ = write.close().await;
                                            break;
                                        }
                                    }
                                }
                                msg_result = read.next() => {
                                    match msg_result {
                                        Some(Ok(Message::Text(text))) => {
                                            let raw = super::ws_shared::WsRawFrame::Text(text.to_string());
                                            match super::ws_shared::WsInboundFrame::parse(raw) {
                                                #reader_message_handler
                                                Err(e) => {
                                                    let _ = event_tx.send(Err(e)).await;
                                                }
                                            }
                                        }
                                        Some(Ok(Message::Binary(data))) => {
                                            let raw = super::ws_shared::WsRawFrame::Binary(data.to_vec());
                                            match super::ws_shared::WsInboundFrame::parse(raw) {
                                                #reader_message_handler
                                                Err(e) => {
                                                    let _ = event_tx.send(Err(e)).await;
                                                }
                                            }
                                        }
                                        Some(Ok(Message::Close(_))) => break,
                                        Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => {}
                                        Some(Err(e)) => {
                                            let _ = event_tx.send(Err(super::ws_shared::WsError::Transport(e))).await;
                                            break;
                                        }
                                        None => break,
                                    }
                                }
                            }
                        }

                        #pending_cleanup

                        if !manual_close {
                            // Commands queued for the lost connection are dropped, never
                            // written to its replacement.
                            manual_close = super::ws_shared::discard_queued(&mut writer_rx);
                        }

                        if manual_close {
                            let _ = state_tx.send(super::ws_shared::WsConnectionState::Closed);
                            break;
                        }

                        let _ = state_tx.send(super::ws_shared::WsConnectionState::Disconnected);
                        active_stream = None;
                    }
                });

                let transport = super::ws_shared::WsTransportHandle {
                    writer_tx,
                    state_rx,
                    supervisor_handle,
                    next_id,
                    pending,
                };

                Ok(Self {
                    transport,
                    event_rx,
                    auth_method,
                    #constructor_extra_fields
                })
            }

            /// Where the handshake credential came from. It names the source
            /// selected when connecting, not proof that the server accepted it.
            pub fn auth_method(&self) -> &schematic_runtime::AuthMethod {
                &self.auth_method
            }

            /// Sends one message; it is written as its payload alone.
            pub async fn send(
                &self,
                message: #client_message,
            ) -> Result<(), super::ws_shared::WsError> {
                self.send_payload(&message).await
            }

            /// Sends an arbitrary JSON value, bypassing the declared messages.
            pub async fn send_value(
                &self,
                message: serde_json::Value,
            ) -> Result<(), super::ws_shared::WsError> {
                self.send_payload(&message).await
            }

            async fn send_payload<M: serde::Serialize + ?Sized>(
                &self,
                message: &M,
            ) -> Result<(), super::ws_shared::WsError> {
                let command = #encode_command;
                self.transport
                    .writer_tx
                    .send(command)
                    .await
                    .map_err(|_| super::ws_shared::WsError::Disconnected)
            }

            #(#lifecycle_helpers)*

            /// Receives the next inbound message, decoded as the message its
            /// selectors name.
            ///
            /// A frame that is not JSON, matches no message, or does not
            /// decode yields [`super::ws_shared::WsError::Decode`] carrying
            /// the frame; the connection stays open. Shares one queue with
            /// [`Self::next_value_event`]: each message is delivered once.
            pub async fn next_event(
                &mut self,
            ) -> Option<Result<#server_message, super::ws_shared::WsError>> {
                self.event_rx
                    .recv()
                    .await
                    .map(|event| event.and_then(#server_message::decode))
            }

            /// Receives the next inbound message as a JSON value, without
            /// selecting a declared message.
            pub async fn next_value_event(
                &mut self,
            ) -> Option<Result<serde_json::Value, super::ws_shared::WsError>> {
                self.event_rx
                    .recv()
                    .await
                    .map(|event| event.map(super::ws_shared::WsInboundFrame::into_value))
            }

            /// A stream of the messages [`Self::next_event`] would return. It
            /// keeps the connection open until it is dropped.
            pub fn events(self) -> super::ws_shared::WsEventStream<#server_message> {
                super::ws_shared::WsEventStream::new(self.event_rx, #server_message::decode, self.transport)
            }

            /// A stream of the values [`Self::next_value_event`] would return.
            pub fn value_events(self) -> super::ws_shared::WsEventStream<serde_json::Value> {
                super::ws_shared::WsEventStream::new(
                    self.event_rx,
                    |frame| Ok(frame.into_value()),
                    self.transport,
                )
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

            #correlation_methods
        }
    }
}

pub(crate) fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    let mut prev_was_lower_or_digit = false;
    let mut prev_was_upper = false;

    while let Some(c) = chars.next() {
        let is_upper = c.is_uppercase();
        let next_is_lower = chars.peek().is_some_and(|ch| ch.is_lowercase());

        if !result.is_empty()
            && is_upper
            && (prev_was_lower_or_digit || (prev_was_upper && next_is_lower))
        {
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

/// Rustdoc lines of an endpoint client. A declared heartbeat is stated
/// with its interval and message: heartbeats are caller-managed, so the
/// caller must know to send them.
fn endpoint_client_docs(ep: &EndpointPlan) -> Vec<String> {
    let mut lines = vec![format!(" Client for the {} endpoint.", ep.id)];
    let Some(heartbeat) = &ep.heartbeat else {
        return lines;
    };
    let message = ep.lifecycle_keepalive.as_ref().map_or_else(
        || "an application heartbeat message".to_string(),
        |keepalive| {
            format!(
                "the `{}` keepalive message (`send_{}`)",
                keepalive.name,
                to_snake_case(&keepalive.name)
            )
        },
    );
    let reply = heartbeat
        .timeout_secs
        .map(|timeout| format!(", and expects the reply within {timeout} s"))
        .unwrap_or_default();
    lines.extend([
        String::new(),
        " ## Heartbeat".to_string(),
        String::new(),
        format!(
            " The server expects {message} every {} s{reply}. This client does not send it: \
             schedule it from the caller.",
            heartbeat.interval_secs
        ),
    ]);
    lines
}

fn generate_lifecycle_helper(method: &LifecycleMessagePlan, kind: &str) -> TokenStream {
    let method_ident = format_ident!("send_{}", to_snake_case(&method.name));
    let payload_type = parse_schema_type_tokens(&method.schema_type);
    let doc = format!("Send the `{}` {} lifecycle message.", method.name, kind);

    quote! {
        #[doc = #doc]
        pub async fn #method_ident(
            &self,
            message: #payload_type,
        ) -> Result<(), super::ws_shared::WsError> {
            self.send_payload(&message).await
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
        CorrelatedPair, CorrelationStrategy, EndpointPlan, WsAuthStrategy, WsRuntimePlan,
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
            handshake_auth: Default::default(),
            headers: vec![],
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
            id_type: RequestIdType::U64,
            pairs: vec![CorrelatedPair {
                request: "Msg".to_string(),
                request_schema: "MsgPayload".to_string(),
                response: "Reply".to_string(),
                response_schema: "ReplyPayload".to_string(),
            }],
        };
        let tokens = generate_ws_client_module(&plan);
        let code = tokens.to_string();
        assert!(code.contains("fn request_msg"), "{code}");
        assert!(code.contains("fn request_value"), "{code}");
    }
}
