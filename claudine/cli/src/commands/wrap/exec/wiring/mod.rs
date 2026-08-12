//! Kimi `--wire` JSON-RPC 2.0 line transport.
//!
//! Wire mode is Kimi's structured client protocol. Stdout carries one
//! JSON-RPC envelope per line (notification, server-initiated request, or
//! response to a previously-sent client request); stdin accepts the same
//! shape on a single serialized writer. The semantic parser
//! ([`claudine::stream::providers::kimi::KimiSemanticStreamParser`]) consumes
//! every stdout line and emits the user-visible event surface, while this
//! module owns the IO loop that:
//!
//! - sends `initialize` (the semantic parser validates the negotiated
//!   protocol version) and fails fast when the server answers `init-1`
//!   with a JSON-RPC error,
//! - sends the resolved prompt as a `prompt` request after initialize,
//! - auto-responds to server-initiated `ApprovalRequest`, `QuestionRequest`,
//!   `ToolCallRequest`, and routes `HookRequest` through Claudine's
//!   dispatch pipeline,
//! - sends `cancel` on Ctrl+C / wall-clock timeout,
//! - flushes after every line so Kimi sees the response before the next
//!   stdin read.
//!
//! The pure builder helpers ([`build_initialize_request`],
//! [`build_prompt_request`], [`build_cancel_request`],
//! [`build_approval_response`], [`build_question_response`],
//! [`build_tool_call_unsupported_error`], [`build_hook_response`]) return
//! `serde_json::Value` envelopes so they can be unit-tested without any
//! child process. [`WireWriter`] serializes them to the child's stdin
//! behind a `Mutex` and flushes after each newline, satisfying the
//! "one serialized writer path" requirement from the Phase 3 plan.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use claudine::events::{AgenticEvent, EnvironmentContext, EventMeta};

use claudine::provider::Provider;
use claudine::stream::logs::EarlyTermination;
use claudine::stream::parser::SemanticStreamParser;
use claudine::stream::progress::LiveMetrics;
use claudine::stream::protocol::kimi::{
    KimiEnvelope, KimiHookRequest, KimiJsonRpcError, KimiWireRequest,
};
use claudine::stream::summary::StreamExecutionSummary;
use color_eyre::eyre::Result;
use serde_json::{Value, json};
use tracing::{Span, debug, info, info_span, warn};

use super::super::stream_io::StreamOutput;
use super::{
    OutputTextCallback, ProcessResult, ProcessTelemetry, ReasoningCallback, SemanticParserBuilder,
};

// Responsibility split of the Kimi wire transport. Each child glob-imports
// the shared `use` block above through `use super::*`:
// - `builders`   — JSON-RPC request/response builders, capabilities, the
//                  `HookOutcome`/`HookDispatchResult` types.
// - `dispatch`   — request classification, hook-event mapping, and the
//                  canonical hook-dispatch glue (`handle_request_dispatch`).
// - `writer`     — the single serialized `WireWriter` over child stdin.
// - `session`    — `run_kimi_wire_session` lifecycle plus its bridge into the
//                  shared cross-platform termination wait loop.
mod builders;
mod dispatch;
mod session;
mod writer;

pub(crate) use builders::*;
pub(crate) use dispatch::*;
pub(crate) use session::*;
pub(crate) use writer::*;

#[cfg(test)]
mod tests;
