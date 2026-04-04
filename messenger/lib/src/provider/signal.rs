use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::capabilities::CapabilitySet;
use crate::dispatch::Dispatch;
use crate::error::MessengerError;
use crate::prepared::PreparedMessage;
use crate::receipt::{MessageRef, ProviderKind, SendReceipt, SignalAuthor, SignalThreadKey};
use crate::target::{SignalAddress, SignalTarget, Target};

/// Configuration for the Signal provider.
pub struct SignalConfig {
    /// URL of the signal-cli JSON-RPC daemon (e.g., `http://localhost:7583`).
    pub rpc_url: String,
    /// The registered Signal account to send from.
    pub account: String,
}

/// Signal provider adapter using signal-cli's JSON-RPC interface via reqwest.
pub struct SignalProvider {
    rpc_url: String,
    account: String,
    client: reqwest::Client,
    request_id: AtomicU64,
}

impl SignalProvider {
    pub fn new(config: SignalConfig) -> Self {
        Self {
            rpc_url: config.rpc_url,
            account: config.account,
            client: reqwest::Client::new(),
            request_id: AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::Relaxed)
    }
}

#[derive(Serialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: &'static str,
    method: &'a str,
    id: u64,
    params: serde_json::Value,
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    #[serde(default)]
    code: Option<i64>,
    #[serde(default)]
    message: Option<String>,
}

fn address_to_string(addr: &SignalAddress) -> String {
    match addr {
        SignalAddress::Phone(p) => p.clone(),
        SignalAddress::Uuid(u) => u.clone(),
    }
}

#[async_trait::async_trait]
impl super::Provider for SignalProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Signal
    }

    fn capabilities(&self) -> CapabilitySet {
        const SIGNAL_CAPABILITIES: CapabilitySet = CapabilitySet {
            supports_markdown_rendering: false,
            supports_reply: true,
            supports_attachments: false,
            supports_location: true,
            supports_silent_delivery: false,
            supports_link_preview_control: false,
        };
        SIGNAL_CAPABILITIES
    }

    #[tracing::instrument(skip_all, fields(provider = "signal", target_kind = tracing::field::Empty))]
    async fn send_prepared(
        &self,
        dispatch: &Dispatch,
        message: &PreparedMessage,
    ) -> Result<SendReceipt, MessengerError> {
        let target = match &dispatch.target {
            Target::Signal(t) => t,
            _ => {
                return Err(MessengerError::InvalidMessage(
                    "expected Signal target".into(),
                ));
            }
        };

        // Render body as plain text (with location text fallback)
        let text = message.render_body_with_location(ProviderKind::Signal);

        // Build JSON-RPC params based on target type
        let (method, target_kind, mut params) = match target {
            SignalTarget::User(addr) => {
                let params = serde_json::json!({
                    "account": self.account,
                    "recipient": [address_to_string(addr)],
                    "message": text,
                });
                ("send", "user", params)
            }
            SignalTarget::Group { group_id_base64 } => {
                let params = serde_json::json!({
                    "account": self.account,
                    "groupId": group_id_base64,
                    "message": text,
                });
                ("sendGroupMessage", "group", params)
            }
            SignalTarget::NoteToSelf => {
                let params = serde_json::json!({
                    "account": self.account,
                    "recipient": [&self.account],
                    "message": text,
                    "noteToSelf": true,
                });
                ("send", "note_to_self", params)
            }
        };
        tracing::Span::current().record("target_kind", target_kind);

        // Add reply quote metadata if present
        if let Some(MessageRef::Signal {
            author,
            timestamp_ms,
            ..
        }) = &dispatch.reply_to
        {
            let author_str = match author {
                SignalAuthor::Phone(p) => p.clone(),
                SignalAuthor::Uuid(u) => u.clone(),
            };
            params["quoteAuthor"] = serde_json::json!(author_str);
            params["quoteTimestamp"] = serde_json::json!(timestamp_ms);
        }

        let rpc_request = JsonRpcRequest {
            jsonrpc: "2.0",
            method,
            id: self.next_id(),
            params,
        };
        tracing::debug!(
            method,
            request_id = rpc_request.id,
            has_reply = dispatch.reply_to.is_some(),
            text_len = text.len(),
            "sending Signal JSON-RPC request"
        );

        let response = self
            .client
            .post(&self.rpc_url)
            .json(&rpc_request)
            .send()
            .await
            .map_err(|e| ProviderKind::Signal.transport_error(e))?;
        tracing::debug!(status = %response.status(), "received Signal response");

        let resp: JsonRpcResponse =
            super::http_helpers::handle_http_response(response, ProviderKind::Signal).await?;

        if let Some(error) = resp.error {
            let msg = error.message.unwrap_or_else(|| "unknown error".to_string());
            tracing::warn!(
                code = ?error.code,
                message = %msg,
                "Signal JSON-RPC returned an error"
            );
            return Err(MessengerError::Provider {
                provider: ProviderKind::Signal,
                code: error.code.map(|c| c.to_string()),
                message: msg,
            });
        }

        // Extract timestamp from result for the message ref
        let timestamp_ms = resp
            .result
            .as_ref()
            .and_then(|r| r.get("timestamp"))
            .and_then(|t| t.as_i64())
            .unwrap_or(0);

        let thread_key = match target {
            SignalTarget::User(addr) => SignalThreadKey::Direct(address_to_string(addr)),
            SignalTarget::Group { group_id_base64 } => {
                SignalThreadKey::Group(group_id_base64.clone())
            }
            SignalTarget::NoteToSelf => SignalThreadKey::Direct(self.account.clone()),
        };

        let author = match &self.account {
            a if a.starts_with('+') => SignalAuthor::Phone(a.clone()),
            a => SignalAuthor::Uuid(a.clone()),
        };
        tracing::debug!(raw_id = %timestamp_ms, "Signal message sent");

        Ok(SendReceipt {
            provider: ProviderKind::Signal,
            message_ref: MessageRef::Signal {
                thread: thread_key,
                author,
                timestamp_ms,
            },
            raw_id: timestamp_ms.to_string(),
            metadata: BTreeMap::new(),
        })
    }
}
