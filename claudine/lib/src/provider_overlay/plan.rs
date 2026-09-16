//! The typed overlay plan and the planner that produces it.
//!
//! One [`OverlayPlan`] answers every question the wrapper used to answer with a
//! `force_shadow_home` boolean, a `needs_shadow_home` boolean, and a bare
//! `Option<PathBuf>`: why an overlay is needed, which provider it is for, where
//! the provider's configuration lives *without* an overlay, where Claudine will
//! store the overlay, what the provider will see, which environment the child
//! receives, what is omitted, what is materialized, and what must stay at its
//! pre-overlay location.
//!
//! Policy is read from generated provider metadata
//! ([`Provider::overlay_capability`] / [`Provider::overlay_selector`]), so
//! planning introduces no `match Provider` dispatch site. Provider-specific
//! path construction stays in the wrapper profile, which mutates a plan through
//! [`OverlayPlan::materialize`] and [`OverlayPlan::pin_external_state`].

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::ClaudineError;
use crate::invocation_context::{EnvBaseline, HomeBaseline};
use crate::provider::{OverlayCapability, OverlayReason, OverlayResourceClass, Provider};

use super::lease::OverlayLease;
use super::selector::{OVERLAY_LAUNCHES_DIR, OverlaySelector, OverlayStorage, provider_visible_root};

/// A set of [`OverlayReason`]s — one launch can need an overlay for several
/// reasons at once, which is exactly what `--repo` plus `--mcp` produces.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct OverlayReasons(u8);

const fn reason_bit(reason: OverlayReason) -> u8 {
    match reason {
        OverlayReason::RepoResources => 1,
        OverlayReason::RepoPrompt => 1 << 1,
        OverlayReason::Mcp => 1 << 2,
    }
}

/// Every activation reason, in the order [`OverlayReasons::iter`] yields them
/// and the order a refusal picks the reason it names.
pub const OVERLAY_REASONS: [OverlayReason; 3] = [
    OverlayReason::RepoResources,
    OverlayReason::RepoPrompt,
    OverlayReason::Mcp,
];

impl OverlayReasons {
    /// The empty set — a launch that needs no overlay.
    pub const fn none() -> Self {
        Self(0)
    }

    /// The set containing exactly `reason`.
    pub const fn single(reason: OverlayReason) -> Self {
        Self(reason_bit(reason))
    }

    /// This set plus `reason`.
    pub const fn with(self, reason: OverlayReason) -> Self {
        Self(self.0 | reason_bit(reason))
    }

    /// Add `reason` to this set.
    pub fn insert(&mut self, reason: OverlayReason) {
        self.0 |= reason_bit(reason);
    }

    /// Whether `reason` is in this set.
    pub fn contains(&self, reason: OverlayReason) -> bool {
        self.0 & reason_bit(reason) != 0
    }

    /// Whether no reason is set.
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// How many reasons are set.
    pub fn len(&self) -> usize {
        self.0.count_ones() as usize
    }

    /// The members, in [`OVERLAY_REASONS`] order.
    pub fn iter(&self) -> impl Iterator<Item = OverlayReason> + '_ {
        OVERLAY_REASONS
            .into_iter()
            .filter(|reason| self.contains(*reason))
    }
}

impl FromIterator<OverlayReason> for OverlayReasons {
    fn from_iter<I: IntoIterator<Item = OverlayReason>>(iter: I) -> Self {
        iter.into_iter().fold(Self::none(), Self::with)
    }
}

impl std::fmt::Debug for OverlayReasons {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set()
            .entries(self.iter().map(<&'static str>::from))
            .finish()
    }
}

/// Whether a materialized entry is a file or a directory tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayEntryKind {
    /// A single file.
    File,
    /// A directory and everything beneath it.
    Directory,
}

/// One entry the overlay must contain as real content rather than as a mirror
/// of the source root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayMaterialization {
    /// Where the content is read from, under the provider source root.
    pub source: PathBuf,
    /// Where it is written, under the provider-visible root.
    pub destination: PathBuf,
    /// File or directory tree.
    pub kind: OverlayEntryKind,
    /// The resource class this entry belongs to.
    pub class: OverlayResourceClass,
    /// Whether the repository's own copy of this resource is merged over the
    /// destination after the source content is placed (Codex prompts).
    pub repo_scoped: bool,
}

impl OverlayMaterialization {
    /// A single file copied into the overlay.
    pub fn file(
        source: impl Into<PathBuf>,
        destination: impl Into<PathBuf>,
        class: OverlayResourceClass,
    ) -> Self {
        Self {
            source: source.into(),
            destination: destination.into(),
            kind: OverlayEntryKind::File,
            class,
            repo_scoped: false,
        }
    }

    /// A directory tree placed into the overlay.
    pub fn directory(
        source: impl Into<PathBuf>,
        destination: impl Into<PathBuf>,
        class: OverlayResourceClass,
    ) -> Self {
        Self {
            source: source.into(),
            destination: destination.into(),
            kind: OverlayEntryKind::Directory,
            class,
            repo_scoped: false,
        }
    }

    /// Mark the entry as taking a repository-scoped overlay on top of the
    /// source content.
    pub fn repo_scoped(mut self) -> Self {
        self.repo_scoped = true;
        self
    }
}

/// The stage of overlay construction a failure occurred in.
///
/// Carried by [`ClaudineError::ProviderOverlayFailed`] so a diagnostic can say
/// what Claudine was doing without quoting the file that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayStage {
    /// Locating the provider's pre-overlay configuration root.
    SourceRoot,
    /// Creating the overlay's storage root.
    StorageRoot,
    /// Populating the overlay from the source root.
    Materialization,
    /// Writing runtime MCP configuration into the overlay's config root.
    McpInjection,
}

impl OverlayStage {
    /// The stable snake_case name projected into diagnostics.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SourceRoot => "source_root",
            Self::StorageRoot => "storage_root",
            Self::Materialization => "materialization",
            Self::McpInjection => "mcp_injection",
        }
    }
}

impl std::fmt::Display for OverlayStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A refusal to plan one activation reason for one provider.
///
/// Returned instead of a weakened plan: Claudine never converts requested
/// isolation into a less isolated launch. Carries only a variable *name* and
/// authored guidance — never a credential value or file content (Invariant 8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayRefusal {
    provider: Provider,
    reason: OverlayReason,
    selector: Option<&'static str>,
    next_action: String,
}

impl OverlayRefusal {
    /// The provider whose overlay was requested.
    pub fn provider(&self) -> Provider {
        self.provider
    }

    /// The single activation reason that cannot be satisfied. Other reasons in
    /// the same request may still be plannable on their own.
    pub fn reason(&self) -> OverlayReason {
        self.reason
    }

    /// The provider-owned variable that would have carried the overlay, when
    /// the provider has one at all.
    pub fn selector(&self) -> Option<&'static str> {
        self.selector
    }

    /// What the caller can do instead.
    pub fn next_action(&self) -> &str {
        &self.next_action
    }
}

impl From<OverlayRefusal> for ClaudineError {
    fn from(refusal: OverlayRefusal) -> Self {
        ClaudineError::ProviderOverlayUnsupported {
            provider: refusal.provider,
            reason: refusal.reason,
            selector: refusal.selector,
            next_action: refusal.next_action,
        }
    }
}

/// Everything one launch needs to redirect a provider's configuration without
/// touching the child's home identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayPlan {
    provider: Provider,
    reasons: OverlayReasons,
    verdicts: Vec<(OverlayReason, OverlayCapability)>,
    source_root: Option<PathBuf>,
    source_root_is_explicit: bool,
    selector: Option<OverlaySelector>,
    storage: Option<OverlayStorage>,
    isolated_resources: &'static [&'static str],
    materializations: Vec<OverlayMaterialization>,
    external_state: Vec<(OsString, OsString)>,
    lease: Option<Arc<OverlayLease>>,
}

impl OverlayPlan {
    /// The provider this plan launches.
    pub fn provider(&self) -> Provider {
        self.provider
    }

    /// Why the overlay was requested.
    pub fn reasons(&self) -> OverlayReasons {
        self.reasons
    }

    /// The verdict recorded for one requested reason, or `None` when the
    /// reason was not requested.
    pub fn capability(&self, reason: OverlayReason) -> Option<OverlayCapability> {
        self.verdicts
            .iter()
            .find(|(candidate, _)| *candidate == reason)
            .map(|(_, capability)| *capability)
    }

    /// Whether any requested reason needs a filesystem overlay.
    ///
    /// False for a plan whose reasons are all satisfied inline — OpenCode MCP
    /// injection, which must acquire no storage root.
    pub fn requires_materialization(&self) -> bool {
        self.storage.is_some()
    }

    /// Where the provider reads its configuration with no overlay in play.
    ///
    /// This is the directory the overlay is built *from*. It is never the
    /// destination, even when the user named it explicitly.
    pub fn source_root(&self) -> Option<&Path> {
        self.source_root.as_deref()
    }

    /// Whether the source root came from an explicit ambient selector value
    /// rather than the provider's documented default.
    pub fn source_root_is_explicit(&self) -> bool {
        self.source_root_is_explicit
    }

    /// The provider-owned variable pointed at the overlay.
    pub fn selector(&self) -> Option<&OverlaySelector> {
        self.selector.as_ref()
    }

    /// The directory the selector's value names: this launch's own overlay
    /// root, which no other launch shares.
    pub fn storage_root(&self) -> Option<&Path> {
        self.storage.as_ref().map(OverlayStorage::storage_root)
    }

    /// The directory the provider reads its configuration from.
    pub fn provider_visible_root(&self) -> Option<&Path> {
        self.storage
            .as_ref()
            .map(OverlayStorage::provider_visible_root)
    }

    /// Entries the overlay carries as real content.
    pub fn materializations(&self) -> &[OverlayMaterialization] {
        &self.materializations
    }

    /// Tie the overlay root's lifetime to this plan.
    ///
    /// Clones share the lease, so the root is removed only after the last
    /// clone — including a retry's recorded launch — is dropped.
    pub fn hold_lease(&mut self, lease: OverlayLease) {
        self.lease = Some(Arc::new(lease));
    }

    /// Record an entry the overlay must contain as real content.
    pub fn materialize(&mut self, entry: OverlayMaterialization) {
        self.materializations.push(entry);
    }

    /// Pin live state outside the overlay through a provider-native state
    /// selector, so a database or journal is never mirrored into it.
    pub fn pin_external_state(&mut self, name: impl Into<OsString>, value: impl Into<OsString>) {
        self.external_state.push((name.into(), value.into()));
    }

    /// Names directly under the source root that the overlay must not mirror.
    ///
    /// Two sources: the provider's documented `--repo` isolation set, and every
    /// top-level entry a materialization already owns — mirroring those would
    /// put a link where a real copy is required.
    pub fn excluded_resources(&self) -> BTreeSet<String> {
        let mut excluded: BTreeSet<String> = self
            .isolated_resources
            .iter()
            .map(|name| (*name).to_string())
            .collect();

        let Some(visible) = self.provider_visible_root() else {
            return excluded;
        };
        for entry in &self.materializations {
            if let Ok(relative) = entry.destination.strip_prefix(visible)
                && let Some(name) = relative.file_name()
                && relative.components().count() == 1
            {
                excluded.insert(name.to_string_lossy().into_owned());
            }
        }
        excluded
    }

    /// The provider-owned environment patch the child receives: the selector
    /// itself, then any external state selector a profile pinned.
    ///
    /// Contains no home variable by construction — a plan has no way to name
    /// one.
    pub fn env_patch(&self) -> Vec<(OsString, OsString)> {
        self.selector
            .iter()
            .map(OverlaySelector::env_entry)
            .chain(self.external_state.iter().cloned())
            .collect()
    }
}

/// Builds an [`OverlayPlan`] from generated provider metadata and the
/// invocation's immutable launch baseline.
///
/// The baseline is the only environment the planner reads: a retry or provider
/// transition plans against the same values the invocation opened with, never
/// against a process the previous attempt may have mutated.
#[derive(Debug, Clone, Copy)]
pub struct OverlayPlanner<'a> {
    home: &'a HomeBaseline,
    env: &'a EnvBaseline,
    launch_id: Option<&'a str>,
}

impl<'a> OverlayPlanner<'a> {
    /// Plan against one invocation's captured home and environment.
    pub fn new(home: &'a HomeBaseline, env: &'a EnvBaseline) -> Self {
        Self {
            home,
            env,
            launch_id: None,
        }
    }

    /// Name the launch's overlay root `id` instead of a fresh unique name, for
    /// a caller that needs a deterministic path. Two live launches given the
    /// same id cannot both materialize.
    pub fn with_launch_id(mut self, id: &'a str) -> Self {
        self.launch_id = Some(id);
        self
    }

    /// Plan the overlay for `provider` and `reasons`.
    ///
    /// ## Errors
    ///
    /// Returns an [`OverlayRefusal`] naming the first reason the provider has
    /// no verified provider-owned mechanism for. The refusal is per reason: a
    /// caller that drops the refused reason can plan the rest.
    pub fn plan(
        &self,
        provider: Provider,
        reasons: OverlayReasons,
    ) -> Result<OverlayPlan, OverlayRefusal> {
        let mut verdicts = Vec::with_capacity(reasons.len());
        let mut needs_storage = false;
        for reason in reasons.iter() {
            let capability = provider.overlay_capability(reason);
            if capability == OverlayCapability::Unsupported {
                return Err(self.refuse(provider, reason));
            }
            needs_storage |= capability == OverlayCapability::NativeRoot;
            verdicts.push((reason, capability));
        }

        let mut plan = OverlayPlan {
            provider,
            reasons,
            verdicts,
            source_root: None,
            source_root_is_explicit: false,
            selector: None,
            storage: None,
            isolated_resources: &[],
            materializations: Vec::new(),
            external_state: Vec::new(),
            lease: None,
        };

        if !needs_storage {
            // Every requested reason is satisfied inline. OpenCode MCP
            // injection lands here and must acquire no storage root.
            return Ok(plan);
        }

        // Every remaining failure is the same class of event — the metadata
        // promised a native root the host cannot supply — so they all refuse
        // the reason that asked for one.
        let unplannable = || self.refuse(provider, self.first_native_reason(reasons, provider));

        let spec = provider.overlay_selector().ok_or_else(unplannable)?;
        let (source_root, explicit) = self.resolve_source_root(provider).ok_or_else(unplannable)?;
        let overlay_home = self.overlay_home().ok_or_else(unplannable)?;
        let launch_root = overlay_home
            .join(OVERLAY_LAUNCHES_DIR)
            .join(provider.as_slug())
            .join(self.launch_id.map_or_else(fresh_launch_id, str::to_string));
        let storage = OverlayStorage::new(&launch_root, spec.shape).ok_or_else(unplannable)?;

        plan.selector = Some(OverlaySelector::new(spec, storage.storage_root()));
        plan.storage = Some(storage);
        plan.source_root = Some(source_root);
        plan.source_root_is_explicit = explicit;
        if reasons.contains(OverlayReason::RepoResources) {
            plan.isolated_resources = repo_isolated_resources(provider.agent_offset());
        }

        Ok(plan)
    }

    /// The pre-overlay provider configuration directory, and whether the user
    /// named it explicitly.
    ///
    /// An ambient selector value is the user's own provider root: it is what
    /// the overlay is built *from*, never where it is built. The provider's
    /// documented default applies only when the user supplied nothing.
    fn resolve_source_root(&self, provider: Provider) -> Option<(PathBuf, bool)> {
        let spec = provider.overlay_selector()?;
        if let Some(ambient) = self.env.get(spec.env_var).filter(|value| !value.is_empty()) {
            return Some((provider_visible_root(spec.shape, Path::new(ambient)), true));
        }
        let home = self.home.resolved()?;
        Some((spec.source_root?.resolve_with_home(home), false))
    }

    /// Claudine's data directory, which hosts every launch's overlay root.
    fn overlay_home(&self) -> Option<PathBuf> {
        Some(self.home.resolved()?.join(".claudine"))
    }

    /// The reason a materialization-shaped refusal names: the first requested
    /// reason whose verdict claims a native root.
    fn first_native_reason(&self, reasons: OverlayReasons, provider: Provider) -> OverlayReason {
        reasons
            .iter()
            .find(|reason| provider.overlay_capability(*reason) == OverlayCapability::NativeRoot)
            .unwrap_or(OverlayReason::RepoResources)
    }

    fn refuse(&self, provider: Provider, reason: OverlayReason) -> OverlayRefusal {
        let selector = provider.overlay_selector().map(|spec| spec.env_var);
        let next_action = match reason {
            OverlayReason::RepoResources => format!(
                "run {provider} without --repo; repository-scoped isolation has no \
                 verified provider-owned mechanism for this provider"
            ),
            OverlayReason::RepoPrompt => format!(
                "run {provider} without repository prompt discovery; it has no \
                 verified provider-owned prompt root"
            ),
            OverlayReason::Mcp => format!(
                "run {provider} without --mcp/--use, or export the servers with \
                 `claudine mcp export`"
            ),
        };
        OverlayRefusal {
            provider,
            reason,
            selector,
            next_action,
        }
    }
}

/// A directory name no other launch on this host uses: the process id keeps
/// concurrent processes apart, and the clock plus a counter keeps one
/// process's successive launches apart.
fn fresh_launch_id() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    format!(
        "{}-{nanos:x}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

/// Top-level names under a provider's configuration root that `--repo` hides.
///
/// Keyed by agent offset because that is how the documented per-provider
/// exclusion table in `docs/topics/repo-isolation.md` is written. This is the
/// single copy; the wrapper's overlay materialization reads it from here.
pub fn repo_isolated_resources(agent_offset: &str) -> &'static [&'static str] {
    match agent_offset {
        ".claude" => &["skills", "commands", "agents", "hooks"],
        ".codex" => &["skills", "agents", "prompts"],
        ".gemini" => &["skills", "agents"],
        ".goose" => &["skills", "agents"],
        ".kimi" => &["skills", "agents"],
        ".opencode" => &["skills"],
        ".qwen" => &["skills", "commands"],
        _ => &["skills", "commands", "agents", "hooks"],
    }
}
