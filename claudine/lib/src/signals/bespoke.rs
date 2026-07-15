//! Bespoke (cross-record / temporal) signal detectors.
//!
//! The declarative engine handles single-payload matching; everything that
//! needs memory across payloads lives here. Detectors run inside
//! [`super::SignalHub::observe_json`] AFTER the declarative pass — the same
//! lock, the same sink — and `claudine signals check` replays the identical
//! detector code over the research corpus's evidence fixtures via
//! [`bespoke_replayer`]. Semantics authority: each record's
//! `bespoke_rationale:` / notes in `docs/research/signals/<provider>.md`.

use claudine_catalog_types::{QwenLoopType, SignalEvent, SignalSource};
use serde_json::{Value, json};
use strum::IntoEnumIterator;

use crate::stream::protocol::kimi::SUPPORTED_WIRE_PROTOCOL_VERSIONS;

/// Lines kept in the wrapper-synthesized exit payload's `stderr_tail`.
///
/// No existing error-report path keeps a stderr *tail* (the agent error
/// report reads only the first line of `stderr_text`), so this reuses the
/// wrapper output stack's one established truncation width
/// (`TOOL_RESULT_BODY_MAX_LINES`).
pub const EXIT_STDERR_TAIL_LINES: usize = 10;

/// Lines kept in the wrapper-synthesized exit payload's `stdout_tail`.
///
/// The stdout counterpart of [`EXIT_STDERR_TAIL_LINES`], kept at the same
/// width so both tails truncate identically.
pub const EXIT_STDOUT_TAIL_LINES: usize = 10;

/// The ratified `exit` source payload:
/// `{"exit_code", "stdout_tail", "stderr_tail"}` (sidecar schema's exit-source
/// contract; see the pi exit fixture for the canonical shape). `stdout_tail` /
/// `stderr_tail` carry the last [`EXIT_STDOUT_TAIL_LINES`] /
/// [`EXIT_STDERR_TAIL_LINES`] lines of the captured streams.
///
/// ## Notes
///
/// Both streams are tailed because a provider's terminal diagnostics are not
/// always on stderr — Antigravity's `agy` writes its auth errors to stdout, so
/// its `source: exit` records match `stdout_tail`.
pub fn exit_source_payload(exit_code: i32, stdout: &str, stderr: &str) -> Value {
    json!({
        "exit_code": exit_code,
        "stdout_tail": tail_lines(stdout, EXIT_STDOUT_TAIL_LINES),
        "stderr_tail": tail_lines(stderr, EXIT_STDERR_TAIL_LINES),
    })
}

/// The last `keep` lines of `text`, joined with newlines (whole text when it
/// has fewer than `keep` lines).
fn tail_lines(text: &str, keep: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(keep)..].join("\n")
}

/// One stateful cross-payload detector. Fed every payload the hub observes,
/// in order, with the observing source; returns at most one event per
/// payload and is responsible for its own emit-once discipline.
trait BespokeDetector: Send {
    fn observe(&mut self, source: SignalSource, payload: &Value) -> Option<SignalEvent>;
}

/// The per-provider detector set one [`super::SignalHub`] owns.
pub(super) struct BespokeChain {
    detectors: Vec<Box<dyn BespokeDetector>>,
}

impl BespokeChain {
    /// No detectors — the table-less hub's chain.
    pub(super) fn empty() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }

    /// The detector set for one provider table, keyed by slug. Pi's chain
    /// (`PiRetriesExhausted`) runs at runtime for `Provider::Pi` runs now that
    /// Pi is wired, and stays reachable by slug for the `signals check` replay
    /// seam.
    pub(super) fn for_slug(slug: &str) -> Self {
        let detectors: Vec<Box<dyn BespokeDetector>> = match slug {
            "claude" => vec![Box::new(ClaudeResultTaint::default())],
            "goose" => vec![Box::new(GooseErrorThenComplete::default())],
            "kimi" => vec![Box::new(KimiProtocolWindow::default())],
            "pi" => vec![Box::new(PiRetriesExhausted::default())],
            "qwen" => vec![
                Box::new(QwenLoopDetected::default()),
                Box::new(QwenExitCode::default()),
            ],
            _ => Vec::new(),
        };
        Self { detectors }
    }

    pub(super) fn observe(&mut self, source: SignalSource, payload: &Value) -> Vec<SignalEvent> {
        self.detectors
            .iter_mut()
            .filter_map(|detector| detector.observe(source, payload))
            .collect()
    }
}

/// Fixture replayers for `claudine signals check`, keyed by bespoke record
/// id. A replayer feeds the evidence payload sequence through the SAME
/// detector type the runtime chain uses and reports whether it fired —
/// production-parity is the point of the seam.
pub fn bespoke_replayer(record_id: &str) -> Option<fn(&[Value]) -> bool> {
    match record_id {
        "stream-session_tainted-result-error" => {
            Some(|payloads| replay_fires(ClaudeResultTaint::default(), payloads))
        }
        "stream-session_tainted-error-then-complete" => {
            Some(|payloads| replay_fires(GooseErrorThenComplete::default(), payloads))
        }
        "stream-retries_exhausted-auto_retry_end" => {
            Some(|payloads| replay_fires(PiRetriesExhausted::default(), payloads))
        }
        "stream-runaway_repetition-result-loop" => {
            Some(|payloads| replay_fires(QwenLoopDetected::default(), payloads))
        }
        _ => None,
    }
}

/// All four replayable bespoke records observe the `stream` source.
fn replay_fires(mut detector: impl BespokeDetector, payloads: &[Value]) -> bool {
    payloads
        .iter()
        .any(|payload| detector.observe(SignalSource::Stream, payload).is_some())
}

fn str_field<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(Value::as_str)
}

// ---------------------------------------------------------------------------
// claude — stream-session_tainted-result-error
// ---------------------------------------------------------------------------

/// Claude's synthetic-billing taint: a terminal `result` whose `subtype`
/// claims `success` while `is_error` is `true` would be classified
/// successful by anything keyed on subtype or process exit, so it is
/// tainted. When `subtype` is an error subtype the failure is already
/// claimed declaratively and no taint fires. The cross-record state (a
/// prior `error` frame or assistant frame carrying an `error` label)
/// supplies the cause.
#[derive(Default)]
struct ClaudeResultTaint {
    prior_error: Option<String>,
    fired: bool,
}

impl BespokeDetector for ClaudeResultTaint {
    fn observe(&mut self, source: SignalSource, payload: &Value) -> Option<SignalEvent> {
        if source != SignalSource::Stream {
            return None;
        }
        match str_field(payload, "type") {
            Some("assistant") => {
                // Synthetic error frames label themselves via a top-level
                // `error` string (e.g. `billing_error`); the human text
                // rides in the message content.
                if let Some(label) = str_field(payload, "error") {
                    let text = payload
                        .pointer("/message/content/0/text")
                        .and_then(Value::as_str);
                    self.prior_error = Some(match text {
                        Some(text) => format!("{label}: {text}"),
                        None => label.to_string(),
                    });
                }
                None
            }
            Some("error") => {
                self.prior_error = payload
                    .pointer("/error/message")
                    .or_else(|| payload.get("message"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .or(self.prior_error.take());
                None
            }
            Some("result") => {
                if self.fired
                    || payload.get("is_error") != Some(&Value::Bool(true))
                    || str_field(payload, "subtype") != Some("success")
                {
                    return None;
                }
                self.fired = true;
                let cause = self
                    .prior_error
                    .clone()
                    .or_else(|| str_field(payload, "result").map(str::to_string))
                    .unwrap_or_else(|| "result.is_error=true on success envelope".to_string());
                Some(SignalEvent::SessionTainted { cause })
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// goose — stream-session_tainted-error-then-complete
// ---------------------------------------------------------------------------

/// Goose can emit an `error` frame, break the stream loop, and still emit
/// the terminal `complete` frame; the contradiction is only visible across
/// the two frames. Cause comes from the error frame's `error` string.
#[derive(Default)]
struct GooseErrorThenComplete {
    error_cause: Option<String>,
    fired: bool,
}

impl BespokeDetector for GooseErrorThenComplete {
    fn observe(&mut self, source: SignalSource, payload: &Value) -> Option<SignalEvent> {
        if source != SignalSource::Stream {
            return None;
        }
        match str_field(payload, "type") {
            Some("error") => {
                self.error_cause = Some(
                    str_field(payload, "error")
                        .unwrap_or("stream error frame")
                        .to_string(),
                );
                None
            }
            Some("complete") if !self.fired => {
                let cause = self.error_cause.clone()?;
                self.fired = true;
                Some(SignalEvent::SessionTainted { cause })
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// pi — stream-retries_exhausted-auto_retry_end (dormant at runtime)
// ---------------------------------------------------------------------------

/// Pi's `auto_retry_end success:false` covers both exhausted retry budgets
/// and user-cancelled retry sleeps (`finalError: "Retry cancelled"`), so
/// classification correlates the preceding `auto_retry_start`'s
/// `attempt`/`maxAttempts` and rejects the cancel spelling.
#[derive(Default)]
struct PiRetriesExhausted {
    last_start: Option<(Option<u32>, Option<u32>)>,
    fired: bool,
}

impl BespokeDetector for PiRetriesExhausted {
    fn observe(&mut self, source: SignalSource, payload: &Value) -> Option<SignalEvent> {
        if source != SignalSource::Stream {
            return None;
        }
        let as_u32 = |key: &str| {
            payload
                .get(key)
                .and_then(Value::as_u64)
                .and_then(|n| u32::try_from(n).ok())
        };
        match str_field(payload, "type") {
            Some("auto_retry_start") => {
                self.last_start = Some((as_u32("attempt"), as_u32("maxAttempts")));
                None
            }
            Some("auto_retry_end") if !self.fired => {
                if payload.get("success") != Some(&Value::Bool(false)) {
                    return None;
                }
                let final_error = str_field(payload, "finalError");
                if final_error == Some("Retry cancelled") {
                    return None;
                }
                let (start_attempt, max_attempts) = self.last_start.unwrap_or((None, None));
                let attempts = as_u32("attempt").or(start_attempt);
                // An end frame without a correlated budget still counts as
                // exhaustion: the cancel spelling is already rejected above.
                if let (Some(attempts), Some(max)) = (attempts, max_attempts)
                    && attempts < max
                {
                    return None;
                }
                self.fired = true;
                Some(SignalEvent::RetriesExhausted {
                    status_code: None,
                    attempts,
                    message: final_error.map(str::to_string),
                })
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// qwen — stream-runaway_repetition-result-loop
// ---------------------------------------------------------------------------

/// Qwen's provider-native loop guard mapped onto Claudine's
/// `runaway_repetition` taxonomy member. The native payload carries no
/// cycle-length or repeat counts (only prose + the `LoopType` token), so
/// both required counter fields are 0 — "provider-native guard fired,
/// counts unknown".
#[derive(Default)]
struct QwenLoopDetected {
    fired: bool,
}

impl BespokeDetector for QwenLoopDetected {
    fn observe(&mut self, source: SignalSource, payload: &Value) -> Option<SignalEvent> {
        if self.fired
            || source != SignalSource::Stream
            || str_field(payload, "type") != Some("result")
            || payload.get("is_error") != Some(&Value::Bool(true))
        {
            return None;
        }
        let message = payload.pointer("/error/message").and_then(Value::as_str)?;
        if !QwenLoopType::iter().any(|token| message.contains(<&str>::from(token))) {
            return None;
        }
        self.fired = true;
        Some(SignalEvent::RunawayRepetition {
            cycle_len: 0,
            repeats: 0,
        })
    }
}

// ---------------------------------------------------------------------------
// qwen — exit codes 53/55/130 (bespoke-with-gaps, ratified; no fixtures)
// ---------------------------------------------------------------------------

/// Qwen native terminations bypass the terminal `result` event: research
/// pins `FatalTurnLimitedError` → 53, `FatalBudgetExceededError` → 55, and
/// `FatalCancellationError` → 130 (qwen.md `gaps:`). Observes the
/// wrapper-synthesized exit payload; all other codes stay silent.
#[derive(Default)]
struct QwenExitCode {
    fired: bool,
}

impl BespokeDetector for QwenExitCode {
    fn observe(&mut self, source: SignalSource, payload: &Value) -> Option<SignalEvent> {
        if self.fired || source != SignalSource::Exit {
            return None;
        }
        let event = match payload.get("exit_code").and_then(Value::as_i64) {
            Some(53) => SignalEvent::TurnLimitReached {
                limit: None,
                message: None,
            },
            Some(55) => SignalEvent::SessionTimeLimitReached { limit: None },
            Some(130) => SignalEvent::Interrupted { message: None },
            _ => return None,
        };
        self.fired = true;
        Some(event)
    }
}

// ---------------------------------------------------------------------------
// kimi — unsupported_protocol_version (wrapper-synthesized negative match)
// ---------------------------------------------------------------------------

/// Kimi wire's initialize result announces `protocol_version`; a version
/// outside [`SUPPORTED_WIRE_PROTOCOL_VERSIONS`] is a negative match the
/// declarative grammar cannot express (kimi.md `gaps:`). No detection
/// record exists, so this detector has no replayer — runtime only.
#[derive(Default)]
struct KimiProtocolWindow {
    fired: bool,
}

impl BespokeDetector for KimiProtocolWindow {
    fn observe(&mut self, source: SignalSource, payload: &Value) -> Option<SignalEvent> {
        if self.fired || source != SignalSource::Stream {
            return None;
        }
        // The initialize response is the one JSON-RPC result carrying
        // `protocol_version` (lib/src/stream/protocol/kimi.rs).
        let version = payload
            .pointer("/result/protocol_version")
            .and_then(Value::as_str)?;
        if SUPPORTED_WIRE_PROTOCOL_VERSIONS.contains(&version) {
            return None;
        }
        self.fired = true;
        Some(SignalEvent::UnsupportedProtocolVersion {
            version: version.to_string(),
            supported: SUPPORTED_WIRE_PROTOCOL_VERSIONS
                .iter()
                .map(|v| v.to_string())
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests;
