//! Shared REST API request/response types used by both the `clipper`
//! service and the `clip` client. Centralizing here means the wire
//! contract is defined once.
//!
//! ## Notes
//!
//! These types intentionally mirror the spec (`spec.md`) — `EntrySummary`,
//! `HistoryResponse`, `SetResponse`, `HealthResponse`, the
//! `{ error: { code, message } }` envelope, and the tagged `SetRequest`
//! discriminator. Both crates serialize/deserialize against the same
//! struct definitions.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::content::ContentType;
use crate::entry::{ClipboardEntry, EntryId};

/// Metadata-only summary of a [`ClipboardEntry`] for REST responses.
///
/// The wire form mirrors the spec exactly:
///
/// ```json
/// {
///   "id": "a1b2c3d4e5f60718",
///   "timestamp": "2026-05-02T14:32:01Z",
///   "content_type": "text",
///   "preview": "Hello world",
///   "size_bytes": 11
/// }
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntrySummary {
    pub id: EntryId,
    pub timestamp: String,
    pub content_type: String,
    pub preview: String,
    pub size_bytes: usize,
}

impl From<&ClipboardEntry> for EntrySummary {
    fn from(entry: &ClipboardEntry) -> Self {
        Self {
            id: entry.id.clone(),
            timestamp: entry.timestamp.to_rfc3339(),
            content_type: content_type_str(entry.primary_content_type()).to_string(),
            preview: entry.preview(),
            size_bytes: entry.total_size_bytes(),
        }
    }
}

/// Snake-case wire form for a [`ContentType`].
pub fn content_type_str(ct: ContentType) -> &'static str {
    match ct {
        ContentType::Text => "text",
        ContentType::Html => "html",
        ContentType::Rtf => "rtf",
        ContentType::Image => "image",
        ContentType::Files => "files",
    }
}

/// Body of `GET /history`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryResponse {
    pub entries: Vec<EntrySummary>,
}

/// Body of a successful `POST /set`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetResponse {
    pub id: EntryId,
}

/// Body of `GET /health`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: String,
    pub entries: usize,
    /// `"running"` when the supervisor reports healthy, `"degraded"`
    /// otherwise. The status code is the primary signal — see
    /// `ErrorResponse` for the degraded path.
    pub watcher: String,
}

/// Tagged-enum request body for `POST /set`.
///
/// Per spec the request looks like:
///
/// ```json
/// { "content_type": "text", "data": "hello" }
/// { "content_type": "image", "data": "<base64>", "width": 100, "height": 100 }
/// ```
///
/// The `Image` variant's `data` is base64-encoded bytes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "content_type", rename_all = "snake_case")]
pub enum SetRequest {
    Text {
        data: String,
    },
    Html {
        data: String,
    },
    Rtf {
        data: String,
    },
    Image {
        /// Base64-encoded image bytes (PNG/JPEG/etc.).
        data: String,
        width: u32,
        height: u32,
    },
    Files {
        data: Vec<PathBuf>,
    },
}

/// Spec-shaped error envelope.
///
/// ```json
/// { "error": { "code": "entry_not_found", "message": "..." } }
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
}

/// Documented error codes; exhaustive set per spec `## Error Responses`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    EntryNotFound,
    BadRequest,
    FormatNotAvailable,
    Internal,
    ServiceUnavailable,
}

impl ErrorResponse {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            error: ErrorBody {
                code,
                message: message.into(),
            },
        }
    }
}

/// Query parameters accepted by `GET /history`.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct HistoryQuery {
    /// RFC3339 timestamp; only entries strictly newer than this are
    /// returned.
    pub since: Option<DateTime<Utc>>,
    /// Maximum entries to return (newest first).
    pub limit: Option<usize>,
}

/// Query parameters accepted by `GET /history/:id/content`.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ContentQuery {
    pub format: Option<ContentType>,
    pub encoding: Option<Encoding>,
}

/// Wire-level encoding requested for binary payloads.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Encoding {
    Base64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_envelope_round_trip() {
        let err = ErrorResponse::new(ErrorCode::EntryNotFound, "missing");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains(r#""code":"entry_not_found""#));
        let back: ErrorResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(err, back);
    }

    #[test]
    fn set_request_text_round_trip() {
        let payload = r#"{"content_type":"text","data":"hello"}"#;
        let req: SetRequest = serde_json::from_str(payload).unwrap();
        assert!(matches!(req, SetRequest::Text { ref data } if data == "hello"));
    }

    #[test]
    fn set_request_image_round_trip() {
        let payload = r#"{"content_type":"image","data":"AAAA","width":10,"height":20}"#;
        let req: SetRequest = serde_json::from_str(payload).unwrap();
        match req {
            SetRequest::Image {
                data,
                width,
                height,
            } => {
                assert_eq!(data, "AAAA");
                assert_eq!(width, 10);
                assert_eq!(height, 20);
            }
            other => panic!("expected Image, got {other:?}"),
        }
    }

    #[test]
    fn set_request_rejects_missing_discriminator() {
        let payload = r#"{"data":"hello"}"#;
        assert!(serde_json::from_str::<SetRequest>(payload).is_err());
    }

    #[test]
    fn error_code_serializes_snake_case() {
        for (code, expected) in [
            (ErrorCode::EntryNotFound, "entry_not_found"),
            (ErrorCode::BadRequest, "bad_request"),
            (ErrorCode::FormatNotAvailable, "format_not_available"),
            (ErrorCode::Internal, "internal"),
            (ErrorCode::ServiceUnavailable, "service_unavailable"),
        ] {
            let json = serde_json::to_string(&code).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
        }
    }
}
