//! Stream-error detection and the consecutive-repeat backstop.
//!
//! The bridge calls [`is_stream_error`] and [`stream_error_fingerprint`] on
//! every structured record to decide whether a `message="stream error"`
//! failure is accumulating. When the identical fingerprint repeats
//! [`MAX_CONSECUTIVE_STREAM_ERRORS`] times with no intervening step advance,
//! [`on_repeated_stream_error`] forces a fail-fast abort so OpenCode's
//! unbounded backoff retries do not spin forever.

use serde_json::{Value, json};

use crate::stream::logs::opencode::classify::error_context;
use crate::stream::logs::opencode::events::OpenCodeLogRecord;
use crate::stream::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};

use super::format::base_extra;
use super::{EarlyTermination, OpenCodeLogBridge, StderrIngestOutcome};

/// Consecutive unrecognized `stream error` records tolerated before the bridge
/// forces a fail-fast abort. Defense-in-depth backstop for OpenCode log-format
/// drift (see fixes/2026-06-21-opencode-log-fix); the classifier handles known
/// terminal caps on the first error, so this only catches future/unknown shapes.
pub(super) const MAX_CONSECUTIVE_STREAM_ERRORS: u32 = 5;

/// Whether a parsed record is an OpenCode `message="stream error"` failure
/// record (1.17.8 format). The keyword lands in the trailing `message` field,
/// or in the `message` tag when a later tag absorbed the rest of the line.
pub(super) fn is_stream_error(record: &OpenCodeLogRecord) -> bool {
    record.message == "stream error"
        || record.tags.get("message").map(|v| v.trim_matches('"')) == Some("stream error")
}

/// Stable identity of a `stream error` record for the consecutive-repeat
/// backstop. Two real retries of the *same* terminal failure must share a
/// fingerprint; two genuinely different failures must not.
///
/// Built from the identifying tags (`providerID`, `modelID`, `session.id`) plus
/// the provider error text ([`error_context`]). The raw line is deliberately
/// **not** used: it carries a per-record `timestamp=`/`+Nms` prefix that differs
/// on every retry, which would make two otherwise-identical backoff retries look
/// distinct and defeat the backstop.
pub(super) fn stream_error_fingerprint(record: &OpenCodeLogRecord) -> String {
    let tag = |key: &str| record.tags.get(key).map(String::as_str).unwrap_or("");
    format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}",
        tag("providerID"),
        tag("modelID"),
        tag("session.id"),
        error_context(record).unwrap_or_default(),
    )
}

impl<S: SemanticEventSink> OpenCodeLogBridge<S> {
    /// Emit a terminal error and request a fail-fast abort after consecutive
    /// `stream error` records crossed [`MAX_CONSECUTIVE_STREAM_ERRORS`].
    pub(super) fn on_repeated_stream_error(
        &mut self,
        record: &OpenCodeLogRecord,
    ) -> StderrIngestOutcome {
        let count = self.consecutive_stream_errors;
        let message = format!(
            "provider stream failed {count} times with no progress; aborting"
        );

        let mut extra_map = base_extra(record, "repeated_stream_error");
        extra_map.insert("count".into(), json!(count));
        if let Some(provider_error) = error_context(record) {
            extra_map.insert("provider_error".into(), serde_json::Value::String(provider_error));
        }

        self.sink.on_semantic_event(SemanticEvent::Error {
            message: message.clone(),
            terminal: true,
            kind: SemanticErrorKind::ApiRemote,
            extra: Value::Object(extra_map),
        });
        self.fire_early_termination(EarlyTermination::RepeatedStreamError { count });
        StderrIngestOutcome::Consumed
    }
}
