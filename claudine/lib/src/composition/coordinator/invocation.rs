//! The command-lifetime layer: immutable invocation inputs and the
//! coordinator-owned run ledger.
//!
//! These share a lifetime but not a mutability story. A proxy may read the
//! inputs and may cause the coordinator to extend the ledger, but it can do
//! neither of the two things that would let a router outrank its caller:
//! rewrite an invocation input, or reset the ledger's hop accounting.

use std::collections::BTreeSet;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::composition::error::CompositionError;
use crate::composition::launch_workspace::LaunchWorkspaceContext;
use crate::composition::types::{
    CompositionMode, InstalledProviderSnapshot, OutputFormat, SessionInteractivitySource,
    SharedApprovalCache,
};
use crate::composition::{MAX_PROXY_HOPS, SequenceStepOverlay, proxy_handoff_allowed};
use crate::provider_id::Provider;
use crate::system_prompt::SystemPromptArgs;

use super::handoff::{HopApproval, ProxyHandoff, ProxyProvenance, ResolvedProxyTarget};

/// How the enclosing command routes the final active document's output.
///
/// Invocation state, not prepared-document state: a proxy changes which
/// document owns the output, never where the output goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutputPolicy {
    /// Universal output format, when the caller fixed one.
    pub format: Option<OutputFormat>,
    /// Whether this invocation is one step of a sequence run.
    pub sequence: bool,
    /// Header-only preflight output.
    pub quiet: bool,
    /// No preflight output at all.
    pub silent: bool,
}

/// The one shared launch-CWD discovery scan, taken during prep.
///
/// Every field is `None` when the caller did not pre-scan;
/// `detection_error` carries the scan's failure so `--repo` can still treat
/// launch-context detection failure as fatal instead of silently launching
/// with an empty context.
#[derive(Debug, Clone, Default)]
pub struct LaunchDiscovery {
    #[allow(missing_docs)]
    pub launch_context: Option<crate::system_prompt::LaunchContext>,
    #[allow(missing_docs)]
    pub env_context: Option<crate::events::EnvironmentContext>,
    #[allow(missing_docs)]
    pub launch_workspace: Option<LaunchWorkspaceContext>,
    /// The shared scan's error, when it failed.
    pub detection_error: Option<String>,
}

/// A mutable draft of [`InvocationInputs`]. Fill it, then
/// [`freeze`](Self::freeze).
///
/// Splitting construction from use is what makes the frozen value's
/// immutability real without writing an accessor per field: the draft has
/// public fields, the frozen wrapper hands out only `&`.
#[derive(Debug, Clone)]
pub struct InvocationInputsDraft {
    /// The file reference exactly as the caller typed it.
    pub file_ref: String,
    /// Caller `key=value` / `--set` frontmatter overrides, as a JSON object.
    pub set_overrides: Option<serde_json::Value>,
    /// Launch-area directory anchoring caller-supplied file references.
    pub file_ref_fallback_dir: Option<PathBuf>,
    /// Which composition command is running.
    pub mode: CompositionMode,
    /// Overrides scoped to the current sequence step, when in a sequence.
    pub step_overrides: Option<SequenceStepOverlay>,
    /// Where the final active document's output goes.
    pub output_policy: CommandOutputPolicy,
    /// Provider fixed by `--claude` / `--codex` / ….
    pub explicit_provider: Option<Provider>,
    /// Model fixed by `--model`.
    pub model: Option<String>,
    /// Raw arguments forwarded to the provider binary.
    pub provider_args: Vec<String>,
    /// Whether the caller supplied [`provider_args`](Self::provider_args)
    /// explicitly, as opposed to them being derived.
    pub provider_args_explicit: bool,
    /// Providers excluded from automatic selection.
    pub excluded: BTreeSet<Provider>,
    /// Whether the provider session is interactive.
    pub session_interactive: bool,
    /// Why the session is (non-)interactive.
    pub session_interactive_source: SessionInteractivitySource,
    /// Provider-specific auto-approval mode.
    pub yolo: bool,
    /// Provider-specific sandboxing.
    pub sandbox: bool,
    /// Ambient working directory at command invocation time.
    pub launch_cwd: PathBuf,
    /// Restrict to repo-scoped resources via a shadow HOME.
    pub repo: bool,
    /// Launch-CWD discovery taken once during prep and reused everywhere.
    ///
    /// These are inputs, not derived state: re-deriving them later would read
    /// a process CWD the wrapper has since moved to the repo root. Canonical
    /// preparation (Phase 5) reads its context from here rather than
    /// recapturing.
    pub launch_discovery: LaunchDiscovery,
    /// Wall-clock timeout intent, unparsed.
    pub timeout: Option<String>,
    /// Stream-silence timeout intent, unparsed.
    pub step_timeout: Option<String>,
    /// OpenCode stalled-generation backstop intent, unparsed.
    pub stall_timeout: Option<String>,
    /// System-prompt CLI intent.
    pub system_prompt_args: SystemPromptArgs,
    /// Claudine-managed MCP session composition.
    pub mcp: bool,
    /// Explicit MCP server IDs or aliases.
    pub mcp_use: Vec<String>,
    /// Treat unresolved or ambiguous MCP tags as hard errors.
    pub strict: bool,
    /// Env vars preserved through sensitive-name filtering.
    pub include: Vec<String>,
    /// `OPERATION` env var value.
    pub operation: Option<String>,
    /// Env overrides injected into both the compose context and the child.
    pub env_overrides: std::collections::BTreeMap<String, String>,
    /// Host installation snapshot taken during prep.
    pub installed_snapshot: Option<InstalledProviderSnapshot>,
    /// Show what would run without launching a child.
    pub dry_run: bool,
}

impl InvocationInputsDraft {
    /// Start a draft from the three inputs every invocation must have.
    pub fn new(file_ref: String, mode: CompositionMode, launch_cwd: PathBuf) -> Self {
        Self {
            file_ref,
            set_overrides: None,
            file_ref_fallback_dir: None,
            mode,
            step_overrides: None,
            output_policy: CommandOutputPolicy {
                format: None,
                sequence: false,
                quiet: false,
                silent: false,
            },
            explicit_provider: None,
            model: None,
            provider_args: Vec::new(),
            provider_args_explicit: false,
            excluded: BTreeSet::new(),
            session_interactive: false,
            session_interactive_source: SessionInteractivitySource::Default,
            yolo: false,
            sandbox: false,
            launch_cwd,
            repo: false,
            launch_discovery: LaunchDiscovery::default(),
            timeout: None,
            step_timeout: None,
            stall_timeout: None,
            system_prompt_args: SystemPromptArgs::default(),
            mcp: false,
            mcp_use: Vec::new(),
            strict: false,
            include: Vec::new(),
            operation: None,
            env_overrides: std::collections::BTreeMap::new(),
            installed_snapshot: None,
            dry_run: false,
        }
    }

    /// Seal the draft. There is no way back: nothing un-freezes.
    pub fn freeze(self) -> InvocationInputs {
        InvocationInputs(self)
    }
}

/// The immutable half of invocation state.
///
/// Read-only by construction: [`Deref`] hands out `&InvocationInputsDraft`
/// and there is no `DerefMut`, so a proxy target — like anything else holding
/// one — can read every caller decision and rewrite none of them.
///
/// `mut` on the binding is deliberate: it isolates the failure to the missing
/// `DerefMut`, so this stays a real test of immutability rather than of the
/// binding mode.
///
/// ```compile_fail
/// # use claudine::composition::{InvocationInputsDraft, CompositionMode};
/// # use std::path::PathBuf;
/// let mut inputs = InvocationInputsDraft::new(
///     "a.md".to_string(),
///     CompositionMode::ChainedDocument,
///     PathBuf::from("/tmp"),
/// )
/// .freeze();
/// // Invocation inputs are immutable; a proxy cannot outrank its caller.
/// inputs.yolo = true;
/// ```
#[derive(Debug, Clone)]
pub struct InvocationInputs(InvocationInputsDraft);

impl Deref for InvocationInputs {
    type Target = InvocationInputsDraft;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Why the ledger refused a hop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HopRejection {
    /// The target is already in the chain — a self-proxy or an A→B→A cycle.
    Cycle,
    /// Accepting the hop would exceed [`MAX_PROXY_HOPS`].
    BudgetExhausted,
}

/// What a transition did, recorded for final output and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionRecord {
    /// A committed proxy hop.
    Proxy {
        /// The provenance of the action that requested it.
        provenance: ProxyProvenance,
        /// The document adopted.
        resolved_target: PathBuf,
    },
    /// A retry of the active document.
    Retry {
        /// The attempt the retry produced.
        attempt: u32,
    },
    /// A resume of the active document's live session.
    Resume {
        /// The attempt the resume produced.
        attempt: u32,
    },
}

/// The mutable half of invocation state.
///
/// Everything here outlives any single document and must survive a handoff:
/// hop accounting exists precisely to bound a chain of documents, so letting a
/// proxy reset it would defeat it.
///
/// Mutation is coordinator-only. Readers are `pub`; every mutator lives on
/// [`LedgerMut`], which only [`access`](Self::access) produces and which is
/// `pub(crate)` — the CLI driver can read the ledger but cannot reach into it:
///
/// ```compile_fail
/// # use claudine::composition::RunLedger;
/// # fn demo(ledger: &mut RunLedger) {
/// // Only the coordinator may mint the mutation capability.
/// let _ = ledger.access();
/// # }
/// ```
#[derive(Debug)]
pub struct RunLedger {
    chain: Vec<PathBuf>,
    approval_cache: SharedApprovalCache,
    command_started: Instant,
    step_started: Option<Instant>,
    transitions: Vec<TransitionRecord>,
}

impl RunLedger {
    /// Open a ledger for a run originating at `origin`.
    ///
    /// The chain is seeded with `origin` so the first hop is already checked
    /// against it — a document that proxies to itself is a cycle from the
    /// first hop, not from the second.
    pub fn new(origin: PathBuf, approval_cache: SharedApprovalCache) -> Self {
        Self {
            chain: vec![origin],
            approval_cache,
            command_started: Instant::now(),
            step_started: None,
            transitions: Vec::new(),
        }
    }

    /// The ordered documents this run has occupied, oldest first.
    pub fn chain(&self) -> &[PathBuf] {
        &self.chain
    }

    /// Whether `path` is already in the chain.
    ///
    /// The one authority for that question. Four call sites open-coded it
    /// before the coordinator existed, each with its own idea of whether the
    /// current document was in the chain yet.
    pub fn contains_document(&self, path: &Path) -> bool {
        self.chain.iter().any(|entry| entry == path)
    }

    /// Hops committed so far. The seeded origin is not a hop.
    pub fn hops(&self) -> usize {
        self.chain.len().saturating_sub(1)
    }

    /// The exact-command approval cache, shared across every document in the
    /// run.
    pub fn approval_cache(&self) -> &SharedApprovalCache {
        &self.approval_cache
    }

    /// When the command started. Unaffected by a handoff.
    pub fn command_started(&self) -> Instant {
        self.command_started
    }

    /// When the current sequence step started, when in a sequence.
    pub fn step_started(&self) -> Option<Instant> {
        self.step_started
    }

    #[allow(missing_docs)]
    pub fn transitions(&self) -> &[TransitionRecord] {
        &self.transitions
    }

    /// Mint the mutation capability.
    ///
    /// Coordinator-only, and deliberately narrow: an adapter that needs to
    /// extend the ledger receives a [`LedgerMut`] for the duration of one
    /// call rather than a `&mut RunLedger` it could hold.
    pub(crate) fn access(&mut self) -> LedgerMut<'_> {
        LedgerMut(self)
    }
}

/// The one invocation-wide [`RunLedger`], shared between the command-owned
/// active-document loop and the provider harness running inside it.
///
/// The command loop owns the ledger; the harness receives a clone of the
/// `Arc` so a terminal-event proxy can be committed — against this same
/// chain, never a harness-local copy — while the source document's stacks
/// are still live to catch a refused hop (spec "Atomic handoff"). Locking is
/// brief and never re-entrant: a commit or chain read completes before the
/// harness returns control to the loop.
pub type SharedRunLedger = std::sync::Arc<std::sync::Mutex<RunLedger>>;

/// The coordinator-supplied capability to mutate a [`RunLedger`].
///
/// Note what is absent: there is no way to shorten the chain, rewind hop
/// accounting, or replace the approval cache. The ledger only grows.
#[derive(Debug)]
pub struct LedgerMut<'a>(&'a mut RunLedger);

impl LedgerMut<'_> {
    /// Decide whether a hop to `target` is permitted and, if so, record it.
    ///
    /// Recording happens here rather than at commit time because the returned
    /// [`HopApproval`] is the only route to a [`ProxyHandoff`]: approving is
    /// the commitment. Delegates the decision to
    /// [`proxy_handoff_allowed`] so the coordinator and any remaining legacy
    /// caller cannot disagree about what a cycle is.
    ///
    /// ## Errors
    ///
    /// Returns [`HopRejection::Cycle`] when `target` is already in the chain,
    /// and [`HopRejection::BudgetExhausted`] when the chain has reached
    /// [`MAX_PROXY_HOPS`].
    pub fn approve_hop(
        &mut self,
        target: ResolvedProxyTarget,
    ) -> Result<HopApproval, HopRejection> {
        if self.0.chain.len() >= MAX_PROXY_HOPS {
            return Err(HopRejection::BudgetExhausted);
        }
        if !proxy_handoff_allowed(&self.0.chain, target.path()) {
            return Err(HopRejection::Cycle);
        }
        self.0.chain.push(target.path().to_path_buf());
        Ok(HopApproval::new(target))
    }

    /// Record a committed handoff for final output and diagnostics.
    pub fn record_proxy(&mut self, handoff: &ProxyHandoff) {
        self.0.transitions.push(TransitionRecord::Proxy {
            provenance: handoff.provenance().clone(),
            resolved_target: handoff.resolved_target().to_path_buf(),
        });
    }

    #[allow(missing_docs)]
    pub fn record_transition(&mut self, record: TransitionRecord) {
        self.0.transitions.push(record);
    }

    /// Anchor the start of a sequence step.
    pub fn begin_step(&mut self, at: Instant) {
        self.0.step_started = Some(at);
    }
}

impl HopRejection {
    /// Build the typed cycle/hop-limit error this rejection stands for.
    pub fn into_error(
        self,
        source_path: PathBuf,
        target: String,
        chain: &[PathBuf],
    ) -> CompositionError {
        CompositionError::LifecycleProxyCycle {
            source_path,
            target,
            chain: chain.iter().map(|p| p.display().to_string()).collect(),
            limit: MAX_PROXY_HOPS,
        }
    }
}
