//! Corpus guard: no archive-executed target may resolve a path at compile time.
//!
//! Every hosted L1, L2, browser, and WSL2 cell runs binaries a *different* job
//! compiled, extracted somewhere the producer never wrote, against a checkout at
//! a different absolute path. `env!("CARGO_MANIFEST_DIR")` and
//! `env!("CARGO_BIN_EXE_…")` freeze the producer's directories into the binary,
//! so a test that reads either is green on every machine that builds it and red
//! only on the consumer. `biscuit_test_harness::manifest_dir!` and
//! `biscuit_test_harness::bin_exe!` ask the environment first —
//! `--workspace-remap` and `NEXTEST_BIN_EXE_*` are what the consumer rewrites.
//!
//! [`ALLOWED`] is the complete set of files that may still bake a path in, each
//! with the reason it is not archive-executed; an entry that matches no live
//! site fails too, so the list burns down instead of becoming a grandfather
//! table.
//!
//! ## Token awareness
//!
//! Source is lexed once into identifiers, puncts, and string literals. Line and
//! block comments (nested included) are dropped, and a string literal is one
//! opaque token, so `env!("CARGO_MANIFEST_DIR")` written in prose or held in a
//! fixture corpus is not an invocation and is not reported. A real invocation is
//! matched as the token sequence `env` `!` `(` *string* `)`, which makes
//! whitespace and line breaks inside the call irrelevant.
//!
//! The hosted-root literal check is deliberately the opposite: it runs as its
//! own pass over string-literal tokens, because a baked
//! `"/home/runner/work/…"` path *is* a string literal. No runtime-fallback
//! recognition can suppress it.
//!
//! ## Recognized runtime-first fallback shapes
//!
//! `biscuit_test_harness::manifest_dir!()` / `bin_exe!()` remain the preferred
//! spelling. An inline fallback is *tolerated* when the compile-time value is
//! the fallback of the same runtime-first expression — and only for that one
//! occurrence. This is pattern matching over a closed set of shapes, not data
//! flow analysis; anything outside the set is reported with the shared macro as
//! the remedy rather than granted a file-wide exemption.
//!
//! Two tails are accepted wherever a tail appears below:
//!
//! ```text
//! T1: .map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("<VAR>")))
//! T2: .map_or_else(|| PathBuf::from(env!("<VAR>")), PathBuf::from)
//! ```
//!
//! The manifest shapes, where `<v>` is any single binding used consistently and
//! `var_os` / `PathBuf` may carry any leading path (`std::env::var_os`, …):
//!
//! ```text
//! M1: var_os("CARGO_MANIFEST_DIR").filter(|<v>| !<v>.is_empty()) <T1>
//! M2: var_os("CARGO_MANIFEST_DIR").filter(|<v>| !<v>.is_empty()) <T2>
//! ```
//!
//! The binary shapes, which must name the *same* target in all three places and
//! must try nextest's variable first, matching
//! [`biscuit_test_harness::bin_exe!`]'s resolution order:
//!
//! ```text
//! B1: var_os("NEXTEST_BIN_EXE_<mangled>").filter(|<v>| !<v>.is_empty())
//!         .or_else(|| var_os("CARGO_BIN_EXE_<name>").filter(|<w>| !<w>.is_empty()))
//!         <T1>
//! B2: … the same head, with <T2>
//! ```
//!
//! `<mangled>` is `<name>` with `-` replaced by `_`. The `env!` literal must be
//! `CARGO_BIN_EXE_<name>` verbatim.
//!
//! The empty-value rejection is part of every shape on purpose: an exported but
//! empty variable is how a shell spells "unset", and a shape that accepts it
//! disagrees with the shared helpers.
//!
//! ## Notes
//!
//! Scan scope comes from [`GuardPlan`], which the CI planner supplies through
//! the `BISCUIT_ARCHIVE_GUARD_PLAN` environment variable. With no plan the scan
//! covers the full eligible tree.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A file that may still bake a path in, and the reason it is not
/// archive-executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllowEntry {
    /// Repository-relative, `/`-separated.
    pub file: &'static str,
    /// Why this file is never run from an archive. Must not be empty.
    pub reason: &'static str,
}

/// The complete exemption list. See [`allowlist_problems`] for what keeps it
/// honest.
pub const ALLOWED: &[AllowEntry] = &[
    AllowEntry {
        file: "biscuit-test-harness/src/bin_exe.rs",
        reason: "defines the runtime lookups; the compile-time value is their documented fallback",
    },
    AllowEntry {
        file: "darkmatter/cli/src/commands/schema/about.rs",
        reason: "`include_str!` embeds the bytes at compile time, so the archived binary carries \
                 the document and opens no path at run time",
    },
    AllowEntry {
        file: "biscuit-icon/lib/src/bin/populate_assets.rs",
        reason: "dev-only `just populate-assets` bin with no test module; it regenerates \
                 committed assets from the checkout it is run in",
    },
    AllowEntry {
        file: "unchained-ai/gen/src/main.rs",
        reason: "`default_output_dir` for the manually-run `gen-models` bin; unreachable from \
                 that bin's own test module and from every test target",
    },
    AllowEntry {
        file: "unchained-ai/gen/src/bin/emit_catalog.rs",
        reason: "same, for the manually-run `emit-catalog` bin: it writes generated source back \
                 into the checkout it was launched from",
    },
];

/// Directories that are never archive-executed, and why.
///
/// Matched against any path component, so `examples` excludes every package's.
pub const SKIPPED_DIRS: &[(&str, &str)] = &[
    ("target", "build output"),
    (".git", "not source"),
    ("node_modules", "not Rust"),
    (".gitnexus", "index, not source"),
    (
        "scripts",
        "`repo-deps` runs from an archive like every other member, but its baked \
         `CARGO_MANIFEST_DIR` sites resolve on every hosted native consumer because those \
         share the producer's checkout path; `ci-rollup-tests.rs::repo_root` already falls \
         back to the run-time checkout, and the rest are the WSL2 leg's problem, not a \
         pull request's",
    ),
    (
        "examples",
        "examples are not test targets and nextest does not archive them",
    ),
    (
        "fuzz",
        "fuzz targets run on nightly from the checkout that built them",
    ),
];

/// The guard's own sources, excluded by exact repository-relative path.
///
/// Exclusion by basename suffix would also drop an unrelated package's file of
/// the same name. Each entry is here because it must be free to *spell* the
/// forbidden forms; the token-aware matcher already ignores the ones that live
/// in comments and string literals, so this is defense in depth for a future
/// edit that writes one as real code.
pub const GUARD_OWN_SOURCES: &[(&str, &str)] = &[
    (
        "tools/test-toolkit/src/archive_guard.rs",
        "the matcher: its constants and documentation name every form it hunts",
    ),
    (
        "tools/test-toolkit/src/archive_guard/matcher_tests.rs",
        "the matcher's fixture corpus is built out of the forbidden forms",
    ),
    (
        "tools/test-toolkit/tests/archive_path_guard.rs",
        "the driver, which quotes the forms in its failure diagnostics",
    ),
];

/// A hosted runner's workspace root, which names the producer and nothing else.
///
/// Deliberately the only absolute-path form checked. `"/Users/…"` and
/// `"/Volumes/…"` are overwhelmingly synthetic *inputs* to pure path functions
/// (`document_within_workspace("/Users/ken/repo/x.md", …)`) rather than lookups,
/// and no textual rule separates the two; a check that flagged them would be
/// suppressed rather than obeyed. A baked producer path that survives this
/// guard is caught instead by the relocation fixtures in
/// `scripts/ci-build-archive-tests.rs`, which run with that path deleted.
pub const HOSTED_ROOT: &str = "/home/runner/work/";

const MANIFEST_VAR: &str = "CARGO_MANIFEST_DIR";
const BIN_EXE_PREFIX: &str = "CARGO_BIN_EXE_";
const NEXTEST_BIN_EXE_PREFIX: &str = "NEXTEST_BIN_EXE_";

const MANIFEST_REMEDY: &str = "use `biscuit_test_harness::manifest_dir!()`";
const BIN_EXE_REMEDY: &str = "use `biscuit_test_harness::bin_exe!(\"<bin>\")`";
const HOSTED_ROOT_REMEDY: &str = "an absolute path to this host's checkout resolves nowhere else; \
                                  build it from a runtime-resolved root";

/// Environment variable naming the canonical resolved-plan JSON.
pub const PLAN_ENV: &str = "BISCUIT_ARCHIVE_GUARD_PLAN";

/// Top-level key the planner writes this guard's scope under.
pub const PLAN_KEY: &str = "archive_guard";

/// The resolved-plan generation this reader understands.
///
/// The same number is `scripts/ci/schema.py::RESOLVED_PLAN_SCHEMA_VERSION` and
/// `resolved_plan.schema_version` in `.github/ci/schemas/contract.json`. That
/// shipped contract is what keeps the Python validator and every Rust reader in
/// step; `the_plan_schema_version_matches_the_frozen_contract` fails here when
/// one side is bumped alone, the way an earlier iteration of this fix stranded
/// the pre-push fixtures.
pub const PLAN_SCHEMA_VERSION: u64 = 5;

/// The closed field set of the plan's [`PLAN_KEY`] object, sorted.
///
/// The same set is `scripts/ci/schema.py::ARCHIVE_GUARD_FIELDS` and
/// `resolved_plan.archive_guard` in `.github/ci/schemas/contract.json`;
/// `the_guard_scope_field_set_matches_the_frozen_contract` fails here when a
/// field is added on one side alone.
pub const PLAN_SCOPE_FIELDS: [&str; 4] = ["mode", "paths", "reason", "selected"];

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Anything that stops the guard from answering its question.
///
/// Every variant is a hard failure: a scan that could not read its inputs is
/// never reported as a pass.
#[derive(Debug)]
pub enum GuardError {
    /// A directory that exists could not be listed.
    ReadDir {
        /// The directory, as the scan reached it.
        path: PathBuf,
        /// The underlying failure.
        source: io::Error,
    },
    /// A file that exists could not be read.
    ReadFile {
        /// The file, as the scan reached it.
        path: PathBuf,
        /// The underlying failure.
        source: io::Error,
    },
    /// A path could not be inspected, so the scan cannot say what it is.
    ///
    /// Distinct from a path that is simply absent: full-tree discovery skips a
    /// vanished entry, a listed one fails as [`GuardError::MissingListedPath`],
    /// and every other stat failure lands here.
    Inspect {
        /// The path, as the scan reached it.
        path: PathBuf,
        /// The underlying failure.
        source: io::Error,
    },
    /// A listed path is absent from the checkout.
    MissingListedPath {
        /// The path exactly as the plan spelled it.
        path: String,
    },
    /// A listed path does not name a scannable file inside the checkout.
    Unscannable {
        /// The path exactly as the plan spelled it.
        path: String,
        /// Why it cannot be scanned.
        detail: String,
    },
    /// The plan named by [`PLAN_ENV`] could not be read.
    PlanUnreadable {
        /// The plan path taken from the environment.
        path: PathBuf,
        /// The underlying failure.
        source: io::Error,
    },
    /// The plan parsed as JSON but does not satisfy the guard's contract.
    MalformedPlan {
        /// The plan path taken from the environment.
        path: PathBuf,
        /// What the plan had to say and did not.
        detail: String,
    },
}

impl fmt::Display for GuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadDir { path, source } => write!(
                f,
                "archive-path guard could not list `{}`: {source}. \
                 Fix the permissions or remove the directory; the guard will not \
                 report a pass it could not verify.",
                path.display()
            ),
            Self::ReadFile { path, source } => write!(
                f,
                "archive-path guard could not read `{}`: {source}. \
                 Fix the permissions or remove the file; the guard will not \
                 report a pass it could not verify.",
                path.display()
            ),
            Self::Inspect { path, source } => write!(
                f,
                "archive-path guard could not inspect `{}`: {source}. \
                 Fix the permissions or remove the entry; the guard will not \
                 report a pass it could not verify.",
                path.display()
            ),
            Self::MissingListedPath { path } => write!(
                f,
                "archive-path guard was given `{path}`, which does not exist. \
                 The planner omits every path its diff reports as deleted, so a listed \
                 path that is absent disappeared after the scope was resolved; the guard \
                 will not report a pass over a file it never read.",
            ),
            Self::Unscannable { path, detail } => write!(
                f,
                "archive-path guard was given `{path}`, which {detail}. \
                 The guard reads files inside the checkout and nothing else; a path \
                 that leaves it is out of scope whether or not it exists.",
            ),
            Self::PlanUnreadable { path, source } => write!(
                f,
                "{PLAN_ENV} points at `{}`, which could not be read: {source}. \
                 Unset {PLAN_ENV} to request a full-tree scan, or repair the plan.",
                path.display()
            ),
            Self::MalformedPlan { path, detail } => write!(
                f,
                "the resolved plan `{}` does not carry a usable `{PLAN_KEY}` scope: {detail}. \
                 A malformed plan is never an empty scan and never an implicit full-tree scan.",
                path.display()
            ),
        }
    }
}

impl std::error::Error for GuardError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReadDir { source, .. }
            | Self::ReadFile { source, .. }
            | Self::Inspect { source, .. }
            | Self::PlanUnreadable { source, .. } => Some(source),
            Self::MissingListedPath { .. }
            | Self::Unscannable { .. }
            | Self::MalformedPlan { .. } => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Scan scope
// ---------------------------------------------------------------------------

/// Why `path` is not a normalized repository-relative path, or `None`.
///
/// `scripts/ci/schema.py` enforces the same rules on the planner's side, and
/// they are repeated here because the plan is an *input* to this process:
/// `root.join("/etc/passwd")` discards `root` entirely, and a `..` component
/// walks out of the checkout without ever looking absolute. The phrasing is
/// the predicate, so a caller can splice it after the offending path.
fn unscannable_spelling(path: &str) -> Option<&'static str> {
    let mut drive = path.chars();
    let drive_letter = matches!(drive.next(), Some(first) if first.is_ascii_alphabetic())
        && drive.next() == Some(':');

    if path.is_empty() {
        Some("is empty, and names no file")
    } else if path != path.trim() {
        Some("is padded with whitespace")
    // Containment before spelling: a native Windows absolute path is both
    // drive-prefixed and backslash-spelled, and "it leaves the checkout" is
    // the more useful of the two answers.
    } else if path.starts_with('/') {
        Some("is absolute, and a scan scope is relative to the checkout")
    } else if drive_letter {
        Some("carries a Windows drive prefix, and a scan scope is relative to the checkout")
    } else if path.contains('\\') {
        Some("is spelled with backslashes rather than one POSIX spelling")
    } else if path.split('/').any(|component| component == "..") {
        Some("climbs out of the checkout with a `..` component")
    } else if path.starts_with("./") {
        Some("is not normalized; drop the leading `./`")
    } else {
        None
    }
}

/// Which files a scan looks at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanMode {
    /// Discover and check the whole eligible corpus.
    FullTree,
    /// Check only these repository-relative paths, after the same eligibility
    /// policy discovery uses. An empty list is a real state — "nothing eligible
    /// changed" — and is not a full scan.
    ///
    /// The planner omits the paths its diff reports as deleted, so every listed
    /// path is one the scan is entitled to read.
    Changed(Vec<String>),
}

impl ScanMode {
    fn label(&self) -> String {
        match self {
            Self::FullTree => "full-tree".to_owned(),
            Self::Changed(paths) => format!("changed({} listed)", paths.len()),
        }
    }
}

/// The guard's scope as the CI planner resolved it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardPlan {
    /// Whether the planner scheduled the guard at all.
    pub selected: bool,
    /// The scope to scan. Meaningless when `selected` is false.
    pub mode: ScanMode,
    /// Why the scope is what it is, echoed in the printed summary.
    ///
    /// Never empty. The plan contract requires a non-blank reason in all three
    /// of its shapes, so the reader rejects a plan without one rather than
    /// modelling an absence the planner cannot express; the standalone default
    /// states its own.
    pub reason: String,
}

impl GuardPlan {
    /// The standalone default: no plan supplied, so check everything.
    #[must_use]
    pub fn full_tree() -> Self {
        Self {
            selected: true,
            mode: ScanMode::FullTree,
            reason: format!("{PLAN_ENV} is unset; the standalone default checks the full tree"),
        }
    }

    /// Read the plan [`PLAN_ENV`] names.
    ///
    /// ## Returns
    ///
    /// [`GuardPlan::full_tree`] when the variable is unset or empty.
    ///
    /// ## Errors
    ///
    /// Returns [`GuardError::PlanUnreadable`] when the variable names a file
    /// that cannot be read, and [`GuardError::MalformedPlan`] for every way the
    /// document can fail the plan contract — see [`GuardPlan::from_plan_json`].
    /// Neither degrades to an empty scan or to a silent full-tree scan.
    pub fn from_env() -> Result<Self, GuardError> {
        match std::env::var_os(PLAN_ENV) {
            Some(raw) if !raw.is_empty() => Self::from_plan_file(Path::new(&raw)),
            _ => Ok(Self::full_tree()),
        }
    }

    /// Read a resolved-plan JSON document from disk.
    ///
    /// ## Errors
    ///
    /// As [`GuardPlan::from_env`].
    pub fn from_plan_file(path: &Path) -> Result<Self, GuardError> {
        let raw = fs::read_to_string(path).map_err(|source| GuardError::PlanUnreadable {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_plan_json(path, &raw)
    }

    /// Parse a resolved-plan JSON document.
    ///
    /// `path` is used only in diagnostics.
    ///
    /// ## Errors
    ///
    /// Returns [`GuardError::MalformedPlan`] for invalid JSON, a
    /// `schema_version` that is missing, non-numeric, or not
    /// [`PLAN_SCHEMA_VERSION`], a missing `archive_guard` key, an
    /// `archive_guard` that is not an object or carries a field outside
    /// [`PLAN_SCOPE_FIELDS`], a missing, blank, or non-string `reason`,
    /// `selected: false` carrying `mode` or `paths`, an unknown `mode`,
    /// `mode: "changed"` without `paths`, `mode: "full"` carrying `paths`, or a
    /// `paths` list that is not an array of sorted, unique, normalized
    /// repository-relative strings — the path rules before any filesystem
    /// access, so a plan can never send the scan out of the checkout.
    ///
    /// These are the rules `scripts/ci/schema.py` applies on the planner's
    /// side. They are enforced again here because
    /// [`PLAN_ENV`] hands this reader a document the Python validator never
    /// saw.
    pub fn from_plan_json(path: &Path, raw: &str) -> Result<Self, GuardError> {
        let malformed = |detail: String| GuardError::MalformedPlan {
            path: path.to_path_buf(),
            detail,
        };

        let document: serde_json::Value = serde_json::from_str(raw)
            .map_err(|err| malformed(format!("it is not valid JSON ({err})")))?;

        // Version before shape, as `validate_resolved_plan` does: a document
        // from another generation usually differs in both, and reporting the
        // field it lacks sends the reader after a corrupt plan when the answer
        // is that the two sides moved apart.
        match document.get("schema_version") {
            None => {
                return Err(malformed(format!(
                    "it declares no `schema_version`; this reader understands \
                     {PLAN_SCHEMA_VERSION}"
                )));
            }
            Some(version) => match version.as_u64() {
                Some(version) if version == PLAN_SCHEMA_VERSION => {}
                Some(version) => {
                    return Err(malformed(format!(
                        "it is resolved-plan schema version {version}; this reader understands \
                         {PLAN_SCHEMA_VERSION}"
                    )));
                }
                None => {
                    return Err(malformed(format!(
                        "`schema_version` is {version}, which is not a schema generation number"
                    )));
                }
            },
        }

        let scope = document
            .get(PLAN_KEY)
            .ok_or_else(|| malformed(format!("no top-level `{PLAN_KEY}` key")))?;

        let reason = match scope.get("reason") {
            None => {
                return Err(malformed(
                    "`reason` is missing; every plan states why it holds the scope it does"
                        .to_owned(),
                ));
            }
            Some(value) => value.as_str().ok_or_else(|| {
                malformed(format!("`reason` is {value}, which is not a string"))
            })?,
        };
        if reason.trim().is_empty() {
            return Err(malformed(
                "`reason` is blank; every plan states why it holds the scope it does".to_owned(),
            ));
        }
        let reason = reason.to_owned();

        let selected = scope
            .get("selected")
            .and_then(serde_json::Value::as_bool)
            .ok_or_else(|| malformed("`selected` is missing or not a boolean".to_owned()))?;

        if !selected {
            for name in ["mode", "paths"] {
                if scope.get(name).is_some() {
                    return Err(malformed(format!(
                        "`selected` is false but `{name}` is present; an unselected guard \
                         describes no scope, and stale scope beside it would scan as if it had \
                         been planned"
                    )));
                }
            }
            return Ok(Self {
                selected: false,
                mode: ScanMode::Changed(Vec::new()),
                reason,
            });
        }

        let mode = scope
            .get("mode")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| malformed("`selected` is true but `mode` is missing".to_owned()))?;
        let paths = scope.get("paths");

        let mode = match (mode, paths) {
            ("full", None) => ScanMode::FullTree,
            ("full", Some(_)) => {
                return Err(malformed(
                    "`mode` is \"full\" but `paths` is present; a full scan takes no path list"
                        .to_owned(),
                ));
            }
            ("changed", Some(paths)) => {
                let listed = paths.as_array().ok_or_else(|| {
                    malformed("`paths` is present but is not an array".to_owned())
                })?;
                let mut collected = Vec::with_capacity(listed.len());
                for entry in listed {
                    let text = entry.as_str().ok_or_else(|| {
                        malformed(format!("`paths` holds a non-string entry {entry}"))
                    })?;
                    if let Some(problem) = unscannable_spelling(text) {
                        return Err(malformed(format!(
                            "`paths` holds `{text}`, which {problem}"
                        )));
                    }
                    collected.push(text.to_owned());
                }
                // One order and one spelling per path, so two readers of the
                // same plan cannot disagree about what is in scope. Sortedness
                // first: it makes any repeat adjacent.
                if let Some(pair) = collected.windows(2).find(|pair| pair[0] > pair[1]) {
                    return Err(malformed(format!(
                        "`paths` is not sorted; `{}` precedes `{}`",
                        pair[0], pair[1]
                    )));
                }
                if let Some(pair) = collected.windows(2).find(|pair| pair[0] == pair[1]) {
                    return Err(malformed(format!(
                        "`paths` lists `{}` more than once; a scan scope names each path once",
                        pair[0]
                    )));
                }
                ScanMode::Changed(collected)
            }
            ("changed", None) => {
                return Err(malformed(
                    "`mode` is \"changed\" but `paths` is missing; an empty scan must be spelled \
                     as an explicit empty list"
                        .to_owned(),
                ));
            }
            (other, _) => {
                return Err(malformed(format!(
                    "`mode` is \"{other}\"; expected \"full\" or \"changed\""
                )));
            }
        };

        Ok(Self {
            selected: true,
            mode,
            reason,
        })
    }
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

/// One compile-time path form that does not survive relocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Repository-relative, `/`-separated.
    pub file: String,
    /// 1-based line of the offending token.
    pub line: usize,
    /// What to write instead.
    pub remedy: &'static str,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "  {}:{} — {}", self.file, self.line, self.remedy)
    }
}

/// What a scan looked at and what it found.
#[derive(Debug, Clone)]
pub struct ScanReport {
    /// The mode the scan ran in.
    pub mode: ScanMode,
    /// How many files were read. Zero is a legitimate result in
    /// [`ScanMode::Changed`] and is never a full-tree pass.
    pub files_checked: usize,
    /// Listed paths the eligibility policy filtered out (not `.rs`, a
    /// `build.rs`, under a skipped directory, or one of the guard's own
    /// sources).
    pub ineligible: Vec<String>,
    /// Every violation, ordered by file then line.
    pub violations: Vec<Violation>,
}

impl ScanReport {
    /// One line naming the mode and the number of files actually read.
    ///
    /// Printed by the driver so an empty changed-file scan can never be
    /// mistaken for full-tree validation.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "archive-path guard: mode={} files-checked={} ineligible={} violations={}",
            self.mode.label(),
            self.files_checked,
            self.ineligible.len(),
            self.violations.len(),
        )
    }
}

// ---------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    Ident(String),
    Punct(char),
    /// The inner text of a string literal, verbatim — escapes are *not*
    /// resolved, which is enough for the fixed variable names and the hosted
    /// root prefix this guard matches.
    Str(String),
    /// Numbers, char literals, lifetimes: matched by nothing, kept so a
    /// pattern cannot accidentally step over one.
    Other,
}

#[derive(Debug, Clone)]
struct Tok {
    kind: Kind,
    line: usize,
}

const fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte >= 0x80
}

const fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}

fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    starts.extend(
        source
            .bytes()
            .enumerate()
            .filter(|(_, byte)| *byte == b'\n')
            .map(|(at, _)| at + 1),
    );
    starts
}

fn line_of(starts: &[usize], offset: usize) -> usize {
    match starts.binary_search(&offset) {
        Ok(index) => index + 1,
        Err(index) => index,
    }
}

/// Index just past the closing quote of a `"`-delimited literal whose body
/// starts at `from`, plus that body.
fn scan_quoted(bytes: &[u8], from: usize) -> (usize, String) {
    let mut at = from;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' => at += 2,
            b'"' => {
                let body = String::from_utf8_lossy(&bytes[from..at]).into_owned();
                return (at + 1, body);
            }
            _ => at += 1,
        }
    }
    (bytes.len(), String::from_utf8_lossy(&bytes[from..]).into_owned())
}

/// Index just past a raw string whose hash run (or opening quote) starts at
/// `from`, plus its body.
fn scan_raw(bytes: &[u8], from: usize) -> (usize, String) {
    let mut at = from;
    let mut hashes = 0usize;
    while bytes.get(at) == Some(&b'#') {
        hashes += 1;
        at += 1;
    }
    if bytes.get(at) != Some(&b'"') {
        // Not a raw string after all (`r` used as an identifier).
        return (from, String::new());
    }
    at += 1;
    let body_start = at;
    while at < bytes.len() {
        if bytes[at] == b'"' && bytes[at + 1..].iter().take(hashes).all(|b| *b == b'#') {
            let closes = bytes.len() - (at + 1) >= hashes;
            if closes {
                let body = String::from_utf8_lossy(&bytes[body_start..at]).into_owned();
                return (at + 1 + hashes, body);
            }
        }
        at += 1;
    }
    (
        bytes.len(),
        String::from_utf8_lossy(&bytes[body_start..]).into_owned(),
    )
}

/// Index just past a `/* … */` comment starting at `from`, honoring nesting.
fn scan_block_comment(bytes: &[u8], from: usize) -> usize {
    let mut at = from + 2;
    let mut depth = 1usize;
    while at < bytes.len() {
        if bytes[at] == b'/' && bytes.get(at + 1) == Some(&b'*') {
            depth += 1;
            at += 2;
        } else if bytes[at] == b'*' && bytes.get(at + 1) == Some(&b'/') {
            depth -= 1;
            at += 2;
            if depth == 0 {
                return at;
            }
        } else {
            at += 1;
        }
    }
    bytes.len()
}

/// Index just past a char literal or lifetime whose `'` sits at `from`.
fn scan_char_or_lifetime(bytes: &[u8], from: usize) -> usize {
    let mut at = from + 1;
    if bytes.get(at) == Some(&b'\\') {
        at += 1;
        while at < bytes.len() && bytes[at] != b'\'' {
            at += 1;
        }
        return (at + 1).min(bytes.len());
    }
    // A char literal is one scalar value followed by `'`; anything else is a
    // lifetime, whose name is an ordinary identifier.
    let width = match bytes.get(at) {
        None => return at,
        Some(byte) if *byte < 0x80 => 1,
        Some(byte) if *byte < 0xE0 => 2,
        Some(byte) if *byte < 0xF0 => 3,
        Some(_) => 4,
    };
    if bytes.get(at + width) == Some(&b'\'') {
        return at + width + 1;
    }
    while at < bytes.len() && is_ident_continue(bytes[at]) {
        at += 1;
    }
    at
}

fn tokenize(source: &str) -> Vec<Tok> {
    let bytes = source.as_bytes();
    let starts = line_starts(source);
    let mut toks = Vec::new();
    let mut at = 0usize;

    while at < bytes.len() {
        let byte = bytes[at];
        let start = at;
        match byte {
            b' ' | b'\t' | b'\r' | b'\n' => {
                at += 1;
            }
            b'/' if bytes.get(at + 1) == Some(&b'/') => {
                while at < bytes.len() && bytes[at] != b'\n' {
                    at += 1;
                }
            }
            b'/' if bytes.get(at + 1) == Some(&b'*') => {
                at = scan_block_comment(bytes, at);
            }
            b'"' => {
                let (next, body) = scan_quoted(bytes, at + 1);
                at = next;
                toks.push(Tok {
                    kind: Kind::Str(body),
                    line: line_of(&starts, start),
                });
            }
            b'\'' => {
                at = scan_char_or_lifetime(bytes, at);
                toks.push(Tok {
                    kind: Kind::Other,
                    line: line_of(&starts, start),
                });
            }
            b'0'..=b'9' => {
                at += 1;
                while at < bytes.len()
                    && (is_ident_continue(bytes[at])
                        || (bytes[at] == b'.'
                            && bytes.get(at + 1).is_some_and(u8::is_ascii_digit)))
                {
                    at += 1;
                }
                toks.push(Tok {
                    kind: Kind::Other,
                    line: line_of(&starts, start),
                });
            }
            _ if is_ident_start(byte) => {
                while at < bytes.len() && is_ident_continue(bytes[at]) {
                    at += 1;
                }
                let name = &source[start..at];
                let line = line_of(&starts, start);
                let next = bytes.get(at).copied();
                if matches!(name, "r" | "br" | "cr" | "rb") && matches!(next, Some(b'#' | b'"')) {
                    let (end, body) = scan_raw(bytes, at);
                    if end > at {
                        at = end;
                        toks.push(Tok {
                            kind: Kind::Str(body),
                            line,
                        });
                        continue;
                    }
                }
                if matches!(name, "b" | "c") && next == Some(b'"') {
                    let (end, body) = scan_quoted(bytes, at + 1);
                    at = end;
                    toks.push(Tok {
                        kind: Kind::Str(body),
                        line,
                    });
                    continue;
                }
                if name == "b" && next == Some(b'\'') {
                    at = scan_char_or_lifetime(bytes, at);
                    toks.push(Tok {
                        kind: Kind::Other,
                        line,
                    });
                    continue;
                }
                toks.push(Tok {
                    kind: Kind::Ident(name.to_owned()),
                    line,
                });
            }
            _ => {
                at += 1;
                toks.push(Tok {
                    kind: Kind::Punct(byte as char),
                    line: line_of(&starts, start),
                });
            }
        }
    }

    toks
}

// ---------------------------------------------------------------------------
// Matcher
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormKind {
    ManifestDir,
    BinExe,
}

impl FormKind {
    const fn remedy(self) -> &'static str {
        match self {
            Self::ManifestDir => MANIFEST_REMEDY,
            Self::BinExe => BIN_EXE_REMEDY,
        }
    }
}

#[derive(Debug, Clone)]
struct Occurrence {
    /// Index of the `env` identifier token, which is how a recognized fallback
    /// shape names the occurrence it exempts.
    token: usize,
    line: usize,
    kind: FormKind,
}

struct Cur<'a> {
    toks: &'a [Tok],
    at: usize,
}

impl<'a> Cur<'a> {
    fn new(toks: &'a [Tok], at: usize) -> Self {
        Self { toks, at }
    }

    fn punct(&mut self, ch: char) -> Option<()> {
        match self.toks.get(self.at)?.kind {
            Kind::Punct(found) if found == ch => {
                self.at += 1;
                Some(())
            }
            _ => None,
        }
    }

    fn ident(&mut self, name: &str) -> Option<()> {
        match &self.toks.get(self.at)?.kind {
            Kind::Ident(found) if found == name => {
                self.at += 1;
                Some(())
            }
            _ => None,
        }
    }

    fn any_ident(&mut self) -> Option<&'a str> {
        let toks = self.toks;
        match &toks.get(self.at)?.kind {
            Kind::Ident(found) => {
                self.at += 1;
                Some(found.as_str())
            }
            _ => None,
        }
    }

    fn str_lit(&mut self) -> Option<&'a str> {
        let toks = self.toks;
        match &toks.get(self.at)?.kind {
            Kind::Str(body) => {
                self.at += 1;
                Some(body.as_str())
            }
            _ => None,
        }
    }

    /// A `::`-joined path whose trailing segments are exactly `tail`.
    ///
    /// Lets the recognized shapes accept `std::env::var_os`, `env::var_os`, and
    /// a bare imported `var_os` without enumerating them.
    fn path_ending(&mut self, tail: &[&str]) -> Option<()> {
        let save = self.at;
        if self.punct(':').is_none() || self.punct(':').is_none() {
            self.at = save;
        }
        let Some(first) = self.any_ident() else {
            self.at = save;
            return None;
        };
        let mut segments = vec![first];
        loop {
            let mark = self.at;
            let joined = self.punct(':').and_then(|()| self.punct(':'));
            match joined.and_then(|()| self.any_ident()) {
                Some(segment) => segments.push(segment),
                None => {
                    self.at = mark;
                    break;
                }
            }
        }
        if segments.len() >= tail.len() && segments[segments.len() - tail.len()..] == *tail {
            Some(())
        } else {
            self.at = save;
            None
        }
    }

    /// `env!("…")`, yielding the `env` token index and the literal.
    fn env_macro(&mut self) -> Option<(usize, &'a str)> {
        let token = self.at;
        self.ident("env")?;
        self.punct('!')?;
        self.punct('(')?;
        let literal = self.str_lit()?;
        self.punct(')')?;
        Some((token, literal))
    }

    /// `[path::]var_os("…")`, yielding the variable name.
    fn var_os_read(&mut self) -> Option<&'a str> {
        self.path_ending(&["var_os"])?;
        self.punct('(')?;
        let name = self.str_lit()?;
        self.punct(')')?;
        Some(name)
    }

    /// `.filter(|v| !v.is_empty())` — the empty-value rejection every
    /// recognized shape requires, with the binding used consistently.
    fn nonempty_filter(&mut self) -> Option<()> {
        self.punct('.')?;
        self.ident("filter")?;
        self.punct('(')?;
        self.punct('|')?;
        let binding = self.any_ident()?;
        self.punct('|')?;
        self.punct('!')?;
        let used = self.any_ident()?;
        if used != binding {
            return None;
        }
        self.punct('.')?;
        self.ident("is_empty")?;
        self.punct('(')?;
        self.punct(')')?;
        self.punct(')')
    }
}

/// `PathBuf::from(env!("…"))`.
fn path_buf_from_env(cur: &mut Cur<'_>) -> Option<(usize, String)> {
    cur.path_ending(&["PathBuf", "from"])?;
    cur.punct('(')?;
    let (token, literal) = cur.env_macro()?;
    let literal = literal.to_owned();
    cur.punct(')')?;
    Some((token, literal))
}

/// Tail T1: `.map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("…")))`.
fn unwrap_or_else_tail(cur: &mut Cur<'_>) -> Option<(usize, String)> {
    cur.punct('.')?;
    cur.ident("map")?;
    cur.punct('(')?;
    cur.path_ending(&["PathBuf", "from"])?;
    cur.punct(')')?;
    cur.punct('.')?;
    cur.ident("unwrap_or_else")?;
    cur.punct('(')?;
    cur.punct('|')?;
    cur.punct('|')?;
    let found = path_buf_from_env(cur)?;
    cur.punct(')')?;
    Some(found)
}

/// Tail T2: `.map_or_else(|| PathBuf::from(env!("…")), PathBuf::from)`.
fn map_or_else_tail(cur: &mut Cur<'_>) -> Option<(usize, String)> {
    cur.punct('.')?;
    cur.ident("map_or_else")?;
    cur.punct('(')?;
    cur.punct('|')?;
    cur.punct('|')?;
    let found = path_buf_from_env(cur)?;
    cur.punct(',')?;
    cur.path_ending(&["PathBuf", "from"])?;
    cur.punct(')')?;
    Some(found)
}

fn compile_time_tail(cur: &mut Cur<'_>) -> Option<(usize, String)> {
    let save = cur.at;
    if let Some(found) = unwrap_or_else_tail(cur) {
        return Some(found);
    }
    cur.at = save;
    if let Some(found) = map_or_else_tail(cur) {
        return Some(found);
    }
    cur.at = save;
    None
}

/// Shapes M1/M2 starting at `at`, yielding the `env` token they exempt.
fn manifest_fallback(toks: &[Tok], at: usize) -> Option<usize> {
    let mut cur = Cur::new(toks, at);
    if cur.var_os_read()? != MANIFEST_VAR {
        return None;
    }
    cur.nonempty_filter()?;
    let (token, literal) = compile_time_tail(&mut cur)?;
    (literal == MANIFEST_VAR).then_some(token)
}

/// Shapes B1/B2 starting at `at`, yielding the `env` token they exempt.
fn bin_exe_fallback(toks: &[Tok], at: usize) -> Option<usize> {
    let mut cur = Cur::new(toks, at);
    let mangled = cur
        .var_os_read()?
        .strip_prefix(NEXTEST_BIN_EXE_PREFIX)?
        .to_owned();
    cur.nonempty_filter()?;

    cur.punct('.')?;
    cur.ident("or_else")?;
    cur.punct('(')?;
    cur.punct('|')?;
    cur.punct('|')?;
    let name = cur.var_os_read()?.strip_prefix(BIN_EXE_PREFIX)?.to_owned();
    cur.nonempty_filter()?;
    cur.punct(')')?;

    // The nextest variable is the same target with hyphens made underscores;
    // a mismatch means this expression proves nothing about the baked path.
    if name.replace('-', "_") != mangled {
        return None;
    }

    let (token, literal) = compile_time_tail(&mut cur)?;
    (literal == format!("{BIN_EXE_PREFIX}{name}")).then_some(token)
}

fn occurrences(toks: &[Tok]) -> Vec<Occurrence> {
    let mut found = Vec::new();
    for at in 0..toks.len() {
        let mut cur = Cur::new(toks, at);
        let Some((token, literal)) = cur.env_macro() else {
            continue;
        };
        let kind = if literal == MANIFEST_VAR {
            FormKind::ManifestDir
        } else if literal.starts_with(BIN_EXE_PREFIX) {
            FormKind::BinExe
        } else {
            continue;
        };
        found.push(Occurrence {
            token,
            line: toks[token].line,
            kind,
        });
    }
    found
}

fn exempted(toks: &[Tok]) -> BTreeSet<usize> {
    let mut exempt = BTreeSet::new();
    for at in 0..toks.len() {
        if let Some(token) = manifest_fallback(toks, at) {
            exempt.insert(token);
        }
        if let Some(token) = bin_exe_fallback(toks, at) {
            exempt.insert(token);
        }
    }
    exempt
}

/// Every compile-time path form in `source`, whether or not it sits in a
/// recognized runtime-first expression.
///
/// This is the input to exemption maintenance: a file whose only occurrence is
/// a *safe* fallback still has a live form, so validating [`ALLOWED`] against
/// the filtered violation list would declare the harness's own implementation
/// stale.
fn raw_forms(source: &str) -> Vec<(usize, FormKind)> {
    occurrences(&tokenize(source))
        .into_iter()
        .map(|found| (found.line, found.kind))
        .collect()
}

/// Every violation in `source`: raw forms minus the recognized safe fallbacks,
/// plus the independent hosted-root literal check.
fn violations_in(source: &str) -> Vec<(usize, &'static str)> {
    let toks = tokenize(source);
    let exempt = exempted(&toks);

    let mut found: Vec<(usize, &'static str)> = occurrences(&toks)
        .into_iter()
        .filter(|occurrence| !exempt.contains(&occurrence.token))
        .map(|occurrence| (occurrence.line, occurrence.kind.remedy()))
        .collect();

    // Independent pass: a baked hosted-root path is a string literal, so no
    // runtime-fallback recognition may suppress it.
    found.extend(toks.iter().filter_map(|tok| match &tok.kind {
        Kind::Str(body) if body.starts_with(HOSTED_ROOT) => Some((tok.line, HOSTED_ROOT_REMEDY)),
        _ => None,
    }));

    found.sort_by_key(|(line, _)| *line);
    found
}

// ---------------------------------------------------------------------------
// Scanner
// ---------------------------------------------------------------------------

/// The repository root, found by walking up from the calling crate's manifest.
///
/// ## Panics
///
/// Panics when no ancestor holds `just/devops.just`, which means the guard is
/// running somewhere that is not a checkout of this repository.
#[must_use]
pub fn repo_root() -> PathBuf {
    biscuit_test_harness::manifest_dir!()
        .ancestors()
        .find(|dir| dir.join("just").join("devops.just").is_file())
        .map(Path::to_path_buf)
        .expect("could not locate the repo root")
}

/// Whether a repository-relative path is in the guard's scan domain.
///
/// The one policy shared by full-tree discovery, changed-file filtering, and
/// (by contract) the CI planner's selection rule.
#[must_use]
pub fn is_eligible(relative: &str) -> bool {
    if !relative.ends_with(".rs") {
        return false;
    }
    // A build script runs on the producer, where the compile-time value is the
    // only correct answer.
    if relative.rsplit('/').next() == Some("build.rs") {
        return false;
    }
    if GUARD_OWN_SOURCES.iter().any(|(own, _)| *own == relative) {
        return false;
    }
    !relative
        .split('/')
        .any(|component| SKIPPED_DIRS.iter().any(|(skip, _)| *skip == component))
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Every eligible `.rs` file under `root`, sorted.
///
/// Directory symlinks that leave the checkout are not followed and symlink
/// cycles terminate: both are decided on *canonicalized* paths, never on the
/// spelling, because macOS's `/tmp` is itself a symlink and Windows
/// canonicalization yields `\\?\` prefixes.
///
/// ## Errors
///
/// Returns [`GuardError::ReadDir`] naming the directory when one that exists
/// cannot be listed, and [`GuardError::Inspect`] when a path cannot be stat'd
/// or `root` itself cannot be resolved. A scan that could not read its inputs
/// is never a pass.
pub fn collect_rust_files(root: &Path) -> Result<Vec<PathBuf>, GuardError> {
    let boundary = canonical_root(root)?;
    let mut visited = BTreeSet::new();
    let mut out = Vec::new();
    walk(root, root, &boundary, &mut visited, &mut out)?;
    out.sort();
    Ok(out)
}

/// The checkout boundary every containment decision is made against.
///
/// Canonicalized, and a failure to canonicalize is fatal: a boundary that fell
/// back to the spelling would compare unequal to every canonical path under it
/// on macOS, where `/tmp` is a symlink, and on Windows, where canonicalization
/// yields a `\\?\` prefix — which silently turns containment into a check that
/// nothing can pass or, worse, one that admits a path from outside.
fn canonical_root(root: &Path) -> Result<PathBuf, GuardError> {
    fs::canonicalize(root).map_err(|source| GuardError::Inspect {
        path: root.to_path_buf(),
        source,
    })
}

fn walk(
    dir: &Path,
    root: &Path,
    boundary: &Path,
    visited: &mut BTreeSet<PathBuf>,
    out: &mut Vec<PathBuf>,
) -> Result<(), GuardError> {
    let canonical = match fs::canonicalize(dir) {
        Ok(canonical) => canonical,
        // A directory that vanished between listing and descent is not a
        // readable directory the guard is refusing to check. Every other
        // failure is one, and a permission error is not an empty directory.
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(GuardError::Inspect {
                path: dir.to_path_buf(),
                source,
            });
        }
    };
    if !canonical.starts_with(boundary) || !visited.insert(canonical) {
        return Ok(());
    }

    let entries = fs::read_dir(dir).map_err(|source| GuardError::ReadDir {
        path: dir.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| GuardError::ReadDir {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();

        let kind = match fs::metadata(&path) {
            Ok(kind) => kind,
            // A broken symlink names nothing that could be scanned.
            Err(source) if source.kind() == io::ErrorKind::NotFound => continue,
            Err(source) => return Err(GuardError::Inspect { path, source }),
        };

        if kind.is_dir() {
            if SKIPPED_DIRS.iter().any(|(skip, _)| *skip == name) {
                continue;
            }
            walk(&path, root, boundary, visited, out)?;
        } else if is_eligible(&relative(root, &path)) {
            out.push(path);
        }
    }

    Ok(())
}

fn read_source(path: &Path) -> Result<String, GuardError> {
    fs::read_to_string(path).map_err(|source| GuardError::ReadFile {
        path: path.to_path_buf(),
        source,
    })
}

/// Scan `root` in `mode` and report what was checked and what was found.
///
/// ## Errors
///
/// Returns [`GuardError::ReadDir`], [`GuardError::ReadFile`], or
/// [`GuardError::Inspect`] naming the path when an existing directory or file
/// cannot be read or stat'd, and [`GuardError::Unscannable`] when a listed path
/// is spelled outside the checkout, resolves outside it, or exists but is not a
/// regular file.
///
/// A listed path that is absent fails with [`GuardError::MissingListedPath`]:
/// the planner omits the deletions its diff recorded, so the remaining paths
/// are ones it expects to exist. Spelling and containment are judged before the
/// filesystem is consulted, so a path that leaves the checkout is reported as
/// such whether or not it happens to exist.
pub fn scan(root: &Path, mode: &ScanMode) -> Result<ScanReport, GuardError> {
    let mut report = ScanReport {
        mode: mode.clone(),
        files_checked: 0,
        ineligible: Vec::new(),
        violations: Vec::new(),
    };

    let targets: Vec<(String, PathBuf)> = match mode {
        ScanMode::FullTree => collect_rust_files(root)?
            .into_iter()
            .map(|path| (relative(root, &path), path))
            .collect(),
        ScanMode::Changed(listed) => {
            let boundary = canonical_root(root)?;
            let mut unique = BTreeSet::new();
            let mut targets = Vec::new();
            for path in listed {
                if !unique.insert(path.clone()) {
                    continue;
                }
                // Before the filesystem, so an escaping path that happens not
                // to exist is reported for leaving the checkout rather than for
                // disappearing.
                if let Some(detail) = unscannable_spelling(path) {
                    return Err(GuardError::Unscannable {
                        path: path.clone(),
                        detail: detail.to_owned(),
                    });
                }
                if !is_eligible(path) {
                    report.ineligible.push(path.clone());
                    continue;
                }
                let absolute = root.join(path);
                match fs::metadata(&absolute) {
                    Ok(kind) if kind.is_file() => {
                        let canonical = fs::canonicalize(&absolute).map_err(|source| {
                            GuardError::Inspect {
                                path: absolute.clone(),
                                source,
                            }
                        })?;
                        // A symlink inside the checkout still stats as a file,
                        // so containment is decided on what it resolves to.
                        if !canonical.starts_with(&boundary) {
                            return Err(GuardError::Unscannable {
                                path: path.clone(),
                                detail: format!(
                                    "resolves to `{}`, outside the checkout at `{}`",
                                    canonical.display(),
                                    boundary.display()
                                ),
                            });
                        }
                        targets.push((path.clone(), canonical));
                    }
                    Ok(_) => {
                        return Err(GuardError::Unscannable {
                            path: path.clone(),
                            detail: "exists but is not a regular file; a changed-path list \
                                     names files, and a directory there is a malformed scope"
                                .to_owned(),
                        });
                    }
                    Err(source) if source.kind() == io::ErrorKind::NotFound => {
                        return Err(GuardError::MissingListedPath { path: path.clone() });
                    }
                    Err(source) => {
                        return Err(GuardError::Inspect {
                            path: absolute,
                            source,
                        });
                    }
                }
            }
            targets
        }
    };

    for (file, path) in targets {
        let source = read_source(&path)?;
        report.files_checked += 1;
        report
            .violations
            .extend(violations_in(&source).into_iter().map(|(line, remedy)| {
                Violation {
                    file: file.clone(),
                    line,
                    remedy,
                }
            }));
    }

    report.violations.sort_by(|left, right| {
        (&left.file, left.line, left.remedy).cmp(&(&right.file, right.line, right.remedy))
    });
    Ok(report)
}

/// Files in the full eligible tree that spell a compile-time path form at all,
/// safe fallback or not.
///
/// Exemption maintenance runs against this rather than against the filtered
/// violations, and runs in both scan modes — see the module documentation.
///
/// ## Errors
///
/// As [`scan`] in [`ScanMode::FullTree`].
pub fn raw_live_form_files(root: &Path) -> Result<BTreeSet<String>, GuardError> {
    let mut live = BTreeSet::new();
    for path in collect_rust_files(root)? {
        if !raw_forms(&read_source(&path)?).is_empty() {
            live.insert(relative(root, &path));
        }
    }
    Ok(live)
}

/// Everything wrong with an exemption list, one actionable line each.
///
/// Catches duplicates, empty reasons, entries whose file no longer exists
/// (deleted or renamed), and entries that match no live form.
#[must_use]
pub fn allowlist_problems(
    entries: &[AllowEntry],
    live: &BTreeSet<String>,
    root: &Path,
) -> Vec<String> {
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for entry in entries {
        *seen.entry(entry.file).or_default() += 1;
    }

    let mut problems = Vec::new();
    for (file, count) in &seen {
        if *count > 1 {
            problems.push(format!(
                "  {file} — listed {count} times; keep exactly one entry with one reason"
            ));
        }
    }

    for entry in entries {
        if entry.reason.trim().is_empty() {
            problems.push(format!(
                "  {} — has no reason; say why it is never archive-executed",
                entry.file
            ));
        }
        if !root.join(entry.file).is_file() {
            problems.push(format!(
                "  {} — names a file that no longer exists; if it was renamed, point the entry \
                 at the new path, and if it was deleted, delete the entry",
                entry.file
            ));
        } else if !live.contains(entry.file) {
            problems.push(format!(
                "  {} — no longer spells a compile-time path form and must be deleted",
                entry.file
            ));
        }
    }

    problems.sort();
    problems.dedup();
    problems
}

#[cfg(test)]
mod matcher_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    const SAFE_SITE: &str = r#"
use std::path::PathBuf;
fn fixture_root() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}
"#;

    const UNSAFE_SITE: &str = r#"
use std::path::PathBuf;
fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
"#;

    fn write(root: &Path, relative: &str, body: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("a relative path has a parent"))
            .expect("create fixture directory");
        fs::write(path, body).expect("write fixture");
    }

    fn changed(paths: &[&str]) -> ScanMode {
        ScanMode::Changed(paths.iter().map(|path| (*path).to_owned()).collect())
    }

    /// A resolved-plan document carrying `scope` as its `archive_guard` object.
    ///
    /// The version is as much a part of the contract as the scope is, so no
    /// fixture that is testing something else spells it by hand.
    fn plan_document(scope: &str) -> String {
        format!(r#"{{"schema_version":{PLAN_SCHEMA_VERSION},"archive_guard":{scope}}}"#)
    }

    /// The reader's verdict on a plan carrying `scope`.
    fn read_scope(scope: &str) -> Result<GuardPlan, GuardError> {
        GuardPlan::from_plan_json(Path::new("plan.json"), &plan_document(scope))
    }

    #[test]
    fn changed_mode_ignores_a_violation_in_a_file_it_was_not_given() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "listed/src/lib.rs", UNSAFE_SITE);
        write(tree.path(), "unlisted/src/lib.rs", UNSAFE_SITE);

        let report = scan(tree.path(), &changed(&["listed/src/lib.rs"])).expect("scan");

        assert_eq!(report.files_checked, 1);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].file, "listed/src/lib.rs");
    }

    #[test]
    fn full_tree_finds_the_violation_a_changed_list_omitted() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "listed/src/lib.rs", SAFE_SITE);
        write(tree.path(), "unlisted/src/lib.rs", UNSAFE_SITE);

        let report = scan(tree.path(), &ScanMode::FullTree).expect("scan");

        assert_eq!(report.files_checked, 2);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].file, "unlisted/src/lib.rs");
    }

    /// The planner removes the deletions its diff recorded, so an absent listed
    /// path is a file that disappeared after the scope was resolved — and a
    /// scan that skipped it would report a pass over source it never read.
    #[test]
    fn an_absent_listed_path_fails_rather_than_being_skipped() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "kept/src/lib.rs", SAFE_SITE);

        let err = scan(
            tree.path(),
            &changed(&["kept/src/lib.rs", "gone/src/lib.rs"]),
        )
        .expect_err("a listed path the planner expected to exist must not vanish silently");

        assert!(matches!(err, GuardError::MissingListedPath { .. }), "{err}");
        assert!(err.to_string().contains("gone/src/lib.rs"), "{err}");
    }

    #[test]
    fn a_rename_destination_is_scanned_like_any_other_listed_path() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "after/src/lib.rs", UNSAFE_SITE);

        let report = scan(tree.path(), &changed(&["after/src/lib.rs"])).expect("scan");

        assert_eq!(report.files_checked, 1);
        assert_eq!(report.violations[0].file, "after/src/lib.rs");
    }

    #[test]
    fn a_duplicate_listed_path_is_scanned_once() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "src/lib.rs", UNSAFE_SITE);

        let report = scan(
            tree.path(),
            &changed(&["src/lib.rs", "src/lib.rs", "src/lib.rs"]),
        )
        .expect("scan");

        assert_eq!(report.files_checked, 1);
        assert_eq!(report.violations.len(), 1);
    }

    #[test]
    fn changed_mode_applies_the_same_eligibility_policy_as_discovery() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "pkg/build.rs", UNSAFE_SITE);
        write(tree.path(), "pkg/examples/demo.rs", UNSAFE_SITE);
        write(tree.path(), "pkg/README.md", UNSAFE_SITE);

        let report = scan(
            tree.path(),
            &changed(&["pkg/build.rs", "pkg/examples/demo.rs", "pkg/README.md"]),
        )
        .expect("scan");

        assert_eq!(report.files_checked, 0);
        assert_eq!(report.ineligible.len(), 3);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn an_empty_changed_list_checks_nothing_and_says_so() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "src/lib.rs", UNSAFE_SITE);

        let report = scan(tree.path(), &changed(&[])).expect("scan");

        assert_eq!(report.files_checked, 0);
        assert!(report.violations.is_empty());
        assert!(report.summary().contains("mode=changed(0 listed)"));
        assert!(!report.summary().contains("full-tree"));
    }

    #[test]
    fn the_full_tree_summary_is_distinguishable_from_an_empty_changed_scan() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "src/lib.rs", SAFE_SITE);

        let report = scan(tree.path(), &ScanMode::FullTree).expect("scan");

        assert!(report.summary().contains("mode=full-tree"));
        assert!(report.summary().contains("files-checked=1"));
    }

    #[test]
    fn an_unset_plan_variable_requests_the_full_tree() {
        let plan = GuardPlan::full_tree();

        assert!(plan.selected);
        assert_eq!(plan.mode, ScanMode::FullTree);
    }

    #[test]
    fn a_plan_selecting_changed_paths_carries_them_verbatim() {
        let plan = read_scope(
            r#"{"selected":true,"mode":"changed","paths":["a/b.rs"],
                "reason":"pull request with an available change inventory"}"#,
        )
        .expect("a well-formed plan parses");

        assert!(plan.selected);
        assert_eq!(plan.mode, changed(&["a/b.rs"]));
        assert_eq!(
            plan.reason,
            "pull request with an available change inventory"
        );
    }

    #[test]
    fn a_plan_with_an_explicitly_empty_path_list_is_not_a_full_scan() {
        let plan = read_scope(
            r#"{"selected":true,"mode":"changed","paths":[],
                "reason":"nothing eligible changed"}"#,
        )
        .expect("an explicitly empty inventory is a real state");

        assert_eq!(plan.mode, changed(&[]));
        assert_ne!(plan.mode, ScanMode::FullTree);
    }

    #[test]
    fn a_plan_that_did_not_select_the_guard_scans_nothing_and_says_why() {
        let plan = read_scope(r#"{"selected":false,"reason":"documentation-only change"}"#)
            .expect("an unselected guard is a valid plan");

        assert!(!plan.selected);
        assert_eq!(plan.reason, "documentation-only change");
    }

    #[test]
    fn the_standalone_default_states_its_own_reason() {
        let plan = GuardPlan::full_tree();

        assert!(
            plan.reason.contains(PLAN_ENV),
            "the summary must say the scope came from no plan: {}",
            plan.reason
        );
    }

    #[test]
    fn malformed_plan_json_is_an_error_rather_than_an_empty_scan() {
        let err = GuardPlan::from_plan_json(Path::new("plan.json"), "{ not json")
            .expect_err("malformed JSON must not degrade to a scan");

        assert!(matches!(err, GuardError::MalformedPlan { .. }));
        assert!(err.to_string().contains("not valid JSON"));
    }

    #[test]
    fn a_plan_from_another_schema_generation_is_an_error() {
        let err = GuardPlan::from_plan_json(
            Path::new("plan.json"),
            &format!(
                r#"{{"schema_version":{},"archive_guard":{{"selected":true,"mode":"full",
                    "reason":"explicit full-scope request"}}}}"#,
                PLAN_SCHEMA_VERSION + 1
            ),
        )
        .expect_err("a plan this reader does not understand is never scanned on faith");

        assert!(matches!(err, GuardError::MalformedPlan { .. }), "{err}");
        assert!(
            err.to_string().contains(&format!(
                "schema version {}; this reader understands {PLAN_SCHEMA_VERSION}",
                PLAN_SCHEMA_VERSION + 1
            )),
            "{err}"
        );
    }

    #[test]
    fn a_plan_without_a_schema_version_is_an_error() {
        let err = GuardPlan::from_plan_json(
            Path::new("plan.json"),
            r#"{"archive_guard":{"selected":true,"mode":"full","reason":"full scope"}}"#,
        )
        .expect_err("an undeclared generation is not this one");

        assert!(matches!(err, GuardError::MalformedPlan { .. }), "{err}");
        assert!(err.to_string().contains("declares no `schema_version`"), "{err}");
    }

    #[test]
    fn a_non_numeric_schema_version_is_an_error() {
        let err = GuardPlan::from_plan_json(
            Path::new("plan.json"),
            r#"{"schema_version":"5","archive_guard":{"selected":true,"mode":"full",
                "reason":"full scope"}}"#,
        )
        .expect_err("a version that is not a number names no generation");

        assert!(matches!(err, GuardError::MalformedPlan { .. }), "{err}");
        assert!(
            err.to_string()
                .contains("not a schema generation number"),
            "{err}"
        );
    }

    #[test]
    fn a_plan_without_the_archive_guard_key_is_an_error() {
        let err = GuardPlan::from_plan_json(
            Path::new("plan.json"),
            &format!(r#"{{"schema_version":{PLAN_SCHEMA_VERSION},"areas":[]}}"#),
        )
        .expect_err("a plan that never mentions the guard cannot scope it");

        assert!(err.to_string().contains("no top-level `archive_guard` key"));
    }

    #[test]
    fn a_plan_without_a_reason_is_an_error() {
        let err = read_scope(r#"{"selected":true,"mode":"full"}"#)
            .expect_err("every plan shape states why it holds the scope it does");

        assert!(matches!(err, GuardError::MalformedPlan { .. }), "{err}");
        assert!(err.to_string().contains("`reason` is missing"), "{err}");
    }

    #[test]
    fn a_blank_reason_is_an_error() {
        let err = read_scope(r#"{"selected":true,"mode":"full","reason":"  \t "}"#)
            .expect_err("whitespace explains nothing");

        assert!(matches!(err, GuardError::MalformedPlan { .. }), "{err}");
        assert!(err.to_string().contains("`reason` is blank"), "{err}");
    }

    #[test]
    fn a_non_string_reason_is_an_error() {
        let err = read_scope(r#"{"selected":true,"mode":"full","reason":7}"#)
            .expect_err("a reason is prose, not a number");

        assert!(matches!(err, GuardError::MalformedPlan { .. }), "{err}");
        assert!(err.to_string().contains("which is not a string"), "{err}");
    }

    #[test]
    fn an_unselected_plan_carrying_scope_is_an_error() {
        for (name, scope) in [
            (
                "mode",
                r#"{"selected":false,"mode":"changed","reason":"documentation-only change"}"#,
            ),
            (
                "paths",
                r#"{"selected":false,"paths":["a/b.rs"],"reason":"documentation-only change"}"#,
            ),
        ] {
            let err = read_scope(scope)
                .expect_err("stale scope beside an unselected guard is not silently dropped");

            assert!(matches!(err, GuardError::MalformedPlan { .. }), "{err}");
            assert!(
                err.to_string()
                    .contains(&format!("`selected` is false but `{name}` is present")),
                "{err}"
            );
        }
    }

    #[test]
    fn a_plan_with_an_unknown_mode_is_an_error() {
        let err = read_scope(r#"{"selected":true,"mode":"partial","reason":"a partial scan"}"#)
            .expect_err("an unknown mode has no defined scope");

        assert!(err.to_string().contains("expected \"full\" or \"changed\""));
    }

    #[test]
    fn a_changed_plan_without_paths_is_an_error() {
        let err = read_scope(r#"{"selected":true,"mode":"changed","reason":"a changed scan"}"#)
            .expect_err("a changed scan needs its path list, even when empty");

        assert!(err.to_string().contains("explicit empty list"));
    }

    #[test]
    fn a_full_plan_carrying_paths_is_an_error() {
        let err = read_scope(
            r#"{"selected":true,"mode":"full","paths":["a.rs"],"reason":"a full scan"}"#,
        )
        .expect_err("a full scan takes no path list");

        assert!(err.to_string().contains("takes no path list"));
    }

    #[test]
    fn an_unsorted_path_list_is_an_error() {
        let err = read_scope(
            r#"{"selected":true,"mode":"changed","paths":["b/second.rs","a/first.rs"],
                "reason":"a changed scan"}"#,
        )
        .expect_err("two readers of one order cannot disagree about scope");

        assert!(matches!(err, GuardError::MalformedPlan { .. }), "{err}");
        assert!(
            err.to_string()
                .contains("`paths` is not sorted; `b/second.rs` precedes `a/first.rs`"),
            "{err}"
        );
    }

    #[test]
    fn a_repeated_path_in_the_plan_is_an_error() {
        let err = read_scope(
            r#"{"selected":true,"mode":"changed","paths":["a/first.rs","a/first.rs"],
                "reason":"a changed scan"}"#,
        )
        .expect_err("a scope names each path once");

        assert!(matches!(err, GuardError::MalformedPlan { .. }), "{err}");
        assert!(
            err.to_string()
                .contains("`paths` lists `a/first.rs` more than once"),
            "{err}"
        );
    }

    #[test]
    fn an_unreadable_plan_file_is_an_error_naming_the_path() {
        let tree = TempDir::new().expect("temp tree");
        let missing = tree.path().join("no-such-plan.json");

        let err = GuardPlan::from_plan_file(&missing).expect_err("an absent plan is not a default");

        assert!(matches!(err, GuardError::PlanUnreadable { .. }));
        assert!(err.to_string().contains("no-such-plan.json"));
    }

    // `std::os::windows::fs::symlink_dir` needs Developer Mode or an elevated
    // process, which no CI runner here guarantees, so the two symlink fixtures
    // are built only where they can be built. The boundary decision they cover
    // is platform-independent: both compare canonicalized paths.
    #[test]
    #[cfg(unix)]
    fn a_directory_symlink_leaving_the_tree_is_not_followed() {
        let tree = TempDir::new().expect("temp tree");
        let outside = TempDir::new().expect("outside tree");
        write(tree.path(), "src/lib.rs", SAFE_SITE);
        write(outside.path(), "src/lib.rs", UNSAFE_SITE);
        std::os::unix::fs::symlink(outside.path(), tree.path().join("escape"))
            .expect("create escaping symlink");

        let report = scan(tree.path(), &ScanMode::FullTree).expect("scan");

        assert_eq!(report.files_checked, 1);
        assert!(report.violations.is_empty());
    }

    #[test]
    #[cfg(unix)]
    fn a_directory_symlink_cycle_terminates() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "src/lib.rs", UNSAFE_SITE);
        std::os::unix::fs::symlink(tree.path(), tree.path().join("src/loop"))
            .expect("create cycle");

        let report = scan(tree.path(), &ScanMode::FullTree).expect("scan");

        assert_eq!(report.files_checked, 1);
    }

    #[test]
    #[cfg(unix)]
    fn an_unreadable_existing_file_is_an_error_naming_its_path() {
        use std::os::unix::fs::PermissionsExt;

        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "src/lib.rs", SAFE_SITE);
        let path = tree.path().join("src/lib.rs");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000))
            .expect("drop read permission");

        let outcome = scan(tree.path(), &changed(&["src/lib.rs"]));

        // Restore before asserting so the TempDir can always clean itself up.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644))
            .expect("restore read permission");

        let err = outcome.expect_err("an unreadable file must not be silently skipped");
        assert!(matches!(err, GuardError::ReadFile { .. }));
        assert!(err.to_string().contains("src/lib.rs"));
    }

    #[test]
    #[cfg(unix)]
    fn an_unreadable_existing_directory_is_an_error_naming_its_path() {
        use std::os::unix::fs::PermissionsExt;

        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "locked/src/lib.rs", SAFE_SITE);
        let locked = tree.path().join("locked");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000))
            .expect("drop directory permission");

        let outcome = collect_rust_files(tree.path());

        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755))
            .expect("restore directory permission");

        let err = outcome.expect_err("an unlistable directory must not be silently skipped");
        assert!(matches!(err, GuardError::ReadDir { .. }));
        assert!(err.to_string().contains("locked"));
    }

    // Readable-but-not-searchable (`0o444`) is the one directory mode that
    // lets `read_dir` yield a name whose `metadata` is then denied, which is
    // how the scanner's stat is reached with a failure that is not `NotFound`.
    // Windows has no mode bit separating list from traverse, so this fixture
    // is Unix-only by construction rather than for convenience; the branch it
    // covers is platform-independent.
    #[test]
    #[cfg(unix)]
    fn an_entry_that_cannot_be_inspected_is_an_error_naming_its_path() {
        use std::os::unix::fs::PermissionsExt;

        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "listable/hidden.rs", UNSAFE_SITE);
        let listable = tree.path().join("listable");
        fs::set_permissions(&listable, fs::Permissions::from_mode(0o444))
            .expect("drop search permission");

        // Root — and any filesystem that ignores the mode — can still stat the
        // child, so probe before asserting rather than claiming a denial the
        // host never produced.
        let denied = fs::metadata(listable.join("hidden.rs")).is_err();
        let outcome = denied.then(|| collect_rust_files(tree.path()));

        fs::set_permissions(&listable, fs::Permissions::from_mode(0o755))
            .expect("restore search permission");

        let Some(outcome) = outcome else { return };
        let err = outcome.expect_err("an entry that cannot be stat'd must not be skipped");
        assert!(matches!(err, GuardError::Inspect { .. }), "{err}");
        assert!(err.to_string().contains("hidden.rs"), "{err}");
    }

    #[test]
    #[cfg(unix)]
    fn a_broken_symlink_is_the_one_stat_failure_the_full_tree_scan_skips() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "src/lib.rs", UNSAFE_SITE);
        std::os::unix::fs::symlink(
            tree.path().join("src/gone.rs"),
            tree.path().join("src/dangling.rs"),
        )
        .expect("create broken symlink");

        let report =
            scan(tree.path(), &ScanMode::FullTree).expect("a broken symlink is not a failure");

        assert_eq!(report.files_checked, 1);
        assert_eq!(report.violations.len(), 1);
    }

    #[test]
    fn a_root_that_cannot_be_resolved_is_an_error_rather_than_an_unbounded_scan() {
        let tree = TempDir::new().expect("temp tree");
        let absent = tree.path().join("no-such-checkout");

        let err =
            collect_rust_files(&absent).expect_err("a root with no canonical form bounds nothing");

        assert!(matches!(err, GuardError::Inspect { .. }), "{err}");
        assert!(err.to_string().contains("no-such-checkout"), "{err}");
    }

    #[test]
    fn a_listed_path_that_is_a_directory_is_an_error_rather_than_a_deletion() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "pkg/src.rs/inner.rs", UNSAFE_SITE);

        let err = scan(tree.path(), &changed(&["pkg/src.rs"]))
            .expect_err("a directory is not a changed Rust file");

        assert!(matches!(err, GuardError::Unscannable { .. }), "{err}");
        assert!(err.to_string().contains("not a regular file"), "{err}");
    }

    #[test]
    fn a_listed_path_climbing_out_of_the_checkout_is_an_error_even_when_it_exists() {
        let tree = TempDir::new().expect("temp tree");
        let checkout = tree.path().join("checkout");
        write(&checkout, "src/lib.rs", SAFE_SITE);
        write(tree.path(), "outside.rs", UNSAFE_SITE);

        let err = scan(&checkout, &changed(&["../outside.rs"]))
            .expect_err("a `..` component leaves the checkout");

        assert!(matches!(err, GuardError::Unscannable { .. }), "{err}");
        assert!(err.to_string().contains("climbs out of the checkout"), "{err}");
    }

    #[test]
    fn a_listed_path_that_escapes_and_does_not_exist_is_not_filed_as_a_deletion() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "src/lib.rs", SAFE_SITE);

        let err = scan(tree.path(), &changed(&["../never-existed.rs"]))
            .expect_err("an escaping path is never an ordinary deletion");

        assert!(matches!(err, GuardError::Unscannable { .. }), "{err}");
    }

    #[test]
    fn an_absolute_listed_path_is_an_error() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "src/lib.rs", SAFE_SITE);

        let err = scan(tree.path(), &changed(&["/etc/hosts.rs"]))
            .expect_err("an absolute path is not a scan scope");

        assert!(matches!(err, GuardError::Unscannable { .. }), "{err}");
        assert!(err.to_string().contains("is absolute"), "{err}");
    }

    /// A file that *is* inside the checkout, named the one way the guard will
    /// not accept.
    ///
    /// The host's own spelling rather than a literal, so the rejection is not
    /// an artifact of naming a path this machine does not have — and asserted
    /// only on the variant, because which rule fires is platform-dependent:
    /// a Windows absolute path trips the drive-prefix rule, a POSIX one the
    /// leading-slash rule.
    #[test]
    fn a_listed_path_spelled_absolutely_is_rejected_rather_than_read() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "src/lib.rs", UNSAFE_SITE);
        let inside = tree.path().join("src/lib.rs");

        let err = scan(tree.path(), &changed(&[&inside.to_string_lossy()]))
            .expect_err("a scan scope is relative, even when it points back inside");

        assert!(matches!(err, GuardError::Unscannable { .. }), "{err}");
    }

    #[test]
    #[cfg(unix)]
    fn a_symlink_inside_the_checkout_pointing_out_of_it_is_never_read() {
        let tree = TempDir::new().expect("temp tree");
        let checkout = tree.path().join("checkout");
        write(&checkout, "src/kept.rs", SAFE_SITE);
        write(tree.path(), "outside/secret.rs", UNSAFE_SITE);
        std::os::unix::fs::symlink(
            tree.path().join("outside/secret.rs"),
            checkout.join("src/escape.rs"),
        )
        .expect("create escaping file symlink");

        let err = scan(&checkout, &changed(&["src/escape.rs"]))
            .expect_err("a symlink target outside the checkout is out of scope");

        assert!(matches!(err, GuardError::Unscannable { .. }), "{err}");
        assert!(err.to_string().contains("outside the checkout"), "{err}");
    }

    #[test]
    fn a_plan_path_leaving_the_checkout_is_malformed_before_any_filesystem_access() {
        for (spelling, fragment) in [
            ("../outside.rs", "climbs out of the checkout"),
            ("/etc/passwd.rs", "is absolute"),
            ("C:/windows/system32.rs", "Windows drive prefix"),
            ("a\\b.rs", "backslashes"),
            ("./a/b.rs", "drop the leading `./`"),
        ] {
            let err = read_scope(&format!(
                r#"{{"selected":true,"mode":"changed","paths":["{}"],
                    "reason":"a changed scan"}}"#,
                spelling.replace('\\', "\\\\")
            ))
            .expect_err(spelling);

            assert!(matches!(err, GuardError::MalformedPlan { .. }), "{err}");
            assert!(err.to_string().contains(fragment), "{spelling}: {err}");
        }
    }

    #[test]
    fn exemption_maintenance_sees_the_raw_form_a_safe_fallback_hides() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "harness/src/lib.rs", SAFE_SITE);

        let report = scan(tree.path(), &ScanMode::FullTree).expect("scan");
        let live = raw_live_form_files(tree.path()).expect("raw scan");

        assert!(report.violations.is_empty());
        assert!(live.contains("harness/src/lib.rs"));
    }

    #[test]
    fn exemption_maintenance_uses_the_full_tree_during_a_changed_scan() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "listed/src/lib.rs", SAFE_SITE);
        write(tree.path(), "exempted/src/lib.rs", UNSAFE_SITE);

        let report = scan(tree.path(), &changed(&["listed/src/lib.rs"])).expect("scan");
        let live = raw_live_form_files(tree.path()).expect("raw scan");

        assert_eq!(report.files_checked, 1);
        assert!(live.contains("exempted/src/lib.rs"));
    }

    #[test]
    fn a_live_allowlist_entry_passes_and_a_stale_one_fails() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "live/src/lib.rs", UNSAFE_SITE);
        write(tree.path(), "clean/src/lib.rs", "fn main() {}\n");
        let live = raw_live_form_files(tree.path()).expect("raw scan");

        let ok = allowlist_problems(
            &[AllowEntry {
                file: "live/src/lib.rs",
                reason: "never archive-executed",
            }],
            &live,
            tree.path(),
        );
        assert!(ok.is_empty(), "{ok:?}");

        let stale = allowlist_problems(
            &[AllowEntry {
                file: "clean/src/lib.rs",
                reason: "never archive-executed",
            }],
            &live,
            tree.path(),
        );
        assert_eq!(stale.len(), 1);
        assert!(stale[0].contains("must be deleted"));
    }

    #[test]
    fn a_renamed_exempted_file_fails_with_a_message_pointing_at_the_rename() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "after/src/lib.rs", UNSAFE_SITE);
        let live = raw_live_form_files(tree.path()).expect("raw scan");

        let problems = allowlist_problems(
            &[AllowEntry {
                file: "before/src/lib.rs",
                reason: "never archive-executed",
            }],
            &live,
            tree.path(),
        );

        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("renamed"));
    }

    #[test]
    fn a_duplicate_allowlist_entry_fails() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "live/src/lib.rs", UNSAFE_SITE);
        let live = raw_live_form_files(tree.path()).expect("raw scan");
        let entry = AllowEntry {
            file: "live/src/lib.rs",
            reason: "never archive-executed",
        };

        let problems = allowlist_problems(&[entry, entry], &live, tree.path());

        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("listed 2 times"));
    }

    #[test]
    fn an_allowlist_entry_without_a_reason_fails() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "live/src/lib.rs", UNSAFE_SITE);
        let live = raw_live_form_files(tree.path()).expect("raw scan");

        let problems = allowlist_problems(
            &[AllowEntry {
                file: "live/src/lib.rs",
                reason: "   ",
            }],
            &live,
            tree.path(),
        );

        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("no reason"));
    }

    /// Every exemption verdict is the same in both scan modes.
    ///
    /// The three cases above each pin one verdict from a full-tree tree; this
    /// one pins that a changed-file scan which listed NONE of the exempted
    /// files still reaches all four. A maintainer who threaded the scan's own
    /// file list into exemption validation — the obvious optimization — would
    /// turn every entry outside the changed set stale on every pull request,
    /// and the three tests above would stay green.
    #[test]
    fn every_exemption_verdict_is_the_same_in_both_scan_modes() {
        let tree = TempDir::new().expect("temp tree");
        write(tree.path(), "listed/src/lib.rs", SAFE_SITE);
        write(tree.path(), "live/src/lib.rs", UNSAFE_SITE);
        write(tree.path(), "clean/src/lib.rs", "fn main() {}\n");
        write(tree.path(), "after/src/lib.rs", UNSAFE_SITE);
        let entries = [
            AllowEntry {
                file: "live/src/lib.rs",
                reason: "never archive-executed",
            },
            AllowEntry {
                file: "clean/src/lib.rs",
                reason: "never archive-executed",
            },
            AllowEntry {
                file: "before/src/lib.rs",
                reason: "never archive-executed",
            },
            AllowEntry {
                file: "live/src/lib.rs",
                reason: "never archive-executed",
            },
        ];

        let mut verdicts = Vec::new();
        for mode in [ScanMode::FullTree, changed(&["listed/src/lib.rs"])] {
            scan(tree.path(), &mode).expect("scan");
            let live = raw_live_form_files(tree.path()).expect("raw scan");
            verdicts.push(allowlist_problems(&entries, &live, tree.path()));
        }

        let full = &verdicts[0];
        assert_eq!(3, full.len(), "stale, renamed, and duplicate: {full:?}");
        assert!(full.iter().any(|problem| problem.contains("must be deleted")));
        assert!(full.iter().any(|problem| problem.contains("renamed")));
        assert!(full.iter().any(|problem| problem.contains("listed 2 times")));
        assert_eq!(full, &verdicts[1]);
    }

    #[test]
    fn the_guards_own_sources_are_excluded_by_exact_path_not_by_basename() {
        assert!(!is_eligible("tools/test-toolkit/src/archive_guard.rs"));
        assert!(!is_eligible(
            "tools/test-toolkit/src/archive_guard/matcher_tests.rs"
        ));
        assert!(!is_eligible("tools/test-toolkit/tests/archive_path_guard.rs"));
        // A different package's file of the same name is still scanned.
        assert!(is_eligible("elsewhere/tests/archive_path_guard.rs"));
    }
}
