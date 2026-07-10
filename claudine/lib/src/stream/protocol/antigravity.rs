//! Typed model for Antigravity's `agy --print --output-format json` envelope.
//!
//! Unlike every other wrapped provider, agy's structured print mode is NOT a
//! line-delimited event stream: it emits ONE buffered JSON object on stdout
//! after the run completes, carrying the final assistant `response`, a `status`
//! (`SUCCESS` or an error state), token `usage`, timing, and an optional
//! top-level `error`. Verified against the installed agy 1.1.0:
//!
//! ```json
//! {"conversation_id":"…","status":"SUCCESS","response":"OK\n",
//!  "duration_seconds":1.785,"num_turns":1,
//!  "usage":{"input_tokens":23791,"output_tokens":6,"thinking_tokens":0,"total_tokens":23797}}
//! ```

use serde::Deserialize;

/// The single `--output-format json` result envelope.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AntigravityEnvelope {
    /// The conversation/session id — the unattended-safe resume selector
    /// (`agy --conversation <id>`).
    #[serde(default)]
    pub conversation_id: Option<String>,
    /// Run status; `SUCCESS` on a clean turn, an error state otherwise.
    #[serde(default)]
    pub status: Option<String>,
    /// The final assistant response text.
    #[serde(default)]
    pub response: Option<String>,
    /// Top-level error message when the run failed.
    #[serde(default)]
    pub error: Option<String>,
    /// Wall-clock duration in fractional seconds.
    #[serde(default)]
    pub duration_seconds: Option<f64>,
    /// Number of agent turns in the run.
    #[serde(default)]
    pub num_turns: Option<u32>,
    /// Token accounting for the run.
    #[serde(default)]
    pub usage: Option<AntigravityUsage>,
}

/// Token-usage block on the envelope.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AntigravityUsage {
    /// Prompt/input tokens.
    #[serde(default)]
    pub input_tokens: Option<u64>,
    /// Completion/output tokens.
    #[serde(default)]
    pub output_tokens: Option<u64>,
    /// Extended-thinking tokens (agy reports these separately from output).
    #[serde(default)]
    pub thinking_tokens: Option<u64>,
    /// Total tokens for the run.
    #[serde(default)]
    pub total_tokens: Option<u64>,
}
