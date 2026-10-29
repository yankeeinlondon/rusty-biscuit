//! Claudine's library keeps no second lifecycle implementation
//! (`2026-10-27-darkmatter-lifecycle-ownership`, acceptance 4).
//!
//! The parser, executor, run guard, catch protocol, control admission, and
//! recovery policy live in `darkmatter::lifecycle`; Claudine re-exports and
//! adapts them. This guard scans `claudine/lib/src` with the shared
//! [`scan`]; `claudine-cli`'s guard of the same name scans `claudine/cli/src`,
//! so each runs when its own package changes. The scan's self-tests live here,
//! beside the scan's source.

// The binary's other source-scan gates load `source_scan.rs` too; the copies
// are stateless, so loading it again is harmless.
#[allow(clippy::duplicate_mod)]
#[path = "../../../../darkmatter/cli/tests/common/source_scan.rs"]
mod source_scan;
#[path = "../common/lifecycle_ownership_scan.rs"]
mod scan;

use std::collections::BTreeMap;

use scan::{Allowed, Owned, Rule, TWIN_SOURCE, allowed, violations};
use source_scan::{module_tree_sources, production_sources, sanitize};

const LIFECYCLE_MOD: &str = "lib/src/composition/lifecycle/mod.rs";
const PREFLIGHT: &str = "lib/src/composition/preflight.rs";

/// Reviewed adapters in `claudine/lib/src`; the CLI guard keeps its own.
const ALLOWED: &[Allowed] = &[
    // -- Definitions: aliases and thin wrappers over Darkmatter's item. --
    allowed(Rule::Definition, LIFECYCLE_MOD, "LifecycleRunGuard", "type alias binding Darkmatter's guard to `ClaudineRunHost`"),
    allowed(
        Rule::Definition,
        "lib/src/composition/lifecycle/runtime.rs",
        "ContainerRunner",
        "type alias binding Darkmatter's container runner to `ClaudineRunHost`",
    ),
    allowed(
        Rule::Definition,
        LIFECYCLE_MOD,
        "validate_no_nested_spans_in_literals",
        "delegates to Darkmatter's check and converts its error to the `CompositionError` twin",
    ),
    allowed(
        Rule::Definition,
        LIFECYCLE_MOD,
        "with_mapping_shape",
        "delegates to Darkmatter's check and converts its error to the `CompositionError` twin",
    ),
    allowed(Rule::Definition, PREFLIGHT, "settle_lifecycle_shell_modes", "thin wrapper keeping the Wave 7 crate-internal signature"),
    allowed(Rule::Definition, PREFLIGHT, "lifecycle_document_roots", "thin wrapper keeping the Wave 7 crate-internal signature"),
    // -- Decision errors: the one mapping from the kernel's refusal. --
    allowed(
        Rule::DecisionError,
        "lib/src/composition/error/mod.rs",
        "from_lifecycle_refusal: LifecycleSetupPhaseRecoveryUnsupported",
        "the one mapping of a kernel refusal onto Claudine's catalog",
    ),
    allowed(
        Rule::DecisionError,
        "lib/src/composition/error/mod.rs",
        "from_lifecycle_refusal: LifecycleResumeWithoutSession",
        "the one mapping of a kernel refusal onto Claudine's catalog",
    ),
    allowed(
        Rule::DecisionError,
        "lib/src/composition/error/mod.rs",
        "from_lifecycle_refusal: LifecycleDeferNotImplemented",
        "the one mapping of a kernel refusal onto Claudine's catalog",
    ),
    // -- Parser twins: the conversion, and Claudine's own grammars that share
    //    a twin's variant. --
    allowed(Rule::ParserTwin, TWIN_SOURCE, "from: *", "the `From<LifecycleError>` conversion itself"),
    allowed(
        Rule::ParserTwin,
        "lib/src/composition/lifecycle/executor.rs",
        "classify_run_failure: LifecycleUnknownEffect",
        "projects the executor's run failure, not a parse, onto Claudine's catalog",
    ),
    allowed(
        Rule::ParserTwin,
        "lib/src/composition/prepare.rs",
        "preflight_bootstrap_shell: LifecycleInvalid",
        "Claudine's shell-free bootstrap frontmatter rule, checked outside the lifecycle parser",
    ),
    allowed(
        Rule::ParserTwin,
        "lib/src/composition/looping/config.rs",
        "check_loop_keys: RemovedGrammar",
        "Claudine's loop grammar retires `loop.fail_fast`",
    ),
    allowed(
        Rule::ParserTwin,
        "lib/src/composition/lifecycle/bindings.rs",
        "removed_loop_read_in_composition: RemovedGrammar",
        "Claudine's retired sequence binding names, read during composition",
    ),
    allowed(
        Rule::ParserTwin,
        "lib/src/composition/lifecycle/bindings.rs",
        "removed_loop_grammar: RemovedGrammar",
        "Claudine's retired `_loop_*` binding names",
    ),
    allowed(
        Rule::ParserTwin,
        "lib/src/composition/lifecycle/bindings/rename.rs",
        "reject: RemovedGrammar",
        "Claudine's renamed binding roots",
    ),
    allowed(
        Rule::ParserTwin,
        "lib/src/composition/sequence/grammar.rs",
        "validate_task_grammar: RemovedGrammar",
        "Claudine's sequence task grammar retires container events on non-inline tasks",
    ),
    allowed(
        Rule::ParserTwin,
        "lib/src/composition/sequence/grammar.rs",
        "removed_task_key: RemovedGrammar",
        "Claudine's sequence task grammar retires `setup`/`teardown`/`execution`/`fail_fast`",
    ),
    // -- Seams: Claudine's adapters to Darkmatter's host traits. --
    allowed(Rule::Seam, LIFECYCLE_MOD, "LifecycleRunHost for ClaudineRunHost", "the run guard's host: source, effects, catalog, interrupts"),
    allowed(Rule::Seam, LIFECYCLE_MOD, "LifecycleRunGuardExt for LifecycleRunGuard", "Claudine's constructor and accessors for the guard"),
    allowed(
        Rule::Seam,
        "lib/src/composition/lifecycle/effects.rs",
        "LifecycleEffects for ClaudineLifecycleEffects",
        "TTS and messaging configuration for lifecycle effects",
    ),
    allowed(
        Rule::Seam,
        "lib/src/composition/lifecycle/executor.rs",
        "LifecycleHost for StackExecutionContext",
        "the executor's host: Claudine's bindings, shells, and prep",
    ),
    allowed(
        Rule::Seam,
        "lib/src/composition/lifecycle/executor.rs",
        "GuardedEventStack for StackExecutionContext",
        "runs one guarded event through Claudine's stack context",
    ),
    allowed(Rule::Seam, "lib/src/composition/lifecycle/executor.rs", "ShellRunner for SystemShellRunner", "Claudine's lifecycle shell process runner"),
    allowed(
        Rule::Seam,
        "lib/src/composition/lifecycle/executor.rs",
        "ShellRunner for DocumentShellRunner",
        "runs lifecycle shells against the document's approved shell values",
    ),
    allowed(
        Rule::Seam,
        "lib/src/composition/lifecycle/runtime.rs",
        "LifecycleCatchProtocolExt for LifecycleCatchProtocol",
        "the catch protocol's old constructor, over Claudine's catalog",
    ),
    allowed(
        Rule::Seam,
        "lib/src/composition/lifecycle/context.rs",
        "LifecycleErrorInfoExt for LifecycleErrorInfo",
        "builds `err` snapshots from Claudine's error types",
    ),
    allowed(
        Rule::Seam,
        "lib/src/composition/lifecycle/cancellation.rs",
        "CancellationCauseExt for CancellationCause",
        "the cancellation error in Claudine's catalog",
    ),
    allowed(Rule::Seam, PREFLIGHT, "ShellAuditBindings for ClaudineShellAuditBindings", "the binding catalog's answers to the shell walk"),
    allowed(
        Rule::Seam,
        "lib/src/composition/frontmatter_excerpt.rs",
        "SourceAppendix for FrontmatterExcerpt",
        "renders the authored frontmatter beside a lifecycle diagnostic",
    ),
    allowed(Rule::Seam, "lib/src/diagnostics/snapshot.rs", "DiagnosticCatalog for ClaudineDiagnostics", "Claudine's diagnostic catalog"),
    allowed(
        Rule::Seam,
        "lib/src/diagnostics/snapshot.rs",
        "DiagnosticSnapshotExt for DiagnosticSnapshot",
        "snapshot constructors over Claudine's catalog",
    ),
    allowed(Rule::Seam, "lib/src/diagnostics/snapshot.rs", "DiagnosticCauseExt for DiagnosticCause", "cause constructors over Claudine's catalog"),
    // -- Registration: Claudine's events and checks. --
    allowed(
        Rule::Registration,
        "lib/src/composition/lifecycle/registry.rs",
        "build: EventRegistry",
        "`claudine_registry()`: Claudine's renames, outcome pair, and loop gate",
    ),
    allowed(
        Rule::Registration,
        "lib/src/composition/preflight/checks.rs",
        "plan: PreflightPlan",
        "`claudine_preflight()` and `sequence_preflight()`: Claudine's checks in run order",
    ),
];

#[test]
fn claudine_keeps_no_second_lifecycle_implementation() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // includes above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../../darkmatter/cli/tests/common/source_scan.rs");
    let _ = include_str!("../common/lifecycle_ownership_scan.rs");
    // Followed from the crate root, so a test module a test file declares by
    // `#[path]` is never read as production.
    let area = biscuit_test_harness::manifest_dir!().join("..");
    let (sources, problems) = module_tree_sources(&area, "claudine", &["lib/src/lib.rs".to_string()]);
    assert!(problems.is_empty(), "{problems:#?}");
    assert!(sources.contains_key("lib/src/composition/lifecycle/mod.rs"), "the library's lifecycle adapters were not read");
    let darkmatter = production_sources(&biscuit_test_harness::manifest_dir!().join("../../darkmatter/lib/src/lifecycle"));
    let owned = Owned::read(&darkmatter, &sources[TWIN_SOURCE]);
    assert!(owned.seams.contains("LifecycleRunHost") && owned.types.contains("LifecycleRunGuard"), "Darkmatter's lifecycle source was not read");
    assert!(owned.twins.contains("LifecycleRenamedEvent"), "the twin conversion was not read");

    let violations = violations(&sources, &owned, ALLOWED);

    assert!(violations.is_empty(), "lifecycle ownership violations:\n{}", violations.join("\n"));
}

// -- self-tests: the scan must reject what it guards against ----------------

fn synthetic(files: &[(&str, &str)]) -> BTreeMap<String, String> {
    files.iter().map(|(path, text)| (path.to_string(), String::from_utf8(sanitize(text)).expect("UTF-8"))).collect()
}

fn owned() -> Owned {
    Owned {
        seams: ["LifecycleRunHost".to_string()].into(),
        types: ["LifecycleRunGuard".to_string(), "LifecycleCatchProtocol".to_string()].into(),
        twins: Default::default(),
    }
}

#[test]
fn an_injected_kernel_function_is_reported() {
    let sources = synthetic(&[("cli/src/driver.rs", "pub(crate) fn decide_control(control: &StackControl) -> bool { true }\n")]);

    let violations = violations(&sources, &owned(), &[]);

    assert_eq!(violations.len(), 1, "{violations:#?}");
    assert!(violations[0].contains("Definition `decide_control` in cli/src/driver.rs"), "{violations:#?}");
}

#[test]
fn a_re_export_and_an_allowed_alias_are_not_reported() {
    let sources = synthetic(&[(
        "lib/src/lifecycle.rs",
        "pub use darkmatter::lifecycle::control::decide_control;\n\
         // fn decide_control() in a comment\n\
         const NAME: &str = \"fn decide_control\";\n\
         pub type LifecycleRunGuard<'a> = darkmatter::lifecycle::run_guard::LifecycleRunGuard<'a, Host<'a>>;\n\
         pub trait LifecycleCatchProtocolExt { fn new() -> Self; }\n",
    )]);
    let allowed = [allowed(Rule::Definition, "lib/src/lifecycle.rs", "LifecycleRunGuard", "alias")];

    assert_eq!(violations(&sources, &owned(), &allowed), Vec::<String>::new());
}

#[test]
fn an_allowlist_entry_matching_nothing_is_stale() {
    let allowed = [allowed(Rule::Definition, "lib/src/gone.rs", "LifecycleRunGuard", "alias")];

    let violations = violations(&BTreeMap::new(), &owned(), &allowed);

    assert_eq!(violations.len(), 1, "{violations:#?}");
    assert!(violations[0].starts_with("stale Definition entry"), "{violations:#?}");
}

#[test]
fn a_compared_hop_limit_is_reported_but_a_reported_limit_is_not() {
    let sources = synthetic(&[(
        "cli/src/proxy.rs",
        "fn refuse(chain: &[P]) -> bool { chain.len() >= super::control::MAX_PROXY_HOPS }\n\
         fn report() -> E { E::Cycle { limit: MAX_PROXY_HOPS } }\n",
    )]);

    let violations = violations(&sources, &owned(), &[]);

    assert_eq!(violations.len(), 1, "{violations:#?}");
    assert!(violations[0].starts_with("cli/src/proxy.rs:1: `MAX_PROXY_HOPS` compared"), "{violations:#?}");
}

#[test]
fn a_direct_refusal_error_is_reported_but_a_pattern_is_not() {
    let sources = synthetic(&[(
        "cli/src/dispatch.rs",
        "fn route(path: PathBuf) -> CompositionError {\n\
         \x20   CompositionError::LifecycleResumeWithoutSession { source_path: path }\n\
         }\n\
         fn is_resume(error: &CompositionError) -> bool {\n\
         \x20   matches!(error, CompositionError::LifecycleResumeWithoutSession { .. })\n\
         }\n\
         fn name(error: &CompositionError) -> &str {\n\
         \x20   match error { CompositionError::LifecycleDeferNotImplemented { source_path } => \"defer\", _ => \"\" }\n\
         }\n",
    )]);

    let violations = violations(&sources, &owned(), &[]);

    assert_eq!(violations.len(), 1, "{violations:#?}");
    assert!(violations[0].contains("DecisionError `route: LifecycleResumeWithoutSession`"), "{violations:#?}");
}

#[test]
fn a_parser_twin_built_outside_its_conversion_is_reported() {
    let sources = synthetic(&[
        (
            TWIN_SOURCE,
            "impl From<LifecycleError> for CompositionError {\n\
             \x20   fn from(error: LifecycleError) -> Self {\n\
             \x20       match error { LifecycleError::LifecycleSayConflict(path) => Self::LifecycleSayConflict(path) }\n\
             \x20   }\n\
             }\n",
        ),
        ("lib/src/parse.rs", "fn parse(path: PathBuf) -> CompositionError { CompositionError::LifecycleSayConflict(path) }\n"),
    ]);
    let allowed = [allowed(Rule::ParserTwin, TWIN_SOURCE, "from: *", "the conversion")];
    let owned = Owned::read(&BTreeMap::new(), &sources[TWIN_SOURCE]);

    let violations = violations(&sources, &owned, &allowed);

    assert_eq!(violations.len(), 1, "{violations:#?}");
    assert!(violations[0].contains("ParserTwin `parse: LifecycleSayConflict` in lib/src/parse.rs"), "{violations:#?}");
}

#[test]
fn an_unreviewed_seam_or_extension_is_reported() {
    let sources = synthetic(&[(
        "lib/src/host.rs",
        "impl darkmatter::lifecycle::run_guard::LifecycleRunHost for OtherHost<'_> {}\n\
         impl<'a> GuardExt<'a> for LifecycleRunGuard<'a> {}\n\
         impl std::fmt::Display for OtherHost<'_> {}\n",
    )]);

    let violations = violations(&sources, &owned(), &[]);

    assert_eq!(violations.len(), 2, "{violations:#?}");
    assert!(violations.iter().any(|violation| violation.contains("Seam `LifecycleRunHost for OtherHost`")), "{violations:#?}");
    assert!(violations.iter().any(|violation| violation.contains("Seam `GuardExt for LifecycleRunGuard`")), "{violations:#?}");
}

#[test]
fn a_twin_built_in_another_packages_source_is_reported() {
    // The CLI guard scans only CLI source and reads the twins from the
    // library's conversion, which it does not scan.
    let conversion = String::from_utf8(sanitize(
        "impl From<LifecycleError> for CompositionError {\n\
         \x20   fn from(error: LifecycleError) -> Self {\n\
         \x20       match error { LifecycleError::LifecycleSayConflict(path) => Self::LifecycleSayConflict(path) }\n\
         \x20   }\n\
         }\n",
    ))
    .expect("UTF-8");
    let owned = Owned::read(&BTreeMap::new(), &conversion);
    let sources = synthetic(&[("cli/src/driver.rs", "fn drive(path: PathBuf) -> CompositionError { CompositionError::LifecycleSayConflict(path) }\n")]);

    let violations = violations(&sources, &owned, &[]);

    assert_eq!(violations.len(), 1, "{violations:#?}");
    assert!(violations[0].contains("ParserTwin `drive: LifecycleSayConflict` in cli/src/driver.rs"), "{violations:#?}");
}
