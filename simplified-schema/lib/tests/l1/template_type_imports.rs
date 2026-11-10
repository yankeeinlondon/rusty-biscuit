//! Template types whose returned definition imports other files: the base
//! directory imports resolve against, the host's read gate
//! (`TemplateReads`), dependency reporting, and read-once behavior.
//!
//! Boundary: the crate's public API, in-process. Each test builds its
//! fixtures in a temporary directory, resolves a schema through
//! `resolve_yaml_schema`, and answers the pending template with a stub host
//! whose gate records every `admit` and `observe` call. Where meaningful the
//! result is compared with the same schema written statically.
//!
//! Every test reads files, so the module needs the `file` feature.
#![cfg(feature = "file")]

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use biscuit_file::FileResolutionContext;
use serde_json::{Value, json};
use simplified_schema::embedded::ShippedSources;
use simplified_schema::resolve::observed::{self, Observed};
use simplified_schema::resolve::{ResolvedSchema, resolve_yaml_schema};
use simplified_schema::slots::{
    SchemaRead, SchemaReadKind, SchemaReadOutcome, SlotAdmissionError, SlotFailure, SlotHost, SlotRequest,
    SlotResolutionError, SlotRunRequest, TemplateReads,
};
use simplified_schema::{SchemaError, Slot, SlotResultKind};

const DENIED: &str = "test.template-read-denied";

const VALID_EXAMPLE: &str = "kind: example\ninvocation: \"x\"\nreturns: \"val\"\ndescription: d\n";

/// One call the resolver made on the gate, paths normalized.
#[derive(Debug, Clone, PartialEq, Eq)]
enum GateEvent {
    Admit(SchemaReadKind, PathBuf),
    Observe { kind: SchemaReadKind, path: PathBuf, exists: bool, has_bytes: bool },
}

/// A recording gate refusing every read whose file name is in `deny`.
#[derive(Default)]
struct Gate {
    deny: Vec<&'static str>,
    log: RefCell<Vec<GateEvent>>,
}

impl Gate {
    fn denying(names: &[&'static str]) -> Rc<Self> {
        Rc::new(Self { deny: names.to_vec(), log: RefCell::default() })
    }

    fn events(&self) -> Vec<GateEvent> {
        self.log.borrow().clone()
    }

    fn admitted(&self, kind: SchemaReadKind, path: &Path) -> bool {
        self.events().contains(&GateEvent::Admit(kind, normalize(path)))
    }

    fn observed(&self) -> Vec<(SchemaReadKind, PathBuf, bool, bool)> {
        self.events()
            .into_iter()
            .filter_map(|event| match event {
                GateEvent::Observe { kind, path, exists, has_bytes } => Some((kind, path, exists, has_bytes)),
                GateEvent::Admit(..) => None,
            })
            .collect()
    }
}

impl TemplateReads for Gate {
    fn admit(&self, _: &Slot, read: &SchemaRead<'_>) -> Result<(), SlotAdmissionError> {
        self.log.borrow_mut().push(GateEvent::Admit(read.kind, normalize(read.path)));
        let name = read.path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if self.deny.contains(&name) {
            return Err(SlotAdmissionError { code: Some(DENIED), message: format!("{name} is denied"), span: None });
        }
        Ok(())
    }

    fn observe(&self, _: &Slot, outcome: &SchemaReadOutcome<'_>) {
        self.log.borrow_mut().push(GateEvent::Observe {
            kind: outcome.kind,
            path: normalize(outcome.path),
            exists: outcome.exists,
            has_bytes: outcome.bytes.is_some(),
        });
    }
}

/// Answers every slot by expression; records each slot's defining file.
struct Host {
    gate: Rc<Gate>,
    answers: Vec<(&'static str, Value)>,
    defining_files: RefCell<Vec<Option<PathBuf>>>,
}

impl Host {
    fn new(gate: Rc<Gate>, answers: &[(&'static str, &str)]) -> Self {
        Self {
            gate,
            answers: answers.iter().map(|(slot, value)| (*slot, json!(value))).collect(),
            defining_files: RefCell::default(),
        }
    }
}

impl SlotHost for Host {
    fn admit(&self, _: &Slot) -> Result<(), SlotAdmissionError> {
        Ok(())
    }

    fn resolve(&self, request: &SlotRequest<'_>) -> Result<Value, SlotFailure> {
        self.defining_files.borrow_mut().push(request.defining_file.map(normalize));
        let (_, value) = self
            .answers
            .iter()
            .find(|(slot, _)| *slot == request.slot.expression)
            .unwrap_or_else(|| panic!("no answer for `{}`", request.slot.expression));
        Ok(value.clone())
    }

    fn template_reads(&self) -> Rc<dyn TemplateReads> {
        self.gate.clone()
    }
}

/// Canonical form of `path`; an absent file is canonicalized through its
/// parent so it compares with the spelling the resolver reports.
fn normalize(path: &Path) -> PathBuf {
    if let Ok(canonical) = biscuit_file::canonicalize_simplified(path) {
        return canonical;
    }
    let parent = path.parent().expect("absent path has a parent");
    normalize(parent).join(path.file_name().expect("absent path has a file name"))
}

fn write(dir: &Path, relative: &str, text: &str) {
    let path = dir.join(relative);
    std::fs::create_dir_all(path.parent().expect("fixture has a parent")).expect("create fixture dir");
    std::fs::write(path, text).expect("write fixture");
}

fn yaml(text: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::from_str(text).expect("test YAML parses")
}

/// Resolves a schema with no slots.
fn resolve_static(dir: &Path, schema: &str) -> Result<ResolvedSchema, SchemaError> {
    resolve_yaml_schema(&yaml(schema), dir, &FileResolutionContext::new(dir), ShippedSources::NONE)
}

/// Resolves a template schema through `host`, observing the replay.
fn resolve_template(dir: &Path, schema: &str, host: &Host) -> (Result<ResolvedSchema, SlotResolutionError>, Observed) {
    let context = FileResolutionContext::new(dir);
    let pending = match resolve_yaml_schema(&yaml(schema), dir, &context, ShippedSources::NONE) {
        Err(SchemaError::UnresolvedSlots(unresolved)) => unresolved.pending.expect("public entry attaches a pending schema"),
        other => panic!("expected an unresolved template, got {other:?}"),
    };
    observed::collect(|| pending.resolve(host, &SlotRunRequest { context }))
}

fn json_text(resolved: &ResolvedSchema) -> String {
    resolved.json_schema.to_string()
}

fn refused_codes(error: &SlotResolutionError) -> Vec<Option<&'static str>> {
    match error {
        SlotResolutionError::Refused(refusals) => refusals.iter().map(|(_, refusal)| refusal.code).collect(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Writes the self-recursive `Person` beside `schema.yaml` holding `who`.
fn person_fixture(dir: &Path) {
    write(
        dir,
        "lib/types.yaml",
        "kind: schema\ntypes:\n  Person:\n    name: string\n    friend: Person@this\n",
    );
    write(dir, "schema.yaml", "$schema:\n  who: \"{{ t }}\"\n");
    write(dir, "static.yaml", "$schema:\n  who: Person@./lib/types.yaml\n");
}

// --- 1. The defining file is the base -------------------------------------

#[test]
fn inline_document_slot_imports_from_the_resolution_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "types.yaml", "kind: schema\ntypes:\n  Person:\n    from_top_dir: string\n");
    write(dir.path(), "sub/types.yaml", "kind: schema\ntypes:\n  Person:\n    from_sub_dir: number\n");
    let host = Host::new(Gate::denying(&[]), &[("t", "Person@./types.yaml")]);

    let (result, _) = resolve_template(dir.path(), "who: \"{{ t }}\"\n", &host);

    let resolved = result.expect("template type resolves");
    assert!(json_text(&resolved).contains("from_top_dir"), "{}", resolved.json_schema);
    assert_eq!(*host.defining_files.borrow(), vec![None], "an inline slot has no defining file");
    let expected = resolve_static(dir.path(), "who: Person@./types.yaml\n").expect("static resolves");
    assert_eq!(resolved.json_schema, expected.json_schema);
    assert_eq!(resolved.imports, vec![normalize(&dir.path().join("types.yaml"))]);
}

#[test]
fn referenced_schema_file_slot_imports_from_its_own_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "types.yaml", "kind: schema\ntypes:\n  Person:\n    from_top_dir: string\n");
    write(dir.path(), "sub/types.yaml", "kind: schema\ntypes:\n  Person:\n    from_sub_dir: number\n");
    write(dir.path(), "sub/schema.yaml", "$schema:\n  who: \"{{ t }}\"\n");
    write(dir.path(), "sub/static.yaml", "$schema:\n  who: Person@./types.yaml\n");
    let host = Host::new(Gate::denying(&[]), &[("t", "Person@./types.yaml")]);

    let (result, _) = resolve_template(dir.path(), "./sub/schema.yaml", &host);

    let resolved = result.expect("template type resolves");
    let text = json_text(&resolved);
    assert!(text.contains("from_sub_dir") && !text.contains("from_top_dir"), "{text}");
    assert_eq!(*host.defining_files.borrow(), vec![Some(normalize(&dir.path().join("sub/schema.yaml")))]);
    let expected = resolve_static(dir.path(), "./sub/static.yaml").expect("static resolves");
    assert_eq!(resolved.json_schema, expected.json_schema);
    assert_eq!(resolved.imports, vec![normalize(&dir.path().join("sub/types.yaml"))]);
}

#[test]
fn slot_inside_an_imported_named_type_imports_from_that_files_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "types.yaml", "kind: schema\ntypes:\n  Person:\n    from_top_dir: string\n");
    write(dir.path(), "nested/types.yaml", "kind: schema\ntypes:\n  Person:\n    from_nested_dir: boolean\n");
    write(dir.path(), "nested/defs.yaml", "kind: schema\ntypes:\n  State: \"{{ t }}\"\n");
    write(dir.path(), "nested/static_defs.yaml", "kind: schema\ntypes:\n  State: Person@./types.yaml\n");
    let host = Host::new(Gate::denying(&[]), &[("t", "Person@./types.yaml")]);

    let (result, _) = resolve_template(dir.path(), "x: State@./nested/defs.yaml\n", &host);

    let resolved = result.expect("template type resolves");
    let text = json_text(&resolved);
    assert!(text.contains("from_nested_dir") && !text.contains("from_top_dir"), "{text}");
    assert_eq!(*host.defining_files.borrow(), vec![Some(normalize(&dir.path().join("nested/defs.yaml")))]);
    let expected = resolve_static(dir.path(), "x: State@./nested/static_defs.yaml\n").expect("static resolves");
    assert_eq!(resolved.json_schema, expected.json_schema);
    assert!(resolved.imports.contains(&normalize(&dir.path().join("nested/types.yaml"))), "{:?}", resolved.imports);
    assert!(!resolved.imports.contains(&normalize(&dir.path().join("types.yaml"))), "{:?}", resolved.imports);
}

// --- 2. A data value never changes the base -------------------------------

/// Reads its value from a data file in another directory, the way a
/// `load_data` slot would; it never says where the value came from.
struct DataFileHost {
    gate: Rc<Gate>,
    data_file: PathBuf,
}

impl SlotHost for DataFileHost {
    fn admit(&self, _: &Slot) -> Result<(), SlotAdmissionError> {
        Ok(())
    }

    fn resolve(&self, _: &SlotRequest<'_>) -> Result<Value, SlotFailure> {
        let text = std::fs::read_to_string(&self.data_file).expect("data file reads");
        Ok(json!(text.trim()))
    }

    fn template_reads(&self) -> Rc<dyn TemplateReads> {
        self.gate.clone()
    }
}

#[test]
fn a_value_read_from_a_data_file_elsewhere_resolves_against_the_defining_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "data/value.txt", "Person@./types.yaml\n");
    write(dir.path(), "data/types.yaml", "kind: schema\ntypes:\n  Person:\n    from_data_dir: string\n");
    write(dir.path(), "schemas/types.yaml", "kind: schema\ntypes:\n  Person:\n    from_schema_dir: number\n");
    write(dir.path(), "schemas/schema.yaml", "$schema:\n  who: \"{{ t }}\"\n");
    let gate = Gate::denying(&[]);
    let host = DataFileHost { gate: gate.clone(), data_file: dir.path().join("data/value.txt") };
    let context = FileResolutionContext::new(dir.path());
    let pending = match resolve_yaml_schema(&yaml("./schemas/schema.yaml"), dir.path(), &context, ShippedSources::NONE) {
        Err(SchemaError::UnresolvedSlots(unresolved)) => unresolved.pending.expect("pending schema"),
        other => panic!("expected an unresolved template, got {other:?}"),
    };

    let resolved = pending.resolve(&host, &SlotRunRequest { context }).expect("template type resolves");

    let text = json_text(&resolved);
    assert!(text.contains("from_schema_dir") && !text.contains("from_data_dir"), "{text}");
    let data_types = normalize(&dir.path().join("data/types.yaml"));
    assert!(
        gate.events().iter().all(|event| match event {
            GateEvent::Admit(_, path) | GateEvent::Observe { path, .. } => *path != data_types,
        }),
        "the data file's directory is never consulted: {:?}",
        gate.events()
    );
}

// --- 3. Local references inside an imported file --------------------------

#[test]
fn local_references_inside_an_imported_file_resolve_in_that_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "lib/types.yaml",
        "kind: schema\ntypes:\n  Person:\n    friend: Other@this\n    buddy: Other\n  Other:\n    label: string(required)\n",
    );
    write(dir.path(), "schema.yaml", "$schema:\n  who: \"{{ t }}\"\n");
    write(dir.path(), "static.yaml", "$schema:\n  who: Person@./lib/types.yaml\n");
    let host = Host::new(Gate::denying(&[]), &[("t", "Person@./lib/types.yaml")]);

    let (result, _) = resolve_template(dir.path(), "./schema.yaml", &host);

    let resolved = result.expect("local references of the imported file resolve");
    assert!(json_text(&resolved).contains("label"), "{}", resolved.json_schema);
    let expected = resolve_static(dir.path(), "./static.yaml").expect("static resolves");
    assert_eq!(resolved.json_schema, expected.json_schema);
}

// --- 4. Local references in returned text ---------------------------------

#[test]
fn returned_local_references_are_rejected_as_local_reference() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "schema.yaml", "$schema:\n  who: \"{{ t }}\"\ntypes:\n  Other: string\n");
    for value in ["Other@this", "Other"] {
        let host = Host::new(Gate::denying(&[]), &[("t", value)]);

        let (result, _) = resolve_template(dir.path(), "./schema.yaml", &host);

        match result {
            Err(SlotResolutionError::Schema(error)) => match *error {
                SchemaError::SlotResult { kind: SlotResultKind::LocalReference(name), .. } => {
                    assert_eq!(name, "Other", "value `{value}`");
                }
                other => panic!("value `{value}`: expected LocalReference, got {other:?}"),
            },
            other => panic!("value `{value}`: expected a schema error, got {other:?}"),
        }
    }
}

// --- 5. Import cycles ------------------------------------------------------

#[test]
fn an_alias_cycle_in_returned_imports_is_the_static_cycle_error_not_a_refusal() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "a.yaml", "kind: schema\ntypes:\n  A: B@./b.yaml\n");
    write(dir.path(), "b.yaml", "kind: schema\ntypes:\n  B: A@./a.yaml\n");
    let static_error = resolve_static(dir.path(), "x: A@./a.yaml\n").expect_err("static cycle fails");
    assert!(
        matches!(static_error, SchemaError::ImportCycle { .. } | SchemaError::NonDescendingCycle { .. }),
        "static: {static_error:?}"
    );
    let host = Host::new(Gate::denying(&[]), &[("t", "A@./a.yaml")]);

    let (result, _) = resolve_template(dir.path(), "x: \"{{ t }}\"\n", &host);

    match result {
        Err(SlotResolutionError::Schema(error)) => {
            assert_eq!(
                std::mem::discriminant(&*error),
                std::mem::discriminant(&static_error),
                "template {error:?} vs static {static_error:?}"
            );
        }
        other => panic!("expected the import cycle, got {other:?}"),
    }
}

// --- 6. Self-recursive imported type --------------------------------------

#[test]
fn a_self_recursive_imported_type_resolves_through_defs() {
    let dir = tempfile::tempdir().expect("tempdir");
    person_fixture(dir.path());
    let host = Host::new(Gate::denying(&[]), &[("t", "Person@./lib/types.yaml")]);

    let (result, _) = resolve_template(dir.path(), "./schema.yaml", &host);

    let resolved = result.expect("recursive import resolves");
    assert!(resolved.json_schema["$defs"]["Person"].is_object(), "{}", resolved.json_schema);
    assert!(json_text(&resolved).contains("\"$ref\""), "{}", resolved.json_schema);
    let expected = resolve_static(dir.path(), "./static.yaml").expect("static resolves");
    assert_eq!(resolved.json_schema, expected.json_schema);
}

// --- 7. Read once ----------------------------------------------------------

#[cfg(feature = "work-counters")]
#[test]
fn a_shared_import_of_two_template_types_is_opened_once() {
    let dir = tempfile::tempdir().expect("tempdir");
    person_fixture(dir.path());
    write(dir.path(), "two.yaml", "$schema:\n  who: \"{{ t }}\"\n  whom: \"{{ u }}\"\n");
    let host = Host::new(Gate::denying(&[]), &[("t", "Person@./lib/types.yaml"), ("u", "Person@./lib/types.yaml")]);

    let (result, observed) = resolve_template(dir.path(), "./two.yaml", &host);

    result.expect("both template types resolve");
    let types = normalize(&dir.path().join("lib/types.yaml"));
    let opens = observed.opened.iter().filter(|path| **path == types).count();
    assert_eq!(opens, 1, "opened: {:?}", observed.opened);
}

// --- 8. Dependencies -------------------------------------------------------

#[test]
fn imports_and_searched_record_what_the_returned_definition_read() {
    let dir = tempfile::tempdir().expect("tempdir");
    person_fixture(dir.path());
    let gate = Gate::denying(&[]);
    let host = Host::new(gate.clone(), &[("t", "Person@./lib/types.yaml")]);

    let (result, _) = resolve_template(dir.path(), "./schema.yaml", &host);

    let resolved = result.expect("template type resolves");
    let types = normalize(&dir.path().join("lib/types.yaml"));
    assert_eq!(resolved.imports, vec![types.clone()]);
    let searched: Vec<PathBuf> = resolved.searched.iter().map(|path| normalize(path)).collect();
    assert!(searched.contains(&types), "searched: {searched:?}");
    let probed: Vec<PathBuf> = gate
        .observed()
        .into_iter()
        .filter(|(kind, ..)| *kind == SchemaReadKind::Probe)
        .map(|(_, path, ..)| path)
        .collect();
    assert!(!probed.is_empty(), "the gate observed the probes: {:?}", gate.events());
    for path in &probed {
        assert!(searched.contains(path), "probe {path:?} missing from searched {searched:?}");
    }
    // Each read is admitted once, probe before content; a later reuse of the
    // memoized file may be observed again but is never asked again.
    let events = gate.events();
    let admits: Vec<&GateEvent> = events.iter().filter(|event| matches!(event, GateEvent::Admit(..))).collect();
    assert_eq!(
        admits,
        vec![&GateEvent::Admit(SchemaReadKind::Probe, types.clone()), &GateEvent::Admit(SchemaReadKind::Content, types.clone())]
    );
    assert_eq!(
        events[..4],
        [
            GateEvent::Admit(SchemaReadKind::Probe, types.clone()),
            GateEvent::Observe { kind: SchemaReadKind::Probe, path: types.clone(), exists: true, has_bytes: false },
            GateEvent::Admit(SchemaReadKind::Content, types.clone()),
            GateEvent::Observe { kind: SchemaReadKind::Content, path: types.clone(), exists: true, has_bytes: true },
        ]
    );
    for (kind, path, exists, has_bytes) in gate.observed() {
        assert_eq!(path, types);
        assert!(exists && has_bytes == (kind == SchemaReadKind::Content), "{:?}", gate.events());
    }
}

#[test]
fn a_missing_returned_import_is_still_observed_as_probed() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "schema.yaml", "$schema:\n  who: \"{{ t }}\"\n");
    let gate = Gate::denying(&[]);
    let host = Host::new(gate.clone(), &[("t", "Missing@./nope.yaml")]);

    let (result, observed) = resolve_template(dir.path(), "./schema.yaml", &host);

    let error = result.expect_err("a missing import fails");
    assert!(!matches!(error, SlotResolutionError::Refused(_)), "{error:?}");
    let nope = normalize(&dir.path().join("nope.yaml"));
    assert!(gate.admitted(SchemaReadKind::Probe, &nope), "{:?}", gate.events());
    assert!(
        gate.observed().contains(&(SchemaReadKind::Probe, nope.clone(), false, false)),
        "the failed probe is observed with exists == false: {:?}",
        gate.events()
    );
    let searched: Vec<PathBuf> = observed.searched.iter().map(|path| normalize(path)).collect();
    assert!(searched.contains(&nope), "searched: {searched:?}");
}

// --- 9. Refusals -----------------------------------------------------------

#[test]
fn a_refused_probe_stops_before_any_read_and_keeps_its_code() {
    let dir = tempfile::tempdir().expect("tempdir");
    person_fixture(dir.path());
    let gate = Gate::denying(&["types.yaml"]);
    let host = Host::new(gate.clone(), &[("t", "Person@./lib/types.yaml")]);

    let (result, observed) = resolve_template(dir.path(), "./schema.yaml", &host);

    let types = normalize(&dir.path().join("lib/types.yaml"));
    assert_eq!(gate.events(), vec![GateEvent::Admit(SchemaReadKind::Probe, types.clone())]);
    // Without `work-counters` there is no open log to read.
    #[cfg(feature = "work-counters")]
    assert!(!observed.opened.contains(&types), "opened: {:?}", observed.opened);
    #[cfg(not(feature = "work-counters"))]
    let _ = observed;
    let error = result.expect_err("a refusal never yields a schema");
    assert_eq!(refused_codes(&error), vec![Some(DENIED)]);
}

#[test]
fn a_refusal_wins_even_when_a_static_property_imports_the_same_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    person_fixture(dir.path());
    write(dir.path(), "mixed.yaml", "$schema:\n  other: Person@./lib/types.yaml\n  who: \"{{ t }}\"\n");
    resolve_static(dir.path(), "other: Person@./lib/types.yaml\n").expect("the static import alone reads fine");
    let gate = Gate::denying(&["types.yaml"]);
    let host = Host::new(gate.clone(), &[("t", "Person@./lib/types.yaml")]);

    let (result, _) = resolve_template(dir.path(), "./mixed.yaml", &host);

    let error = result.expect_err("the template type's read is refused");
    assert_eq!(refused_codes(&error), vec![Some(DENIED)]);
    let types = normalize(&dir.path().join("lib/types.yaml"));
    assert!(gate.admitted(SchemaReadKind::Probe, &types), "{:?}", gate.events());
}

// --- 10. Examples reached by a returned import ----------------------------

fn example_fixture(dir: &Path) {
    write(dir, "lib/types.yaml", "kind: schema\ntypes:\n  Code: \"string(example(./ex.yaml))\"\n");
    write(dir, "lib/ex.yaml", VALID_EXAMPLE);
    write(dir, "schema.yaml", "$schema:\n  who: \"{{ t }}\"\n");
    write(dir, "static.yaml", "$schema:\n  who: Code@./lib/types.yaml\n");
}

#[test]
fn an_example_reached_by_a_returned_import_is_gated() {
    let dir = tempfile::tempdir().expect("tempdir");
    example_fixture(dir.path());
    let gate = Gate::denying(&[]);
    let host = Host::new(gate.clone(), &[("t", "Code@./lib/types.yaml")]);

    let (result, _) = resolve_template(dir.path(), "./schema.yaml", &host);

    let resolved = result.expect("template type with an example resolves");
    let example = normalize(&dir.path().join("lib/ex.yaml"));
    let events = gate.events();
    let probe = events.iter().position(|e| *e == GateEvent::Admit(SchemaReadKind::Probe, example.clone()));
    let content = events.iter().position(|e| *e == GateEvent::Admit(SchemaReadKind::Content, example.clone()));
    assert!(matches!((probe, content), (Some(p), Some(c)) if p < c), "probe then content: {events:?}");
    assert!(resolved.examples.contains(&example), "examples: {:?}", resolved.examples);
    let expected = resolve_static(dir.path(), "./static.yaml").expect("static resolves");
    assert_eq!(resolved.json_schema, expected.json_schema);
}

#[test]
fn refusing_only_the_example_refuses_the_resolution() {
    let dir = tempfile::tempdir().expect("tempdir");
    example_fixture(dir.path());
    let gate = Gate::denying(&["ex.yaml"]);
    let host = Host::new(gate.clone(), &[("t", "Code@./lib/types.yaml")]);

    let (result, _) = resolve_template(dir.path(), "./schema.yaml", &host);

    let error = result.expect_err("the example's read is refused");
    assert_eq!(refused_codes(&error), vec![Some(DENIED)]);
    assert!(gate.admitted(SchemaReadKind::Content, &dir.path().join("lib/types.yaml")), "{:?}", gate.events());
}

// --- 11. Examples inside tuple positions ----------------------------------

const TUPLE_ELEMENT: &str = "tuple([string(example(./ex.yaml))])";
const TUPLE_SPREAD: &str = "tuple([number, ...string(example(./ex.yaml))[]])";

/// An `ex.yaml` the example loader rejects, so a test that reads it fails.
const INVALID_EXAMPLE: &str = "kind: example\n";

/// The two places a slot is written: inline (no defining file) and in
/// `schema.yaml`, which this writes into `dir`.
fn tuple_schemas(dir: &Path) -> [&'static str; 2] {
    write(dir, "schema.yaml", "$schema:\n  who: \"{{ t }}\"\n");
    ["who: \"{{ t }}\"\n", "./schema.yaml"]
}

#[test]
fn an_example_in_a_returned_tuple_position_is_gated_and_observed() {
    for returned in [TUPLE_ELEMENT, TUPLE_SPREAD] {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "ex.yaml", VALID_EXAMPLE);
        for schema in tuple_schemas(dir.path()) {
            let gate = Gate::denying(&[]);
            let host = Host::new(gate.clone(), &[("t", returned)]);

            let (result, _) = resolve_template(dir.path(), schema, &host);

            let resolved = result.unwrap_or_else(|error| panic!("{returned} in {schema}: {error:?}"));
            let example = normalize(&dir.path().join("ex.yaml"));
            let events = gate.events();
            let probe = events.iter().position(|e| *e == GateEvent::Admit(SchemaReadKind::Probe, example.clone()));
            let content = events.iter().position(|e| *e == GateEvent::Admit(SchemaReadKind::Content, example.clone()));
            assert!(matches!((probe, content), (Some(p), Some(c)) if p < c), "{returned} in {schema}: {events:?}");
            assert!(
                gate.observed().contains(&(SchemaReadKind::Content, example.clone(), true, true)),
                "{returned} in {schema}: the example's content read is observed: {events:?}"
            );
            assert!(resolved.examples.contains(&example), "{returned} in {schema}: {:?}", resolved.examples);
        }
    }
}

#[test]
fn refusing_an_example_in_a_returned_tuple_position_refuses_the_resolution() {
    for returned in [TUPLE_ELEMENT, TUPLE_SPREAD] {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "ex.yaml", VALID_EXAMPLE);
        for schema in tuple_schemas(dir.path()) {
            let gate = Gate::denying(&["ex.yaml"]);
            let host = Host::new(gate.clone(), &[("t", returned)]);

            let (result, observed) = resolve_template(dir.path(), schema, &host);

            let error = result.expect_err("the example's read is refused");
            assert_eq!(refused_codes(&error), vec![Some(DENIED)], "{returned} in {schema}");
            let example = normalize(&dir.path().join("ex.yaml"));
            assert!(
                !gate.admitted(SchemaReadKind::Content, &example),
                "{returned} in {schema}: refused at its probe: {:?}",
                gate.events()
            );
            #[cfg(feature = "work-counters")]
            assert!(!observed.opened.contains(&example), "{returned} in {schema}: opened {:?}", observed.opened);
            #[cfg(not(feature = "work-counters"))]
            let _ = observed;
        }
    }
}

/// `lib/types.yaml` holding `Pair` (an element example), `Rest` (a spread
/// example), and `Tree` (a recursive type, stored once, whose tuple holds
/// one), a valid `lib/ex.yaml` beside it, and an invalid outer `ex.yaml` that
/// only a wrong base directory would read.
fn imported_tuple_fixture(dir: &Path) {
    write(
        dir,
        "lib/types.yaml",
        &format!(
            "kind: schema\ntypes:\n  Pair: \"{TUPLE_ELEMENT}\"\n  Rest: \"{TUPLE_SPREAD}\"\n  \
             Tree:\n    leaf: \"{TUPLE_SPREAD}\"\n    kids: Tree[]@this\n"
        ),
    );
    write(dir, "lib/ex.yaml", VALID_EXAMPLE);
    write(dir, "ex.yaml", INVALID_EXAMPLE);
}

#[test]
fn an_example_in_an_imported_tuple_position_reads_from_the_defining_file() {
    for name in ["Pair", "Rest", "Tree"] {
        let dir = tempfile::tempdir().expect("tempdir");
        imported_tuple_fixture(dir.path());
        let reference = format!("{name}@./lib/types.yaml");
        let gate = Gate::denying(&[]);
        let host = Host::new(gate.clone(), &[("t", reference.as_str())]);

        let (result, _) = resolve_template(dir.path(), "who: \"{{ t }}\"\n", &host);

        let resolved = result.unwrap_or_else(|error| panic!("{name}: {error:?}"));
        let example = normalize(&dir.path().join("lib/ex.yaml"));
        assert_eq!(resolved.examples, vec![example.clone()], "{name}");
        assert!(gate.admitted(SchemaReadKind::Content, &example), "{name}: {:?}", gate.events());
        assert!(!gate.admitted(SchemaReadKind::Content, &normalize(&dir.path().join("ex.yaml"))), "{name}");
        // The same import written statically reads the same example.
        let expected = resolve_static(dir.path(), &format!("who: {reference}\n")).expect("static resolves");
        assert_eq!(resolved.json_schema, expected.json_schema, "{name}");
        assert_eq!(expected.examples, vec![example], "{name}: the static import keeps its base too");
    }
}

#[test]
fn refusing_an_example_in_an_imported_tuple_position_refuses_the_resolution() {
    for name in ["Pair", "Rest", "Tree"] {
        let dir = tempfile::tempdir().expect("tempdir");
        imported_tuple_fixture(dir.path());
        let reference = format!("{name}@./lib/types.yaml");
        let gate = Gate::denying(&["ex.yaml"]);
        let host = Host::new(gate.clone(), &[("t", reference.as_str())]);

        let (result, _) = resolve_template(dir.path(), "who: \"{{ t }}\"\n", &host);

        let error = result.expect_err("the example's read is refused");
        assert_eq!(refused_codes(&error), vec![Some(DENIED)], "{name}");
        assert!(gate.admitted(SchemaReadKind::Probe, &dir.path().join("lib/ex.yaml")), "{name}: {:?}", gate.events());
    }
}

#[test]
fn an_example_in_a_transitively_imported_tuple_position_is_gated_from_its_own_file() {
    for name in ["Pair", "Rest"] {
        let dir = tempfile::tempdir().expect("tempdir");
        imported_tuple_fixture(dir.path());
        write(dir.path(), "outer.yaml", &format!("kind: schema\ntypes:\n  Outer:\n    inner: {name}@./lib/types.yaml\n"));
        let example = normalize(&dir.path().join("lib/ex.yaml"));

        let gate = Gate::denying(&[]);
        let host = Host::new(gate.clone(), &[("t", "Outer@./outer.yaml")]);
        let (result, _) = resolve_template(dir.path(), "who: \"{{ t }}\"\n", &host);
        let resolved = result.unwrap_or_else(|error| panic!("{name}: {error:?}"));
        assert_eq!(resolved.examples, vec![example.clone()], "{name}");
        assert!(gate.observed().contains(&(SchemaReadKind::Content, example, true, true)), "{name}: {:?}", gate.events());

        let gate = Gate::denying(&["ex.yaml"]);
        let host = Host::new(gate.clone(), &[("t", "Outer@./outer.yaml")]);
        let (result, _) = resolve_template(dir.path(), "who: \"{{ t }}\"\n", &host);
        assert_eq!(refused_codes(&result.expect_err("refused")), vec![Some(DENIED)], "{name}");
    }
}

/// Every other position a returned or imported definition holds an example
/// in is gated the same way: refusing only `ex.yaml` refuses the resolution.
#[test]
fn an_example_in_every_returned_or_imported_position_is_gated() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "ex.yaml", VALID_EXAMPLE);
    write(dir.path(), "lib/ex.yaml", VALID_EXAMPLE);
    write(
        dir.path(),
        "lib/types.yaml",
        "kind: schema\ntypes:\n  Scalar: \"string(example(./ex.yaml))\"\n  List: \"string[](example(./ex.yaml))\"\n  \
         Obj:\n    a: \"string(example(./ex.yaml))\"\n  Arm: \"number | string(example(./ex.yaml))\"\n  \
         Outer:\n    inner: Keys@./deep.yaml\n",
    );
    write(dir.path(), "lib/deep.yaml", "kind: schema\ntypes:\n  Keys:\n    \"<pattern::a>\": \"string(example(./ex.yaml))\"\n");
    let returned = [
        "string(example(./ex.yaml))",
        "string[](example(./ex.yaml))",
        "{ a: string(example(./ex.yaml)) }",
        "number | string(example(./ex.yaml))",
        "{ <pattern::a>: string(example(./ex.yaml)) }",
        "Scalar@./lib/types.yaml",
        "List@./lib/types.yaml",
        "Obj@./lib/types.yaml",
        "Arm@./lib/types.yaml",
        "Outer@./lib/types.yaml",
    ];
    for value in returned {
        let gate = Gate::denying(&["ex.yaml"]);
        let host = Host::new(gate.clone(), &[("t", value)]);

        let (result, _) = resolve_template(dir.path(), "who: \"{{ t }}\"\n", &host);

        assert_eq!(refused_codes(&result.expect_err("refused")), vec![Some(DENIED)], "{value}");
    }
}

/// A function signature and a `property(...)` declaration never lower an
/// example: the returned definition is refused without reading it.
#[test]
fn an_example_a_returned_definition_cannot_lower_is_never_read() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "ex.yaml", VALID_EXAMPLE);
    for value in [
        "function(parameters(a: string(example(./ex.yaml))); returns(string))",
        "function(parameters(a: string); returns(string(example(./ex.yaml))))",
        "schema(property(kind, string(example(./ex.yaml))))",
    ] {
        let gate = Gate::denying(&[]);
        let host = Host::new(gate.clone(), &[("t", value)]);

        let (result, _) = resolve_template(dir.path(), "who: \"{{ t }}\"\n", &host);

        let error = result.expect_err("the returned definition is refused");
        assert!(!matches!(error, SlotResolutionError::Refused(_)), "{value}: {error:?}");
        assert!(!gate.admitted(SchemaReadKind::Content, &dir.path().join("ex.yaml")), "{value}: {:?}", gate.events());
    }
}
