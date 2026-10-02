//! The lifecycle binding matrix: every catalog global in every scope, checked
//! through Darkmatter's passive validation and its runtime session alike.

use darkmatter::markdown::compose::expression::{
    AuthoredMode, BindingError, EvaluationLookup, EvaluationSession, ExpressionError, evaluate,
    parse, prepare_value, validate_prepared,
};
use serde_json::{Map, Value, json};

use super::*;

/// A document whose properties share the globals' names, so any fall-through
/// from a global to the document is visible.
struct Document(Map<String, Value>);

impl Document {
    fn shadowing() -> Self {
        let Value::Object(map) = json!({
            "err": "document err",
            "timing": "document timing",
            "group": "document group",
            "outputs": "document outputs",
        }) else {
            unreachable!()
        };
        Self(map)
    }
}

impl EvaluationLookup for Document {
    fn get(&self, path: &str) -> Option<Value> {
        let path = path.strip_prefix("doc.").unwrap_or(path);
        let mut segments = path.split('.');
        let mut current = self.0.get(segments.next()?)?;
        for segment in segments {
            current = current.get(segment)?;
        }
        Some(current.clone())
    }
}

/// What a scope declares for one global.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Expect {
    /// Available; reading it yields this scope's runtime value.
    Available,
    /// Unavailable with this reason code.
    Unavailable(&'static str),
    /// Execution-dependent: deferred by passive validation.
    Deferred,
    /// Not declared in this scope: a document property.
    Document,
}

/// The ratified matrix (design-contracts C3, narrowed by NR-3/NR-4), with
/// group membership unknown — the passive-check posture.
fn expected(scope: LifecycleScope, root: &str) -> Expect {
    use Expect::*;
    use LifecycleScope as S;
    use LifecycleSignal as E;
    let approval = matches!(scope, S::LifecycleShellApproval | S::SequenceShellApproval);
    match root {
        "err" => match scope {
            S::Event(E::Blocked | E::Failure | E::Finalize) | S::TaskTeardown => Available,
            _ if approval => Unavailable(PREFLIGHT_UNAVAILABLE),
            _ => Unavailable(EVENT_HAS_NO_ERROR),
        },
        "timing" if approval => Unavailable(PREFLIGHT_UNAVAILABLE),
        "timing" => Available,
        "group" if approval => Unavailable(PREFLIGHT_UNAVAILABLE),
        "group" => Deferred,
        "outputs" if scope == S::SequenceShellApproval => Unavailable(PREFLIGHT_UNAVAILABLE),
        "outputs" => Document,
        other => unreachable!("{other}"),
    }
}

const ROOTS: [&str; 4] = ["err", "timing", "group", "outputs"];

#[test]
fn the_matrix_has_every_scope() {
    assert_eq!(LifecycleScope::ALL.len(), 11);
    let ids: std::collections::BTreeSet<_> = LifecycleScope::ALL.iter().map(|s| s.id()).collect();
    assert_eq!(ids.len(), LifecycleScope::ALL.len(), "scope ids are distinct");
}

#[test]
fn passive_validation_follows_the_matrix_in_an_inactive_branch() {
    for scope in LifecycleScope::ALL {
        let view = binding_view(scope, GroupMembership::Unknown);
        for root in ROOTS {
            // The read sits in a branch that never runs: a definite violation
            // must still be found, without evaluating anything.
            let text = format!("{{{{ false ? {root}.code : 'x' }}}}");
            let prepared = prepare_value(&Value::String(text), AuthoredMode::InterpolatedValue)
                .expect("prepares");
            let found: Vec<_> = validate_prepared(&prepared, &view)
                .into_iter()
                .filter_map(|diagnostic| match diagnostic.error {
                    ExpressionError::Binding(binding) => match *binding {
                        BindingError::Unavailable(read) => Some((read.root, read.reason.code().to_string())),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();
            match expected(scope, root) {
                Expect::Unavailable(code) => assert_eq!(
                    found,
                    vec![(root.to_string(), code.to_string())],
                    "{scope:?}: `{root}`"
                ),
                Expect::Available | Expect::Deferred | Expect::Document => {
                    assert!(found.is_empty(), "{scope:?}: `{root}` reported {found:?}");
                }
            }
        }
    }
}

#[test]
fn runtime_resolution_follows_the_matrix_and_never_reads_the_document() {
    let err = LifecycleErrorInfo::from_action_failure("shell", "exit 2");
    let timing = LifecycleTiming {
        document_ms: Some(7),
        total_ms: None,
        step_ms: None,
    };
    let group = Map::from_iter([("label".to_string(), json!("g"))]);
    let document = Document::shadowing();
    for scope in LifecycleScope::ALL {
        for membership in [None, Some(&group)] {
            let values = LifecycleValues {
                err: Some(&err),
                timing: Some(&timing),
                group: membership,
            };
            let (view, entries) = runtime_bindings(scope, values);
            let session = EvaluationSession::associate(view, &document, entries)
                .expect("the catalog's entries satisfy its own view");
            for root in ROOTS {
                let result = evaluate(&parse(root).unwrap(), &session);
                let expect = match (expected(scope, root), membership) {
                    (Expect::Deferred, Some(_)) => Expect::Available,
                    (Expect::Deferred, None) => Expect::Unavailable(OUTSIDE_GROUP),
                    (other, _) => other,
                };
                match expect {
                    Expect::Available => {
                        let value = result.unwrap_or_else(|e| panic!("{scope:?} `{root}`: {e}"));
                        let wanted = match root {
                            "err" => err.to_value(),
                            "timing" => timing.to_value(),
                            "group" => Value::Object(group.clone()),
                            _ => unreachable!(),
                        };
                        assert_eq!(value, wanted, "{scope:?}: `{root}` is the runtime value");
                    }
                    Expect::Unavailable(code) => {
                        let error = result.expect_err(&format!("{scope:?}: `{root}` is unavailable"));
                        let ExpressionError::Binding(binding) = error else {
                            panic!("{scope:?} `{root}`: expected a binding error, got {error:?}");
                        };
                        let BindingError::Unavailable(read) = *binding else {
                            panic!("{scope:?} `{root}`: expected an unavailable read");
                        };
                        assert_eq!(read.reason.code(), code, "{scope:?}: `{root}`");
                        assert_eq!(read.scope.as_str(), scope.id());
                    }
                    Expect::Document => {
                        assert_eq!(result.unwrap(), json!(format!("document {root}")));
                    }
                    Expect::Deferred => unreachable!(),
                }
                // `doc.<root>` always reads the document, whatever the global.
                assert_eq!(
                    evaluate(&parse(&format!("doc.{root}")).unwrap(), &session).unwrap(),
                    json!(format!("document {root}")),
                    "{scope:?}: `doc.{root}`"
                );
            }
        }
    }
}

#[test]
fn err_is_an_explicit_null_where_an_error_may_be_absent() {
    let document = Document::shadowing();
    for scope in [
        LifecycleScope::Event(LifecycleSignal::Finalize),
        LifecycleScope::TaskTeardown,
    ] {
        let (view, entries) = runtime_bindings(scope, LifecycleValues::default());
        let session = EvaluationSession::associate(view, &document, entries).unwrap();
        assert_eq!(
            evaluate(&parse("err").unwrap(), &session).unwrap(),
            Value::Null,
            "{scope:?}: no failure is `null`, never the document's `err`"
        );
        assert_eq!(evaluate(&parse("err.code").unwrap(), &session).unwrap(), Value::Null);
    }
}

#[test]
fn an_omitted_entry_fails_association_but_an_explicit_unavailable_entry_does_not() {
    let document = Document::shadowing();
    let scope = LifecycleScope::Event(LifecycleSignal::Start);
    let (view, mut entries) = runtime_bindings(scope, LifecycleValues::default());
    assert!(matches!(entries["err"], RuntimeBinding::Unavailable(_)));
    assert!(EvaluationSession::associate(view.clone(), &document, entries.clone()).is_ok());

    entries.remove("err");
    let error = EvaluationSession::associate(view, &document, entries)
        .expect_err("unavailability is an entry, never an omission");
    assert!(
        matches!(&error, BindingError::OmittedDeclaredGlobal { root, .. } if root == "err"),
        "{error:?}"
    );
}

#[test]
fn approval_diagnostics_name_late_globals_and_namespaces_in_any_branch() {
    for (scope, raw, root) in [
        (LifecycleScope::LifecycleShellApproval, "echo {{ err.msg }}", "err"),
        (LifecycleScope::LifecycleShellApproval, "echo {{ false ? timing : 'x' }}", "timing"),
        (LifecycleScope::LifecycleShellApproval, "echo {{ group.label }}", "group"),
        (LifecycleScope::LifecycleShellApproval, "echo {{ current.branch }}", "current"),
        (LifecycleScope::SequenceShellApproval, "echo {{ outputs[0] }}", "outputs"),
        (LifecycleScope::SequenceShellApproval, "echo {{ x || current_env.HOME }}", "current_env"),
    ] {
        let diagnostics = approval_diagnostics(raw, scope).expect("parses");
        let roots: Vec<_> = diagnostics.iter().filter_map(|d| unavailable_root(&d.error)).collect();
        assert_eq!(roots, vec![root], "{scope:?}: `{raw}`");
    }
    // Document data, `doc.*`, and namespaces with values at approval pass.
    for raw in ["echo {{ doc.err }} {{ ctx.repo }} {{ env.HOME }} {{ plan }}"] {
        for scope in [LifecycleScope::LifecycleShellApproval, LifecycleScope::SequenceShellApproval] {
            assert!(approval_diagnostics(raw, scope).unwrap().is_empty(), "{scope:?}: `{raw}`");
        }
    }
    // `outputs` is only refused across a whole sequence.
    assert!(approval_diagnostics("echo {{ outputs }}", LifecycleScope::LifecycleShellApproval)
        .unwrap()
        .is_empty());
}
