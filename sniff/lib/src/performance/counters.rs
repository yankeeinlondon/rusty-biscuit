//! Stable work-counter names.
//!
//! Each name identifies one Sniff-owned unit of work at a boundary Sniff
//! controls — not every standard-library call it transitively causes. A
//! counter is only added where one increment corresponds to something a
//! reviewer would recognize as work: an entry the walker yielded, a manifest
//! Sniff chose to parse, a process Sniff chose to spawn.
//!
//! ## Notes
//!
//! These names are the comparison keys between phase baselines, so treat them
//! as a contract: renaming one silently invalidates every archived baseline
//! that used it. Counters absent from a report mean zero — recording never
//! writes a zero.

// ---------------------------------------------------------------------------
// Filesystem walking and classification
// ---------------------------------------------------------------------------

/// Directory entries the shared walker yielded, of any file type.
pub const FS_WALK_ENTRIES: &str = "filesystem.walk.entries_visited";

/// Shared descendant walks started for a request.
pub const FS_WALK_STARTS: &str = "filesystem.walk.walks_started";

/// Files accepted into the inventory and classified.
pub const FS_INVENTORY_ACCEPTED: &str = "filesystem.file_inventory.files_accepted";

/// Entries rejected because the accepted-classification cap was already met.
///
/// A nonzero value means the inventory is truncated.
pub const FS_INVENTORY_SATURATED: &str = "filesystem.file_inventory.entries_over_cap";

/// Markdown documents parsed for metadata during a walk.
pub const FS_DOCS_PARSED: &str = "filesystem.docs.documents_parsed";

// ---------------------------------------------------------------------------
// Filesystem primitives Sniff controls
// ---------------------------------------------------------------------------

/// Files Sniff opened or read whole.
pub const FS_FILE_OPENS: &str = "filesystem.io.file_opens";

/// Bytes read by Sniff-initiated file reads.
pub const FS_BYTES_READ: &str = "filesystem.io.bytes_read";

/// Metadata probes: `metadata`, `symlink_metadata`, and `exists` checks.
pub const FS_METADATA_PROBES: &str = "filesystem.io.metadata_probes";

/// Directories enumerated outside the shared walker (`read_dir`).
pub const FS_READ_DIRS: &str = "filesystem.io.read_dirs";

/// `canonicalize` calls.
pub const FS_CANONICALIZATIONS: &str = "filesystem.io.canonicalizations";

// ---------------------------------------------------------------------------
// Repository structure
// ---------------------------------------------------------------------------

/// Package manifests parsed (`Cargo.toml`, `package.json`, `pyproject.toml`,
/// `go.mod`, and peers).
pub const REPO_MANIFEST_PARSES: &str = "filesystem.repo.manifest_parses";

/// Lockfiles parsed (`Cargo.lock`, `pnpm-lock.yaml`, and peers).
pub const REPO_LOCKFILE_PARSES: &str = "filesystem.repo.lockfile_parses";

/// Root-scoped configuration inputs parsed (test-runner and tool config).
pub const REPO_CONFIG_PARSES: &str = "filesystem.repo.config_parses";

/// Package boundaries enriched with languages, dependencies, and frameworks.
pub const REPO_PACKAGE_ENRICHMENTS: &str = "filesystem.repo.package_enrichments";

/// Nested-workspace-marker fallback walks started.
///
/// Nonzero means repo detection enumerated the tree for marker evidence the
/// request-scoped observation index did not supply — the structure-only path,
/// or a root the index deliberately was not built for.
pub const REPO_NESTED_MARKER_WALKS: &str = "filesystem.repo.nested_marker_walks";

/// Membership-glob fallback subtree walks started.
///
/// Nonzero means glob expansion enumerated a pattern's literal-prefix subtree
/// because no observed manifest-directory evidence was supplied. Counted
/// separately from [`FS_READ_DIRS`], which several unrelated detectors share.
pub const REPO_MEMBERSHIP_GLOB_WALKS: &str = "filesystem.repo.membership_glob_walks";

// ---------------------------------------------------------------------------
// Git
// ---------------------------------------------------------------------------

/// Upward repository discoveries.
pub const GIT_DISCOVERIES: &str = "git.repository_discoveries";

/// Repository opens at a known path (no upward search).
pub const GIT_OPENS: &str = "git.repository_opens";

/// Working-tree status walks.
pub const GIT_STATUS_WALKS: &str = "git.status_walks";

/// Blobs or worktree file sides loaded for diffing.
pub const GIT_BLOB_LOADS: &str = "git.blob_loads";

/// Per-file content diffs computed.
pub const GIT_FILE_DIFFS: &str = "git.file_diffs";

/// Reference enumerations (ref globs and peels over the ref store).
pub const GIT_REF_WALKS: &str = "git.ref_walks";

/// Commits visited by a revision or ancestry walk.
pub const GIT_COMMIT_VISITS: &str = "git.commit_visits";

/// Linked worktrees opened as repositories.
pub const GIT_WORKTREE_OPENS: &str = "git.worktree_opens";

// ---------------------------------------------------------------------------
// Subprocesses
// ---------------------------------------------------------------------------

/// Child processes spawned.
pub const PROC_SPAWNS: &str = "process.spawns";

/// Child processes killed for exceeding their deadline.
pub const PROC_TIMEOUTS: &str = "process.timeouts";

// ---------------------------------------------------------------------------
// Remote and network
// ---------------------------------------------------------------------------

/// Remote provider API requests, counted per logical operation.
///
/// Paired with [`remote_operation`] so a report distinguishes a duplicated
/// root-tree request from a correctness-preserving continuation request.
pub const REMOTE_REQUESTS: &str = "remote.api_requests";

/// Per-operation remote request counter name, e.g. `remote.api_requests.tree`.
pub fn remote_operation(operation: &str) -> String {
    format!("{REMOTE_REQUESTS}.{operation}")
}

/// WAN IP endpoint attempts.
pub const NETWORK_WAN_ATTEMPTS: &str = "network.wan_ip.endpoint_attempts";
