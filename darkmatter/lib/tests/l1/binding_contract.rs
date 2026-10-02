//! The host binding model through Darkmatter's public API: how a root is
//! classified (reserved namespace, registered global, document property), how
//! a host's registration is checked before anything runs, and how passive
//! validation reports what runtime evaluation would reject.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::compose::expression::{
    AuthoredMode, Availability, BindingError, BindingView, EvaluationLookup, EvaluationSession,
    ExpressionError, ParseMode, ResolvedBinding, RootClass, RuntimeBinding, ScopeId,
    UnavailabilityReason,
    evaluate, evaluate_prepared, parse, prepare_value, reserved_root_descriptors,
    validate_prepared,
};
use darkmatter::markdown::compose::{EffectiveState, EffectiveStateBuilder};
use serde_json::{Value, json};

const NO_ERROR: &str = "host.event-has-no-error";
const OUTSIDE_GROUP: &str = "host.outside-group";

fn document(frontmatter: Value) -> EffectiveState {
    let map: HashMap<String, Value> = serde_json::from_value(frontmatter).unwrap();
    EffectiveStateBuilder::new().with_frontmatter(map).build().unwrap()
}

fn reason(code: &str) -> UnavailabilityReason {
    UnavailabilityReason::new(code)
}

/// `err` available, `group` unavailable, `timing` decided at invocation.
fn lifecycle_view() -> Arc<BindingView> {
    Arc::new(
        BindingView::builder(ScopeId::new("finalize"))
            .declare("err", "The error that ended the run.", Availability::Available)
            .declare(
                "group",
                "Variables of the enclosing group.",
                Availability::Unavailable(reason(OUTSIDE_GROUP)),
            )
            .declare("timing", "Durations captured so far.", Availability::ExecutionDependent)
            .build()
            .unwrap(),
    )
}

fn session<'a>(
    view: Arc<BindingView>,
    document: &'a EffectiveState,
    runtime: Vec<(&str, RuntimeBinding<'a>)>,
) -> Result<EvaluationSession<'a>, BindingError> {
    EvaluationSession::associate(
        view,
        document,
        runtime.into_iter().map(|(root, binding)| (root.to_string(), binding)),
    )
}

fn render(text: &str, session: &EvaluationSession<'_>) -> Result<Value, MarkdownError> {
    let prepared = prepare_value(&json!(text), AuthoredMode::InterpolatedValue).unwrap();
    evaluate_prepared(&prepared, session)
}

fn unavailable_root(error: &MarkdownError) -> Option<(&str, &str)> {
    match error {
        MarkdownError::Interpolation { cause, .. } => match cause.as_ref() {
            ExpressionError::Binding(binding) => match binding.as_ref() {
                BindingError::Unavailable(read) => Some((read.root.as_str(), read.reason.code())),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

mod resolution {
    use super::*;

    /// The three outcomes a bare name can have stay distinct: an available
    /// `null` global, an unavailable global, and an absent document property.
    #[test]
    fn available_null_unavailable_and_absent_property_are_distinct() {
        let doc = document(json!({ "group": "document group" }));
        let session = session(
            lifecycle_view(),
            &doc,
            vec![
                ("err", RuntimeBinding::eager(Value::Null)),
                ("group", RuntimeBinding::unavailable(reason(OUTSIDE_GROUP))),
                ("timing", RuntimeBinding::eager(json!({ "total_ms": 5 }))),
            ],
        )
        .unwrap();

        // Available null: a global holding null, falsy, rendering empty.
        assert_eq!(session.resolve("err").unwrap(), ResolvedBinding::Global { value: Value::Null });
        assert_eq!(render("[{{ err }}]", &session).unwrap(), json!("[]"));
        assert_eq!(render("{{ err ? 'failed' : 'clean' }}", &session).unwrap(), json!("clean"));
        assert_eq!(render("{{ err }}", &session).unwrap(), Value::Null);

        // Unavailable: a typed error, never the document's `group`.
        let error = session.resolve("group").unwrap_err();
        assert!(matches!(
            &error,
            ExpressionError::Binding(binding) if matches!(
                binding.as_ref(),
                BindingError::Unavailable(read)
                    if read.root == "group" && read.path == "group"
                        && read.scope.as_str() == "finalize"
                        && read.reason.code() == OUTSIDE_GROUP && read.span.is_none()
            )
        ));
        assert!(error.is_authoring_fatal());
        let rendered = render("in {{ group }}", &session).unwrap_err();
        assert_eq!(unavailable_root(&rendered), Some(("group", OUTSIDE_GROUP)));
        let as_value = render("{{ group }}", &session).unwrap_err();
        assert_eq!(unavailable_root(&as_value), Some(("group", OUTSIDE_GROUP)));
        // A fallback is not a way around an unavailable global.
        let fallback = render("{{ group || 'none' }}", &session).unwrap_err();
        assert_eq!(unavailable_root(&fallback), Some(("group", OUTSIDE_GROUP)));
        // `doc.group` always selects document data.
        assert_eq!(render("{{ doc.group }}", &session).unwrap(), json!("document group"));

        // Absent document property: a document read of nothing, null as a value.
        assert_eq!(session.resolve("missing").unwrap(), ResolvedBinding::Document { value: None });
        assert_eq!(render("{{ missing }}", &session).unwrap(), Value::Null);
        assert_eq!(render("[{{ missing }}]", &session).unwrap(), json!("[]"));
    }

    /// A registered global shadows a same-named document property, and a
    /// missing member stays inside the global.
    #[test]
    fn a_global_shadows_a_document_property_of_the_same_name() {
        let doc = document(json!({ "err": { "message": "from the document", "code": 7 } }));
        let session = session(
            lifecycle_view(),
            &doc,
            vec![
                ("err", RuntimeBinding::eager(json!({ "message": "boom" }))),
                ("group", RuntimeBinding::unavailable(reason(OUTSIDE_GROUP))),
                ("timing", RuntimeBinding::eager(json!({}))),
            ],
        )
        .unwrap();

        assert_eq!(render("{{ err.message }}", &session).unwrap(), json!("boom"));
        assert_eq!(
            session.resolve("err.code").unwrap(),
            ResolvedBinding::Global { value: Value::Null },
            "a member the global lacks must not fall through to the document"
        );
        assert_eq!(render("[{{ err.code }}]", &session).unwrap(), json!("[]"));
        assert_eq!(render("{{ doc.err.message }}", &session).unwrap(), json!("from the document"));
    }

    /// `doc.doc`, `doc.ctx`, and `doc.env` are ordinary document properties.
    #[test]
    fn doc_prefixed_reserved_names_read_document_data() {
        let doc = document(json!({
            "doc": "doc property",
            "ctx": { "note": "ctx property" },
            "env": { "note": "env property" },
        }));
        let session = session(
            lifecycle_view(),
            &doc,
            vec![
                ("err", RuntimeBinding::eager(Value::Null)),
                ("group", RuntimeBinding::unavailable(reason(OUTSIDE_GROUP))),
                ("timing", RuntimeBinding::eager(json!({}))),
            ],
        )
        .unwrap();

        assert_eq!(render("{{ doc.doc }}", &session).unwrap(), json!("doc property"));
        assert_eq!(render("{{ doc.ctx.note }}", &session).unwrap(), json!("ctx property"));
        assert_eq!(render("{{ doc.env.note }}", &session).unwrap(), json!("env property"));
        assert!(matches!(session.resolve("doc.doc").unwrap(), ResolvedBinding::Namespace { .. }));
    }

    /// A lazy global runs once per session, on first read, even when it
    /// yields `null`; a new session runs it again.
    #[test]
    fn a_lazy_global_runs_once_per_session_including_a_null_result() {
        let calls = AtomicUsize::new(0);
        let doc = document(json!({ "title": "t" }));
        let provider = || {
            calls.fetch_add(1, Ordering::SeqCst);
            Value::Null
        };
        let runtime = || {
            vec![
                ("err", RuntimeBinding::lazy(provider)),
                ("group", RuntimeBinding::unavailable(reason(OUTSIDE_GROUP))),
                ("timing", RuntimeBinding::eager(json!({}))),
            ]
        };

        let first = session(lifecycle_view(), &doc, runtime()).unwrap();
        assert_eq!(render("{{ title }}", &first).unwrap(), json!("t"));
        assert_eq!(calls.load(Ordering::SeqCst), 0, "an unreferenced lazy global never runs");

        assert_eq!(render("{{ err }}", &first).unwrap(), Value::Null);
        assert_eq!(render("[{{ err.message }}]", &first).unwrap(), json!("[]"));
        assert_eq!(render("{{ err || 'none' }}", &first).unwrap(), json!("none"));
        assert_eq!(
            evaluate(&parse("err").unwrap(), &first).unwrap(),
            Value::Null
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let second = session(lifecycle_view(), &doc, runtime()).unwrap();
        assert_eq!(render("{{ err }}", &second).unwrap(), Value::Null);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    /// A lookup that implements only `get` keeps working: every root it
    /// answers is a document property.
    #[test]
    fn a_get_only_lookup_resolves_as_document_data() {
        struct GetOnly;
        impl EvaluationLookup for GetOnly {
            fn get(&self, path: &str) -> Option<Value> {
                (path == "name").then(|| json!("Alice"))
            }
        }
        assert_eq!(
            GetOnly.resolve("name").unwrap(),
            ResolvedBinding::Document { value: Some(json!("Alice")) }
        );
        assert_eq!(GetOnly.resolve("other").unwrap(), ResolvedBinding::Document { value: None });
        assert!(GetOnly.binding_view().is_none());
        assert_eq!(GetOnly.get_string("name"), "Alice");
    }

    /// The baseline view a static consumer without host descriptors uses: the
    /// reserved namespaces come from the catalog, `null` is the literal, and
    /// every other bare root — a context-variable name or a lifecycle global
    /// included — is a document property. A host view keeps its globals.
    #[test]
    fn the_baseline_view_reads_every_unreserved_root_as_a_document_property() {
        let baseline = BindingView::baseline();
        assert_eq!(baseline.globals().count(), 0);
        for root in reserved_root_descriptors() {
            assert_eq!(baseline.classify_root(root.name), RootClass::Namespace, "{}", root.name);
            assert!(!baseline.names_document_property(root.name), "{}", root.name);
        }
        assert!(!baseline.names_document_property("null"));
        for root in ["title", "repo", "today", "err", "timing", "group", "spec_name"] {
            assert_eq!(baseline.classify_root(root), RootClass::Document, "{root}");
            assert!(baseline.names_document_property(root), "{root}");
        }

        let host = lifecycle_view();
        assert!(!host.names_document_property("err"));
        assert!(!host.names_document_property("group"));
        assert!(host.names_document_property("title"));
    }
}

mod registration {
    use super::*;

    fn panicking_provider() -> RuntimeBinding<'static> {
        RuntimeBinding::lazy(|| panic!("a lazy provider ran during registration"))
    }

    fn empty_view() -> Arc<BindingView> {
        Arc::new(BindingView::builder(ScopeId::new("start")).build().unwrap())
    }

    /// Every reserved root (from Darkmatter's one table) is rejected as a
    /// declaration and as a runtime entry, before any provider runs.
    #[test]
    fn every_reserved_root_is_rejected_before_any_provider_runs() {
        let names: Vec<&str> = reserved_root_descriptors().iter().map(|root| root.name).collect();
        assert_eq!(names, ["doc", "ctx", "env", "current", "current_env"]);

        let doc = document(json!({}));
        for name in names {
            let declared = BindingView::builder(ScopeId::new("start"))
                .declare(name, "shadow", Availability::Available)
                .build();
            assert_eq!(declared.unwrap_err(), BindingError::ReservedName { root: name.to_string() });

            let associated = session(empty_view(), &doc, vec![(name, panicking_provider())]);
            assert_eq!(
                associated.unwrap_err(),
                BindingError::ReservedName { root: name.to_string() },
                "{name}"
            );
        }
    }

    /// A declared global with no runtime entry fails association, so even a
    /// reference in a branch that would never run is never evaluated.
    #[test]
    fn an_omitted_declared_global_fails_association() {
        let doc = document(json!({ "group": "document group" }));
        let prepared =
            prepare_value(&json!("{{ false ? group : 'unused' }}"), AuthoredMode::InterpolatedValue)
                .unwrap();
        for omitted in ["err", "group", "timing"] {
            let runtime: Vec<_> = vec![
                ("err", RuntimeBinding::eager(Value::Null)),
                ("group", RuntimeBinding::unavailable(reason(OUTSIDE_GROUP))),
                ("timing", RuntimeBinding::eager(json!({}))),
            ]
            .into_iter()
            .filter(|(root, _)| *root != omitted)
            .collect();
            let error = session(lifecycle_view(), &doc, runtime).unwrap_err();
            assert_eq!(
                error,
                BindingError::OmittedDeclaredGlobal {
                    root: omitted.to_string(),
                    scope: ScopeId::new("finalize"),
                }
            );
        }
        // With a complete registration the same prepared value evaluates.
        let complete = session(
            lifecycle_view(),
            &doc,
            vec![
                ("err", RuntimeBinding::eager(Value::Null)),
                ("group", RuntimeBinding::unavailable(reason(OUTSIDE_GROUP))),
                ("timing", RuntimeBinding::eager(json!({}))),
            ],
        )
        .unwrap();
        assert_eq!(evaluate_prepared(&prepared, &complete).unwrap(), json!("unused"));
    }

    /// Association enforces each definite declaration and rejects malformed,
    /// duplicate, or undeclared entries, all without running a provider.
    #[test]
    fn association_rejects_every_inconsistent_registration() {
        let doc = document(json!({}));
        let base = || {
            vec![
                ("err", RuntimeBinding::eager(Value::Null)),
                ("group", RuntimeBinding::unavailable(reason(OUTSIDE_GROUP))),
                ("timing", RuntimeBinding::eager(json!({}))),
            ]
        };
        let replace = |root: &'static str, binding: RuntimeBinding<'static>| {
            let mut runtime = base();
            for entry in &mut runtime {
                if entry.0 == root {
                    entry.1 = binding.clone();
                }
            }
            runtime
        };
        let contradicts = |error: BindingError, expected_root: &str| {
            matches!(error, BindingError::ContradictsDeclaration { ref root, .. } if root == expected_root)
        };

        // Control: the unedited registration associates.
        assert!(session(lifecycle_view(), &doc, base()).is_ok());

        // `available` needs a value.
        let error = session(lifecycle_view(), &doc, replace("err", RuntimeBinding::unavailable(reason(NO_ERROR))))
            .unwrap_err();
        assert!(contradicts(error, "err"));
        // `unavailable` cannot be rebound to data, nor carry another reason.
        let error = session(lifecycle_view(), &doc, replace("group", RuntimeBinding::eager(json!({}))))
            .unwrap_err();
        assert!(contradicts(error, "group"));
        let error = session(lifecycle_view(), &doc, replace("group", panicking_provider())).unwrap_err();
        assert!(contradicts(error, "group"));
        let error = session(
            lifecycle_view(),
            &doc,
            replace("group", RuntimeBinding::unavailable(reason(NO_ERROR))),
        )
        .unwrap_err();
        assert!(contradicts(error, "group"));
        // `execution-dependent` accepts either kind.
        assert!(session(
            lifecycle_view(),
            &doc,
            replace("timing", RuntimeBinding::unavailable(reason(NO_ERROR)))
        )
        .is_ok());

        let mut duplicate = base();
        duplicate.push(("err", RuntimeBinding::eager(json!(1))));
        assert_eq!(
            session(lifecycle_view(), &doc, duplicate).unwrap_err(),
            BindingError::Duplicate { root: "err".to_string() }
        );

        let mut undeclared = base();
        undeclared.push(("extra", panicking_provider()));
        assert_eq!(
            session(lifecycle_view(), &doc, undeclared).unwrap_err(),
            BindingError::UnknownGlobal { root: "extra".to_string(), scope: ScopeId::new("finalize") }
        );

        let mut dotted = base();
        dotted.push(("err.message", panicking_provider()));
        assert_eq!(
            session(lifecycle_view(), &doc, dotted).unwrap_err(),
            BindingError::InvalidRoot { root: "err.message".to_string() }
        );

        let error = session(
            lifecycle_view(),
            &doc,
            replace("timing", RuntimeBinding::unavailable(reason("not-namespaced"))),
        )
        .unwrap_err();
        assert!(matches!(error, BindingError::InvalidReasonCode { ref code, .. } if code == "not-namespaced"));
    }

    /// The declaration side applies the same root and reason rules.
    #[test]
    fn a_view_rejects_invalid_duplicate_and_unnamespaced_declarations() {
        let builder = || BindingView::builder(ScopeId::new("start"));
        assert_eq!(
            builder().declare("a.b", "", Availability::Available).build().unwrap_err(),
            BindingError::InvalidRoot { root: "a.b".to_string() }
        );
        assert_eq!(
            builder().declare("", "", Availability::Available).build().unwrap_err(),
            BindingError::InvalidRoot { root: String::new() }
        );
        assert_eq!(
            builder()
                .declare("err", "", Availability::Available)
                .declare("err", "", Availability::ExecutionDependent)
                .build()
                .unwrap_err(),
            BindingError::Duplicate { root: "err".to_string() }
        );
        for code in ["plain", ".leading", "trailing.", "Upper.case", "a..b"] {
            assert!(
                matches!(
                    builder()
                        .declare("err", "", Availability::Unavailable(reason(code)))
                        .build()
                        .unwrap_err(),
                    BindingError::InvalidReasonCode { .. }
                ),
                "{code}"
            );
        }
        let view = builder()
            .declare("err", "", Availability::Unavailable(reason("claudine.event-has-no-error")))
            .build()
            .unwrap();
        assert_eq!(view.globals().count(), 1);
    }
}

mod passive_validation {
    use super::*;

    fn codes(prepared: &darkmatter::markdown::compose::expression::PreparedValue) -> Vec<String> {
        validate_prepared(prepared, &lifecycle_view())
            .into_iter()
            .map(|diagnostic| match diagnostic.error {
                ExpressionError::Binding(binding) => match *binding {
                    BindingError::Unavailable(read) => {
                        format!("unavailable:{}:{}", read.root, read.reason.code())
                    }
                    other => format!("binding:{other}"),
                },
                ExpressionError::UnknownFunction { name } => {
                    format!("unknown-function:{}", name.lines().next().unwrap_or(""))
                }
                other => format!("other:{other}"),
            })
            .collect()
    }

    /// An unavailable root is reported even in the branch a run never takes,
    /// while the runtime, which only reads the chosen branch, succeeds.
    #[test]
    fn an_unavailable_root_in_an_inactive_branch_fails_validation() {
        let prepared = prepare_value(
            &json!("{{ ready ? 'go' : group.name }}"),
            AuthoredMode::InterpolatedValue,
        )
        .unwrap();
        assert_eq!(codes(&prepared), ["unavailable:group:host.outside-group"]);

        let diagnostics = validate_prepared(&prepared, &lifecycle_view());
        // `group.name` starts at byte 18 of the leaf `{{ ready ? 'go' : group.name }}`.
        assert_eq!(diagnostics[0].pointer, "");
        assert_eq!(diagnostics[0].span, 18..28);

        let doc = document(json!({ "ready": true }));
        let runtime = session(
            lifecycle_view(),
            &doc,
            vec![
                ("err", RuntimeBinding::eager(Value::Null)),
                ("group", RuntimeBinding::unavailable(reason(OUTSIDE_GROUP))),
                ("timing", RuntimeBinding::eager(json!({}))),
            ],
        )
        .unwrap();
        assert_eq!(evaluate_prepared(&prepared, &runtime).unwrap(), json!("go"));
    }

    /// Every unavailable read is located by pointer and leaf span, including
    /// in fallbacks and function arguments; available and execution-dependent
    /// roots, namespaces, and document properties are not reported.
    #[test]
    fn validation_walks_every_position_and_defers_execution_dependent_roots() {
        let prepared = prepare_value(
            &json!({
                "message": "{{ err }} {{ timing.total_ms }} {{ ctx.today }} {{ title }}",
                "steps": ["fine", "{{ missing || length(group) }}"],
                "count": 3,
            }),
            AuthoredMode::Subtree,
        )
        .unwrap();
        let diagnostics = validate_prepared(&prepared, &lifecycle_view());
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].pointer, "/steps/1");
        let leaf = "{{ missing || length(group) }}";
        assert_eq!(&leaf[diagnostics[0].span.clone()], "group");
    }

    /// Unknown functions are reported in every branch; aliases and any letter
    /// case the evaluator accepts are not.
    #[test]
    fn unknown_functions_are_reported_in_every_branch() {
        let prepared = prepare_value(
            &json!("{{ ok ? LENGTH(items) : isEmpty(items) || lenght(items) }}"),
            AuthoredMode::InterpolatedValue,
        )
        .unwrap();
        assert_eq!(codes(&prepared), ["unknown-function:lenght"]);
        let diagnostics = validate_prepared(&prepared, &lifecycle_view());
        let leaf = "{{ ok ? LENGTH(items) : isEmpty(items) || lenght(items) }}";
        assert_eq!(&leaf[diagnostics[0].span.clone()], "lenght(items)");
    }

    /// Preparation and validation read no runtime value: a counting provider
    /// stays at zero until the value is evaluated.
    #[test]
    fn preparation_and_validation_invoke_no_provider() {
        let calls = AtomicUsize::new(0);
        let doc = document(json!({}));
        let runtime = session(
            lifecycle_view(),
            &doc,
            vec![
                ("err", RuntimeBinding::eager(Value::Null)),
                ("group", RuntimeBinding::unavailable(reason(OUTSIDE_GROUP))),
                (
                    "timing",
                    RuntimeBinding::lazy(|| {
                        calls.fetch_add(1, Ordering::SeqCst);
                        json!({ "total_ms": 12 })
                    }),
                ),
            ],
        )
        .unwrap();

        let prepared = prepare_value(&json!("{{ timing.total_ms }}"), AuthoredMode::InterpolatedValue)
            .unwrap();
        assert!(validate_prepared(&prepared, &lifecycle_view()).is_empty());
        assert!(validate_prepared(&prepared, runtime.view()).is_empty());
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        assert_eq!(evaluate_prepared(&prepared, &runtime).unwrap(), json!(12));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    /// The three authored modes: a condition keeps `||` as logical OR and its
    /// typed result, a whole-value span keeps its type, and a subtree keeps
    /// non-string values.
    #[test]
    fn each_authored_mode_evaluates_with_its_own_rules() {
        let doc = document(json!({ "count": 2, "label": "" }));
        let runtime = session(
            lifecycle_view(),
            &doc,
            vec![
                ("err", RuntimeBinding::eager(json!({ "message": "boom" }))),
                ("group", RuntimeBinding::unavailable(reason(OUTSIDE_GROUP))),
                ("timing", RuntimeBinding::eager(json!({}))),
            ],
        )
        .unwrap();

        let condition =
            prepare_value(&json!("label || count > 1"), AuthoredMode::Expression(ParseMode::Condition))
                .unwrap();
        assert_eq!(evaluate_prepared(&condition, &runtime).unwrap(), json!(true));
        let fallback = prepare_value(
            &json!("label || 'unnamed'"),
            AuthoredMode::Expression(ParseMode::Interpolation),
        )
        .unwrap();
        assert_eq!(evaluate_prepared(&fallback, &runtime).unwrap(), json!("unnamed"));

        let typed = prepare_value(&json!("{{ count }}"), AuthoredMode::InterpolatedValue).unwrap();
        assert_eq!(evaluate_prepared(&typed, &runtime).unwrap(), json!(2));

        let tree = prepare_value(
            &json!({ "text": "{{ err.message }} x{{ count }}", "flag": true, "n": [1, "{{ count }}"] }),
            AuthoredMode::Subtree,
        )
        .unwrap();
        assert_eq!(
            evaluate_prepared(&tree, &runtime).unwrap(),
            json!({ "text": "boom x2", "flag": true, "n": [1, 2] })
        );

        let constant = prepare_value(&json!(false), AuthoredMode::Expression(ParseMode::Condition))
            .unwrap();
        assert!(constant.expressions().is_empty());
        assert_eq!(evaluate_prepared(&constant, &runtime).unwrap(), json!(false));
    }

    /// A malformed expression fails preparation with its location, before any
    /// view or session is involved.
    #[test]
    fn a_malformed_expression_fails_preparation_with_its_location() {
        let error = prepare_value(
            &json!({ "ok": "{{ title }}", "bad": ["x", "before {{ a + }} after"] }),
            AuthoredMode::Subtree,
        )
        .unwrap_err();
        assert_eq!(error.pointer, "/bad/1");
        assert_eq!(&"before {{ a + }} after"[error.span.clone()], "{{ a + }}");
        assert!(matches!(*error.cause, ExpressionError::Parse(_)));

        let condition_error =
            prepare_value(&json!("a &&"), AuthoredMode::Expression(ParseMode::Condition)).unwrap_err();
        assert_eq!(condition_error.pointer, "");
    }

    /// Literals and escapes are not expressions, so they are neither parsed
    /// nor validated.
    #[test]
    fn literals_are_not_prepared() {
        let prepared = prepare_value(
            &json!("{{{ group }}} and {{ title }}"),
            AuthoredMode::InterpolatedValue,
        )
        .unwrap();
        assert_eq!(prepared.expressions().len(), 1);
        assert_eq!(prepared.expressions()[0].text(), "title");
        assert!(validate_prepared(&prepared, &lifecycle_view()).is_empty());
    }
}
