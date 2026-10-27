//! The lifecycle binding catalog: which Claudine globals each lifecycle scope
//! declares, and the runtime entries that satisfy those declarations.
//!
//! Claudine owns this policy (spec R4); Darkmatter owns everything else. A
//! scope's [`BindingView`] is handed to Darkmatter's passive validation
//! (`validate_prepared`) and to runtime association (`EvaluationSession`), so
//! prepare time and event time cannot disagree about whether a bare name is a
//! lifecycle global or a document property. Claudine never walks an expression
//! to enforce it.
//!
//! `seq`, `group`, and `loop` are lexical whole roots, declared by name from
//! Claudine's host catalog ([`crate::composition::schema::globals`]), which
//! owns their shapes. Explicit `fm.*`
//! always reads document properties. `seq` is available in every event,
//! including cancellation, and standalone documents expose only its outputs.
//! `group` and `loop` retain typed outside-scope errors. Approval scopes keep
//! late roots unavailable; sequence discovery separately permits typed
//! templates for `seq`, `group`, and `loop`.
//!
//! An event's `err` and `timing` availability is Claudine's event registry's
//! ([`super::registry`]): `err` is unavailable in initialize/start/success/loop
//! and available in blocked/failure/cancelled/finalize, and `timing` is
//! available in every event. Both are unavailable during shell approval. Retired loop and sequence
//! reads are checked on parsed expressions before effects.

mod containers;
mod rename;

pub use containers::{GroupBinding, GroupLegResult, LegOutcome, SequenceBinding};
pub use rename::{reject_removed_lifecycle_reads, reject_removed_sequence_reads, retired_names_present, RETIRED_SEQUENCE_NAMES};

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use darkmatter::markdown::compose::expression::{
    Availability, AuthoredMode, BindingView, Expr, ExpressionError, ExpressionFinder,
    ExpressionLocation, HostGlobals, PreparationError, RuntimeBinding, ScopeId,
    UnavailabilityReason,
    ValidationDiagnostic, parse, parse_condition, prepare_value, static_access_reads, static_variable_reads,
    validate_prepared,
};
use serde_json::{Map, Value};

use super::LifecycleSignal;
use super::context::{LifecycleErrorInfo, LifecycleTiming};
use crate::composition::error::CompositionError;
use crate::composition::looping::LoopAmbient;
use crate::composition::reserved::{LOOP_ROOT, RETIRED_LOOP_NAMES};
use crate::composition::schema::globals::host_global;

/// `err` read where no error can exist.
pub const EVENT_HAS_NO_ERROR: &str = "claudine.event-has-no-error";
/// A late value read while shell bytes are fixed, before any event fires.
pub const PREFLIGHT_UNAVAILABLE: &str = "claudine.preflight-unavailable";
/// `group` read outside any group.
pub const OUTSIDE_GROUP: &str = "claudine.outside-group";
/// `loop` read outside any loop.
pub const OUTSIDE_LOOP: &str = "claudine.outside-loop";

/// A retired `_loop_*` name read by an expression that composition evaluated.
pub const REMOVED_LOOP_NAME: &str = "claudine.removed-loop-name";
/// An undeclared retired sequence root introduced by a later render.
pub const REMOVED_SEQUENCE_NAME: &str = "claudine.removed-sequence-name";

/// The scope of a document's composition (body and compose-time frontmatter).
const COMPOSITION_SCOPE: &str = "claudine.composition";

/// The reserved namespaces whose values exist only once an event fires.
const LATE_NAMESPACES: [&str; 1] = ["current"];

/// One place lifecycle expressions are evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifecycleScope {
    /// A document lifecycle event.
    Event(LifecycleSignal),
    /// A sequence task's `setup:` stack.
    TaskSetup,
    /// A sequence task's `teardown:` stack.
    TaskTeardown,
    /// Fixing a document's lifecycle shell bytes at preparation.
    LifecycleShellApproval,
    /// Fixing every shell command a sequence can run, before its first step.
    SequenceShellApproval,
}

impl LifecycleScope {
    /// Every scope, events in [`LifecycleSignal::ALL`] order first.
    pub const ALL: [Self; 12] = [
        Self::Event(LifecycleSignal::Initialize),
        Self::Event(LifecycleSignal::Start),
        Self::Event(LifecycleSignal::Success),
        Self::Event(LifecycleSignal::Blocked),
        Self::Event(LifecycleSignal::Failure),
        Self::Event(LifecycleSignal::Finalize),
        Self::Event(LifecycleSignal::Loop),
        Self::Event(LifecycleSignal::Cancelled),
        Self::TaskSetup,
        Self::TaskTeardown,
        Self::LifecycleShellApproval,
        Self::SequenceShellApproval,
    ];

    /// The scope's identity, carried by every unavailable-binding error.
    pub fn id(self) -> &'static str {
        match self {
            Self::Event(LifecycleSignal::Initialize) => "claudine.lifecycle.initialize",
            Self::Event(LifecycleSignal::Start) => "claudine.lifecycle.start",
            Self::Event(LifecycleSignal::Success) => "claudine.lifecycle.success",
            Self::Event(LifecycleSignal::Blocked) => "claudine.lifecycle.blocked",
            Self::Event(LifecycleSignal::Failure) => "claudine.lifecycle.failure",
            Self::Event(LifecycleSignal::Finalize) => "claudine.lifecycle.finalize",
            Self::Event(LifecycleSignal::Loop) => "claudine.lifecycle.loop",
            Self::Event(LifecycleSignal::Cancelled) => "claudine.lifecycle.cancelled",
            Self::TaskSetup => "claudine.task.setup",
            Self::TaskTeardown => "claudine.task.teardown",
            Self::LifecycleShellApproval => "claudine.approval.lifecycle-shell",
            Self::SequenceShellApproval => "claudine.approval.sequence-shell",
        }
    }

    fn is_approval(self) -> bool {
        matches!(self, Self::LifecycleShellApproval | Self::SequenceShellApproval)
    }

    /// `err` and `timing` as the scope declares them outside approval: an
    /// event's from Claudine's registry, a task stack's by its phase.
    fn generic_globals(self) -> (Availability, Availability) {
        let Self::Event(signal) = self else {
            let err = if self == Self::TaskTeardown {
                Availability::Available
            } else {
                Availability::Unavailable(reason(EVENT_HAS_NO_ERROR))
            };
            return (err, Availability::Available);
        };
        let registry = super::registry::claudine_registry();
        let id = registry.id_of(signal).expect("Claudine registers an event for every slot");
        let declared = |root: &str| {
            registry
                .globals(id)
                .global(root)
                .map(|global| global.availability().clone())
                .expect("Claudine's event views declare `err` and `timing`")
        };
        (declared("err"), declared("timing"))
    }
}

/// What is known about the enclosing sequence group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupMembership {
    /// Inside an established group.
    Member,
    /// Known to be outside any group.
    Outside,
    /// Not knowable yet: a reusable document checked before invocation.
    Unknown,
}

/// What is known about an enclosing loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMembership {
    /// Inside a loop: its gate, or one iteration's events and shell bytes.
    Member,
    /// Known to be outside any loop.
    Outside,
    /// Not knowable yet: a document checked before it runs.
    Unknown,
}

fn reason(code: &str) -> UnavailabilityReason {
    UnavailabilityReason::new(code)
}

/// The scope's declaration of every lifecycle global, with loop membership
/// not yet known.
pub fn binding_view(scope: LifecycleScope, group: GroupMembership) -> Arc<BindingView> {
    binding_view_in(scope, group, LoopMembership::Unknown)
}

/// The scope's declaration of every lifecycle global.
pub fn binding_view_in(
    scope: LifecycleScope,
    group: GroupMembership,
    in_loop: LoopMembership,
) -> Arc<BindingView> {
    let builder = BindingView::builder(ScopeId::new(scope.id())).declare(
        host_global("seq").name(), "The innermost sequence's values.",
        if scope.is_approval() { Availability::Unavailable(reason(PREFLIGHT_UNAVAILABLE)) }
        else { Availability::Available },
    );
    let loop_availability = if scope == LifecycleScope::SequenceShellApproval {
        Availability::Unavailable(reason(PREFLIGHT_UNAVAILABLE))
    } else {
        match in_loop {
            LoopMembership::Member => Availability::Available,
            LoopMembership::Outside => Availability::Unavailable(reason(OUTSIDE_LOOP)),
            LoopMembership::Unknown => Availability::ExecutionDependent,
        }
    };
    let builder = builder.declare(host_global(LOOP_ROOT).name(), "The enclosing loop's values.", loop_availability);
    let builder = if scope.is_approval() {
        let preflight = || Availability::Unavailable(reason(PREFLIGHT_UNAVAILABLE));
        builder
            .declare("err", "The active failure.", preflight())
            .declare("timing", "Observed durations.", preflight())
            .declare(host_global("group").name(), "The enclosing group's variables.", preflight())
    } else {
        let (err, timing) = scope.generic_globals();
        let group = match group {
            GroupMembership::Member => Availability::Available,
            GroupMembership::Outside => Availability::Unavailable(reason(OUTSIDE_GROUP)),
            GroupMembership::Unknown => Availability::ExecutionDependent,
        };
        builder
            .declare("err", "The active failure; `null` when there is none.", err)
            .declare("timing", "Observed durations.", timing)
            .declare(host_global("group").name(), "The enclosing group's variables.", group)
    };
    Arc::new(
        builder
            .build()
            .expect("the lifecycle catalog declares single, unreserved roots once each"),
    )
}

/// The values one evaluation in a lifecycle scope can read.
#[derive(Debug, Clone, Copy, Default)]
pub struct LifecycleValues<'a> {
    /// The active failure, when the scope has one.
    pub err: Option<&'a LifecycleErrorInfo>,
    /// Durations captured so far.
    pub timing: Option<&'a LifecycleTiming>,
    /// The enclosing group's variables; `None` outside any group.
    pub group: Option<&'a Map<String, Value>>,
    /// The complete innermost sequence root; absent means standalone outputs.
    pub seq: Option<&'a Value>,
}

/// The scope's view and one runtime entry per declared global, outside any
/// loop: [`runtime_bindings_in_loop`] with no loop values.
pub fn runtime_bindings(
    scope: LifecycleScope,
    values: LifecycleValues<'_>,
) -> (Arc<BindingView>, HashMap<String, RuntimeBinding<'static>>) {
    runtime_bindings_in_loop(scope, values, None)
}

/// The scope's view and one runtime entry per declared global.
///
/// Every declared global gets an entry; unavailability is an explicit entry,
/// never an omission. `err` in an error-carrying scope with no error, and
/// `timing` when nothing was measured, are an explicit `null`. `in_loop` is
/// the enclosing loop iteration's values; `None` is known to be outside any
/// loop.
pub fn runtime_bindings_in_loop(
    scope: LifecycleScope,
    values: LifecycleValues<'_>,
    in_loop: Option<&LoopAmbient>,
) -> (Arc<BindingView>, HashMap<String, RuntimeBinding<'static>>) {
    let enclosing = match in_loop {
        Some(ambient) => EnclosingLoop::Iteration(ambient),
        None => EnclosingLoop::Outside,
    };
    runtime_bindings_enclosed(scope, values, enclosing)
}

/// What an evaluation knows about its enclosing loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnclosingLoop<'a> {
    /// Inside one loop iteration, or the gate that follows it.
    Iteration(&'a LoopAmbient),
    /// A loop document prepared before its first iteration (the loop seed,
    /// the bootstrap read, the launch audit): the values are not known yet,
    /// so `loop` is execution-dependent and reads `null`, as the retired
    /// `_loop_*` names did before the loop seeded them.
    Pending,
    /// Known to be outside any loop.
    Outside,
}

impl<'a> EnclosingLoop<'a> {
    /// The iteration's values when there are some; otherwise pending for a
    /// loop document and outside for any other.
    pub fn of(values: Option<&'a LoopAmbient>, loop_document: bool) -> Self {
        match (values, loop_document) {
            (Some(ambient), _) => Self::Iteration(ambient),
            (None, true) => Self::Pending,
            (None, false) => Self::Outside,
        }
    }

    fn membership(self) -> LoopMembership {
        match self {
            Self::Iteration(_) => LoopMembership::Member,
            Self::Pending => LoopMembership::Unknown,
            Self::Outside => LoopMembership::Outside,
        }
    }

    /// The `loop` entry: an iteration's values resolve `loop.<member>`
    /// without building the whole object, which only a whole-root read
    /// builds ([`RuntimeBinding::members`]). The binding owns a clone of the
    /// ambient, whose previous result is shared rather than copied.
    fn binding(self) -> RuntimeBinding<'static> {
        match self {
            Self::Iteration(ambient) => {
                let members = Arc::new(ambient.clone());
                let whole = Arc::clone(&members);
                RuntimeBinding::members(
                    move |name| members.member_named(name),
                    move || whole.to_value(),
                )
            }
            Self::Pending | Self::Outside => RuntimeBinding::eager(Value::Null),
        }
    }
}

/// [`runtime_bindings_in_loop`] for any [`EnclosingLoop`].
pub fn runtime_bindings_enclosed(
    scope: LifecycleScope,
    values: LifecycleValues<'_>,
    enclosing: EnclosingLoop<'_>,
) -> (Arc<BindingView>, HashMap<String, RuntimeBinding<'static>>) {
    let membership = if values.group.is_some() {
        GroupMembership::Member
    } else {
        GroupMembership::Outside
    };
    let view = binding_view_in(scope, membership, enclosing.membership());
    let entries = view
        .globals()
        .map(|global| {
            let entry = match global.availability() {
                Availability::Unavailable(reason) => RuntimeBinding::unavailable(reason.clone()),
                Availability::Available | Availability::ExecutionDependent if global.root() == LOOP_ROOT => {
                    enclosing.binding()
                }
                Availability::Available | Availability::ExecutionDependent => {
                    RuntimeBinding::eager(match global.root() {
                        "seq" => values.seq.cloned().unwrap_or_else(|| SequenceBinding::standalone(Vec::new()).to_value()),
                        "err" => values.err.map_or(Value::Null, LifecycleErrorInfo::to_value),
                        "timing" => values.timing.map_or(Value::Null, LifecycleTiming::to_value),
                        "group" => values
                            .group
                            .map_or(Value::Null, |group| Value::Object(group.clone())),
                        other => unreachable!("undeclared lifecycle global `{other}`"),
                    })
                }
            };
            (global.root().to_string(), entry)
        })
        .collect();
    (view, entries)
}

/// Composition's lexical roots and unavailable retired loop names.
///
/// `loop` follows `enclosing`: the iteration's values, `null` for a loop
/// document before its first iteration ([`EnclosingLoop::Pending`]), and
/// unavailable with [`OUTSIDE_LOOP`] outside any loop. Either way `fm.loop`
/// reads the `loop:` configuration.
///
/// The retired names are unavailable with [`REMOVED_LOOP_NAME`] and a
/// `replacement` parameter, so an expression a later render introduces (a
/// transcluded document) is rejected when it is evaluated; the authored
/// document's own reads were rejected before it composed
/// ([`reject_removed_loop_reads_in_document`]).
pub(crate) fn composition_host_globals(
    enclosing: EnclosingLoop<'_>, seq: Value, group: Option<&Map<String, Value>>,
) -> HostGlobals {
    let (availability, entry) = match enclosing {
        EnclosingLoop::Outside => (
            Availability::Unavailable(reason(OUTSIDE_LOOP)),
            RuntimeBinding::unavailable(reason(OUTSIDE_LOOP)),
        ),
        EnclosingLoop::Iteration(_) => (Availability::Available, enclosing.binding()),
        EnclosingLoop::Pending => (Availability::ExecutionDependent, RuntimeBinding::eager(Value::Null)),
    };
    let retired_reason = |member: &str| {
        reason(REMOVED_LOOP_NAME)
            .with_parameter("replacement", Value::String(format!("{LOOP_ROOT}.{member}")))
    };
    let builder = RETIRED_LOOP_NAMES.iter().fold(
        BindingView::builder(ScopeId::new(COMPOSITION_SCOPE)).declare(
            LOOP_ROOT,
            "The enclosing loop's values.",
            availability,
        ),
        |builder, (removed, member)| {
            builder.declare(
                *removed,
                "A retired loop name.",
                Availability::Unavailable(retired_reason(member)),
            )
        },
    );
    let builder = builder.declare("seq", "The innermost sequence's values.", Availability::Available)
        .declare("group", "The innermost group's values.", if group.is_some() {
            Availability::Available
        } else { Availability::Unavailable(reason(OUTSIDE_GROUP)) });
    let builder = RETIRED_SEQUENCE_NAMES.iter().fold(builder, |builder, (removed, _)| {
        builder.declare(*removed, "A retired sequence name unless the document declares it.", Availability::ExecutionDependent)
    });
    let view = Arc::new(
        builder
            .build()
            .expect("the composition catalog declares single, unreserved roots once each"),
    );
    let entries = [
        (LOOP_ROOT.to_string(), entry),
        ("seq".to_string(), RuntimeBinding::eager(seq)),
        ("group".to_string(), group.map_or_else(|| RuntimeBinding::unavailable(reason(OUTSIDE_GROUP)),
            |value| RuntimeBinding::eager(Value::Object(value.clone())))),
    ].into_iter().chain(
        RETIRED_LOOP_NAMES.iter().map(|(removed, member)| {
            (removed.to_string(), RuntimeBinding::unavailable(retired_reason(member)))
        }),
    );
    let entries = entries.chain(RETIRED_SEQUENCE_NAMES.iter().map(|(removed, replacement)| (
        removed.to_string(), RuntimeBinding::unavailable(reason(REMOVED_SEQUENCE_NAME)
            .with_parameter("replacement", Value::String(replacement.to_string()))),
    )));
    HostGlobals::associate(view, entries)
        .expect("every declared composition global has an agreeing entry")
        .with_document_fallbacks(RETIRED_SEQUENCE_NAMES.iter().map(|(name, _)| *name))
}

/// The removed-grammar error for a composition failure that read a retired
/// `_loop_*` name ([`composition_host_globals`]), if it is one.
///
/// The read was introduced by a render after the document was checked, so it
/// has no authored path in `source_path`; `path` names it as `render`.
pub(crate) fn removed_loop_read_in_composition(
    source_path: &Path,
    error: &darkmatter::markdown::MarkdownError,
) -> Option<CompositionError> {
    let read = error.unavailable_binding()?;
    if read.reason.code() == REMOVED_SEQUENCE_NAME {
        let (_, replacement) = RETIRED_SEQUENCE_NAMES.iter().find(|(name, _)| *name == read.root)?;
        let suffix = read.path.strip_prefix(&read.root).unwrap_or("");
        let replacement = if read.path == "state.sequence_id" { "seq.run_id".to_string() }
            else { format!("{replacement}{suffix}") };
        return Some(CompositionError::RemovedGrammar {
            source_path: source_path.to_path_buf(), path: "render".into(),
            removed: read.path.clone(), replacement: format!("use `{replacement}`"),
        });
    }
    if read.reason.code() != REMOVED_LOOP_NAME {
        return None;
    }
    let retired = retired(&read.root)?;
    Some(removed_loop_grammar(source_path, "render".to_string(), retired))
}

/// Every reason the authored shell text `raw` cannot have its bytes fixed in
/// approval `scope`, in source order.
///
/// Darkmatter's passive validation reports each definitely unavailable global
/// and unknown function in every branch; each read of the late namespace
/// `current` follows as an unavailable binding.
/// Nothing is evaluated and no provider runs.
///
/// ## Errors
///
/// The first expression in `raw` that does not parse.
pub fn approval_diagnostics(
    raw: &str,
    scope: LifecycleScope,
) -> Result<Vec<ValidationDiagnostic>, PreparationError> {
    debug_assert!(scope.is_approval(), "{scope:?} is not an approval scope");
    let view = binding_view(scope, GroupMembership::Outside);
    let prepared = prepare_value(&Value::String(raw.to_string()), AuthoredMode::InterpolatedValue)?;
    let mut diagnostics = validate_prepared(&prepared, &view);
    for expression in prepared.expressions() {
        for read in static_variable_reads(expression.expr()) {
            if LATE_NAMESPACES.contains(&read.root()) {
                diagnostics.push(ValidationDiagnostic {
                    pointer: expression.pointer().to_string(),
                    span: read.span.clone(),
                    error: unavailable(read.root(), read.path, &view),
                });
            }
        }
    }
    Ok(diagnostics)
}

/// The unavailable-binding error for a late namespace read at approval.
fn unavailable(root: &str, path: &str, view: &BindingView) -> ExpressionError {
    use darkmatter::markdown::compose::expression::{BindingError, UnavailableBinding};
    ExpressionError::Binding(Box::new(BindingError::Unavailable(Box::new(UnavailableBinding {
        root: root.to_string(),
        path: path.to_string(),
        scope: view.scope().clone(),
        reason: reason(PREFLIGHT_UNAVAILABLE),
        span: None,
    }))))
}

pub use darkmatter::lifecycle::preflight::unavailable_root;

/// A retired `_loop_*` name and the `loop` member that replaced it.
pub(crate) type RetiredLoopName = (&'static str, &'static str);

fn retired(name: &str) -> Option<RetiredLoopName> {
    RETIRED_LOOP_NAMES.iter().copied().find(|(removed, _)| *removed == name)
}

/// The first retired `_loop_*` name `expr` reads, in source order.
///
/// A read is a variable whose root is a retired name (`_loop_count`,
/// `_loop_count.x`), or an explicit frontmatter read of one whose first
/// member names it (`fm._loop_count`, `fm['_loop_count']`, `(fm)._loop_count`).
/// Reads come from Darkmatter's [`static_access_reads`], so string literals
/// are data, an unrelated property such as `_loop_notes` is not retired, and
/// a dynamically computed key (`fm[name]`) keeps ordinary lookup semantics
/// (its index is read on its own).
pub(crate) fn removed_loop_read(expr: &Expr) -> Option<RetiredLoopName> {
    use darkmatter::markdown::compose::expression::{AccessStep, FRONTMATTER_ROOT};
    static_access_reads(expr).iter().find_map(|read| {
        let mut segments = read.variable.split('.');
        let root = segments.next()?;
        if root != FRONTMATTER_ROOT {
            return retired(root);
        }
        match (segments.next(), read.steps.first()) {
            (Some(member), _) => retired(member),
            (None, Some(AccessStep::Member(name))) => name.split('.').next().and_then(retired),
            (None, Some(AccessStep::Key(key))) => retired(key),
            (None, Some(AccessStep::Index { .. }) | None) => None,
        }
    })
}

/// The first retired `_loop_*` read in the `{{ … }}` spans of authored text.
///
/// Text outside a span is data. A span that does not parse is left to the
/// evaluation that reports it.
pub(crate) fn removed_loop_read_in_text(text: &str) -> Option<RetiredLoopName> {
    first_read_in_spans(&ExpressionFinder::find_all_plain(text)).map(|(_, retired)| retired)
}

fn first_read_in_spans(spans: &[ExpressionLocation]) -> Option<(usize, RetiredLoopName)> {
    spans.iter().find_map(|span| {
        let expr = parse(span.expression.trim()).ok()?;
        removed_loop_read(&expr).map(|retired| (span.start, retired))
    })
}

/// The removed-grammar error for a retired `_loop_*` read at `path`.
pub(crate) fn removed_loop_grammar(
    source_path: &Path,
    path: String,
    (removed, member): RetiredLoopName,
) -> CompositionError {
    CompositionError::RemovedGrammar {
        source_path: source_path.to_path_buf(),
        path,
        removed: removed.to_string(),
        replacement: format!("use `{LOOP_ROOT}.{member}`"),
    }
}

/// The authored text of one document, as the retired `_loop_*` check reads
/// it before it composes.
#[derive(Debug, Clone, Copy)]
pub struct AuthoredDocument<'a> {
    /// The document's authored frontmatter, before composition.
    pub frontmatter: &'a Value,
    /// The document's authored body.
    pub body: &'a str,
    /// The number of file lines before the body's first line.
    pub body_line_offset: usize,
}

/// Rejects a read of a retired `_loop_*` name in a document's authored
/// frontmatter and body, before it composes.
///
/// Checked in order: the loop's `while`/`until` conditions and `action`
/// value templates, every other frontmatter string's `{{ … }}` spans (paths
/// such as `title` or `steps[0].prompt`), and the body's `{{ … }}` spans
/// outside code blocks (path `body:{line}`, the span's line in the file).
/// Lifecycle stacks are parsed first and checked by
/// [`reject_removed_loop_reads_in_lifecycle`]. Only parsed expressions are
/// inspected: plain text, quoted literals, code examples, and unrelated
/// properties such as `_loop_notes` are data.
///
/// ## Errors
///
/// [`CompositionError::RemovedGrammar`] for the first read, naming the
/// `loop` member that replaced it.
pub fn reject_removed_loop_reads_in_document(
    source_path: &Path,
    document: AuthoredDocument<'_>,
) -> Result<(), CompositionError> {
    if let Some((path, retired)) = removed_loop_read_in_frontmatter(document.frontmatter) {
        return Err(removed_loop_grammar(source_path, path, retired));
    }
    let interpolate_code_blocks = document
        .frontmatter
        .get("interpolate_code_blocks")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let spans = if interpolate_code_blocks {
        ExpressionFinder::find_all_plain(document.body)
    } else {
        ExpressionFinder::new(document.body).find_all()
    };
    if let Some((offset, retired)) = first_read_in_spans(&spans) {
        let line = document.body[..offset].matches('\n').count() + 1 + document.body_line_offset;
        return Err(removed_loop_grammar(source_path, format!("body:{line}"), retired));
    }
    Ok(())
}

/// Rejects a read of a retired `_loop_*` name in any lifecycle stack, in
/// [`LifecycleSignal::ALL`] order through both branches, at its authored path
/// (`start[0].info.text`, `success[1].when`, `loop.gate[0].warn.text`).
///
/// ## Errors
///
/// [`CompositionError::RemovedGrammar`] for the first read.
pub fn reject_removed_loop_reads_in_lifecycle(
    source_path: &Path,
    lifecycle: &super::LifecycleConfig,
) -> Result<(), CompositionError> {
    match super::binding_checks::find_removed_loop_read(lifecycle) {
        Some((path, retired)) => Err(removed_loop_grammar(source_path, path.to_string(), retired)),
        None => Ok(()),
    }
}

/// The first retired read in the frontmatter outside the lifecycle stacks,
/// which [`super::binding_checks::find_removed_loop_read`] covers.
fn removed_loop_read_in_frontmatter(frontmatter: &Value) -> Option<(String, RetiredLoopName)> {
    let Value::Object(map) = frontmatter else {
        return None;
    };
    let lifecycle_keys = super::claudine_registry().authored_event_keys();
    for (key, value) in map {
        if key == LOOP_ROOT {
            let Value::Object(loop_block) = value else {
                continue;
            };
            for condition in ["while", "until"] {
                let read = loop_block
                    .get(condition)
                    .and_then(Value::as_str)
                    .and_then(|text| parse_condition(text).ok())
                    .and_then(|expr| removed_loop_read(&expr));
                if let Some(retired) = read {
                    return Some((format!("{LOOP_ROOT}.{condition}"), retired));
                }
            }
            for action_key in ["action", "actions"] {
                if let Some(found) = loop_block
                    .get(action_key)
                    .and_then(|actions| read_in_strings(actions, format!("{LOOP_ROOT}.{action_key}")))
                {
                    return Some(found);
                }
            }
        } else if !lifecycle_keys.contains(&key.as_str())
            && let Some(found) = read_in_strings(value, key.clone())
        {
            return Some(found);
        }
    }
    None
}

/// The first retired read in the `{{ … }}` spans of any string leaf of
/// `value`, named by its path below `path`.
fn read_in_strings(value: &Value, path: String) -> Option<(String, RetiredLoopName)> {
    match value {
        Value::String(text) => removed_loop_read_in_text(text).map(|retired| (path, retired)),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .find_map(|(i, item)| read_in_strings(item, format!("{path}[{i}]"))),
        Value::Object(map) => map
            .iter()
            .find_map(|(key, item)| read_in_strings(item, format!("{path}.{key}"))),
        Value::Null | Value::Bool(_) | Value::Number(_) => None,
    }
}

#[cfg(test)]
mod tests;
