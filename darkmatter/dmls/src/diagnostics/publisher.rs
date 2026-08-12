//! Push-diagnostics publisher.
//!
//! DMLS uses push diagnostics (v1): the server sends
//! `textDocument/publishDiagnostics` whenever a document's analysis changes.
//! Each publish carries the document `version` so a client discards a publish
//! that a newer edit has already superseded.

use crossbeam_channel::Sender;
use lsp_server::{Message, Notification};
use lsp_types::notification::{Notification as _, PublishDiagnostics};
use lsp_types::{Diagnostic, PublishDiagnosticsParams, Uri};

/// Sends `publishDiagnostics` notifications over the LSP connection.
#[derive(Clone)]
pub struct DiagnosticsPublisher {
    sender: Sender<Message>,
}

impl DiagnosticsPublisher {
    /// Wraps the connection's outbound sender.
    pub fn new(sender: Sender<Message>) -> Self {
        Self { sender }
    }

    /// Publishes `diagnostics` for `uri` at `version` (fire-and-forget).
    pub fn publish(&self, uri: Uri, version: Option<i32>, diagnostics: Vec<Diagnostic>) {
        let params = PublishDiagnosticsParams {
            uri,
            diagnostics,
            version,
        };
        if let Ok(value) = serde_json::to_value(params) {
            let notification = Notification::new(PublishDiagnostics::METHOD.to_string(), value);
            let _ = self.sender.send(Message::Notification(notification));
        }
    }
}
