//! Core types for composition workflows.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use darkmatter::markdown::Markdown;
use serde::{Deserialize, Serialize};

use super::lifecycle::LifecycleConfig;
use crate::events::Provider;
use crate::harness::shell::CachedApprovalDecision;

/// Shared approval cache that can be reused across composition runs
/// (e.g. steps of one sequence) so that previously approved commands
/// do not prompt again.
pub type SharedApprovalCache = Arc<Mutex<HashMap<String, CachedApprovalDecision>>>;

/// Which composition mode to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionMode {
    /// Use the frontmatter `prompt` property as input; replace body with output.
    InlineFrontmatterPrompt,
    /// Compose the full document and send as prompt; no file mutation.
    ChainedDocument,
}

/// A resolved and loaded composition source document.
#[derive(Debug, Clone)]
pub struct ResolvedCompositionSource {
    /// The original file reference string.
    pub original_ref: String,
    /// The resolved absolute path.
    pub resolved_path: PathBuf,
    /// The original on-disk document text.
    pub original_text: String,
    /// The parsed Markdown document.
    pub markdown: Markdown,
}

/// Why a particular provider was selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionReason {
    /// The caller explicitly specified the provider (wrapper subcommand).
    ExplicitProvider,
    /// Only one installed provider remained after exclusion filtering.
    SingleInstalled,
    /// The source document's `agent` frontmatter selected the provider.
    FrontmatterHint,
    /// The user's config favorite (`settings.linking.preference[0]`).
    ConfigFavorite,
    /// The user chose interactively.
    InteractiveChoice,
}

/// A provider selected for composition execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedProvider {
    /// The selected provider.
    pub provider: Provider,
    /// Why this provider was selected.
    pub reason: SelectionReason,
}

/// A composition prepared with effective (composed) frontmatter.
///
/// This struct carries the full effective frontmatter after Darkmatter
/// composition. All downstream code (harness, MCP, provider selection)
/// must read from `effective_frontmatter`, never from raw source state.
#[derive(Debug, Clone)]
pub struct PreparedComposition {
    /// Which composition mode produced this.
    pub mode: CompositionMode,
    /// Resolved absolute path to the source file.
    pub resolved_path: PathBuf,
    /// Git repo root derived from the source document's location.
    ///
    /// Used for favorite-provider lookup, guardrails, MCP defaults,
    /// and harness path resolution. `None` when the source document
    /// is not inside a git repository.
    pub source_repo_root: Option<PathBuf>,
    /// The composed prompt text.
    pub prompt: String,
    /// Full frontmatter after Darkmatter composition.
    pub effective_frontmatter: serde_json::Value,
    /// The `agent` value from effective frontmatter, if present.
    pub effective_agent_hint: Option<serde_json::Value>,
    /// Closure plan for post-execution file updates.
    pub closure: CompositionClosurePlan,
    /// Parsed lifecycle notification config from effective frontmatter.
    pub lifecycle: LifecycleConfig,
}

/// How the composition result should be applied after provider execution.
#[derive(Debug, Clone)]
pub enum CompositionClosurePlan {
    /// No file mutation; provider output goes to stdout.
    Direct,
    /// Rewrite the source file with the provider's body output.
    Inline(InlineClosurePlan),
}

/// State captured before inline composition for deterministic closure.
#[derive(Debug, Clone)]
pub struct InlineClosurePlan {
    /// The original on-disk document text (frontmatter + body).
    pub original_document_text: String,
    /// Hash of the original body (for unchanged-body detection).
    pub original_body_hash: u64,
}

/// Universal output format for composed provider execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Json,
    Text,
    Stream,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json => write!(f, "json"),
            Self::Text => write!(f, "text"),
            Self::Stream => write!(f, "stream"),
        }
    }
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "text" => Ok(Self::Text),
            "stream" | "stream-json" => Ok(Self::Stream),
            _ => Err(format!(
                "unknown output format '{value}'; expected json, text, or stream"
            )),
        }
    }
}

/// A fully-specified request to execute a composition through the
/// wrapper-grade pipeline.
#[derive(Debug, Clone)]
pub struct CompositionExecutionRequest {
    /// Which composition mode.
    pub mode: CompositionMode,
    /// The raw file reference string (for display/logging).
    pub file_ref: String,
    /// The prepared composition with effective frontmatter.
    pub prepared: PreparedComposition,
    /// Explicitly chosen provider (from `--claude`, `--codex`, etc.).
    pub explicit_provider: Option<Provider>,
    /// Providers to exclude from automatic selection.
    pub excluded: BTreeSet<Provider>,
    /// Enable provider-specific YOLO/auto-approval mode.
    pub yolo: bool,
    /// Preserve these env vars even when they match sensitive-name filters.
    pub include: Vec<String>,
    /// Override the model used by the provider.
    pub model: Option<String>,
    /// Set the output format (json, text, stream).
    pub output: Option<OutputFormat>,
    /// Parsed system prompt CLI args for the session.
    pub system_prompt_args: crate::system_prompt::SystemPromptArgs,
    /// Timeout in seconds for non-interactive mode.
    pub timeout: Option<u64>,
    /// Step-silence timeout in seconds for structured streaming runs.
    ///
    /// Resets on every stream event; when silence exceeds this budget the
    /// child is killed with `TimedOut`. Ignored in capture and passthrough
    /// modes (warning emitted). `None` means no silence deadline is applied.
    pub step_timeout: Option<u64>,
    /// OPERATION env var value for the composed session.
    pub operation: Option<String>,
    /// Enable provider-specific sandboxing.
    pub sandbox: bool,
    /// Use only repo-scoped resources via a shadow HOME.
    pub repo: bool,
    /// Show what would be executed without launching the child.
    pub dry_run: bool,
    /// Enable Claudine-managed MCP session composition.
    pub mcp: bool,
    /// Explicit MCP server IDs or aliases to activate.
    pub mcp_use: Vec<String>,
    /// Treat unresolved or ambiguous MCP tags as hard errors.
    pub strict: bool,
    /// Whether the provider session should be interactive (`-i`).
    pub session_interactive: bool,
    /// Show only header; suppress env details and info.
    pub quiet: bool,
    /// Suppress all preflight output.
    pub silent: bool,
    /// Extra environment variables to inject into both the composition
    /// context (used for preflight and prompt interpolation) and the
    /// spawned child process. Currently used by sequence execution to
    /// propagate `FAIL_FAST`.
    pub env_overrides: BTreeMap<String, String>,
    /// Optional shared shell-approval cache. When provided, harness
    /// shell preflight reuses previously approved commands from this
    /// cache instead of prompting again. Used by sequence execution to
    /// honour "allow once" for the whole run, not just one step.
    pub shared_approval_cache: Option<SharedApprovalCache>,
    /// Whether this request is part of a sequence run. When `true`, the
    /// execution header shows a `Sequence` badge.
    pub sequence: bool,
}

/// Describes where the sequence definition was found.
#[derive(Debug, Clone)]
pub enum SequenceSource {
    /// The sequence was defined inline in the document's frontmatter.
    Inline,
    /// The sequence was loaded from an external YAML file.
    External { path: PathBuf },
}

/// A single step in a sequence.
#[derive(Debug, Clone)]
pub struct SequenceStep {
    /// Zero-based index in the sequence list.
    pub index: usize,
    /// Display name for the step (scalar value or the `name` field of an object).
    pub name: String,
    /// The full state value: a JSON string for scalar steps, a JSON object for object steps.
    pub raw_state: serde_json::Value,
}

/// A validated, normalized sequence plan ready for execution.
#[derive(Debug, Clone)]
pub struct SequencePlan {
    /// Where the sequence definition came from.
    pub source: SequenceSource,
    /// Ordered list of steps.
    pub steps: Vec<SequenceStep>,
    /// The document's `fail_fast` setting (defaults to `true`).
    pub document_fail_fast: bool,
}

/// Per-step overlay values injected into each composition run.
#[derive(Debug, Clone)]
pub struct SequenceStepOverlay {
    /// Current step's state value.
    pub state: serde_json::Value,
    /// Previous step's state value, or `null` for the first step.
    pub previous_state: serde_json::Value,
    /// Next step's state value, or `null` for the last step.
    pub next_state: serde_json::Value,
    /// `true` if this is the first step.
    pub is_first: bool,
    /// `true` if this is the last step.
    pub is_last: bool,
    /// 1-based step number.
    pub step: usize,
    /// Total number of steps in the sequence.
    pub total_steps: usize,
}

impl SequenceStepOverlay {
    /// Reserved overlay keys that must always win over user `--set` values.
    pub const RESERVED_KEYS: &[&str] = &[
        "state",
        "previous_state",
        "next_state",
        "is_first",
        "is_last",
        "step",
        "total_steps",
    ];

    /// Build a `serde_json::Value::Object` suitable for `set_overrides`.
    ///
    /// Merge order: user `--set` first, then overlay (overlay wins on conflict).
    pub fn as_set_overrides(&self, user_set: Option<serde_json::Value>) -> serde_json::Value {
        let mut map = serde_json::Map::new();

        // 1. Start with user --set values
        if let Some(serde_json::Value::Object(user_map)) = user_set {
            for (key, value) in user_map {
                map.insert(key, value);
            }
        }

        // 2. Overlay reserved keys (always win)
        map.insert("state".into(), self.state.clone());
        map.insert("previous_state".into(), self.previous_state.clone());
        map.insert("next_state".into(), self.next_state.clone());
        map.insert("is_first".into(), serde_json::Value::Bool(self.is_first));
        map.insert("is_last".into(), serde_json::Value::Bool(self.is_last));
        map.insert("step".into(), serde_json::Value::Number(self.step.into()));
        map.insert(
            "total_steps".into(),
            serde_json::Value::Number(self.total_steps.into()),
        );

        serde_json::Value::Object(map)
    }
}

/// Options for sequence execution at the CLI level.
#[derive(Debug, Clone)]
pub struct SequenceExecutionOptions {
    /// CLI `--fail-fast` override. `None` means use the document default.
    pub fail_fast_override: Option<bool>,
}

/// Summary of a completed sequence run.
#[derive(Debug, Clone)]
pub struct SequenceRunSummary {
    /// Total steps in the sequence.
    pub total_steps: usize,
    /// Number of steps that succeeded.
    pub succeeded: usize,
    /// Number of steps that failed.
    pub failed: usize,
    /// Per-step results.
    pub steps: Vec<SequenceStepResult>,
}

/// Result of a single sequence step.
#[derive(Debug, Clone)]
pub struct SequenceStepResult {
    /// 1-based step number.
    pub step: usize,
    /// Display name of the step.
    pub name: String,
    /// Whether the step succeeded (exit code 0).
    pub success: bool,
    /// Error message if the step failed.
    pub error: Option<String>,
    /// Wall-clock duration of the step.
    pub duration: std::time::Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sequence_step_overlay_first_step() {
        let overlay = SequenceStepOverlay {
            state: json!("one"),
            previous_state: serde_json::Value::Null,
            next_state: json!("two"),
            is_first: true,
            is_last: false,
            step: 1,
            total_steps: 3,
        };
        assert!(overlay.is_first);
        assert!(!overlay.is_last);
        assert_eq!(overlay.step, 1);
        assert_eq!(overlay.total_steps, 3);
        assert!(overlay.previous_state.is_null());
    }

    #[test]
    fn sequence_step_overlay_last_step() {
        let overlay = SequenceStepOverlay {
            state: json!("three"),
            previous_state: json!("two"),
            next_state: serde_json::Value::Null,
            is_first: false,
            is_last: true,
            step: 3,
            total_steps: 3,
        };
        assert!(!overlay.is_first);
        assert!(overlay.is_last);
        assert_eq!(overlay.step, 3);
        assert!(overlay.next_state.is_null());
    }

    #[test]
    fn sequence_step_overlay_as_overrides_reserves_keys() {
        let overlay = SequenceStepOverlay {
            state: json!("one"),
            previous_state: serde_json::Value::Null,
            next_state: json!("two"),
            is_first: true,
            is_last: false,
            step: 1,
            total_steps: 3,
        };
        let overrides = overlay.as_set_overrides(None);
        let obj = overrides.as_object().unwrap();
        assert_eq!(obj.get("state"), Some(&json!("one")));
        assert_eq!(obj.get("is_first"), Some(&json!(true)));
        assert_eq!(obj.get("is_last"), Some(&json!(false)));
        assert_eq!(obj.get("step"), Some(&json!(1)));
        assert_eq!(obj.get("total_steps"), Some(&json!(3)));
        assert!(obj.get("previous_state").unwrap().is_null());
        assert_eq!(obj.get("next_state"), Some(&json!("two")));
    }

    #[test]
    fn sequence_step_overlay_merges_user_set_but_reserved_wins() {
        let overlay = SequenceStepOverlay {
            state: json!("one"),
            previous_state: serde_json::Value::Null,
            next_state: json!("two"),
            is_first: true,
            is_last: false,
            step: 1,
            total_steps: 3,
        };
        let user_set = json!({
            "color": "red",
            "state": "should-be-overridden",
            "step": 99
        });
        let overrides = overlay.as_set_overrides(Some(user_set));
        let obj = overrides.as_object().unwrap();
        // User key preserved
        assert_eq!(obj.get("color"), Some(&json!("red")));
        // Reserved keys overwritten by overlay
        assert_eq!(obj.get("state"), Some(&json!("one")));
        assert_eq!(obj.get("step"), Some(&json!(1)));
    }

    #[test]
    fn sequence_plan_display_source() {
        let plan = SequencePlan {
            source: SequenceSource::Inline,
            steps: vec![],
            document_fail_fast: true,
        };
        assert!(matches!(plan.source, SequenceSource::Inline));

        let plan2 = SequencePlan {
            source: SequenceSource::External {
                path: std::path::PathBuf::from("data.yaml"),
            },
            steps: vec![],
            document_fail_fast: false,
        };
        assert!(!plan2.document_fail_fast);
    }
}
