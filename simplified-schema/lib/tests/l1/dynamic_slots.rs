//! Resolving templates through the host hook: `PendingSchema::resolve` and
//! `SimplifiedSchema::resolve_slots` with a stub host.
//!
//! Boundary: the crate's public API, in-process. The stub host stands in
//! for Darkmatter: it evaluates nothing, answering each slot from a table
//! and, for `load_data("./file", …)`, reading the file beside the slot's
//! defining file through the resolver's own reads. Each Input Robustness
//! Matrix 1 cell edits one thing in the control row (the slot's value).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use serde_json::{Value, json};
use serde_yaml_ng::Value as YamlValue;
use simplified_schema::embedded::{EmbeddedDocument, ShippedSources};
use simplified_schema::resolve::{InlineOwner, ResolvedSchema, resolve_yaml_schema, resolve_yaml_schema_owned_by};
use simplified_schema::slots::{
    PendingSchema, RefuseTemplateReads, SlotAdmissionError, SlotFailure, SlotHost, SlotRequest, SlotResolutionError,
    SlotRunRequest, TemplateReads,
};
use simplified_schema::{SchemaError, SimplifiedSchema, SimplifiedSchemaError, Slot, SlotResultKind};

/// Answers slots by expression; `load_data("./x", …)` reads `x` beside the
/// slot's defining file (or the context's base directory) as YAML.
#[derive(Default)]
struct StubHost {
    answers: HashMap<String, Value>,
    refuse: Vec<String>,
    fail: Vec<String>,
    cancel_after: Option<usize>,
    evaluated: RefCell<Vec<(String, Option<PathBuf>)>>,
    checks: Cell<usize>,
}

#[derive(Debug, thiserror::Error)]
#[error("the stub could not evaluate `{0}`")]
struct StubError(String);

impl StubHost {
    fn answering(pairs: &[(&str, Value)]) -> Self {
        Self { answers: pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(), ..Self::default() }
    }

    #[cfg(feature = "file")]
    fn loading() -> Self {
        Self::default()
    }

    fn evaluations(&self) -> usize {
        self.evaluated.borrow().len()
    }
}

impl SlotHost for StubHost {
    fn admit(&self, slot: &Slot) -> Result<(), SlotAdmissionError> {
        if self.refuse.contains(&slot.expression) {
            return Err(SlotAdmissionError { code: Some("stub.refused"), message: "refused".into(), span: None });
        }
        Ok(())
    }

    fn resolve(&self, request: &SlotRequest<'_>) -> Result<Value, SlotFailure> {
        let expression = &request.slot.expression;
        self.evaluated.borrow_mut().push((expression.clone(), request.defining_file.map(Path::to_path_buf)));
        if self.fail.contains(expression) {
            return Err(SlotFailure::new(StubError(expression.clone()), Some("stub.failed")));
        }
        if let Some(answer) = self.answers.get(expression) {
            return Ok(answer.clone());
        }
        let file = expression
            .strip_prefix("load_data(\"")
            .and_then(|rest| rest.split('"').next())
            .unwrap_or_else(|| panic!("the stub has no answer for `{expression}`"));
        let dir = request
            .defining_file
            .and_then(Path::parent)
            .unwrap_or_else(|| request.context.base_dir())
            .to_path_buf();
        let bytes = simplified_schema::resolve::buffers::read(&dir.join(file))
            .map_err(|error| SlotFailure::new(error, Some("stub.io")))?;
        let yaml: YamlValue = serde_yaml_ng::from_slice(&bytes).expect("stub data is YAML");
        Ok(serde_json::to_value(yaml).expect("stub data is JSON-shaped"))
    }

    fn template_reads(&self) -> std::rc::Rc<dyn TemplateReads> {
        std::rc::Rc::new(RefuseTemplateReads)
    }

    fn is_canceled(&self) -> bool {
        let checks = self.checks.get() + 1;
        self.checks.set(checks);
        self.cancel_after.is_some_and(|after| self.evaluations() >= after)
    }
}

#[cfg(feature = "file")]
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dynamic_slots")
}

fn context(dir: &Path) -> FileResolutionContext {
    FileResolutionContext::new(dir)
}

/// The pending handle of an inline `$schema` mapping authored in `dir`.
fn pending_inline(yaml: &str, dir: &Path) -> PendingSchema {
    let value: YamlValue = serde_yaml_ng::from_str(yaml).unwrap();
    pending_of(resolve_yaml_schema(&value, dir, &context(dir), ShippedSources::NONE))
}

fn pending_of(result: Result<ResolvedSchema, SchemaError>) -> PendingSchema {
    match result {
        Err(SchemaError::UnresolvedSlots(unresolved)) => unresolved.pending.clone().expect("an entry point attaches a handle"),
        Err(other) => panic!("expected a template, got {other}"),
        Ok(_) => panic!("expected a template, got a concrete schema"),
    }
}

fn resolve(pending: &PendingSchema, host: &StubHost, dir: &Path) -> Result<ResolvedSchema, SlotResolutionError> {
    pending.resolve(host, &SlotRunRequest { context: context(dir) })
}

fn slot_error(error: SlotResolutionError) -> (Slot, SlotResultKind) {
    match error {
        SlotResolutionError::Schema(error) => match *error {
            SchemaError::SlotResult { slot, kind, .. } => (*slot, kind),
            other => panic!("expected a slot result error, got {other}"),
        },
        other => panic!("expected a schema error, got {other}"),
    }
}

/// The lowered property `name`, past the `null` alternative an optional
/// property's value is wrapped with.
fn property<'a>(resolved: &'a ResolvedSchema, name: &str) -> &'a Value {
    value_node(&resolved.json_schema["properties"][name])
}

fn value_node(node: &Value) -> &Value {
    match node.get("anyOf").and_then(Value::as_array) {
        Some(arms) if arms.len() == 2 && arms.iter().any(|arm| arm == &json!({ "type": "null" })) => {
            arms.iter().find(|arm| *arm != &json!({ "type": "null" })).expect("a value arm")
        }
        _ => node,
    }
}

fn enum_members(resolved: &ResolvedSchema, name: &str) -> Value {
    let node = property(resolved, name);
    node.get("enum").cloned().unwrap_or_else(|| panic!("{node}"))
}

fn suggestions(resolved: &ResolvedSchema, name: &str) -> Value {
    let node = property(resolved, name);
    node.get("x-darkmatter-suggest").cloned().unwrap_or_else(|| panic!("{node}"))
}

/// A property-level keyword, wherever the optional wrapper put it.
fn keyword(resolved: &ResolvedSchema, name: &str, key: &str) -> Value {
    let outer = &resolved.json_schema["properties"][name];
    outer.get(key).or_else(|| property(resolved, name).get(key)).cloned().unwrap_or(Value::Null)
}

/// The control row: the shipped fixture, its data read beside it.
#[cfg(feature = "file")]
mod control {
    use super::*;

    fn fixture_pending(name: &str) -> PendingSchema {
        let dir = fixtures();
        let reference = YamlValue::String(format!("./{name}"));
        pending_of(resolve_yaml_schema(&reference, &dir, &context(&dir), ShippedSources::NONE))
    }

    #[test]
    fn the_enum_fixture_resolves_to_its_data_with_the_repeated_literal_once() {
        let dir = fixtures();
        let pending = fixture_pending("schema-with-slot.yaml");
        let host = StubHost::loading();
        let resolved = resolve(&pending, &host, &dir).unwrap();
        let items = &property(&resolved, "os_support")["items"];
        assert_eq!(items["enum"], json!(["macOS", "Linux", "Windows"]), "{items}");
        assert_eq!(resolved.json_schema["required"], json!(["os_support"]), "the `; required` tail is kept");
        assert_eq!(host.evaluations(), 1);
        // The slot was written in the schema file, so it evaluates there.
        let schema_file = biscuit_file::canonicalize_simplified(&dir.join("schema-with-slot.yaml")).unwrap();
        assert_eq!(host.evaluated.borrow()[0].1.as_deref(), Some(schema_file.as_path()));
        // The concrete result names the same edges as any referenced file.
        assert_eq!(resolved.referenced_files, vec![schema_file]);
        assert!(resolved.simplified.is_some_and(|root| simplified_schema::slots::collect_slots(&root, &resolved.definitions).is_empty()));
    }

    #[test]
    fn the_suggestion_fixture_keeps_literals_and_data_in_order() {
        let dir = fixtures();
        let resolved = resolve(&fixture_pending("suggest-slot.yaml"), &StubHost::loading(), &dir).unwrap();
        assert_eq!(suggestions(&resolved, "platform"), json!(["FreeBSD", "macOS", "Linux", "Windows", "Plan9"]));
    }

    #[test]
    fn a_slot_in_an_imported_type_evaluates_in_its_defining_file() {
        let dir = fixtures();
        let host = StubHost::loading();
        let resolved = resolve(&fixture_pending("imported-types.yaml"), &host, &dir).unwrap();
        assert_eq!(enum_members(&resolved, "host_os"), json!(["FreeBSD", "Haiku"]));
        let defining = biscuit_file::canonicalize_simplified(&dir.join("nested/platform-types.yaml")).unwrap();
        assert_eq!(host.evaluated.borrow()[0].1.as_deref(), Some(defining.as_path()));
        assert_eq!(resolved.imports, vec![defining]);
    }

    #[test]
    fn every_template_constraint_takes_its_shape() {
        let dir = fixtures();
        let resolved = resolve(&fixture_pending("template-constraints.yaml"), &StubHost::loading(), &dir).unwrap();
        assert_eq!(property(&resolved, "revision")["const"], json!(5));
        let retries = property(&resolved, "retries");
        assert_eq!((retries["minimum"].clone(), retries["maximum"].clone()), (json!(1), json!(10)), "{retries}");
        assert_eq!(keyword(&resolved, "retries", "default"), json!(3));
        let slug = property(&resolved, "slug");
        assert!(slug.to_string().contains("^[a-z][a-z0-9-]*$"), "{slug}");
    }

    /// The pending handle serves the texts the first pass read: a change
    /// on the disk after it never mixes into the snapshot (the key a host
    /// computes from `read_sources` describes exactly what resolves).
    #[test]
    fn resolution_reads_the_recorded_schema_texts_never_the_disk_again() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();
        std::fs::create_dir(dir.join("nested")).unwrap();
        std::fs::copy(fixtures().join("imported-types.yaml"), dir.join("imported-types.yaml")).unwrap();
        std::fs::copy(fixtures().join("nested/platform-types.yaml"), dir.join("nested/platform-types.yaml")).unwrap();
        std::fs::write(dir.join("nested/os.yaml"), "- FreeBSD\n").unwrap();
        let reference = YamlValue::String("./imported-types.yaml".into());
        let pending = pending_of(resolve_yaml_schema(&reference, dir, &context(dir), ShippedSources::NONE));

        let read: Vec<String> = pending
            .read_sources()
            .iter()
            .map(|(path, _)| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(read, vec!["imported-types.yaml", "platform-types.yaml"]);
        let types_text = std::fs::read_to_string(dir.join("nested/platform-types.yaml")).unwrap();
        assert_eq!(&*pending.read_sources()[1].1, types_text.as_str());

        std::fs::write(dir.join("nested/platform-types.yaml"), "kind: schema\ntypes:\n    platform: string\n").unwrap();
        let resolved = resolve(&pending, &StubHost::loading(), dir).unwrap();
        assert_eq!(enum_members(&resolved, "host_os"), json!(["FreeBSD"]), "the recorded text still holds the slot");
    }
}

/// Input Robustness Matrix 1: one edit to the slot's value per cell.
mod result_shapes {
    use super::*;

    const ENUM: &str = "os: enum(macOS, {{ data }})\n";
    const ONLY_SLOT: &str = "os: enum({{ data }})\n";
    const SUGGEST: &str = "platform: string(suggest(FreeBSD, {{ data }}))\n";

    fn resolve_with(yaml: &str, value: Value) -> Result<ResolvedSchema, SlotResolutionError> {
        let temp = tempfile::tempdir().unwrap();
        resolve(&pending_inline(yaml, temp.path()), &StubHost::answering(&[("data", value)]), temp.path())
    }

    #[test]
    fn the_control_row_resolves() {
        let resolved = resolve_with(ENUM, json!(["macOS", "Linux", "Windows"])).unwrap();
        assert_eq!(enum_members(&resolved, "os"), json!(["macOS", "Linux", "Windows"]));
    }

    #[test]
    fn a_list_slot_takes_only_an_array() {
        for (value, found) in [(json!(null), "null"), (json!("macOS"), "string"), (json!(1), "number"), (json!({ "a": 1 }), "mapping")] {
            let (slot, kind) = slot_error(resolve_with(ENUM, value.clone()).unwrap_err());
            assert_eq!(slot.expression, "data");
            assert_eq!(kind, SlotResultKind::WrongShape { expected: "an array of strings", found: found.into() }, "{value}");
        }
    }

    #[test]
    fn an_array_is_flattened_one_level_and_a_nested_array_is_refused() {
        let (_, kind) = slot_error(resolve_with(ENUM, json!([["Linux"]])).unwrap_err());
        assert!(matches!(&kind, SlotResultKind::WrongShape { found, .. } if found == "an array holding a array"), "{kind:?}");
    }

    #[test]
    fn an_empty_result_adds_nothing_and_an_empty_list_is_its_own_error() {
        assert_eq!(enum_members(&resolve_with(ENUM, json!([])).unwrap(), "os"), json!(["macOS"]));
        let (slot, kind) = slot_error(resolve_with(ONLY_SLOT, json!([])).unwrap_err());
        assert_eq!((slot.expression.as_str(), kind), ("data", SlotResultKind::Empty));
    }

    #[test]
    fn enum_members_must_each_be_strings() {
        for value in [json!(["Linux", 1]), json!([1]), json!([true]), json!([null])] {
            let (_, kind) = slot_error(resolve_with(ENUM, value.clone()).unwrap_err());
            assert!(matches!(kind, SlotResultKind::WrongShape { expected: "an array of strings", .. }), "{value}");
        }
    }

    #[test]
    fn suggestions_take_every_scalar_and_refuse_a_mapping_element() {
        let resolved = resolve_with(SUGGEST, json!(["a", 2.5, 7, true, null])).unwrap();
        assert_eq!(suggestions(&resolved, "platform"), json!(["FreeBSD", "a", "2.5", "7", "true", "null"]));
        let (_, kind) = slot_error(resolve_with(SUGGEST, json!(["a", {}])).unwrap_err());
        assert!(matches!(&kind, SlotResultKind::WrongShape { found, .. } if found == "an array holding a mapping"), "{kind:?}");
    }

    #[test]
    fn a_numeric_suggestion_list_reads_numbers_by_their_canonical_text() {
        let resolved = resolve_with("n: number(suggest(1, {{ data }}))\n", json!([1.0, 2.50, "3"])).unwrap();
        assert_eq!(suggestions(&resolved, "n"), json!([1, 2.5, 3]), "`1.0` repeats the literal `1`");
    }

    #[test]
    fn the_first_occurrence_of_each_value_is_kept() {
        let resolved = resolve_with(ENUM, json!(["Linux", "macOS", "Linux"])).unwrap();
        assert_eq!(enum_members(&resolved, "os"), json!(["macOS", "Linux"]));
        let two_slots = "os: enum({{ data }}, Haiku, {{ more }})\n";
        let temp = tempfile::tempdir().unwrap();
        let host = StubHost::answering(&[("data", json!(["a", "Haiku"])), ("more", json!(["a", "b"]))]);
        let resolved = resolve(&pending_inline(two_slots, temp.path()), &host, temp.path()).unwrap();
        assert_eq!(enum_members(&resolved, "os"), json!(["a", "Haiku", "b"]));
    }

    #[test]
    fn data_holding_list_syntax_stays_one_value() {
        let resolved = resolve_with(ENUM, json!(["a, b; c) {{ d }}"])).unwrap();
        assert_eq!(enum_members(&resolved, "os"), json!(["macOS", "a, b; c) {{ d }}"]));
    }
}

/// One valid and one invalid shape per template constraint, through the same
/// validator as the lists.
mod template_constraints {
    use super::*;

    fn resolve_with(yaml: &str, answers: &[(&str, Value)]) -> Result<ResolvedSchema, SlotResolutionError> {
        let temp = tempfile::tempdir().unwrap();
        resolve(&pending_inline(yaml, temp.path()), &StubHost::answering(answers), temp.path())
    }

    #[test]
    fn literal_takes_a_scalar_with_its_type_kept() {
        let yaml = "revision: literal({{ v }}; required)\n";
        assert_eq!(property(&resolve_with(yaml, &[("v", json!(5))]).unwrap(), "revision")["const"], json!(5));
        assert_eq!(property(&resolve_with(yaml, &[("v", json!("5"))]).unwrap(), "revision")["const"], json!("5"));
        assert_eq!(property(&resolve_with(yaml, &[("v", json!(false))]).unwrap(), "revision")["const"], json!(false));
        for (value, found) in [(json!(null), "null"), (json!([5]), "array")] {
            let (_, kind) = slot_error(resolve_with(yaml, &[("v", value)]).unwrap_err());
            assert_eq!(kind, SlotResultKind::WrongShape { expected: "a string, number, or boolean", found: found.into() });
        }
    }

    #[test]
    fn default_takes_any_value_but_null_judged_against_the_type() {
        let yaml = "retries: number(default({{ d }}))\n";
        assert_eq!(keyword(&resolve_with(yaml, &[("d", json!(3))]).unwrap(), "retries", "default"), json!(3));
        let (_, kind) = slot_error(resolve_with(yaml, &[("d", json!(null))]).unwrap_err());
        assert_eq!(kind, SlotResultKind::WrongShape { expected: "a value other than null", found: "null".into() });
        let (slot, kind) = slot_error(resolve_with(yaml, &[("d", json!("three"))]).unwrap_err());
        assert_eq!(slot.expression, "d");
        assert!(matches!(&kind, SlotResultKind::Inadmissible(reason) if reason.contains("not a valid value")), "{kind:?}");

        let members = "os: enum(macOS, Linux; default({{ d }}))\n";
        assert_eq!(keyword(&resolve_with(members, &[("d", json!("Linux"))]).unwrap(), "os", "default"), json!("Linux"));
        assert!(resolve_with(members, &[("d", json!("Haiku"))]).is_err(), "a default outside the members is refused");
    }

    #[test]
    fn min_and_max_take_numbers_and_count_where_they_count() {
        let yaml = "n: number(min({{ low }}); max({{ high }}))\n";
        let resolved = resolve_with(yaml, &[("low", json!(1)), ("high", json!(2.5))]).unwrap();
        assert_eq!((property(&resolved, "n")["minimum"].clone(), property(&resolved, "n")["maximum"].clone()), (json!(1), json!(2.5)));
        let (_, kind) = slot_error(resolve_with(yaml, &[("low", json!("1")), ("high", json!(2))]).unwrap_err());
        assert_eq!(kind, SlotResultKind::WrongShape { expected: "a number", found: "string".into() });

        let lengths = resolve_with("s: string(min({{ n }}))\n", &[("n", json!(2))]).unwrap();
        assert_eq!(property(&lengths, "s")["minLength"], json!(2));
        let items = resolve_with("s: string[](max({{ n }}))\n", &[("n", json!(4))]).unwrap();
        assert_eq!(property(&items, "s")["maxItems"], json!(4));
        for count in [json!(-1), json!(1.5)] {
            let (_, kind) = slot_error(resolve_with("s: string(min({{ n }}))\n", &[("n", count.clone())]).unwrap_err());
            assert!(matches!(&kind, SlotResultKind::Inadmissible(reason) if reason.contains("non-negative integer")), "{count}");
        }
    }

    #[test]
    fn a_lower_bound_above_the_upper_is_refused_at_the_later_slot() {
        let yaml = "n: number(min({{ low }}); max({{ high }}))\n";
        let (slot, kind) = slot_error(resolve_with(yaml, &[("low", json!(11)), ("high", json!(10))]).unwrap_err());
        assert_eq!(slot.expression, "high");
        assert!(matches!(&kind, SlotResultKind::Inadmissible(reason) if reason.contains("11") && reason.contains("10")));
        let (slot, _) = slot_error(resolve_with("n: number(min(5); max({{ high }}))\n", &[("high", json!(4))]).unwrap_err());
        assert_eq!(slot.expression, "high", "a static bound crossed by a slot names the slot");
    }

    #[test]
    fn pattern_takes_a_string_the_linear_engine_compiles() {
        let yaml = "slug: string(pattern({{ p }}))\n";
        let resolved = resolve_with(yaml, &[("p", json!("^[a-z]+$"))]).unwrap();
        assert_eq!(property(&resolved, "slug")["pattern"], json!("^[a-z]+$"));
        let (_, kind) = slot_error(resolve_with(yaml, &[("p", json!(5))]).unwrap_err());
        assert_eq!(kind, SlotResultKind::WrongShape { expected: "a string", found: "number".into() });
        for (pattern, reason) in [
            ("^(?=a)a$", "lookaround"),
            (r"(a)\1", "backreference"),
            (r"(?:\w{100}){100}", "does not compile"),
        ] {
            let (_, kind) = slot_error(resolve_with(yaml, &[("p", json!(pattern))]).unwrap_err());
            assert!(matches!(&kind, SlotResultKind::PatternRejected(text) if text.contains(reason)), "{pattern}: {kind:?}");
        }
        let (_, kind) = slot_error(resolve_with(yaml, &[("p", json!("a".repeat(4097)))]).unwrap_err());
        assert!(matches!(&kind, SlotResultKind::PatternRejected(text) if text.contains("at most 4096")));
        // An authored pattern keeps today's behavior: lookaround is allowed.
        let temp = tempfile::tempdir().unwrap();
        let value: YamlValue = serde_yaml_ng::from_str("slug: string(pattern('^(?=a)a$'))\n").unwrap();
        assert!(resolve_yaml_schema(&value, temp.path(), &context(temp.path()), ShippedSources::NONE).is_ok());
    }

    #[test]
    fn a_slot_bound_beside_an_inherited_one_keeps_the_stricter() {
        let template = |low: Value| {
            let schema = SimplifiedSchema::try_from(
                "kind: schema\n$schema:\n  count: Count(min({{ low }}))\ntypes:\n  Count: number(min(3); default(4))\n",
            )
            .unwrap();
            schema.resolve_slots(&StubHost::answering(&[("low", low)])).map(|schema| schema.to_json_schema().unwrap())
        };
        let minimum = |json: std::sync::Arc<Value>| value_node(&json["properties"]["count"])["minimum"].clone();
        assert_eq!(minimum(template(json!(5)).unwrap()), json!(5));
        assert_eq!(minimum(template(json!(1)).unwrap()), json!(3));
        let defaults = |d: Value| {
            SimplifiedSchema::try_from("kind: schema\n$schema:\n  count: Count(default({{ d }}))\ntypes:\n  Count: number(default(4))\n")
                .unwrap()
                .resolve_slots(&StubHost::answering(&[("d", d)]))
        };
        assert!(defaults(json!(4)).is_ok(), "an equal default agrees");
        let (_, kind) = slot_error(defaults(json!(5)).unwrap_err());
        assert!(matches!(&kind, SlotResultKind::Inadmissible(reason) if reason.contains("disagrees")), "{kind:?}");
    }
}

mod host_contract {
    use super::*;

    const TWO: &str = "a: enum({{ first }})\nb: enum({{ second }})\n";

    #[test]
    fn a_refused_slot_stops_the_resolution_before_anything_is_evaluated() {
        let temp = tempfile::tempdir().unwrap();
        let host = StubHost { refuse: vec!["second".into()], ..StubHost::answering(&[("first", json!(["x"]))]) };
        match resolve(&pending_inline(TWO, temp.path()), &host, temp.path()).unwrap_err() {
            SlotResolutionError::Refused(refused) => {
                assert_eq!(refused.len(), 1);
                assert_eq!((refused[0].0.expression.as_str(), refused[0].1.code), ("second", Some("stub.refused")));
            }
            other => panic!("{other}"),
        }
        assert_eq!(host.evaluations(), 0, "admission is passive and comes first");
    }

    #[test]
    fn every_failing_slot_is_attempted_and_keeps_its_typed_cause() {
        let temp = tempfile::tempdir().unwrap();
        let host = StubHost { fail: vec!["first".into(), "second".into()], ..StubHost::default() };
        match resolve(&pending_inline(TWO, temp.path()), &host, temp.path()).unwrap_err() {
            SlotResolutionError::Failed(failed) => {
                let names: Vec<&str> = failed.iter().map(|(slot, _)| slot.expression.as_str()).collect();
                assert_eq!(names, vec!["first", "second"]);
                assert_eq!(failed[0].1.code, Some("stub.failed"));
                assert_eq!(failed[0].1.downcast_ref::<StubError>().map(|error| error.0.as_str()), Some("first"));
            }
            other => panic!("{other}"),
        }
    }

    #[test]
    fn cancellation_publishes_nothing_and_the_template_still_resolves() {
        let temp = tempfile::tempdir().unwrap();
        let pending = pending_inline(TWO, temp.path());
        let answers = [("first", json!(["x"])), ("second", json!(["y"]))];
        let canceled = StubHost { cancel_after: Some(1), ..StubHost::answering(&answers) };
        assert!(matches!(resolve(&pending, &canceled, temp.path()), Err(SlotResolutionError::Canceled)));
        assert_eq!(canceled.evaluations(), 1, "the second slot was never evaluated");
        let resolved = resolve(&pending, &StubHost::answering(&answers), temp.path()).unwrap();
        assert_eq!(enum_members(&resolved, "b"), json!(["y"]));
    }

    #[test]
    fn repeated_resolutions_start_from_the_template() {
        let temp = tempfile::tempdir().unwrap();
        let pending = pending_inline("os: enum({{ data }})\n", temp.path());
        let first = resolve(&pending, &StubHost::answering(&[("data", json!(["a"]))]), temp.path()).unwrap();
        let second = resolve(&pending, &StubHost::answering(&[("data", json!(["b"]))]), temp.path()).unwrap();
        assert_eq!((enum_members(&first, "os"), enum_members(&second, "os")), (json!(["a"]), json!(["b"])));
        assert_eq!(pending.slots().len(), 1);
        let (root, _) = pending.template().expect("the expanded template is kept");
        assert!(!simplified_schema::slots::collect_slots(root, &Default::default()).is_empty());
    }

    // File references resolve only with the `file` feature.
    #[cfg(feature = "file")]
    #[test]
    fn a_slot_in_a_named_type_used_twice_is_evaluated_once() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("types.yaml"), "kind: schema\ntypes:\n  Os: enum({{ data }})\n").unwrap();
        let yaml = "a: Os@./types.yaml\nb: Os@./types.yaml\n";
        let host = StubHost::answering(&[("data", json!(["x"]))]);
        let resolved = resolve(&pending_inline(yaml, temp.path()), &host, temp.path()).unwrap();
        assert_eq!((enum_members(&resolved, "a"), enum_members(&resolved, "b")), (json!(["x"]), json!(["x"])));
        assert_eq!(host.evaluations(), 1);
    }

    // File references resolve only with the `file` feature.
    #[cfg(feature = "file")]
    #[test]
    fn slots_in_later_root_union_arms_are_reached_and_resolved() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("arm.yaml"), "$schema:\n  kind: literal(b)\n  os: enum({{ second }})\n").unwrap();
        let yaml = "- kind: literal(a)\n  os: enum({{ first }})\n- ./arm.yaml\n";
        let host = StubHost::answering(&[("first", json!(["x"])), ("second", json!(["y"]))]);
        let resolved = resolve(&pending_inline(yaml, temp.path()), &host, temp.path()).unwrap();
        let text = resolved.json_schema.to_string();
        assert!(text.contains(r#"["x"]"#) && text.contains(r#"["y"]"#), "{text}");
        assert_eq!(host.evaluations(), 2);
    }
}

mod entry_points {
    use super::*;

    #[test]
    fn a_recursive_named_type_with_a_slot_resolves_into_its_definition() {
        let schema = SimplifiedSchema::try_from(
            "kind: schema\n$schema:\n  root: Node\ntypes:\n  Node:\n    kind: enum({{ kinds }})\n    children: Node[]\n",
        )
        .unwrap();
        assert!(!schema.is_resolved());
        let resolved = schema.resolve_slots(&StubHost::answering(&[("kinds", json!(["leaf", "branch"]))])).unwrap();
        assert!(resolved.is_resolved());
        let json = resolved.to_json_schema().unwrap();
        assert!(json["$defs"].to_string().contains(r#"["leaf","branch"]"#), "{json}");
        assert!(resolved.validate_strict(&json!({ "root": { "kind": "leaf", "children": [{ "kind": "branch" }] } })).is_ok());
        assert!(resolved.validate_strict(&json!({ "root": { "kind": "other" } })).is_err());
    }

    #[test]
    fn resolve_slots_returns_a_new_concrete_schema_and_leaves_the_template() {
        let template = SimplifiedSchema::try_from(include_str!("../fixtures/dynamic_slots/schema-with-slot.yaml")).unwrap();
        let host = StubHost::answering(&[(r#"load_data("./os.yaml", "string[]")"#, json!(["Linux", "Windows"]))]);
        let concrete = template.resolve_slots(&host).unwrap();
        assert!(concrete.validate_strict(&json!({ "os_support": ["macOS", "Linux"] })).is_ok());
        assert!(concrete.validate_strict(&json!({ "os_support": ["Haiku"] })).is_err());
        assert!(!template.is_resolved(), "the template is unchanged");
        assert!(matches!(template.to_json_schema(), Err(SimplifiedSchemaError::Unresolved(_))));
        // A refused value publishes nothing.
        let bad = StubHost::answering(&[(r#"load_data("./os.yaml", "string[]")"#, json!("Linux"))]);
        assert!(matches!(template.resolve_slots(&bad), Err(SlotResolutionError::Schema(_))));
        assert!(concrete.resolve_slots(&bad).is_ok(), "a concrete schema has nothing to resolve");
    }

    #[test]
    fn layering_a_resolved_template_equals_layering_its_static_twin() {
        let base = SimplifiedSchema::try_from("$schema:\n  title: string(required)\n  os: string\n").unwrap();
        let template = SimplifiedSchema::try_from("$schema:\n  os: enum(macOS, {{ data }}; required)\n").unwrap();
        let twin = SimplifiedSchema::try_from("$schema:\n  os: enum(macOS, Linux; required)\n").unwrap();
        let mut resolved = template.resolve_slots(&StubHost::answering(&[("data", json!(["Linux"]))])).unwrap();
        let mut static_twin = twin.clone();
        resolved.layer_under(&[&base]).unwrap();
        static_twin.layer_under(&[&base]).unwrap();
        assert_eq!(resolved.to_json_schema().unwrap(), static_twin.to_json_schema().unwrap());
        // Whole-property precedence: the resolved `os` replaced the base's.
        let os = resolved.to_json_schema().unwrap()["properties"]["os"].clone();
        assert_eq!(os["enum"], json!(["macOS", "Linux"]), "{os}");
        let mut layered_template = template.clone();
        assert!(matches!(layered_template.layer_under(&[&base]), Err(SimplifiedSchemaError::Unresolved(_))));
    }

    // File references resolve only with the `file` feature.
    #[cfg(feature = "file")]
    #[test]
    fn a_template_resolves_with_the_same_edges_as_its_static_twin() {
        let temp = tempfile::tempdir().unwrap();
        for (folder, members) in [("template", "{{ data }}"), ("twin", "Linux")] {
            let dir = temp.path().join(folder);
            std::fs::create_dir(&dir).unwrap();
            std::fs::write(dir.join("types.yaml"), format!("kind: schema\ntypes:\n  Os: enum({members})\n")).unwrap();
            std::fs::write(dir.join("schema.yaml"), "$schema:\n  os: Os@./types.yaml\n").unwrap();
        }
        let reference = YamlValue::String("./schema.yaml".into());
        let resolve_in = |folder: &str| {
            let dir = temp.path().join(folder);
            (resolve_yaml_schema(&reference, &dir, &context(&dir), ShippedSources::NONE), dir)
        };
        let (template, template_dir) = resolve_in("template");
        let resolved = resolve(&pending_of(template), &StubHost::answering(&[("data", json!(["Linux"]))]), &template_dir).unwrap();
        let (twin, twin_dir) = resolve_in("twin");
        let twin = twin.unwrap();
        let relative = |paths: &[PathBuf], dir: &Path| -> Vec<PathBuf> {
            let dir = biscuit_file::canonicalize_simplified(dir).unwrap();
            paths.iter().map(|path| path.strip_prefix(&dir).unwrap_or(path).to_path_buf()).collect()
        };
        assert_eq!(relative(&resolved.imports, &template_dir), relative(&twin.imports, &twin_dir));
        assert_eq!(relative(&resolved.referenced_files, &template_dir), relative(&twin.referenced_files, &twin_dir));
        assert_eq!(relative(&resolved.examples, &template_dir), relative(&twin.examples, &twin_dir));
        assert_eq!(resolved.json_schema, twin.json_schema);
    }

    #[test]
    fn only_entry_points_carry_a_handle_and_lowering_alone_never_does() {
        let temp = tempfile::tempdir().unwrap();
        let value: YamlValue = serde_yaml_ng::from_str("os: enum({{ data }})\n").unwrap();
        let catalog = simplified_schema::resolve::resolve_catalog_declaration(
            &serde_yaml_ng::from_str("os: enum({{ data }})\n").unwrap(),
            temp.path(),
            &context(temp.path()),
            ShippedSources::NONE,
        );
        assert!(matches!(catalog, Err(SchemaError::UnresolvedSlots(unresolved)) if unresolved.pending.is_none()));
        let root = simplified_schema::simplified::parse_yaml_schema(&value).unwrap();
        assert!(matches!(
            simplified_schema::simplified::to_json_schema(&root),
            Err(SchemaError::UnresolvedSlots(unresolved)) if unresolved.pending.is_none()
        ));
        let json = serde_json::to_value(&value).unwrap();
        let pending = pending_of(simplified_schema::resolve::resolve_schema(&json, temp.path(), &context(temp.path()), ShippedSources::NONE));
        assert_eq!(pending.slots()[0].origin, None, "an inline document's own slot has no file");
    }

    #[test]
    fn a_trigger_payload_slot_evaluates_in_the_trigger_file() {
        let temp = tempfile::tempdir().unwrap();
        let trigger = temp.path().join("trigger.yaml");
        std::fs::write(&trigger, "kind: schema-trigger\n").unwrap();
        let value: YamlValue = serde_yaml_ng::from_str("os: enum({{ data }})\n").unwrap();
        let result = resolve_yaml_schema_owned_by(
            &value,
            InlineOwner::Trigger(&trigger),
            temp.path(),
            &[],
            &context(temp.path()),
            ShippedSources::NONE,
        );
        let canonical = biscuit_file::canonicalize_simplified(&trigger).unwrap();
        assert_eq!(pending_of(result).slots()[0].origin.as_deref(), Some(canonical.as_path()));
    }

    #[test]
    fn an_embedded_schema_refuses_slots() {
        let documents = [EmbeddedDocument { path: "schemas/embedded.yaml", text: "$schema:\n  os: enum({{ data }})\n" }];
        let error = simplified_schema::resolve::prepare_embedded_schema("schemas/embedded.yaml", &documents, ShippedSources::NONE)
            .unwrap_err();
        assert!(error.to_string().contains("cannot contain `{{ … }}` slots"), "{error}");
    }
}

/// A host reading a slot's data file reads unsaved editor text the same way
/// the resolver reads schema sources.
mod buffer_reads {
    use std::sync::Arc;

    use simplified_schema::resolve::buffers::{self, SourceBuffers};

    #[test]
    fn read_serves_an_open_buffer_inside_with_and_the_disk_outside() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("os.yaml");
        std::fs::write(&data, "- macOS\n").unwrap();
        let mut open = SourceBuffers::default();
        open.insert(&data, Arc::from("- Haiku\n"));
        open.insert(&temp.path().join("unsaved.yaml"), Arc::from("- FreeBSD\n"));

        assert_eq!(buffers::read(&data).unwrap(), b"- macOS\n");
        buffers::with(&open, || {
            assert_eq!(buffers::read(&data).unwrap(), b"- Haiku\n");
            assert_eq!(buffers::read(&temp.path().join("unsaved.yaml")).unwrap(), b"- FreeBSD\n");
        });
        assert_eq!(buffers::read(&data).unwrap(), b"- macOS\n");
        assert_eq!(buffers::read(&temp.path().join("unsaved.yaml")).unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }
}

/// `PendingSchema::without_properties`: a host whose layering replaces some
/// top-level properties resolves only the template that survives.
#[cfg(feature = "file")]
mod surviving_properties {
    use super::*;

    const TWO: &str = "os: enum({{ first }})\narch: enum({{ second }})\n";

    /// The slots' expressions, sorted.
    fn names(slots: &[Slot]) -> Vec<&str> {
        let mut names: Vec<&str> = slots.iter().map(|slot| slot.expression.as_str()).collect();
        names.sort_unstable();
        names
    }

    /// The stub panics on an expression it has no answer for, so answering
    /// only `second` proves `first` is never evaluated.
    #[test]
    fn a_discarded_property_s_slot_is_neither_listed_nor_evaluated() {
        let temp = tempfile::tempdir().unwrap();
        let surviving = pending_inline(TWO, temp.path()).without_properties(&["os".to_string()]);
        assert_eq!(names(surviving.slots()), ["second"]);
        assert_eq!(surviving.omitted_properties(), ["os"]);
        let resolved = resolve(&surviving, &StubHost::answering(&[("second", json!(["arm64"]))]), temp.path()).unwrap();
        assert_eq!(enum_members(&resolved, "arch"), json!(["arm64"]));
        assert!(resolved.json_schema["properties"].get("os").is_none(), "{:#}", resolved.json_schema);
    }

    #[test]
    fn a_template_left_without_slots_resolves_without_a_host() {
        let temp = tempfile::tempdir().unwrap();
        let pending = pending_inline("os: enum({{ first }})\ntitle: string\n", temp.path());
        assert!(matches!(pending.resolve_without_slots(), Err(SchemaError::UnresolvedSlots(_))));
        let surviving = pending.without_properties(&["os".to_string()]);
        assert!(surviving.slots().is_empty());
        let resolved = surviving.resolve_without_slots().unwrap();
        assert!(resolved.json_schema["properties"].get("os").is_none(), "{:#}", resolved.json_schema);
        assert!(resolved.json_schema["properties"].get("title").is_some(), "{:#}", resolved.json_schema);
    }

    /// `Platform` is an object two properties use, so it is stored once: its
    /// slot belongs to whichever of them survives.
    #[test]
    fn a_stored_definition_s_slot_survives_only_with_a_property_that_reaches_it() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("types.yaml"), "kind: schema\ntypes:\n    Platform: \"{ os: enum({{ first }}) }\"\n").unwrap();
        let pending = pending_inline(
            "host: Platform@./types.yaml\nguest: Platform@./types.yaml\nname: enum({{ second }})\n",
            temp.path(),
        );
        assert_eq!(names(pending.slots()), ["first", "second"]);
        let one = pending.without_properties(&["host".to_string()]);
        assert_eq!(names(one.slots()), ["first", "second"]);
        let both = pending.without_properties(&["host".to_string(), "guest".to_string()]);
        assert_eq!(names(both.slots()), ["second"]);
        let resolved = resolve(&both, &StubHost::answering(&[("second", json!(["a"]))]), temp.path()).unwrap();
        assert!(resolved.json_schema.get("$defs").is_none(), "{:#}", resolved.json_schema);
    }

    #[test]
    fn an_undeclared_name_or_a_root_union_arm_changes_nothing() {
        let temp = tempfile::tempdir().unwrap();
        let pending = pending_inline(TWO, temp.path());
        assert_eq!(pending.without_properties(&["title".to_string()]), pending);
        let arm = pending_inline("- os: enum({{ first }})\n- title: string\n", temp.path());
        assert!(arm.is_union_arm());
        assert_eq!(arm.without_properties(&["os".to_string()]), arm);
    }

    /// The arm a template was met in speaks for the whole root union: its
    /// declared properties are those every arm declares, read before any
    /// slot is evaluated, so a host discards what the resolved union would.
    #[test]
    fn a_root_union_arm_template_declares_what_every_arm_declares() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("other.yaml"), "kind: schema
$schema:
    os: string
    extra: number
").unwrap();
        let every = pending_inline("- os: enum({{ first }})\n  arch: string\n- os: string\n  arch: string\n- ./other.yaml\n", temp.path());
        assert!(every.is_union_arm());
        assert_eq!(every.declared_properties(), ["os"]);
        let later_arm = pending_inline("- os: string\n- os: enum({{ first }})\n  title: string\n", temp.path());
        assert_eq!(later_arm.declared_properties(), ["os"]);
        let some = pending_inline("- os: enum({{ first }})\n- title: string\n", temp.path());
        assert_eq!(some.declared_properties(), Vec::<String>::new());
        std::fs::write(temp.path().join("bare.yaml"), "kind: schema\n$schema:\n    extra: number\n").unwrap();
        let file_arm = pending_inline("- os: enum({{ first }})\n- ./bare.yaml\n", temp.path());
        assert_eq!(file_arm.declared_properties(), Vec::<String>::new());
        assert_eq!(pending_inline(TWO, temp.path()).declared_properties(), ["os", "arch"]);
    }
}

/// The hook is host-neutral: the slot modules name no host, so a host's
/// types reach them only through `SlotHost`. The crate-wide dependency
/// guard (`dependency_direction_guard`) covers manifests and crate paths;
/// this covers prose and identifiers too.
#[test]
fn the_slot_modules_name_no_host() {
    let sources = [
        ("slots.rs", include_str!("../../src/slots.rs")),
        ("slots/host.rs", include_str!("../../src/slots/host.rs")),
        ("slots/pending.rs", include_str!("../../src/slots/pending.rs")),
        ("slots/substitute.rs", include_str!("../../src/slots/substitute.rs")),
    ];
    for (name, text) in sources {
        let lower = text.to_lowercase();
        for host in ["darkmatter", "dmls", "claudine"] {
            assert!(!lower.contains(host), "src/{name} names `{host}`");
        }
    }
}

/// A host that supplies the text an inline `$schema` value was written in
/// gets slot spans in that text, kept on the template and on a failure, so
/// a failure is positioned without reading the file again; without the
/// text, or when the text does not prove the value, no position is claimed.
#[cfg(feature = "file")]
mod authored_source {
    use std::sync::Arc;

    use super::*;
    use simplified_schema::resolve::{AuthoredSchemaValue, resolve_authored_yaml_schema};
    use simplified_schema::SlotResolutionFailure;

    /// A frontmatter document: its text, and the YAML of its block.
    fn document(yaml: &str) -> (Arc<str>, std::ops::Range<usize>) {
        let text = format!("---\n{yaml}---\nBody.\n");
        (text.into(), 4..4 + yaml.len())
    }

    fn authored_pending(yaml: &str, value_yaml: &str, owner: InlineOwner<'_>, file: Option<PathBuf>) -> (PendingSchema, Arc<str>) {
        let dir = fixtures();
        let (text, mapping) = document(yaml);
        let authored = AuthoredSchemaValue { file, text: Arc::clone(&text), mapping };
        let value: YamlValue = serde_yaml_ng::from_str(value_yaml).unwrap();
        let result =
            resolve_authored_yaml_schema(&value, owner, Some(&authored), &dir, &[], &context(&dir), ShippedSources::NONE);
        (pending_of(result), text)
    }

    #[test]
    fn a_document_template_constraint_slot_is_spanned_in_the_document_through_its_carrier() {
        let yaml = "title: x\n$schema:\n  os: \"enum(macOS, {{ \\\"x\\\" }})\"\n";
        let file = PathBuf::from("/docs/doc.md");
        let (pending, text) = authored_pending(yaml, "os: \"enum(macOS, {{ \\\"x\\\" }})\"\n", InlineOwner::Document, Some(file.clone()));
        let slot = &pending.slots()[0];
        assert_eq!(&text[slot.span.clone()], "{{ \\\"x\\\" }}");
        let (kept_file, kept_text) = pending.slot_text(slot).expect("the document text is kept");
        assert_eq!((kept_file, kept_text.as_ref()), (Some(file.as_path()), text.as_ref()));
        assert_eq!(pending.authored_source().map(|authored| authored.mapping.clone()), Some(4..4 + yaml.len()));
    }

    #[test]
    fn a_failure_of_the_template_keeps_the_text_for_its_slot() {
        let yaml = "$schema:\n  os: enum({{ wrong }})\n";
        let (pending, text) = authored_pending(yaml, "os: enum({{ wrong }})\n", InlineOwner::Document, None);
        let error = Arc::new(resolve(&pending, &StubHost::answering(&[("wrong", json!(1))]), &fixtures()).unwrap_err());
        let SlotResolutionError::Schema(schema) = error.as_ref() else { panic!("{error:?}") };
        let SchemaError::SlotResult { slot, .. } = schema.as_ref() else { panic!("{schema:?}") };
        let slot = slot.as_ref().clone();
        assert_eq!(&text[slot.span.clone()], "{{ wrong }}");
        let failure = SlotResolutionFailure::of_pending(Arc::clone(&error), &pending);
        assert_eq!(failure.slot_text(&slot).map(|(file, text)| (file, text.as_ref())), Some((None, text.as_ref())));
        assert!(SlotResolutionFailure::new(failure.error.clone(), failure.sources.clone()).slot_text(&slot).is_none());
    }

    #[test]
    fn a_trigger_payload_slot_names_its_file_and_is_spanned_in_it() {
        let dir = fixtures();
        let trigger = dir.join("a.trigger.yaml");
        let text: Arc<str> = "kind: schema-trigger\nmatch:\n  topic: literal(x)\n$schema:\n  os: enum({{ wrong }})\n".into();
        let authored = AuthoredSchemaValue { file: Some(trigger.clone()), text: Arc::clone(&text), mapping: 0..text.len() };
        let value: YamlValue = serde_yaml_ng::from_str("os: enum({{ wrong }})\n").unwrap();
        let pending = pending_of(resolve_authored_yaml_schema(
            &value,
            InlineOwner::Trigger(&trigger),
            Some(&authored),
            &dir,
            &[],
            &context(&dir),
            ShippedSources::NONE,
        ));
        let slot = &pending.slots()[0];
        assert_eq!(&text[slot.span.clone()], "{{ wrong }}");
        let (file, kept) = pending.slot_text(slot).expect("the trigger text is kept");
        assert_eq!(file, slot.origin.as_deref(), "the file is the one the slot names");
        assert_eq!(kept.as_ref(), text.as_ref());
    }

    #[test]
    fn a_root_union_arm_slot_is_spanned_in_the_document() {
        let yaml = "$schema:\n  - name: string\n  - os: enum({{ wrong }})\n";
        let (pending, text) =
            authored_pending(yaml, "- name: string\n- os: enum({{ wrong }})\n", InlineOwner::Document, None);
        assert_eq!(&text[pending.slots()[0].span.clone()], "{{ wrong }}");
    }

    #[test]
    fn text_that_does_not_say_what_the_value_is_proves_nothing() {
        let yaml = "$schema:\n  os: enum({{ other }})\n";
        let (pending, _) = authored_pending(yaml, "os: enum({{ wrong }})\n", InlineOwner::Document, None);
        assert!(pending.authored_source().is_none());
        assert!(pending.slot_text(&pending.slots()[0]).is_none());
    }

    #[test]
    fn without_the_text_a_slot_is_spanned_in_its_type_string() {
        let pending = pending_inline("os: enum({{ wrong }})\n", &fixtures());
        let slot = &pending.slots()[0];
        assert_eq!(&"enum({{ wrong }})"[slot.span.clone()], "{{ wrong }}");
        assert!(pending.slot_text(slot).is_none());
    }

    /// The Markdown factory re-parses a root union's inline arms when an arm
    /// is a file reference; those arms' slots are in the document too.
    #[test]
    fn a_markdown_root_union_with_a_file_arm_spans_its_inline_slot_in_the_document() {
        use simplified_schema::SchemaSource;

        let dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        std::fs::write(root.join("other.yaml"), "$schema:\n  name: string\n").unwrap();
        let text = "---\n$schema:\n  - ./other.yaml\n  - os: enum({{ wrong }})\n---\nBody.\n";
        std::fs::write(root.join("doc.md"), text).unwrap();
        let schema =
            SimplifiedSchema::try_from_markdown(SchemaSource::from(root.join("doc.md")).bound(FileResolutionContext::new(&root)))
                .expect("a template is admitted");
        let slot = &schema.pending_slots()[0];
        assert_eq!(&text[slot.span.clone()], "{{ wrong }}");
    }
}
