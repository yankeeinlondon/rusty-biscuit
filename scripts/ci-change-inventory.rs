//! The resolved plan's change inventory, and the one projection both reports
//! render from it.
//!
//! ## Notes
//!
//! Included by `ci-plan` (terminal, `TerminalRenderable`) and by `ci-rollup
//! summarize` (GitHub Markdown) with `#[path]`, so the local and hosted reports
//! read the same field through the same typed model. Spec section 7 requires
//! both renderers to consume one payload; a second deserializer is how they
//! would come to describe different changes.
//!
//! Deliberately links nothing: `ci-rollup` is the always-runs merge-gate binary
//! and builds with `--no-default-features`.

// Cargo compiles this file once per including bin, and each uses a subset:
// `ci-plan` formats the plain entries, `ci-rollup` the Markdown ones. Every item
// here has a caller — just not in both compilations.
#![allow(dead_code)]

use std::collections::BTreeMap;

use serde::Deserialize;

/// Inventory buckets in report order. Mirrors
/// `scripts/ci/schema.py::CHANGE_BUCKETS`, which is exhaustive over the changed
/// paths; an unknown bucket in the plan is still rendered, after these, rather
/// than dropped.
const BUCKETS: [&str; 4] = ["configuration", "documentation", "source", "other"];

/// What changed, bucketed once by the planner.
///
/// A manual full-scope request consulted no diff and says so through
/// `diff_available: false` plus a `reason`; `paths` and `counts` are present
/// exactly when a diff was consulted. Empty buckets are absent from the
/// rendered entries — "nothing changed here" is not a finding — but a
/// consulted diff with no path at all is stated outright, because silence
/// there reads as a renderer that failed.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct ChangeInventory {
    #[serde(default)]
    diff_available: bool,
    #[serde(default)]
    paths: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    counts: BTreeMap<String, u64>,
    #[serde(default)]
    reason: String,
}

impl ChangeInventory {
    /// The headline both reports print above [`Self::entries`].
    pub fn headline(&self) -> String {
        if !self.diff_available {
            let reason = if self.reason.is_empty() {
                "the plan recorded no reason".to_owned()
            } else {
                self.reason.clone()
            };
            return format!("**Change inventory** — no diff was consulted: {reason}.");
        }
        match self.counts.get("total").copied().unwrap_or_default() {
            0 => "**Change inventory** — the diff named no path.".to_owned(),
            total => format!("**Change inventory** — {total} changed path(s)."),
        }
    }

    /// The non-empty buckets in report order, each with the paths it holds.
    ///
    /// This is the shared payload: both renderers format *these* pairs, so a
    /// path one of them names is a path the other names. Paths are named rather
    /// than counted because a documentation-only change accounts for itself by
    /// naming its documents (AC11). Diff bodies are not embedded: they are
    /// unbounded and duplicate the review UI.
    pub fn buckets(&self) -> Vec<(&str, &[String])> {
        if !self.diff_available {
            return Vec::new();
        }
        let declared = BUCKETS.iter().copied();
        let extra = self
            .paths
            .keys()
            .map(String::as_str)
            .filter(|name| !BUCKETS.contains(name));
        declared
            .chain(extra)
            .filter_map(|bucket| {
                let paths = self.paths.get(bucket)?.as_slice();
                (!paths.is_empty()).then_some((bucket, paths))
            })
            .collect()
    }

    /// One Markdown list entry per non-empty bucket.
    pub fn markdown_entries(&self) -> Vec<String> {
        self.buckets()
            .into_iter()
            .map(|(bucket, paths)| {
                format!(
                    "**{bucket}** ({}) — {}",
                    paths.len(),
                    paths
                        .iter()
                        .map(|path| format!("`{path}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect()
    }

    /// One unstyled entry per non-empty bucket, for the terminal.
    ///
    /// Unstyled deliberately: the terminal renderer reads inline Markdown, and
    /// `.github/workflows/_area-ci.yml` … `_package-ci.yml` is a left-flanking
    /// `_` pair that it emphasizes — mangling the very path names this report
    /// exists to state. Backticks do not protect them.
    pub fn plain_entries(&self) -> Vec<String> {
        self.buckets()
            .into_iter()
            .map(|(bucket, paths)| format!("{bucket} ({}) — {}", paths.len(), paths.join(", ")))
            .collect()
    }
}

/// The affirmative statement a plan with no scheduled cell makes (spec section
/// 7). It is a successful scheduling decision, so it is neither a warning nor a
/// claim about whether the change may merge — `ci-gate` owns that.
pub const NO_PACKAGE_TESTS: &str =
    "No package test is required: the resolved plan schedules no package cell.";

/// One inventory payload, in the bytes both renderers' suites parse.
///
/// Validation step 4 asks for proof that the terminal and the Markdown report
/// consume the *identical* payload. Each suite reads this text and asserts its
/// own renderer names every path in it, so a renderer that reads a different
/// field, or drops a path the other keeps, fails against the same input rather
/// than against a fixture someone kept in step by hand.
#[cfg(test)]
pub const FIXTURE_INVENTORY: &str = r#"{
    "diff_available": true,
    "paths": {
        "configuration": ["Cargo.toml"],
        "documentation": ["README.md", "docs/ci.md", "scripts/ci/_underscore.md"],
        "other": [],
        "source": ["scripts/ci/schema.py"]
    },
    "counts": {
        "configuration": 1, "documentation": 3, "other": 0, "source": 1,
        "total": 5
    }
}"#;

/// Every path [`FIXTURE_INVENTORY`] holds. Both renderers must name all of
/// them; one that is spelled differently by one renderer is the drift
/// Validation step 4 exists to catch. `_underscore.md` is here because the
/// terminal's inline Markdown emphasizes a flanking `_` pair.
#[cfg(test)]
pub const FIXTURE_PATHS: [&str; 5] = [
    "Cargo.toml",
    "README.md",
    "docs/ci.md",
    "scripts/ci/_underscore.md",
    "scripts/ci/schema.py",
];

/// The Markdown entries [`FIXTURE_INVENTORY`] must project to, in report order.
#[cfg(test)]
pub const FIXTURE_MARKDOWN_ENTRIES: [&str; 3] = [
    "**configuration** (1) — `Cargo.toml`",
    "**documentation** (3) — `README.md`, `docs/ci.md`, `scripts/ci/_underscore.md`",
    "**source** (1) — `scripts/ci/schema.py`",
];

/// The same three entries, unstyled, as the terminal renderer takes them.
#[cfg(test)]
pub const FIXTURE_PLAIN_ENTRIES: [&str; 3] = [
    "configuration (1) — Cargo.toml",
    "documentation (3) — README.md, docs/ci.md, scripts/ci/_underscore.md",
    "source (1) — scripts/ci/schema.py",
];
