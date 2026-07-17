//! Discovery-registry and effective-selection tests.
//!
//! The probe types exist because the production registry is a closed downcast
//! list and every Claudine diagnostic that can terminate a chain today is
//! `Semantic`. The deepest-transparent rule and the guards' keep-the-best
//! behavior are therefore unreachable through [`as_diagnostic`] until a
//! transparent leaf type ships — but they are contract, so they are tested
//! through the same walk with a probe registry.

use std::fmt;
use std::path::PathBuf;

use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::ErrorHeader;
use biscuit_terminal::errors::StatusBlockExt;
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::MarkdownError;

use super::*;
use crate::diagnostics::{Category, Disposition, Origin};

fn file_reference_error() -> biscuit_file::FileReferenceError {
    biscuit_file::FileReferenceError::InvalidSyntax("@@bad".to_string())
}

fn erase<'a>(error: &'a (dyn StdError + 'static)) -> &'a (dyn StdError + 'static) {
    error
}

// -- as_diagnostic: the registry ------------------------------------------

#[test]
fn every_registered_diagnostic_is_discoverable_after_erasure() {
    // The registry baseline: the three production `impl Diagnostic for …`
    // types must survive erasure to `dyn Error`. A fourth impl landing without
    // a registry entry is caught by the source-parity test, not this one.
    let composition = CompositionError::FileNotFound("missing.md".to_string());
    let claudine = ClaudineError::ProviderNotAvailable("codex".to_string());
    let harness = HarnessError::RepoRootRequired {
        path: "@/x".to_string(),
    };

    let cases: Vec<(&(dyn StdError + 'static), &str)> = vec![
        (&composition, composition.code()),
        (&claudine, claudine.code()),
        (&harness, harness.code()),
    ];

    for (erased, expected_code) in cases {
        let found = as_diagnostic(erased)
            .unwrap_or_else(|| panic!("`{erased}` is not in the discovery registry"));
        assert_eq!(found.code(), expected_code);
    }
}

#[test]
fn unregistered_error_is_not_discoverable() {
    let err = MarkdownError::Transform("not a claudine diagnostic".to_string());
    assert!(as_diagnostic(erase(&err)).is_none());
}

// -- selection: roles ------------------------------------------------------

#[test]
fn semantic_diagnostic_wins_over_its_lower_layer_cause() {
    // Claudine → biscuit-file. The authoring error owns the identity; the
    // filesystem cause must not steal it.
    let err = CompositionError::InvalidReference {
        reference: "@@bad".to_string(),
        source: file_reference_error(),
    };

    let selected = select_effective_diagnostic(erase(&err)).expect("a diagnostic is selected");

    assert_eq!(
        selected.diagnostic().map(|d| d.code()),
        Some("composition.invalid_file_reference")
    );
    assert_eq!(selected.diagnostic().map(|d| d.category()), Some(Category::Composition));
    assert_eq!(selected.diagnostic().map(|d| d.origin()), Some(Origin::Author));
}

#[test]
fn transparent_wrapper_does_not_shadow_its_inner_diagnostic() {
    // Claudine (transparent) → Claudine (semantic) → biscuit-file.
    let inner = CompositionError::InvalidReference {
        reference: "@@bad".to_string(),
        source: file_reference_error(),
    };
    let wrapped = CompositionError::LifecycleEvaluationAlreadyEmitted {
        inner: Box::new(inner),
    };

    let selected = select_effective_diagnostic(erase(&wrapped)).expect("a diagnostic is selected");

    // The wrapper is skipped even though it is itself registered: without the
    // role it would answer `composition.failed` for a file-reference failure.
    assert_eq!(
        selected.diagnostic().map(|d| d.code()),
        Some("composition.invalid_file_reference")
    );
    let selected_ptr = selected.diagnostic().unwrap() as *const dyn Diagnostic as *const ();
    let wrapper_ptr = &wrapped as *const CompositionError as *const ();
    assert_ne!(
        selected_ptr, wrapper_ptr,
        "selection returned the transparent wrapper itself"
    );
}

#[test]
fn selection_renders_and_classifies_the_same_value() {
    // One value answers both questions — the render target cannot drift from
    // the classified cause.
    let inner = CompositionError::FileNotFound("missing.md".to_string());
    let wrapped = CompositionError::LifecycleEvaluationAlreadyEmitted {
        inner: Box::new(inner),
    };
    let selected = select_effective_diagnostic(erase(&wrapped)).expect("a diagnostic is selected");

    let rendered = selected.block_error().report_block_error_optimistic(Some(120));
    assert!(rendered.contains("missing.md"), "rendered: {rendered}");
    assert_eq!(
        selected.diagnostic().map(|d| d.code()),
        Some("composition.invalid_file_reference")
    );
}

#[test]
fn lower_layer_cause_is_selected_when_no_claudine_diagnostic_is_present() {
    // A bare Darkmatter error still renders: `as_block_error` composes with the
    // Claudine registry rather than being replaced by it.
    let err = MarkdownError::Transform("bare darkmatter error".to_string());
    let selected = select_effective_diagnostic(erase(&err)).expect("a block error is selected");

    assert!(selected.diagnostic().is_none(), "darkmatter carries no facets");
    assert!(
        selected
            .block_error()
            .report_block_error_optimistic(Some(120))
            .contains("bare darkmatter error")
    );
}

#[test]
fn nothing_is_selected_from_an_unstructured_chain() {
    // The generic fallback path stays reachable for genuinely untyped errors.
    let err = Link::new("plain failure", None);
    assert!(select_effective_diagnostic(erase(&err)).is_none());
}

#[test]
fn deepest_transparent_diagnostic_wins_when_none_is_semantic() {
    let deepest = Probe::new("deepest", DiagnosticRole::Transparent, None);
    let middle = Probe::new("middle", DiagnosticRole::Transparent, Some(Box::new(deepest)));
    let outer = Probe::new("outer", DiagnosticRole::Transparent, Some(Box::new(middle)));

    let selected = select_with(erase(&outer), probe_registry).expect("a diagnostic is selected");

    assert_eq!(probe_name(selected), "deepest");
}

#[test]
fn first_semantic_diagnostic_stops_the_walk() {
    let deepest = Probe::new("deepest", DiagnosticRole::Transparent, None);
    let middle = Probe::new("middle", DiagnosticRole::Semantic, Some(Box::new(deepest)));
    let outer = Probe::new("outer", DiagnosticRole::Transparent, Some(Box::new(middle)));

    let selected = select_with(erase(&outer), probe_registry).expect("a diagnostic is selected");

    assert_eq!(probe_name(selected), "middle");
}

// -- selection: traversal guards ------------------------------------------

#[test]
fn a_cyclic_source_chain_terminates_and_keeps_the_best_candidate() {
    // `SelfCycle::source` returns itself, which would spin forever without the
    // identity guard. The transparent probe above it is already the best
    // candidate and must survive the stop.
    let outer = Probe::new(
        "outer",
        DiagnosticRole::Transparent,
        Some(Box::new(SelfCycle)),
    );

    let selected = select_with(erase(&outer), probe_registry).expect("a diagnostic is selected");

    assert_eq!(probe_name(selected), "outer");
}

#[test]
fn a_cyclic_chain_with_no_candidate_terminates_empty() {
    assert!(select_effective_diagnostic(erase(&SelfCycle)).is_none());
}

#[test]
fn an_over_depth_chain_terminates_and_keeps_the_best_candidate() {
    // `beyond` is deeper than `candidate`, so deepest-transparent would pick it
    // — but it sits past the cap. The walk stops and keeps what it had.
    let mut chain: Box<dyn StdError + Send + Sync + 'static> = Box::new(Probe::new(
        "beyond",
        DiagnosticRole::Transparent,
        None,
    ));
    for i in 0..(MAX_SELECTION_DEPTH * 3) {
        chain = Box::new(Link::new(&format!("link {i}"), Some(chain)));
    }
    let root = Probe::new("candidate", DiagnosticRole::Transparent, Some(chain));

    let selected = select_with(erase(&root), probe_registry).expect("a diagnostic is selected");

    assert_eq!(probe_name(selected), "candidate");
}

// -- one-level cause projection --------------------------------------------

#[test]
fn the_next_registered_cause_is_the_diagnostic_below_the_primary() {
    let wrapped = CompositionError::LifecycleEvaluationAlreadyEmitted {
        inner: Box::new(CompositionError::FileNotFound("missing.md".to_string())),
    };
    let primary = as_diagnostic(erase(&wrapped)).expect("the wrapper is registered");

    assert_eq!(
        next_registered_cause(primary).map(|c| c.code()),
        Some("composition.invalid_file_reference")
    );
}

#[test]
fn a_chain_ending_in_an_unregistered_error_has_no_registered_cause() {
    // biscuit-file's error is a real typed source, but it is not a Claudine
    // diagnostic — so there is nothing for `err.cause.*` to project.
    let err = CompositionError::InvalidReference {
        reference: "@@bad".to_string(),
        source: file_reference_error(),
    };
    let primary = as_diagnostic(erase(&err)).unwrap();

    assert!(StdError::source(&err).is_some(), "test premise: a source exists");
    assert!(next_registered_cause(primary).is_none());
}

#[test]
fn the_cause_walk_passes_through_unregistered_links() {
    // "Next *registered* cause" — prose links in between are stepped over
    // rather than terminating the projection.
    let buried = Probe::new("buried", DiagnosticRole::Semantic, None);
    let link = Link::new("untyped context", Some(Box::new(buried)));
    let outer = Probe::new("outer", DiagnosticRole::Semantic, Some(Box::new(link)));

    let found = next_registered_cause_with(&outer, probe_registry).expect("a cause is found");

    assert_eq!(found.to_string(), "buried");
}

#[test]
fn the_cause_walk_terminates_on_a_cyclic_chain() {
    let outer = Probe::new(
        "outer",
        DiagnosticRole::Semantic,
        Some(Box::new(SelfCycle)),
    );
    assert!(next_registered_cause_with(&outer, probe_registry).is_none());
}

#[test]
fn the_cause_walk_terminates_on_an_over_depth_chain() {
    // The cause sits past the depth cap: the projection stops empty rather
    // than walking a pathological chain to find it.
    let mut chain: Box<dyn StdError + Send + Sync + 'static> =
        Box::new(Probe::new("beyond", DiagnosticRole::Semantic, None));
    for i in 0..(MAX_SELECTION_DEPTH * 2) {
        chain = Box::new(Link::new(&format!("link {i}"), Some(chain)));
    }
    let outer = Probe::new("outer", DiagnosticRole::Semantic, Some(chain));

    assert!(next_registered_cause_with(&outer, probe_registry).is_none());
}

// -- probes ----------------------------------------------------------------

/// A plain untyped error with an optional cause — chain filler.
#[derive(Debug)]
struct Link {
    message: String,
    source: Option<Box<dyn StdError + Send + Sync + 'static>>,
}

impl Link {
    fn new(message: &str, source: Option<Box<dyn StdError + Send + Sync + 'static>>) -> Self {
        Self {
            message: message.to_string(),
            source,
        }
    }
}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl StdError for Link {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_ref().map(|s| s.as_ref() as &(dyn StdError + 'static))
    }
}

/// An error whose `source` is itself.
#[derive(Debug)]
struct SelfCycle;

impl fmt::Display for SelfCycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("self-referential error")
    }
}

impl StdError for SelfCycle {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self)
    }
}

/// A `Diagnostic` with a caller-chosen role — the shapes the closed production
/// registry cannot express yet.
#[derive(Debug)]
struct Probe {
    name: String,
    role: DiagnosticRole,
    source: Option<Box<dyn StdError + Send + Sync + 'static>>,
}

impl Probe {
    fn new(
        name: &str,
        role: DiagnosticRole,
        source: Option<Box<dyn StdError + Send + Sync + 'static>>,
    ) -> Self {
        Self {
            name: name.to_string(),
            role,
            source,
        }
    }
}

impl fmt::Display for Probe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl StdError for Probe {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_ref().map(|s| s.as_ref() as &(dyn StdError + 'static))
    }
}

impl BlockError for Probe {
    fn status_block(&self, _term: &Terminal) -> StatusBlock {
        StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new("Probe", "probe"))
            .body(self.name.clone())
    }
}

impl Diagnostic for Probe {
    fn category(&self) -> Category {
        Category::Internal
    }

    fn code(&self) -> &'static str {
        "internal.invariant"
    }

    fn disposition(&self) -> Disposition {
        Disposition::Unrecoverable
    }

    fn origin(&self) -> Origin {
        Origin::Internal
    }

    fn role(&self) -> DiagnosticRole {
        self.role
    }
}

/// A registry that sees the probes plus everything production sees.
fn probe_registry<'a>(
    error: &'a (dyn StdError + 'static),
) -> Option<&'a (dyn Diagnostic + 'static)> {
    if let Some(probe) = error.downcast_ref::<Probe>() {
        return Some(probe);
    }
    as_diagnostic(error)
}

fn probe_name(selected: EffectiveDiagnostic<'_>) -> String {
    selected
        .diagnostic()
        .expect("probe selections carry facets")
        .to_string()
}

// -- role defaults ---------------------------------------------------------

#[test]
fn role_defaults_to_semantic() {
    // Owning the classification is the norm; delegating is the deliberate act.
    let err = HarnessError::PathResolutionFailed {
        raw: "@/missing".to_string(),
        detail: "no such file".to_string(),
    };
    assert_eq!(err.role(), DiagnosticRole::Semantic);
}

#[test]
fn only_the_delegating_composition_wrappers_are_transparent() {
    let inner = || Box::new(CompositionError::FileNotFound("missing.md".to_string()));

    assert_eq!(
        CompositionError::LifecycleEvaluationAlreadyEmitted { inner: inner() }.role(),
        DiagnosticRole::Transparent
    );
    // A variant that carries a typed Darkmatter cause still owns its code: the
    // cause has no facets to delegate to.
    assert_eq!(
        CompositionError::FrontmatterParse(MarkdownError::Transform("x".to_string())).role(),
        DiagnosticRole::Semantic
    );
    assert_eq!(
        CompositionError::SchemaLoad {
            source_path: PathBuf::from("p.md"),
            message: "missing".to_string(),
        }
        .role(),
        DiagnosticRole::Semantic
    );
}

#[test]
fn transparent_wrappers_expose_their_inner_as_the_diagnostic_source() {
    // `thiserror` gives these boxed wrappers no `source()`; without the
    // override the walk would stop at the wrapper and never reach `inner`.
    let wrapped = CompositionError::LifecycleEvaluationAlreadyEmitted {
        inner: Box::new(CompositionError::FileNotFound("missing.md".to_string())),
    };
    assert!(StdError::source(&wrapped).is_none());
    assert_eq!(
        wrapped.diagnostic_source().map(|s| s.to_string()),
        Some("file not found: missing.md".to_string())
    );
}
