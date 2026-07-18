//! Regression enforcement for Claudine's diagnostic contract — the three
//! guards of error-propagation Phase 6 (D2, D7, D8).
//!
//! Each guard exists because a property the feature relies on is *not*
//! enforceable by the type system:
//!
//! - **D2 — registry parity.** Stable Rust cannot upcast `&dyn Error` to
//!   `&dyn Diagnostic`, so `as_diagnostic` is a hand-authored downcast list.
//!   Nothing makes it exhaustive except [`registry_lists_every_diagnostic_impl`],
//!   which re-derives the truth from the sources. A fourth `impl Diagnostic`
//!   landing unregistered is exactly the motivating incident.
//! - **D7 — catalog parity.** A registered code promises a catalog-shaped
//!   `detail` object. `serde_json` will happily insert an undeclared key or
//!   return a top-level `null`, and every downstream `err.detail.*` lookup then
//!   silently reads wrong.
//! - **D8 — lossy boundaries.** A typed error collapsed to prose still compiles
//!   and still prints something plausible. Only a scan can see it.
//!
//! `just lint-transport` runs the D8 guard; `just test` runs all three.
//!
//! Exceptions live in [`ALLOWLIST_FILE`], keyed to an enclosing symbol and
//! carrying a written reason. See
//! `claudine/features/2026-07-13-error-propogation/spec.md` §D2, §D7, §D8.

// A test target's crate root is this file, so a plain `mod` would resolve
// against `tests/` — where every `.rs` is its own test binary.
#[path = "error_guards/source_scan.rs"]
mod source_scan;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;

use claudine::diagnostics::{CODES, DiagnosticSnapshot, code_spec};
use serde::Deserialize;
use serde_json::Value;

use source_scan::{
    BoxedError, Finding, Shape, area_root, discovery_registry, scan_production_sources,
};

/// Area-relative path of the D8 exception list.
const ALLOWLIST_FILE: &str = "cli/tests/error_guards/transport-allow.toml";

// ---------------------------------------------------------------------------
// The allowlist
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct Allowlist {
    #[serde(default)]
    allow: Vec<AllowEntry>,
}

/// One exception, tied to the symbol that owns it.
///
/// The predecessor grep guard matched on a *trimmed source line*, so an entry
/// added for one file silently suppressed an identical line in every other file.
/// Keying on `(shape, file, symbol)` is what makes an exception mean "this
/// boundary, for this reason".
#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct AllowEntry {
    shape: String,
    file: String,
    symbol: String,
    /// The workstream that owns this exception, mirroring the
    /// grandfather-with-burn-down convention of `dispatch_inventory.rs`.
    /// `retained` marks a permanent exception; anything else is a burn-down
    /// bucket some future feature closes.
    tag: String,
    /// Why the typed cause does not survive here. Required, and required to be
    /// substantive — an exception without a rationale is a silent regression.
    reason: String,
}

impl AllowEntry {
    fn covers(&self, finding: &Finding) -> bool {
        self.shape == finding.shape.as_str()
            && self.file == finding.file
            && self.symbol == finding.symbol
    }
}

fn load_allowlist() -> Vec<AllowEntry> {
    let path = area_root().join(ALLOWLIST_FILE);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {ALLOWLIST_FILE}: {error}"));
    let parsed: Allowlist = toml::from_str(&text)
        .unwrap_or_else(|error| panic!("failed to parse {ALLOWLIST_FILE}: {error}"));
    parsed.allow
}

fn describe(findings: &[&Finding]) -> String {
    let mut out = String::new();
    for finding in findings {
        let _ = writeln!(
            out,
            "  [{}] {}:{} in `{}`\n      {}",
            finding.shape, finding.file, finding.line, finding.symbol, finding.evidence
        );
    }
    out
}

// ---------------------------------------------------------------------------
// D8 — lossy-boundary guard
// ---------------------------------------------------------------------------

#[test]
fn no_unallowlisted_typed_error_collapses() {
    let scan = scan_production_sources();
    let allowlist = load_allowlist();

    let unexpected: Vec<&Finding> = scan
        .findings
        .iter()
        .filter(|finding| !allowlist.iter().any(|entry| entry.covers(finding)))
        .collect();

    assert!(
        unexpected.is_empty(),
        "{} typed-error collapse(s) with no allowlist entry:\n{}\n\
         Preserve the typed cause as a `#[source]` field or a `#[from]` variant. \
         If the value genuinely has no typed source, add an entry to {ALLOWLIST_FILE} \
         naming the shape, file, and enclosing symbol, with a reason.",
        unexpected.len(),
        describe(&unexpected)
    );
}

#[test]
fn every_allowlist_entry_still_matches_a_live_site() {
    // A stale entry is worse than no entry: it is a standing exception for code
    // that no longer exists, ready to suppress an unrelated future defect that
    // happens to land in the same symbol.
    let scan = scan_production_sources();
    let allowlist = load_allowlist();

    let stale: Vec<&AllowEntry> = allowlist
        .iter()
        .filter(|entry| !scan.findings.iter().any(|finding| entry.covers(finding)))
        .collect();

    let rendered: Vec<String> = stale
        .iter()
        .map(|entry| format!("  [{}] {} in `{}`", entry.shape, entry.file, entry.symbol))
        .collect();

    assert!(
        stale.is_empty(),
        "{} allowlist entr(y/ies) match no live site — delete them:\n{}",
        stale.len(),
        rendered.join("\n")
    );
}

/// Burn-down buckets an entry may claim. A tag outside this set is a typo, and
/// a typo'd tag is an exception nobody will ever find again.
const KNOWN_TAGS: &[&str] = &[
    // Permanent: no typed source exists, or the boundary is downstream of the
    // final render.
    "retained",
    // A ratified cross-spec deferral: a typed error is in hand, but typing it
    // would move an authored `err.*` matching surface, which §D10 and D-2/D-12
    // reserve for a separate versioned migration. Not a burn-down — the set
    // does not grow. (The `error-propagation-followup` burn-down is closed.)
    "d10-routing-change",
];

#[test]
fn every_allowlist_entry_names_a_known_shape_tag_and_reason() {
    for entry in load_allowlist() {
        assert!(
            Shape::parse(&entry.shape).is_some(),
            "allowlist entry for `{}` names unknown shape `{}`",
            entry.symbol,
            entry.shape
        );
        assert!(
            KNOWN_TAGS.contains(&entry.tag.as_str()),
            "allowlist entry [{}] {} in `{}` claims unknown tag `{}`; known tags: {KNOWN_TAGS:?}",
            entry.shape,
            entry.file,
            entry.symbol,
            entry.tag
        );
        assert!(
            entry.reason.split_whitespace().count() >= 5,
            "allowlist entry [{}] {} in `{}` has no substantive reason: {:?}",
            entry.shape,
            entry.file,
            entry.symbol,
            entry.reason
        );
    }
}

// ---------------------------------------------------------------------------
// D2 — source parity between the sources and the discovery registry
// ---------------------------------------------------------------------------

#[test]
fn registry_lists_every_diagnostic_impl() {
    let scan = scan_production_sources();
    let registered = discovery_registry();

    let implemented: BTreeSet<String> = scan
        .trait_impls
        .iter()
        .filter(|item| item.trait_name == "Diagnostic")
        .map(|item| item.type_name.clone())
        .collect();

    let missing: Vec<&String> = implemented.difference(&registered).collect();
    assert!(
        missing.is_empty(),
        "{:?} implement `Diagnostic` but `as_diagnostic` cannot see them, so the CLI walker \
         renders them as a generic `Error:` line. Add a `downcast_ref::<T>()` arm in \
         lib/src/diagnostics/discovery.rs.",
        missing
    );

    let phantom: Vec<&String> = registered.difference(&implemented).collect();
    assert!(
        phantom.is_empty(),
        "`as_diagnostic` downcasts to {:?}, which no production `impl Diagnostic for …` \
         defines. The registry has drifted from the sources.",
        phantom
    );
}

#[test]
fn every_diagnostic_impl_also_implements_block_error() {
    // `Diagnostic: BlockError`, so this cannot fail while both impls are in
    // scope — but it can fail *the scan*, which is the point: a `Diagnostic`
    // impl the scan pairs with no `BlockError` impl means the scan lost a file,
    // and a blind guard is a guard that passes.
    let scan = scan_production_sources();
    let blocks: BTreeSet<&str> = scan
        .trait_impls
        .iter()
        .filter(|item| item.trait_name == "BlockError")
        .map(|item| item.type_name.as_str())
        .collect();

    for item in scan.trait_impls.iter().filter(|i| i.trait_name == "Diagnostic") {
        assert!(
            blocks.contains(item.type_name.as_str()),
            "`{}` implements Diagnostic but the scan found no `impl BlockError for {}` — \
             the scan's file set is wrong",
            item.type_name,
            item.type_name
        );
    }
}

#[test]
fn the_scan_finds_the_diagnostic_impls_known_to_exist() {
    // Anchors the parity test against a scan that silently matches nothing.
    let scan = scan_production_sources();
    let implemented: BTreeSet<&str> = scan
        .trait_impls
        .iter()
        .filter(|item| item.trait_name == "Diagnostic")
        .map(|item| item.type_name.as_str())
        .collect();

    for expected in ["ClaudineError", "CompositionError", "HarnessError"] {
        assert!(
            implemented.contains(expected),
            "scan did not find `impl Diagnostic for {expected}`; found {implemented:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// D-13 — reachability of a registered diagnostic through a `Box`
// ---------------------------------------------------------------------------

/// Area-relative path of the boxed-diagnostic exception list.
const BOXED_ALLOWLIST_FILE: &str = "cli/tests/error_guards/boxed-diagnostic-allow.toml";

#[derive(Debug, Deserialize)]
struct BoxedAllowlist {
    #[serde(default)]
    allow: Vec<BoxedAllowEntry>,
}

#[derive(Debug, Deserialize)]
struct BoxedAllowEntry {
    site: String,
    file: String,
    symbol: String,
    /// The boxed type, so an entry cannot widen to cover a second `Box<T>` that
    /// later lands in the same symbol.
    boxed: String,
    tag: String,
    reason: String,
}

impl BoxedAllowEntry {
    fn covers(&self, boxed: &BoxedError) -> bool {
        self.site == boxed.site.as_str()
            && self.file == boxed.file
            && self.symbol == boxed.symbol
            && self.boxed == boxed.type_name
    }
}

fn load_boxed_allowlist() -> Vec<BoxedAllowEntry> {
    let path = area_root().join(BOXED_ALLOWLIST_FILE);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {BOXED_ALLOWLIST_FILE}: {error}"));
    let parsed: BoxedAllowlist = toml::from_str(&text)
        .unwrap_or_else(|error| panic!("failed to parse {BOXED_ALLOWLIST_FILE}: {error}"));
    parsed.allow
}

/// Every `Box<T>` on a cause chain where `T` is a registered diagnostic.
fn boxed_registered_diagnostics() -> Vec<&'static BoxedError> {
    let registered = discovery_registry();
    scan_production_sources()
        .boxed_errors
        .iter()
        .filter(|boxed| registered.contains(&boxed.type_name))
        .collect()
}

#[test]
fn no_registered_diagnostic_is_reachable_only_through_a_box() {
    // D-7 proved that a `Box<E>` publishes `Box<E>` to the cause chain, not `E`:
    // `downcast_ref::<E>()` returns `None`, and `Box`'s own `source()` delegates
    // to `E::source()`, so the walk skips `E` at every depth. `as_diagnostic` is
    // a downcast allowlist over concrete types, so a registered diagnostic
    // behind a `Box` is invisible however correctly it is registered.
    //
    // D-13 widened the shape D-7 asked for. Its live instance was not a
    // `#[source] Box<T>` field but a `Result<_, Box<CompositionError>>` whose
    // error went `.into()` a `Report` — making the *report's root* a `Box`. The
    // source-parity, transport, and headless-render guards all passed on it; a
    // real terminal found it. Hence both sites are scanned.
    let unexpected: Vec<&BoxedError> = boxed_registered_diagnostics()
        .into_iter()
        .filter(|boxed| {
            !load_boxed_allowlist()
                .iter()
                .any(|entry| entry.covers(boxed))
        })
        .collect();

    let rendered: Vec<String> = unexpected
        .iter()
        .map(|boxed| {
            format!(
                "  [{}] {}:{} in `{}` — Box<{}>",
                boxed.site.as_str(),
                boxed.file,
                boxed.line,
                boxed.symbol,
                boxed.type_name
            )
        })
        .collect();

    assert!(
        unexpected.is_empty(),
        "{} boxed registered diagnostic(s) with no allowlist entry:\n{}\n\
         `as_diagnostic` cannot downcast through a `Box`, so these render as a generic \
         `Error:` line. Carry the diagnostic unboxed (box a context struct instead, as \
         `CompositionError::InvalidFileReference` does), or unbox at the boundary \
         (`Report::from(*error)`). If neither is possible, add an entry to \
         {BOXED_ALLOWLIST_FILE} naming the site, file, symbol, and boxed type, with a \
         reason and the test that proves the value still resolves.",
        unexpected.len(),
        rendered.join("\n")
    );
}

#[test]
fn every_boxed_allowlist_entry_still_matches_a_live_site() {
    let live = boxed_registered_diagnostics();
    let stale: Vec<String> = load_boxed_allowlist()
        .iter()
        .filter(|entry| !live.iter().any(|boxed| entry.covers(boxed)))
        .map(|entry| {
            format!(
                "  [{}] {} in `{}` — Box<{}>",
                entry.site, entry.file, entry.symbol, entry.boxed
            )
        })
        .collect();

    assert!(
        stale.is_empty(),
        "{} boxed-diagnostic allowlist entr(y/ies) match no live site — delete them:\n{}",
        stale.len(),
        stale.join("\n")
    );
}

#[test]
fn every_boxed_allowlist_entry_names_a_known_site_tag_and_reason() {
    for entry in load_boxed_allowlist() {
        assert!(
            ["source_field", "result_error"].contains(&entry.site.as_str()),
            "boxed allowlist entry for `{}` names unknown site `{}`",
            entry.symbol,
            entry.site
        );
        assert!(
            KNOWN_TAGS.contains(&entry.tag.as_str()),
            "boxed allowlist entry [{}] {} in `{}` claims unknown tag `{}`; known tags: \
             {KNOWN_TAGS:?}",
            entry.site,
            entry.file,
            entry.symbol,
            entry.tag
        );
        assert!(
            entry.reason.split_whitespace().count() >= 5,
            "boxed allowlist entry [{}] {} in `{}` has no substantive reason: {:?}",
            entry.site,
            entry.file,
            entry.symbol,
            entry.reason
        );
    }
}

#[test]
fn the_boxed_scan_finds_the_sites_known_to_exist() {
    // Anchors the guard against a scan that silently matches nothing — the
    // failure mode that would make every assertion above vacuously true.
    let boxed = boxed_registered_diagnostics();
    assert!(
        boxed
            .iter()
            .any(|b| b.site == source_scan::BoxSite::ResultError),
        "scan found no `Result<_, Box<RegisteredDiagnostic>>`, but D-13's \
         `preflight_proxy_target` is one; found {boxed:?}"
    );
}

#[test]
fn the_boxed_scan_still_sees_a_source_field_box() {
    // The `SourceField` arm has no production site left: narrowing
    // `atomic_write` to `io::Result` retired `CompositionError::AtomicWriteFailed`'s
    // `#[source] Box<ClaudineError>`, which was D-7's only live instance and the
    // anchor this test used to assert against. Zero sites is the goal, but a
    // guard arm with nothing to match is a guard arm that can rot silently — so
    // the arm is anchored on a fixture instead of on a defect.
    let scanned = source_scan::scan_text(
        "lib/src/fixture.rs",
        r#"
        #[derive(Debug, thiserror::Error)]
        pub enum Fixture {
            #[error("boxed")]
            Boxed {
                #[source]
                source: Box<ClaudineError>,
            },
        }
        "#,
    );
    let registered = source_scan::discovery_registry();
    assert!(
        scanned.boxed_errors.iter().any(|b| {
            b.site == source_scan::BoxSite::SourceField
                && b.type_name == "ClaudineError"
                && registered.contains(&b.type_name)
        }),
        "the `source_field` arm no longer detects a `#[source] Box<RegisteredDiagnostic>`; \
         found {:?}",
        scanned.boxed_errors
    );
}

// ---------------------------------------------------------------------------
// D7 — catalog parity
// ---------------------------------------------------------------------------

#[test]
fn every_code_a_diagnostic_claims_is_a_registered_code() {
    let scan = scan_production_sources();
    let mut unregistered: Vec<String> = Vec::new();

    for item in scan.trait_impls.iter().filter(|i| i.trait_name == "Diagnostic") {
        for code in &item.codes {
            // `code()` bodies hold only the dotted codes they return; anything
            // else in the literal set would be a doc string or a message, and
            // those do not look like `category.slug`.
            if !code.contains('.') || code.contains(' ') {
                continue;
            }
            if code_spec(code).is_none() {
                unregistered.push(format!("{}::code() → `{code}`", item.type_name));
            }
        }
    }

    assert!(
        unregistered.is_empty(),
        "diagnostics claim codes absent from the locked catalog: {unregistered:?}"
    );
}

#[test]
fn detail_projections_write_only_declared_keys() {
    // The ad hoc key check. `base["retry_after"] = …` on a code that declares
    // `retry_after_ms` compiles, projects, and reads as `null` forever at
    // `err.detail.retry_after_ms`. Nothing but this catches the typo.
    let scan = scan_production_sources();
    let mut violations: Vec<String> = Vec::new();

    for item in scan.trait_impls.iter().filter(|i| i.trait_name == "Diagnostic") {
        let declared: BTreeSet<&str> = item
            .codes
            .iter()
            .filter_map(|code| code_spec(code))
            .flat_map(|spec| spec.detail.iter().copied())
            .collect();
        if declared.is_empty() {
            continue;
        }
        for key in &item.detail_keys {
            if !declared.contains(key.as_str()) {
                violations.push(format!(
                    "{}::detail() writes `{key}`, which none of its codes declare",
                    item.type_name
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "undeclared `detail` keys:\n  {}",
        violations.join("\n  ")
    );
}

#[test]
fn a_diagnostic_claiming_a_registered_code_projects_a_detail() {
    // The default `Diagnostic::detail` returns `Value::Null`. Inheriting it
    // while claiming a registered code is precisely the D7 top-level-null
    // defect, in its static form.
    let scan = scan_production_sources();

    for item in scan.trait_impls.iter().filter(|i| i.trait_name == "Diagnostic") {
        let claims_registered = item.codes.iter().any(|code| code_spec(code).is_some());
        if !claims_registered {
            continue;
        }
        assert!(
            item.overrides_detail,
            "`{}` ({}) claims registered codes but inherits the default `detail()`, which \
             returns a top-level `Value::Null` — `err.detail.<field>` would fail against a scalar",
            item.type_name, item.file
        );
    }
}

#[test]
fn from_code_projects_a_catalog_shaped_detail_for_every_registered_code() {
    // The synthesized-label path (`from_action_failure` → `code_for_error_kind`
    // → `from_code`) is where the null detail used to come from: it knows the
    // code but none of the per-instance values. It must still project the
    // object, with every declared key present and `null`.
    for spec in CODES {
        let snapshot = DiagnosticSnapshot::from_code(spec.code, String::new())
            .unwrap_or_else(|| panic!("`{}` is in CODES but from_code missed it", spec.code));

        let Value::Object(detail) = &snapshot.detail else {
            panic!(
                "`{}` projects a top-level non-object detail: {:?}",
                spec.code, snapshot.detail
            );
        };

        let present: BTreeSet<&str> = detail.keys().map(String::as_str).collect();
        let declared: BTreeSet<&str> = spec.detail.iter().copied().collect();
        assert_eq!(
            present, declared,
            "`{}` projects detail keys {present:?} but declares {declared:?}",
            spec.code
        );
        for (key, value) in detail {
            assert!(
                value.is_null(),
                "`{}` projects `{value}` for `{key}`, which it cannot know from the label alone",
                spec.code
            );
        }
    }
}

#[test]
fn every_diagnostic_in_the_corpus_projects_its_catalog_key_set() {
    // The runtime half of the catalog-parity guard: a constructed diagnostic's
    // `detail()` must carry exactly the key set its `code()` declares — no
    // missing declared key, no ad hoc extra.
    for (label, error) in corpus::all() {
        let code = error.code();
        let spec = code_spec(code)
            .unwrap_or_else(|| panic!("{label} claims unregistered code `{code}`"));

        let detail = error.detail();
        let Value::Object(object) = &detail else {
            panic!("{label} (`{code}`) projects a top-level non-object detail: {detail:?}");
        };

        let present: BTreeSet<&str> = object.keys().map(String::as_str).collect();
        let declared: BTreeSet<&str> = spec.detail.iter().copied().collect();
        assert_eq!(
            present, declared,
            "{label} (`{code}`) projects {present:?} but the catalog declares {declared:?}"
        );
    }
}

#[test]
fn the_corpus_covers_every_code_a_diagnostic_can_return() {
    // Completeness. Without this the corpus is a spot-check that quietly stops
    // covering a code the moment a new variant maps to one.
    let scan = scan_production_sources();
    let mut claimed: BTreeSet<String> = BTreeSet::new();
    for item in scan.trait_impls.iter().filter(|i| i.trait_name == "Diagnostic") {
        for code in &item.codes {
            if code_spec(code).is_some() {
                claimed.insert(code.to_string());
            }
        }
    }

    let mut covered: BTreeMap<String, usize> = BTreeMap::new();
    for (_, error) in corpus::all() {
        *covered.entry(error.code().to_string()).or_default() += 1;
    }

    let uncovered: Vec<&String> = claimed
        .iter()
        .filter(|code| !covered.contains_key(*code))
        .collect();

    assert!(
        uncovered.is_empty(),
        "a production diagnostic can return {uncovered:?}, but no corpus entry does — add one to \
         `corpus::all()` so its detail projection is checked against the catalog"
    );
}

/// Constructed diagnostics covering every registered code a production
/// `Diagnostic` can return.
///
/// Held here rather than beside each error type because the property under test
/// is cross-type: it is the *catalog* that must agree with every projection, and
/// [`the_corpus_covers_every_code_a_diagnostic_can_return`] is what keeps this
/// list honest as variants land.
mod corpus {
    use std::path::PathBuf;

    use claudine::composition::{CompositionError, ShellApprovalFailure};
    use claudine::diagnostics::Diagnostic;
    use claudine::error::ClaudineError;
    use claudine::harness::error::{HarnessError, PathResolutionFailure};
    use claudine::hook_adapters::AdapterError;
    use claudine::provider_id::Provider;
    use darkmatter::markdown::compose::expression::ExpressionError;
    use darkmatter::markdown::{MarkdownError, SourceRef};

    pub fn all() -> Vec<(&'static str, Box<dyn Diagnostic>)> {
        vec![
            // --- ClaudineError ---
            (
                "ClaudineError::ProviderNotAvailable",
                Box::new(ClaudineError::ProviderNotAvailable("codex".to_string())),
            ),
            (
                "ClaudineError::SystemPromptFileNotFound",
                Box::new(ClaudineError::SystemPromptFileNotFound("sp.md".to_string())),
            ),
            (
                "ClaudineError::LockError",
                Box::new(ClaudineError::LockError {
                    path: PathBuf::from("/tmp/x.lock"),
                }),
            ),
            (
                "ClaudineError::Io(PermissionDenied)",
                Box::new(ClaudineError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "denied",
                ))),
            ),
            (
                "ClaudineError::ConfigValidation",
                Box::new(ClaudineError::ConfigValidation("bad field".to_string())),
            ),
            (
                "ClaudineError::McpCatalogNotFound",
                Box::new(ClaudineError::McpCatalogNotFound),
            ),
            (
                "ClaudineError::McpProviderNotSupported",
                Box::new(ClaudineError::McpProviderNotSupported {
                    provider: Provider::Goose,
                    reason: "no MCP".to_string(),
                }),
            ),
            (
                "ClaudineError::TemplateError",
                Box::new(ClaudineError::TemplateError("boom".to_string())),
            ),
            (
                "ClaudineError::LinkingError",
                Box::new(ClaudineError::LinkingError("link failed".to_string())),
            ),
            (
                "ClaudineError::UrlError",
                Box::new(ClaudineError::UrlError(
                    url::Url::parse("not a url").expect_err("`not a url` has no scheme"),
                )),
            ),
            (
                "ClaudineError::Adapter",
                Box::new(ClaudineError::Adapter(AdapterError::MissingField(
                    "session_id",
                ))),
            ),
            // --- HarnessError ---
            (
                "HarnessError::InvalidTimeout",
                Box::new(HarnessError::InvalidTimeout {
                    source_path: PathBuf::from("run.md"),
                    raw: "5x".to_string(),
                    detail: "unknown unit".to_string(),
                }),
            ),
            (
                "HarnessError::ShellCommandDenied",
                Box::new(HarnessError::ShellCommandDenied {
                    command: "rm -rf /".to_string(),
                }),
            ),
            (
                "HarnessError::RepoRootRequired",
                Box::new(HarnessError::RepoRootRequired {
                    path: "@/x".to_string(),
                }),
            ),
            (
                "HarnessError::PathResolutionFailed",
                Box::new(HarnessError::PathResolutionFailed {
                    raw: "nope.md".to_string(),
                    failure: PathResolutionFailure::TargetMissing,
                    source_path: Some(PathBuf::from("/repo/run.md")),
                    resolved: Some(PathBuf::from("/repo/nope.md")),
                    resolution: None,
                }),
            ),
            // --- CompositionError ---
            (
                "CompositionError::FileNotFound",
                Box::new(CompositionError::FileNotFound("missing.md".to_string())),
            ),
            (
                "CompositionError::FrontmatterParse",
                Box::new(CompositionError::FrontmatterParse(MarkdownError::AstParse(
                    "bad yaml".to_string(),
                ))),
            ),
            (
                "CompositionError::ComposeFailed",
                Box::new(CompositionError::ComposeFailed(MarkdownError::AstParse(
                    "compose".to_string(),
                ))),
            ),
            (
                "CompositionError::PreFlightFailed",
                Box::new(CompositionError::PreFlightFailed("preflight".to_string())),
            ),
            (
                "CompositionError::ShellApprovalUnavailable",
                Box::new(CompositionError::ShellApprovalUnavailable {
                    command: "rm -rf /".to_string(),
                    source_file: PathBuf::from("run.md"),
                    line: 4,
                    failure: ShellApprovalFailure::Blacklisted("destructive".to_string()),
                }),
            ),
            (
                "CompositionError::SchemaLoad",
                Box::new(CompositionError::SchemaLoad {
                    source_path: PathBuf::from("run.md"),
                    message: "no such schema".to_string(),
                }),
            ),
            (
                "CompositionError::SchemaParse",
                Box::new(CompositionError::SchemaParse {
                    source_path: PathBuf::from("schema.yaml"),
                    property: Some("agent".to_string()),
                    message: "unknown type".to_string(),
                    span: None,
                }),
            ),
            (
                "CompositionError::SchemaValidation",
                Box::new(CompositionError::SchemaValidation {
                    source_path: PathBuf::from("run.md"),
                    message: "agent must be a string".to_string(),
                    problems: Vec::new(),
                }),
            ),
            (
                "CompositionError::MissingProperties",
                Box::new(CompositionError::MissingProperties {
                    source_path: PathBuf::from("run.md"),
                    missing: Vec::new(),
                    frontmatter_description: None,
                    pointer_paths: Vec::new(),
                }),
            ),
            // `compose_failed_code` routes a Darkmatter interpolation failure to a
            // finer code than `ComposeFailed`'s own arm does, so these three are the
            // only way to reach `unknown_function` / `expression_invalid` and the
            // `ComposeFailed`-borne `invalid_file_reference`.
            (
                "CompositionError::ComposeFailed(UnknownFunction)",
                Box::new(CompositionError::ComposeFailed(interpolation_failure(
                    ExpressionError::UnknownFunction {
                        name: "upper_case".to_string(),
                    },
                ))),
            ),
            (
                "CompositionError::ComposeFailed(ExpressionInvalid)",
                Box::new(CompositionError::ComposeFailed(interpolation_failure(
                    ExpressionError::Parse("unbalanced parenthesis".to_string()),
                ))),
            ),
        ]
    }

    /// A Darkmatter interpolation failure carrying `cause`.
    fn interpolation_failure(cause: ExpressionError) -> MarkdownError {
        MarkdownError::Interpolation {
            key: Some("agent".to_string()),
            expression: "upper_case(x)".to_string(),
            source: Box::new(SourceRef::Effective {
                rendered: "upper_case(x)".to_string(),
                origin_key: Some("agent".to_string()),
            }),
            cause: Box::new(cause),
        }
    }
}
