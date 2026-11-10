//! Slot admission, evaluation, and the host, in-process.
//!
//! Admission is judged on hand-built slots. Evaluation runs through the real
//! resolver: a schema file on disk is prepared with `simplified-schema`'s
//! public entry point, its pending handle is resolved through
//! [`DarkmatterSlotHost`], and the resulting concrete schema is inspected.

use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Value, json};
use simplified_schema::SchemaError;
use simplified_schema::simplified::Slot;
use simplified_schema::slots::{PendingSchema, SlotHost, SlotResolutionError, SlotRunRequest};

use super::codes::*;
use super::evaluator::{ContextInput, SlotEvaluator};
use super::host::DarkmatterSlotHost;
use crate::markdown::compose::DataRuntime;
use crate::markdown::compose::expression::ExpressionError;
use crate::markdown::compose::expression::data::ObservationKind;
use crate::markdown::compose::expression::functions::SlotClass;
use crate::markdown::compose::expression::iteration::IterationLimits;
use crate::markdown::compose::expression::read_gate::{GateRefusal, ReadGate, ReadTarget};
use crate::markdown::compose::report_diagnostics::slot_resolution_diagnostics;

#[derive(Debug)]
struct AllowAll;

impl ReadGate for AllowAll {
    fn check(&self, _target: &ReadTarget) -> Result<(), GateRefusal> {
        Ok(())
    }
}

/// A slot whose text is `{{ expression }}` at the start of its source.
fn slot(expression: &str) -> Slot {
    Slot {
        expression: expression.to_string(),
        span: 0..expression.len() + 6,
        expression_span: 3..3 + expression.len(),
        origin: None,
    }
}

fn host() -> DarkmatterSlotHost {
    DarkmatterSlotHost::new(SlotEvaluator::new(DataRuntime::new()))
}

fn reading_host() -> DarkmatterSlotHost {
    DarkmatterSlotHost::new(SlotEvaluator::new(DataRuntime::new()).with_gate(Arc::new(AllowAll)))
}

mod admission {
    use super::*;

    fn refusal(expression: &str) -> (Option<&'static str>, String, std::ops::Range<usize>) {
        let error = host().admit(&slot(expression)).expect_err(expression);
        (error.code, error.message, error.span.expect("a refused part"))
    }

    #[test]
    fn document_bindings_and_identity_are_refused_at_their_read() {
        for (expression, refused) in [
            ("fm.x", "fm.x"),
            ("fm['x']", "fm['x']"),
            ("title", "title"),
            ("loop.index", "loop.index"),
            ("ctx.cwd", "ctx.cwd"),
            ("ctx.self", "ctx.self"),
            ("ctx.last_updated", "ctx.last_updated"),
            ("ctx.hash", "ctx.hash"),
            ("ctx.id", "ctx.id"),
            ("ctx.sid", "ctx.sid"),
            ("ctx['cwd']", "ctx['cwd']"),
            ("ctx", "ctx"),
            ("current.today", "current.today"),
            ("local.x", "local.x"),
        ] {
            let (code, message, span) = refusal(expression);
            assert_eq!(code, Some(SLOT_FORBIDDEN_BINDING_CODE), "{expression}: {message}");
            let start = expression.find(refused).unwrap() + 3;
            assert_eq!(span, start..start + refused.len(), "{expression}: the span is the read, in slot coordinates");
        }
    }

    #[test]
    fn a_computed_ctx_key_is_refused() {
        let (code, _, span) = refusal("ctx[env.KEY]");
        assert_eq!((code, span), (Some(SLOT_FORBIDDEN_BINDING_CODE), 3..6));
    }

    #[test]
    fn every_forbidden_function_is_refused_at_its_call() {
        for call in [
            "as_markdown('# x')",
            "pr(1)",
            "pr_list(1)",
            "cicd()",
            "cicd_list(1)",
            "ping('localhost')",
            "ping_under('localhost', 10)",
            "has_alias('x')",
            "has_builtin_function('x')",
            "has_user_function('x')",
            "can_execute('x')",
            "AS_MARKDOWN('# x')",
        ] {
            let (code, message, span) = refusal(call);
            assert_eq!(code, Some(SLOT_FORBIDDEN_FUNCTION_CODE), "{call}: {message}");
            assert_eq!(span, 3..3 + call.len(), "{call}");
        }
    }

    #[test]
    fn a_forbidden_read_or_call_in_a_branch_that_would_not_run_is_still_refused() {
        for (expression, code) in [
            ("true ? 1 : fm.x", SLOT_FORBIDDEN_BINDING_CODE),
            ("false ? title : 2", SLOT_FORBIDDEN_BINDING_CODE),
            ("env.HOME || title", SLOT_FORBIDDEN_BINDING_CODE),
            ("'a' || ctx.cwd", SLOT_FORBIDDEN_BINDING_CODE),
            ("true ? 1 : as_markdown('x')", SLOT_FORBIDDEN_FUNCTION_CODE),
            ("env.HOME || can_execute('x')", SLOT_FORBIDDEN_FUNCTION_CODE),
            ("true && fm.flag", SLOT_FORBIDDEN_BINDING_CODE),
            ("map([1], x => fm.y)", SLOT_FORBIDDEN_BINDING_CODE),
            ("map([1], x => pr(x))", SLOT_FORBIDDEN_FUNCTION_CODE),
        ] {
            assert_eq!(refusal(expression).0, Some(code), "{expression}");
        }
    }

    #[test]
    fn the_schemas_own_context_is_admitted() {
        for expression in [
            "ctx.today",
            "ctx['today']",
            "env.HOME",
            "env",
            "env[ctx.today]",
            "null",
            "true",
            "load_data('os.yaml', 'string[]')",
            "map(load_data('os.yaml', 'string[]'), os => lower(os))",
            "map([1, 2], x => current.today)",
            "map([1, 2], item => local.item)",
            "filter(['a'], { name } => name)",
            "is_today(ctx.today) ? ['a'] : ['b']",
            "recent_commits(1)",
            "not_a_function()",
        ] {
            host().admit(&slot(expression)).unwrap_or_else(|error| panic!("{expression}: {error}"));
        }
    }

    #[test]
    fn a_slot_that_does_not_parse_is_a_positioned_syntax_refusal() {
        let error = host().admit(&slot("load_data(")).unwrap_err();
        assert_eq!(error.code, Some(SLOT_SYNTAX_CODE));
        let span = error.span.unwrap();
        assert!(span.start >= 3 && span.start <= 3 + "load_data(".len(), "{span:?}");
    }

    #[test]
    fn the_class_report_joins_every_reachable_call() {
        use super::super::admission::slot_class_report;
        let class = |expression: &str| {
            slot_class_report(&simplified_schema::expression::parse_spanned(expression).unwrap()).class
        };
        assert_eq!(class("lower('A')"), SlotClass::Pure);
        assert_eq!(class("ctx.today"), SlotClass::Pure);
        assert_eq!(class("is_today('2024-01-01')"), SlotClass::Captured);
        assert_eq!(class("x || load_data('a.yaml', 'string[]')"), SlotClass::InputReading);
        assert_eq!(class("true ? lower('a') : recent_commits(1)"), SlotClass::Volatile);
        assert_eq!(class("map([1], x => file_exists('a'))"), SlotClass::InputReading);
    }
}

/// A temporary tree with `files` written under it, and its canonical root.
fn tree(files: &[(&str, &str)]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    for (path, text) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    (dir, root)
}

fn context(root: &Path) -> biscuit_file::FileResolutionContext {
    biscuit_file::FileResolutionContext::new(root).with_env(std::collections::HashMap::new())
}

/// The pending handle of the schema file at `schema`, named by reference
/// the way a document's `$schema: ./file` names it.
fn pending(root: &Path, schema: &str) -> PendingSchema {
    let reference = serde_yaml_ng::Value::String(format!("./{schema}"));
    match simplified_schema::resolve::resolve_yaml_schema(
        &reference,
        root,
        &context(root),
        crate::markdown::schemas::embedded::SHIPPED_SOURCES,
    ) {
        Err(SchemaError::UnresolvedSlots(unresolved)) => unresolved.pending.expect("an entry point attaches a handle"),
        other => panic!("expected a template, got {other:?}"),
    }
}

fn resolve(root: &Path, schema: &str, host: &DarkmatterSlotHost) -> Result<simplified_schema::resolve::ResolvedSchema, SlotResolutionError> {
    pending(root, schema).resolve(host, &SlotRunRequest { context: context(root) })
}

/// The members of `property`'s enum, wherever its lowering put them (an
/// optional property is a union with `null`; a named type sits under
/// `$defs`).
fn members(resolved: &simplified_schema::resolve::ResolvedSchema, property: &str) -> Value {
    fn find(value: &Value, root: &Value) -> Option<Value> {
        match value {
            Value::Object(map) => {
                if let Some(members) = map.get("enum") {
                    return Some(members.clone());
                }
                if let Some(Value::String(reference)) = map.get("$ref") {
                    let target = reference.strip_prefix("#/").map(|path| path.split('/').fold(root, |node, key| &node[key]));
                    if let Some(found) = target.and_then(|target| find(target, root)) {
                        return Some(found);
                    }
                }
                map.values().find_map(|child| find(child, root))
            }
            Value::Array(items) => items.iter().find_map(|item| find(item, root)),
            _ => None,
        }
    }
    let root = &resolved.json_schema;
    find(&root["properties"][property], root).unwrap_or(Value::Null)
}

const SANDBOX: &str = "kind: schema\n$schema:\n    os: enum({{ load_data(\"../os.yaml\", \"string[]\") }}; required)\n";
const OS_LIST: &str = "- macOS\n- Linux\n- Windows\n";

mod evaluation {
    use super::*;

    #[test]
    fn a_load_data_slot_resolves_to_its_data_relative_to_the_schema_file() {
        let (_dir, root) = tree(&[("os.yaml", OS_LIST), ("schemas/sandbox.yaml", SANDBOX), ("schemas/os.yaml", "- Decoy\n")]);
        let host = reading_host();
        let resolved = resolve(&root, "schemas/sandbox.yaml", &host).unwrap();
        assert_eq!(members(&resolved, "os"), json!(["macOS", "Linux", "Windows"]));
        assert_eq!(resolved.json_schema["required"], json!(["os"]));

        let evaluations = host.evaluations();
        assert_eq!(evaluations.len(), 1);
        let (slot, evaluation) = &evaluations[0];
        assert_eq!(slot.origin.as_deref(), Some(root.join("schemas/sandbox.yaml").as_path()));
        let read = evaluation.observations.iter().find(|observation| observation.kind == ObservationKind::File).unwrap();
        assert_eq!(read.entry, biscuit_file::to_portable_string(&root.join("os.yaml")));
        assert_eq!(read.hash, Some(biscuit_hash::blake3_hash_bytes(OS_LIST.as_bytes())));
    }

    #[test]
    fn a_slot_in_an_imported_schema_reads_relative_to_its_own_file() {
        let (_dir, root) = tree(&[
            ("schemas/main.yaml", "kind: schema\n$schema:\n    os: Os@../shared/types.yaml\n"),
            ("schemas/os-list.yaml", "- Decoy\n"),
            ("shared/types.yaml", "kind: schema\ntypes:\n    Os: enum({{ load_data(\"./os-list.yaml\", \"string[]\") }})\n"),
            ("shared/os-list.yaml", OS_LIST),
        ]);
        let resolved = resolve(&root, "schemas/main.yaml", &reading_host()).unwrap();
        assert_eq!(members(&resolved, "os"), json!(["macOS", "Linux", "Windows"]));
    }

    #[test]
    fn without_a_grant_the_default_gate_refuses_the_read_and_reads_nothing() {
        let (_dir, root) = tree(&[("os.yaml", OS_LIST), ("schemas/sandbox.yaml", SANDBOX)]);
        let data = DataRuntime::new();
        let host = DarkmatterSlotHost::new(SlotEvaluator::new(data.clone()));
        let SlotResolutionError::Failed(failed) = resolve(&root, "schemas/sandbox.yaml", &host).unwrap_err() else {
            panic!("expected a failed slot");
        };
        assert_eq!(failed[0].1.code, Some(SLOT_APPROVAL_REQUIRED_CODE));
        assert!(matches!(failed[0].1.downcast_ref::<ExpressionError>(), Some(ExpressionError::SlotReadRefused { .. })));
        assert_eq!(data.stats().reads, 0);
    }

    #[test]
    fn the_command_read_set_gains_what_a_slot_read() {
        let (_dir, root) = tree(&[("os.yaml", OS_LIST), ("schemas/sandbox.yaml", SANDBOX)]);
        let data = DataRuntime::new();
        let host = DarkmatterSlotHost::new(SlotEvaluator::new(data.clone()).with_gate(Arc::new(AllowAll)));
        resolve(&root, "schemas/sandbox.yaml", &host).unwrap();
        assert_eq!(data.read_set(), [biscuit_file::to_portable_string(&root.join("os.yaml"))]);
    }

    #[test]
    fn a_work_limit_stops_the_slot_with_the_iteration_code() {
        let schema = "kind: schema\n$schema:\n    n: enum({{ map(['a', 'b', 'c', 'd'], x => x) }})\n";
        let (_dir, root) = tree(&[("schema.yaml", schema)]);
        let limits = IterationLimits::default().with_max_visits(NonZeroU64::new(2).unwrap());
        let host = DarkmatterSlotHost::new(SlotEvaluator::new(DataRuntime::new()).with_limits(limits));
        let SlotResolutionError::Failed(failed) = resolve(&root, "schema.yaml", &host).unwrap_err() else {
            panic!("expected a failed slot");
        };
        assert_eq!(failed[0].1.code, Some(crate::markdown::compose::expression::iteration::VISIT_LIMIT_CODE));
        // The same expression within the default allowance resolves.
        let resolved = resolve(&root, "schema.yaml", &host_with_default_limits()).unwrap();
        assert_eq!(members(&resolved, "n"), json!(["a", "b", "c", "d"]));
    }

    fn host_with_default_limits() -> DarkmatterSlotHost {
        host()
    }

    /// A captured function reads the request's clock reading, not the live
    /// clock: with a reading on 2024-06-16, `is_today` answers for that day.
    #[test]
    fn a_date_function_in_a_slot_reads_the_requests_captured_clock() {
        use crate::markdown::compose::context::capture::{CapturedObservations, ClockReading};
        let schema = "kind: schema\n$schema:\n    day: \"enum({{ is_today('2024-06-16') ? ['today'] : ['other'] }})\"\n";
        let (_dir, root) = tree(&[("schema.yaml", schema)]);
        let utc = chrono::DateTime::parse_from_rfc3339("2024-06-16T12:00:00Z").unwrap().with_timezone(&chrono::Utc);
        let reading = ClockReading::at(utc, chrono::FixedOffset::east_opt(0).unwrap());
        let evaluator =
            SlotEvaluator::new(DataRuntime::new()).with_observations(CapturedObservations::for_test_clock(reading));
        let resolved = resolve(&root, "schema.yaml", &DarkmatterSlotHost::new(evaluator)).unwrap();
        assert_eq!(members(&resolved, "day"), json!(["today"]));
    }

    #[test]
    fn context_inputs_are_kept_as_digests() {
        let schema = "kind: schema\n$schema:\n    tag: string(suggest({{ [ctx.year, env.SLOT_TAG, env.SLOT_UNSET || 'x'] }}))\n";
        let (_dir, root) = tree(&[("schema.yaml", schema)]);
        let host = host();
        let environment = std::collections::HashMap::from([("SLOT_TAG".to_string(), "beta".to_string())]);
        let request = SlotRunRequest { context: biscuit_file::FileResolutionContext::new(&root).with_env(environment) };
        pending(&root, "schema.yaml").resolve(&host, &request).unwrap();
        let (_, evaluation) = &host.evaluations()[0];
        let inputs: Vec<_> = evaluation.context_digests.iter().map(|digest| digest.input.clone()).collect();
        assert_eq!(
            inputs,
            [
                ContextInput::Group("DateTime".into()),
                ContextInput::Env("SLOT_TAG".into()),
                ContextInput::Env("SLOT_UNSET".into()),
            ]
        );
        let tag = &evaluation.context_digests[1];
        assert_eq!(tag.digest, Some(biscuit_hash::blake3_hash_bytes(b"beta")));
        assert_eq!(evaluation.context_digests[2].digest, None, "an unset variable has no digest");
        let text = format!("{:?}", evaluation.context_digests);
        assert!(!text.contains("beta"), "no value text is kept");
    }
}

mod failures {
    use super::*;

    /// The text of each file a projection positions in.
    fn texts(root: &Path) -> impl Fn(&simplified_schema::simplified::Slot) -> Option<(Option<PathBuf>, Arc<str>)> + '_ {
        move |slot| {
            let path = root.join(slot.origin.as_deref()?);
            let text = std::fs::read_to_string(&path).ok()?;
            Some((Some(path), Arc::from(text)))
        }
    }

    fn slot_position(text: &str) -> (usize, usize) {
        crate::markdown::span::line_col_of_offset(text, text.find("{{").unwrap())
    }

    #[test]
    fn a_malformed_data_file_keeps_its_data_code_and_location_and_is_positioned_at_the_slot() {
        let (_dir, root) = tree(&[("os.yaml", "- macOS\n- [Linux\n"), ("schemas/sandbox.yaml", SANDBOX)]);
        let error = resolve(&root, "schemas/sandbox.yaml", &reading_host()).unwrap_err();
        let SlotResolutionError::Failed(failed) = &error else { panic!("expected a failed slot: {error:?}") };
        let cause = failed[0].1.downcast_ref::<ExpressionError>().unwrap();
        assert!(matches!(cause, ExpressionError::Data(_)), "{cause:?}");
        assert_eq!(failed[0].1.code, Some(crate::markdown::compose::expression::data::MALFORMED_CODE));

        let records = slot_resolution_diagnostics(&error, &texts(&root));
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.code, crate::markdown::compose::expression::data::MALFORMED_CODE);
        let source = record.source.as_ref().unwrap();
        assert_eq!(source.file.as_deref(), Some(root.join("schemas/sandbox.yaml").as_path()));
        let (line, column) = slot_position(SANDBOX);
        assert_eq!((source.line, source.column), (Some(line), Some(column)));
        let location = record.data_location.as_ref().expect("the data file position stays secondary");
        assert_eq!(location.file, biscuit_file::to_portable_string(&root.join("os.yaml")));
        assert!(location.line.is_some() && location.column.is_some(), "{location:?}");
    }

    #[test]
    fn a_missing_data_file_keeps_its_reference_fields() {
        let (_dir, root) = tree(&[("schemas/sandbox.yaml", SANDBOX)]);
        let error = resolve(&root, "schemas/sandbox.yaml", &reading_host()).unwrap_err();
        let SlotResolutionError::Failed(failed) = &error else { panic!("expected a failed slot: {error:?}") };
        assert!(matches!(failed[0].1.downcast_ref::<ExpressionError>(), Some(ExpressionError::FileReference(_))));
        assert_eq!(failed[0].1.code, Some(SLOT_FAILED_CODE));
        let record = &slot_resolution_diagnostics(&error, &texts(&root))[0];
        let details = record.reference_details.as_ref().expect("reference fields");
        assert_eq!(details.construct.as_deref(), Some("load_data()"));
        assert_eq!(details.reference.as_deref(), Some("../os.yaml"));
        assert_eq!(record.source.as_ref().unwrap().line, Some(slot_position(SANDBOX).0));
    }

    #[test]
    fn a_refused_binding_is_positioned_at_the_read_and_nothing_is_evaluated() {
        let schema = "kind: schema\n$schema:\n    os: enum({{ fm.platforms }})\n";
        let (_dir, root) = tree(&[("schema.yaml", schema)]);
        let host = reading_host();
        let error = resolve(&root, "schema.yaml", &host).unwrap_err();
        assert!(matches!(error, SlotResolutionError::Refused(_)), "{error:?}");
        assert!(host.evaluations().is_empty());
        let record = &slot_resolution_diagnostics(&error, &texts(&root))[0];
        assert_eq!(record.code, SLOT_FORBIDDEN_BINDING_CODE);
        let expected = crate::markdown::span::line_col_of_offset(schema, schema.find("fm.platforms").unwrap());
        let source = record.source.as_ref().unwrap();
        assert_eq!((source.line, source.column), (Some(expected.0), Some(expected.1)));
    }

    #[test]
    fn a_wrong_shape_and_an_empty_list_get_their_slot_codes() {
        for (value, code) in [("'macOS'", SLOT_WRONG_SHAPE_CODE), ("[]", SLOT_EMPTY_CODE)] {
            let schema = format!("kind: schema\n$schema:\n    os: enum({{{{ {value} }}}})\n");
            let (_dir, root) = tree(&[("schema.yaml", &schema)]);
            let error = resolve(&root, "schema.yaml", &host()).unwrap_err();
            let record = &slot_resolution_diagnostics(&error, &texts(&root))[0];
            assert_eq!(record.code, code, "{value}");
            assert_eq!(record.source.as_ref().unwrap().line, Some(3));
        }
        let schema = "kind: schema\n$schema:\n    slug: string(pattern({{ '(a)\\\\1' }}))\n";
        let (_dir, root) = tree(&[("schema.yaml", schema)]);
        let error = resolve(&root, "schema.yaml", &host()).unwrap_err();
        assert_eq!(slot_resolution_diagnostics(&error, &texts(&root))[0].code, SLOT_PATTERN_REJECTED_CODE);
    }

    #[test]
    fn a_position_without_text_stays_null() {
        let (_dir, root) = tree(&[("schemas/sandbox.yaml", SANDBOX)]);
        let error = resolve(&root, "schemas/sandbox.yaml", &reading_host()).unwrap_err();
        let record = &slot_resolution_diagnostics(&error, &|_| None)[0];
        let source = record.source.as_ref().unwrap();
        assert_eq!((source.line, source.column), (None, None));
        assert!(record.span.is_none());
    }
}

/// Input Robustness Matrix 1, Darkmatter's wiring rows: for each slot kind,
/// the control evaluates through the host into the concrete schema, and one
/// invalid shape is refused at the slot with the slot code.
mod positions {
    use super::*;

    fn resolved(property: &str) -> Result<Value, String> {
        let schema = format!("kind: schema\n$schema:\n    value: {property}\n");
        let (_dir, root) = tree(&[("schema.yaml", &schema)]);
        match resolve(&root, "schema.yaml", &host()) {
            Ok(resolved) => Ok(resolved.json_schema["properties"]["value"].clone()),
            Err(error) => Err(slot_resolution_diagnostics(&error, &|_| None)[0].code.clone()),
        }
    }

    #[test]
    fn each_position_takes_its_control_value_and_refuses_one_invalid_shape() {
        let text = |value: &Value| value.to_string();
        let control = [
            ("string(suggest({{ ['a', 2, true] }}))", "\"a\""),
            ("literal({{ 5 }})", "5"),
            ("number(default({{ 3 }}))", "3"),
            ("number(min({{ 1 }}); max({{ 10 }}))", "10"),
            ("string(pattern({{ '^[a-z]+$' }}))", "^[a-z]+$"),
        ];
        for (property, expected) in control {
            let lowered = resolved(property).unwrap_or_else(|code| panic!("{property}: {code}"));
            assert!(text(&lowered).contains(expected), "{property}: {lowered}");
        }
        for property in [
            "string(suggest({{ [['a']] }}))",
            "literal({{ null }})",
            "number(default({{ null }}))",
            "number(min({{ 'one' }}))",
            "string(pattern({{ 5 }}))",
            "enum({{ [1] }})",
        ] {
            assert_eq!(resolved(property), Err(SLOT_WRONG_SHAPE_CODE.to_string()), "{property}");
        }
    }
}
