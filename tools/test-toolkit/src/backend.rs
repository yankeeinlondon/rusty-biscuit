//! Stable machine identity for the real-terminal harnesses that back Level 2.
//!
//! [`Level`](crate::Level) says *how much* a test needs; [`Backend`] says
//! *which* emulator. The two are independent: a CI leg can provision `tmux`
//! and nothing else, and must be able to demand that tmux-backed tests run
//! while WezTerm-backed tests still skip cleanly.
//!
//! The identifier spellings here are shared vocabulary, not free choices.
//! `scripts/ci/affected_scope.py::KNOWN_L2_BACKENDS` and the `backends` arrays
//! in each package's `[package.metadata.ci.tests]` use the same strings, and
//! CI's per-package L2 legs set `BISCUIT_TEST_REQUIRED_BACKENDS` to the
//! package's declared backends intersected with the provisioned (CI-hostable)
//! set — today tmux only.

use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fmt;
use std::str::FromStr;

/// Environment variable naming the backends whose absence must be a hard
/// failure rather than a clean skip.
///
/// Comma-separated, case-insensitive, whitespace around each entry ignored:
/// `tmux`, `tmux,wezterm`, `Tmux , KITTY`. An unset or all-whitespace value
/// means "no backend is required" and every gate keeps its skip behavior.
///
/// Entries are matched **exactly** against [`Backend::as_str`]. `tmux2` and
/// `wez` are errors, not near-misses.
pub const BISCUIT_TEST_REQUIRED_BACKENDS: &str = "BISCUIT_TEST_REQUIRED_BACKENDS";

/// A real-terminal harness backend, identified by a stable machine string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Backend {
    /// `TmuxHarness` — headless, the default L2 backend.
    Tmux,
    /// `WezTermHarness` — needs a reachable WezTerm mux server.
    WezTerm,
    /// `KittyHarness` — needs a Kitty instance with remote control.
    Kitty,
    /// `AppleTerminalHarness` — macOS GUI automation via AppleScript.
    AppleTerminal,
}

impl Backend {
    /// Every known backend, in identifier order.
    pub const ALL: [Backend; 4] = [
        Backend::Tmux,
        Backend::WezTerm,
        Backend::Kitty,
        Backend::AppleTerminal,
    ];

    /// The stable machine identifier, shared with the package manifests and
    /// `affected_scope.py`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Backend::Tmux => "tmux",
            Backend::WezTerm => "wezterm",
            Backend::Kitty => "kitty",
            Backend::AppleTerminal => "apple-terminal",
        }
    }

    /// The human-readable name used in skip and panic diagnostics.
    ///
    /// Deliberately distinct from [`as_str`](Self::as_str): reword this freely
    /// without breaking CI policy files.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Backend::Tmux => "tmux",
            Backend::WezTerm => "WezTerm",
            Backend::Kitty => "kitty",
            Backend::AppleTerminal => "Apple Terminal",
        }
    }

    /// Parse one identifier, trimming surrounding whitespace and lowercasing.
    ///
    /// ## Errors
    ///
    /// [`BackendParseError::Empty`] when `raw` is blank, and
    /// [`BackendParseError::Unknown`] when it is not exactly one of
    /// [`Backend::ALL`]'s identifiers.
    pub fn parse(raw: &str) -> Result<Self, BackendParseError> {
        let normalized = raw.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return Err(BackendParseError::Empty);
        }
        Backend::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == normalized)
            .ok_or(BackendParseError::Unknown(normalized))
    }

    /// This backend as a [`HarnessSpec`], carrying both its identifier and its
    /// diagnostic label.
    #[must_use]
    pub const fn spec(self) -> HarnessSpec<'static> {
        HarnessSpec {
            backend: Some(self),
            label: self.label(),
        }
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Backend {
    type Err = BackendParseError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        Backend::parse(raw)
    }
}

/// Why a single backend identifier could not be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendParseError {
    /// The entry was empty or whitespace only.
    Empty,
    /// The entry did not match any known backend identifier.
    Unknown(String),
}

impl fmt::Display for BackendParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendParseError::Empty => {
                write!(f, "empty backend identifier; known backends: {}", known())
            }
            BackendParseError::Unknown(value) => write!(
                f,
                "unknown backend `{value}`; known backends: {}",
                known()
            ),
        }
    }
}

impl Error for BackendParseError {}

/// A malformed [`BISCUIT_TEST_REQUIRED_BACKENDS`] value.
///
/// Carries the offending entry as well as the whole value so the diagnostic
/// points at the typo instead of making the reader diff two lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredBackendsError {
    /// The full environment variable value as read.
    pub value: String,
    /// The comma-separated entry that failed to parse.
    pub entry: String,
    /// The underlying per-entry failure.
    pub source: BackendParseError,
}

impl fmt::Display for RequiredBackendsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{BISCUIT_TEST_REQUIRED_BACKENDS}=\"{}\" is invalid at entry \"{}\": {}",
            self.value, self.entry, self.source
        )
    }
}

impl Error for RequiredBackendsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

fn known() -> String {
    Backend::ALL
        .iter()
        .map(|backend| backend.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Parse a comma-separated backend list into a normalized, deduplicated set.
///
/// A blank `raw` yields an empty set. Any other value must consist entirely of
/// non-empty, recognized entries — a typo is an error, never a silently
/// dropped requirement, because a dropped requirement can never fail again.
///
/// ## Errors
///
/// [`RequiredBackendsError`] naming the first entry that is empty or unknown.
pub fn parse_required_backends(raw: &str) -> Result<BTreeSet<Backend>, RequiredBackendsError> {
    if raw.trim().is_empty() {
        return Ok(BTreeSet::new());
    }

    raw.split(',')
        .map(|entry| {
            Backend::parse(entry).map_err(|source| RequiredBackendsError {
                value: raw.to_string(),
                entry: entry.to_string(),
                source,
            })
        })
        .collect()
}

/// Read [`BISCUIT_TEST_REQUIRED_BACKENDS`] from the process environment.
///
/// ## Errors
///
/// [`RequiredBackendsError`] when the variable is set to a value containing an
/// empty or unrecognized entry.
pub fn required_backends() -> Result<BTreeSet<Backend>, RequiredBackendsError> {
    match env::var(BISCUIT_TEST_REQUIRED_BACKENDS) {
        Ok(raw) => parse_required_backends(&raw),
        Err(_) => Ok(BTreeSet::new()),
    }
}

/// What a gated test needs, as passed to
/// [`require_level!`](crate::require_level).
///
/// Constructed by `Into`, so a call site supplies either a bare label (no
/// machine identity, cannot be demanded by CI) or a [`Backend`] (identity plus
/// label).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HarnessSpec<'a> {
    backend: Option<Backend>,
    label: &'a str,
}

impl<'a> HarnessSpec<'a> {
    /// A requirement with a diagnostic label but no machine identity.
    #[must_use]
    pub const fn labeled(label: &'a str) -> Self {
        Self {
            backend: None,
            label,
        }
    }

    /// The stable backend identity, when the call site supplied one.
    #[must_use]
    pub const fn backend(self) -> Option<Backend> {
        self.backend
    }

    /// The human-readable name for skip and panic messages.
    #[must_use]
    pub const fn label(self) -> &'a str {
        self.label
    }
}

impl<'a> From<&'a str> for HarnessSpec<'a> {
    fn from(label: &'a str) -> Self {
        HarnessSpec::labeled(label)
    }
}

impl<'a> From<&'a String> for HarnessSpec<'a> {
    fn from(label: &'a String) -> Self {
        HarnessSpec::labeled(label.as_str())
    }
}

impl From<Backend> for HarnessSpec<'static> {
    fn from(backend: Backend) -> Self {
        backend.spec()
    }
}

#[cfg(test)]
mod tests {
    use super::{Backend, BackendParseError, HarnessSpec, parse_required_backends};

    #[test]
    fn identifiers_match_the_ci_policy_vocabulary() {
        let ids: Vec<&str> = Backend::ALL.iter().map(|b| b.as_str()).collect();
        assert_eq!(ids, ["tmux", "wezterm", "kitty", "apple-terminal"]);
    }

    #[test]
    fn parse_normalizes_case_and_whitespace() {
        assert_eq!(Backend::parse("  TMUX "), Ok(Backend::Tmux));
        assert_eq!(Backend::parse("Apple-Terminal"), Ok(Backend::AppleTerminal));
    }

    #[test]
    fn parse_rejects_near_misses_without_substring_matching() {
        for candidate in ["tmux2", "wez", "tmu", "apple", "kitty-x", "xtmux"] {
            assert_eq!(
                Backend::parse(candidate),
                Err(BackendParseError::Unknown(candidate.to_ascii_lowercase())),
                "{candidate} must not resolve",
            );
        }
    }

    #[test]
    fn blank_list_is_an_empty_set() {
        assert!(parse_required_backends("").unwrap().is_empty());
        assert!(parse_required_backends("   ").unwrap().is_empty());
    }

    #[test]
    fn list_parse_dedupes_and_sorts() {
        let set = parse_required_backends("wezterm, tmux ,WEZTERM").unwrap();
        assert_eq!(
            set.into_iter().collect::<Vec<_>>(),
            vec![Backend::Tmux, Backend::WezTerm]
        );
    }

    #[test]
    fn list_parse_rejects_empty_entries() {
        let err = parse_required_backends("tmux,,wezterm").unwrap_err();
        assert_eq!(err.source, BackendParseError::Empty);

        let trailing = parse_required_backends("tmux,").unwrap_err();
        assert_eq!(trailing.source, BackendParseError::Empty);
    }

    #[test]
    fn list_parse_reports_the_offending_entry() {
        let err = parse_required_backends("tmux, weztermm").unwrap_err();
        assert_eq!(err.entry, " weztermm");
        assert!(err.to_string().contains("BISCUIT_TEST_REQUIRED_BACKENDS"));
        assert!(err.to_string().contains("weztermm"));
    }

    #[test]
    fn spec_from_backend_carries_identity_and_label() {
        let spec: HarnessSpec<'_> = Backend::WezTerm.into();
        assert_eq!(spec.backend(), Some(Backend::WezTerm));
        assert_eq!(spec.label(), "WezTerm");
    }

    #[test]
    fn spec_from_label_has_no_identity() {
        let spec: HarnessSpec<'_> = "PTY (/dev/ptmx)".into();
        assert_eq!(spec.backend(), None);
        assert_eq!(spec.label(), "PTY (/dev/ptmx)");
    }
}
