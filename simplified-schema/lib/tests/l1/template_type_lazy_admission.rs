//! A template type's returned definition selects the file a reference names
//! exactly as a static reference does, and the host's read gate
//! (`TemplateReads::admit`) is asked about each location only immediately
//! before the ordered search probes it: a schema root, candidate, or sibling
//! the first-match search never reaches is never asked about.
//!
//! Boundary: the crate's public API, in-process. Each test builds two schema
//! roots, `first` and `second`, and a document folder `doc` in a temporary
//! directory, resolves an inline schema through
//! `resolve_yaml_schema_with_roots`, and answers the pending template with a
//! stub host whose gate grants only paths under the folders it names and
//! records every `admit` and `observe` call. Each success is compared with the
//! same schema written statically.
//!
//! The last section adds an open, unsaved buffer ([`buffers::with`]): a
//! buffer at an earlier candidate is that candidate's match, so a later file
//! on disk neither wins nor is asked about.
//!
//! Every test reads files, so the module needs the `file` feature.
#![cfg(feature = "file")]

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use serde_json::{Value, json};
use simplified_schema::embedded::ShippedSources;
use simplified_schema::resolve::buffers::{self, SourceBuffers};
use simplified_schema::resolve::observed::{self, Observed};
use simplified_schema::resolve::{ResolvedSchema, SelectionOutcome, resolve_yaml_schema_with_roots, select_schema_file};
use simplified_schema::{Prepared, SchemaError, Slot};
use simplified_schema::slots::{
    SchemaRead, SchemaReadKind, SchemaReadOutcome, SlotAdmissionError, SlotFailure, SlotHost, SlotRequest,
    SlotResolutionError, SlotRunRequest, TemplateReads,
};

const UNGRANTED: &str = "test.template-read-ungranted";

const PERSON: &str = "kind: schema\ntypes:\n  Person:\n    name: string\n";

const VALID_EXAMPLE: &str = "kind: example\ninvocation: \"x\"\nreturns: \"val\"\ndescription: d\n";

/// A recording gate granting only paths under `granted`.
struct Gate {
    granted: Vec<PathBuf>,
    admits: RefCell<Vec<(SchemaReadKind, PathBuf)>>,
    observed: RefCell<Vec<(SchemaReadKind, PathBuf, bool)>>,
}

impl Gate {
    fn granting(granted: &[PathBuf]) -> Rc<Self> {
        Rc::new(Self {
            granted: granted.iter().map(|path| normalize(path)).collect(),
            admits: RefCell::default(),
            observed: RefCell::default(),
        })
    }

    fn probes_asked(&self) -> Vec<PathBuf> {
        self.asked(SchemaReadKind::Probe)
    }

    fn asked(&self, kind: SchemaReadKind) -> Vec<PathBuf> {
        self.admits.borrow().iter().filter(|(asked, _)| *asked == kind).map(|(_, path)| path.clone()).collect()
    }

    /// Each observed probe once, in first-observation order: a memoized
    /// selection reused by a later expansion is observed again.
    fn probes_observed(&self) -> Vec<(PathBuf, bool)> {
        let mut probes: Vec<(PathBuf, bool)> = Vec::new();
        for (kind, path, exists) in self.observed.borrow().iter() {
            let probe = (path.clone(), *exists);
            if *kind == SchemaReadKind::Probe && !probes.contains(&probe) {
                probes.push(probe);
            }
        }
        probes
    }
}

impl TemplateReads for Gate {
    fn admit(&self, _: &Slot, read: &SchemaRead<'_>) -> Result<(), SlotAdmissionError> {
        let path = normalize(read.path);
        self.admits.borrow_mut().push((read.kind, path.clone()));
        if self.granted.iter().any(|granted| path.starts_with(granted)) {
            return Ok(());
        }
        Err(SlotAdmissionError { code: Some(UNGRANTED), message: format!("{} is not granted", path.display()), span: None })
    }

    fn observe(&self, _: &Slot, outcome: &SchemaReadOutcome<'_>) {
        self.observed.borrow_mut().push((outcome.kind, normalize(outcome.path), outcome.exists));
    }
}

/// Answers the slot `t` with one definition.
struct Host {
    gate: Rc<Gate>,
    answer: Value,
}

impl SlotHost for Host {
    fn admit(&self, _: &Slot) -> Result<(), SlotAdmissionError> {
        Ok(())
    }

    fn resolve(&self, _: &SlotRequest<'_>) -> Result<Value, SlotFailure> {
        Ok(self.answer.clone())
    }

    fn template_reads(&self) -> Rc<dyn TemplateReads> {
        self.gate.clone()
    }
}

/// The fixture folders: two schema roots and the folder holding the schema.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    first: PathBuf,
    second: PathBuf,
    doc: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = normalize(dir.path());
        let (first, second, doc) = (root.join("first"), root.join("second"), root.join("doc"));
        for folder in [&first, &second, &doc] {
            std::fs::create_dir_all(folder).expect("create fixture folder");
        }
        Self { _dir: dir, root, first, second, doc }
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.root.join(relative);
        std::fs::create_dir_all(path.parent().expect("fixture has a parent")).expect("create fixture dir");
        std::fs::write(path, text).expect("write fixture");
    }

    fn roots(&self) -> Vec<PathBuf> {
        vec![self.first.clone(), self.second.clone()]
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::new(&self.doc)
    }

    /// Resolves `who: <definition>` written statically.
    fn resolve_static(&self, definition: &str, context: &FileResolutionContext) -> (Result<ResolvedSchema, SchemaError>, Observed) {
        let schema = yaml(&format!("who: \"{definition}\"\n"));
        observed::collect(|| resolve_yaml_schema_with_roots(&schema, &self.doc, &self.roots(), context, ShippedSources::NONE))
    }

    /// Resolves `who: "{{ t }}"` with the host answering `definition`.
    fn resolve_template(
        &self,
        definition: &str,
        gate: &Rc<Gate>,
        context: &FileResolutionContext,
    ) -> (Result<ResolvedSchema, SlotResolutionError>, Observed) {
        let schema = yaml("who: \"{{ t }}\"\n");
        let pending =
            match resolve_yaml_schema_with_roots(&schema, &self.doc, &self.roots(), context, ShippedSources::NONE) {
                Err(SchemaError::UnresolvedSlots(unresolved)) => {
                    unresolved.pending.expect("public entry attaches a pending schema")
                }
                other => panic!("expected an unresolved template, got {other:?}"),
            };
        let host = Host { gate: Rc::clone(gate), answer: json!(definition) };
        observed::collect(|| pending.resolve(&host, &SlotRunRequest { context: context.clone() }))
    }

    /// Asserts the template resolves to the static schema for `definition`,
    /// and that both searched the same locations.
    fn assert_matches_static(&self, definition: &str, gate: &Rc<Gate>) -> ResolvedSchema {
        let context = self.context();
        let (expected, _) = self.resolve_static(definition, &context);
        let expected = expected.expect("the static control resolves");
        let (resolved, _) = self.resolve_template(definition, gate, &context);
        let resolved = resolved.unwrap_or_else(|error| {
            panic!("`{definition}` resolves; asked {:?}: {error:?}", gate.admits.borrow())
        });
        assert_eq!(resolved.json_schema, expected.json_schema, "{definition}");
        assert_eq!(normalized(&resolved.searched), normalized(&expected.searched), "{definition}");
        resolved
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

fn normalized<'a>(paths: impl IntoIterator<Item = &'a PathBuf>) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = paths.into_iter().map(|path| normalize(path)).collect();
    paths.sort();
    paths
}

fn yaml(text: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::from_str(text).expect("test YAML parses")
}

fn refused_codes(error: &SlotResolutionError) -> Vec<Option<&'static str>> {
    match error {
        SlotResolutionError::Refused(refusals) => refusals.iter().map(|(_, refusal)| refusal.code).collect(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// --- Bare names stop at the first root holding them -----------------------

#[test]
fn a_returned_bare_name_import_is_asked_only_about_the_first_root_holding_it() {
    let fixture = Fixture::new();
    fixture.write("first/types.yaml", PERSON);
    let gate = Gate::granting(std::slice::from_ref(&fixture.first));

    let resolved = fixture.assert_matches_static("Person@types.yaml", &gate);

    let types = fixture.first.join("types.yaml");
    assert_eq!(gate.probes_asked(), vec![types.clone()]);
    assert_eq!(gate.asked(SchemaReadKind::Content), vec![types.clone()]);
    assert_eq!(gate.probes_observed(), vec![(types.clone(), true)]);
    assert_eq!(normalized(&resolved.searched), vec![types]);
}

#[test]
fn a_returned_explicit_import_is_asked_only_about_its_file() {
    let fixture = Fixture::new();
    fixture.write("first/types.yaml", PERSON);
    let gate = Gate::granting(std::slice::from_ref(&fixture.first));

    fixture.assert_matches_static("Person@../first/types.yaml", &gate);

    assert_eq!(gate.probes_asked(), vec![fixture.first.join("types.yaml")]);
}

#[test]
fn a_returned_bare_name_example_is_asked_only_about_the_first_root_holding_it() {
    let fixture = Fixture::new();
    fixture.write("first/example.yaml", VALID_EXAMPLE);
    let gate = Gate::granting(std::slice::from_ref(&fixture.first));

    fixture.assert_matches_static("string(example(example.yaml))", &gate);

    let example = fixture.first.join("example.yaml");
    assert_eq!(gate.probes_asked(), vec![example.clone()]);
    assert_eq!(gate.asked(SchemaReadKind::Content), vec![example.clone()]);
    assert_eq!(gate.probes_observed(), vec![(example, true)]);
}

#[test]
fn a_returned_explicit_example_is_asked_only_about_its_file() {
    let fixture = Fixture::new();
    fixture.write("first/example.yaml", VALID_EXAMPLE);
    let gate = Gate::granting(std::slice::from_ref(&fixture.first));

    fixture.assert_matches_static("string(example(../first/example.yaml))", &gate);

    assert_eq!(gate.probes_asked(), vec![fixture.first.join("example.yaml")]);
}

#[test]
fn a_transitive_bare_name_import_is_asked_only_about_the_first_root_holding_it() {
    let fixture = Fixture::new();
    fixture.write("first/types.yaml", PERSON);
    fixture.write("first/outer.yaml", "kind: schema\ntypes:\n  Outer:\n    who: Person@types.yaml\n");
    let gate = Gate::granting(std::slice::from_ref(&fixture.first));

    fixture.assert_matches_static("Outer@../first/outer.yaml", &gate);

    assert_eq!(gate.probes_asked(), vec![fixture.first.join("outer.yaml"), fixture.first.join("types.yaml")]);
}

#[test]
fn the_static_bare_name_selection_probes_only_the_first_root_holding_it() {
    let fixture = Fixture::new();
    fixture.write("first/types.yaml", PERSON);
    fixture.write("second/types.yaml", PERSON);

    let (resolved, observed) = fixture.resolve_static("Person@types.yaml", &fixture.context());

    let types = fixture.first.join("types.yaml");
    assert_eq!(normalized(&resolved.expect("the static schema resolves").imports), vec![types.clone()]);
    assert_eq!(normalized(&observed.searched), vec![types]);
}

// --- A location the search does reach still needs a grant -----------------

#[test]
fn a_bare_name_missing_from_the_first_root_asks_about_the_second_before_probing_it() {
    let fixture = Fixture::new();
    fixture.write("second/types.yaml", PERSON);
    let (first_types, second_types) = (fixture.first.join("types.yaml"), fixture.second.join("types.yaml"));
    let only_first = Gate::granting(std::slice::from_ref(&fixture.first));

    let (result, observed) = fixture.resolve_template("Person@types.yaml", &only_first, &fixture.context());

    let error = result.expect_err("the ungranted second root refuses the resolution");
    assert_eq!(refused_codes(&error), vec![Some(UNGRANTED)]);
    assert_eq!(only_first.probes_asked(), vec![first_types.clone(), second_types.clone()]);
    assert!(only_first.asked(SchemaReadKind::Content).is_empty());
    assert_eq!(only_first.probes_observed(), vec![(first_types.clone(), false)]);
    assert_eq!(normalized(&observed.searched), vec![first_types.clone()]);

    let both = Gate::granting(&fixture.roots());
    let resolved = fixture.assert_matches_static("Person@types.yaml", &both);

    assert_eq!(both.probes_asked(), vec![first_types.clone(), second_types.clone()]);
    assert_eq!(both.probes_observed(), vec![(first_types, false), (second_types.clone(), true)]);
    assert_eq!(normalized(&resolved.imports), vec![second_types]);
}

// --- The sibling beside the schema ----------------------------------------

#[test]
fn the_sibling_of_a_bare_name_is_asked_about_only_after_every_root_missed() {
    let fixture = Fixture::new();
    fixture.write("doc/types.yaml", PERSON);
    let sibling = fixture.doc.join("types.yaml");
    let roots_only = Gate::granting(&fixture.roots());

    let (result, _) = fixture.resolve_template("Person@types.yaml", &roots_only, &fixture.context());

    assert_eq!(refused_codes(&result.expect_err("the ungranted sibling refuses")), vec![Some(UNGRANTED)]);
    assert_eq!(
        roots_only.probes_asked(),
        vec![fixture.first.join("types.yaml"), fixture.second.join("types.yaml"), sibling.clone()]
    );

    let everything = Gate::granting(std::slice::from_ref(&fixture.root));
    let (result, _) = fixture.resolve_template("Person@types.yaml", &everything, &fixture.context());
    let (expected, _) = fixture.resolve_static("Person@types.yaml", &fixture.context());

    let error = result.expect_err("a sibling-only bare name fails as statically");
    assert!(!matches!(error, SlotResolutionError::Refused(_)), "{error:?}");
    assert!(format!("{error:?}").contains("BareNameSiblingExists"), "expected the static sibling error, got {error:?}");
    assert!(matches!(expected, Err(SchemaError::BareNameSiblingExists { .. })), "{expected:?}");
    assert!(everything.probes_observed().contains(&(sibling, true)), "{:?}", everything.observed.borrow());
}

#[test]
fn the_sibling_is_never_asked_about_when_a_root_holds_the_name() {
    let fixture = Fixture::new();
    fixture.write("first/types.yaml", PERSON);
    fixture.write("doc/types.yaml", PERSON);
    let gate = Gate::granting(std::slice::from_ref(&fixture.first));

    fixture.assert_matches_static("Person@types.yaml", &gate);

    assert_eq!(gate.probes_asked(), vec![fixture.first.join("types.yaml")]);
}

// --- Multi-candidate references outside the schema roots ------------------

#[test]
fn an_implicit_relative_import_stops_at_its_first_candidate_holding_it() {
    let fixture = Fixture::new();
    fixture.write("doc/lib/types.yaml", PERSON);
    fixture.write("lib/types.yaml", PERSON);
    let context = fixture.context().with_repository_root(&fixture.root);
    let gate = Gate::granting(std::slice::from_ref(&fixture.doc));

    let (resolved, _) = fixture.resolve_template("Person@lib/types.yaml", &gate, &context);

    resolved.unwrap_or_else(|error| panic!("asked {:?}: {error:?}", gate.admits.borrow()));
    assert_eq!(gate.probes_asked(), vec![fixture.doc.join("lib/types.yaml")]);
}

#[test]
fn an_implicit_relative_import_asks_about_its_repository_candidate_only_when_needed() {
    let fixture = Fixture::new();
    fixture.write("lib/types.yaml", PERSON);
    let context = fixture.context().with_repository_root(&fixture.root);
    let (local, repository) = (fixture.doc.join("lib/types.yaml"), fixture.root.join("lib/types.yaml"));
    let local_only = Gate::granting(std::slice::from_ref(&fixture.doc));

    let (result, _) = fixture.resolve_template("Person@lib/types.yaml", &local_only, &context);

    assert_eq!(refused_codes(&result.expect_err("the ungranted fallback refuses")), vec![Some(UNGRANTED)]);
    assert_eq!(local_only.probes_asked(), vec![local.clone(), repository.clone()]);
    assert_eq!(local_only.probes_observed(), vec![(local.clone(), false)]);

    let everything = Gate::granting(std::slice::from_ref(&fixture.root));
    let (resolved, _) = fixture.resolve_template("Person@lib/types.yaml", &everything, &context);

    let resolved = resolved.unwrap_or_else(|error| panic!("asked {:?}: {error:?}", everything.admits.borrow()));
    assert_eq!(normalized(&resolved.imports), vec![repository.clone()]);
    assert_eq!(everything.probes_asked(), vec![local, repository]);
}

// --- An open buffer is a match at its own candidate -----------------------

/// `Person` as a file on disk later in the search declares it: an `age`, not
/// the open buffer's `name`.
const AGED_PERSON: &str = "kind: schema\ntypes:\n  Person:\n    age: number\n";

impl Fixture {
    /// The snapshot with one open buffer at `relative`.
    fn open(&self, relative: &str, text: &str) -> SourceBuffers {
        let mut open = SourceBuffers::default();
        open.insert(&self.root.join(relative), Arc::from(text));
        open
    }

    /// The document folder's context, with the fixture root as the
    /// repository root an implicit relative reference falls back to.
    fn repository_context(&self) -> FileResolutionContext {
        self.context().with_repository_root(&self.root)
    }

    /// The returned definition `definition`, resolved with `open` in force.
    fn resolve_open_template(&self, definition: &str, gate: &Rc<Gate>, open: &SourceBuffers) -> (ResolvedSchema, Observed) {
        let (resolved, observed) =
            buffers::with(open, || self.resolve_template(definition, gate, &self.repository_context()));
        let resolved = resolved.unwrap_or_else(|error| panic!("`{definition}` resolves; asked {:?}: {error:?}", gate.admits.borrow()));
        (resolved, observed)
    }

    /// `definition` written statically, resolved with `open` in force.
    fn resolve_open_static(&self, definition: &str, open: &SourceBuffers) -> (ResolvedSchema, Observed) {
        let (resolved, observed) = buffers::with(open, || self.resolve_static(definition, &self.repository_context()));
        (resolved.unwrap_or_else(|error| panic!("`{definition}` resolves: {error:?}")), observed)
    }
}

/// Whether `resolved` strictly accepts `who` holding `value`.
fn accepts(resolved: &ResolvedSchema, context: &FileResolutionContext, value: Value) -> bool {
    Prepared::new(Arc::new(resolved.json_schema.clone()), None, context.clone())
        .expect("the resolved schema compiles")
        .validate_strict(&json!({ "who": value }))
        .is_ok()
}

/// Asserts `resolved` is the open buffer's `Person`: `name` is accepted and
/// the disk file's `age` rejected.
fn assert_buffer_person(resolved: &ResolvedSchema, context: &FileResolutionContext) {
    assert!(accepts(resolved, context, json!({ "name": "x" })), "{}", resolved.json_schema);
    assert!(!accepts(resolved, context, json!({ "age": 3 })), "{}", resolved.json_schema);
}

#[test]
fn a_returned_import_selects_an_open_buffer_at_its_first_candidate_over_a_later_file() {
    let fixture = Fixture::new();
    fixture.write("lib/types.yaml", AGED_PERSON);
    let open = fixture.open("doc/lib/types.yaml", PERSON);
    let local = fixture.doc.join("lib/types.yaml");
    let everything = Gate::granting(std::slice::from_ref(&fixture.root));

    let (resolved, observed) = fixture.resolve_open_template("Person@lib/types.yaml", &everything, &open);

    assert_eq!(normalized(&resolved.imports), vec![local.clone()]);
    assert_eq!(everything.probes_asked(), vec![local.clone()]);
    assert_eq!(everything.probes_observed(), vec![(local.clone(), true)]);
    assert_eq!(normalized(&observed.searched), vec![local]);
    assert_buffer_person(&resolved, &fixture.repository_context());
}

#[test]
fn a_returned_import_of_an_open_buffer_never_asks_about_the_later_candidate() {
    let fixture = Fixture::new();
    fixture.write("lib/types.yaml", AGED_PERSON);
    let open = fixture.open("doc/lib/types.yaml", PERSON);
    let local = fixture.doc.join("lib/types.yaml");
    let local_only = Gate::granting(std::slice::from_ref(&fixture.doc));

    let (resolved, _) = fixture.resolve_open_template("Person@lib/types.yaml", &local_only, &open);

    assert_eq!(normalized(&resolved.imports), vec![local.clone()]);
    assert_eq!(local_only.probes_asked(), vec![local.clone()]);
    assert_eq!(local_only.asked(SchemaReadKind::Content), vec![local]);
    assert_buffer_person(&resolved, &fixture.repository_context());
}

#[test]
fn a_returned_import_of_an_open_buffer_with_no_later_file_never_probes_further() {
    let fixture = Fixture::new();
    let open = fixture.open("doc/lib/types.yaml", PERSON);
    let local = fixture.doc.join("lib/types.yaml");
    let local_only = Gate::granting(std::slice::from_ref(&fixture.doc));

    let (resolved, observed) = fixture.resolve_open_template("Person@lib/types.yaml", &local_only, &open);

    assert_eq!(normalized(&resolved.imports), vec![local.clone()]);
    assert_eq!(local_only.probes_asked(), vec![local.clone()]);
    assert_eq!(normalized(&observed.searched), vec![local]);
}

#[test]
fn a_transitive_returned_import_selects_the_open_buffer_its_file_names() {
    let fixture = Fixture::new();
    fixture.write("lib/types.yaml", AGED_PERSON);
    fixture.write("doc/outer.yaml", "kind: schema\ntypes:\n  Outer:\n    who: Person@lib/types.yaml\n");
    let open = fixture.open("doc/lib/types.yaml", PERSON);
    let local = fixture.doc.join("lib/types.yaml");
    let everything = Gate::granting(std::slice::from_ref(&fixture.root));

    let (resolved, _) = fixture.resolve_open_template("Outer@./outer.yaml", &everything, &open);

    assert_eq!(normalized(&resolved.imports), normalized(&[local.clone(), fixture.doc.join("outer.yaml")]));
    assert!(!everything.probes_asked().contains(&fixture.root.join("lib/types.yaml")), "{:?}", everything.probes_asked());
    let context = fixture.repository_context();
    assert!(accepts(&resolved, &context, json!({ "who": { "name": "x" } })), "{}", resolved.json_schema);
    assert!(!accepts(&resolved, &context, json!({ "who": { "age": 3 } })), "{}", resolved.json_schema);
}

#[test]
fn a_static_import_selects_an_open_buffer_at_its_first_candidate_over_a_later_file() {
    let fixture = Fixture::new();
    fixture.write("lib/types.yaml", AGED_PERSON);
    let open = fixture.open("doc/lib/types.yaml", PERSON);
    let local = fixture.doc.join("lib/types.yaml");

    let (resolved, observed) = fixture.resolve_open_static("Person@lib/types.yaml", &open);

    assert_eq!(normalized(&resolved.imports), vec![local.clone()]);
    assert_eq!(normalized(&resolved.searched), vec![local.clone()]);
    assert_eq!(normalized(&observed.searched), vec![local]);
    assert_buffer_person(&resolved, &fixture.repository_context());
}

#[test]
fn a_returned_example_selects_an_open_buffer_at_its_first_candidate_over_a_later_file() {
    let fixture = Fixture::new();
    fixture.write("lib/example.yaml", VALID_EXAMPLE);
    let open = fixture.open("doc/lib/example.yaml", VALID_EXAMPLE);
    let local = fixture.doc.join("lib/example.yaml");
    let local_only = Gate::granting(std::slice::from_ref(&fixture.doc));

    let (resolved, _) = fixture.resolve_open_template("string(example(lib/example.yaml))", &local_only, &open);

    assert_eq!(normalized(&resolved.examples), vec![local.clone()]);
    assert_eq!(local_only.probes_asked(), vec![local]);
}

#[test]
fn a_static_example_selects_an_open_buffer_at_its_first_candidate_over_a_later_file() {
    let fixture = Fixture::new();
    fixture.write("lib/example.yaml", VALID_EXAMPLE);
    let open = fixture.open("doc/lib/example.yaml", VALID_EXAMPLE);
    let local = fixture.doc.join("lib/example.yaml");

    let (resolved, _) = fixture.resolve_open_static("string(example(lib/example.yaml))", &open);

    assert_eq!(normalized(&resolved.examples), vec![local.clone()]);
    assert_eq!(normalized(&resolved.searched), vec![local]);
}

#[test]
fn a_root_schema_reference_selects_an_open_buffer_at_its_first_candidate_over_a_later_file() {
    let fixture = Fixture::new();
    fixture.write("lib/schema.yaml", "$schema:\n  age: number\n");
    let open = fixture.open("doc/lib/schema.yaml", "$schema:\n  name: string\n");
    let local = fixture.doc.join("lib/schema.yaml");
    let context = fixture.repository_context();

    let resolved = buffers::with(&open, || {
        resolve_yaml_schema_with_roots(&yaml("lib/schema.yaml"), &fixture.doc, &fixture.roots(), &context, ShippedSources::NONE)
    })
    .expect("the referenced buffer resolves");

    assert_eq!(normalized(&resolved.referenced_files), vec![local.clone()]);
    assert_eq!(normalized(&resolved.searched), vec![local]);
    let properties = &resolved.json_schema["properties"];
    assert!(properties.get("name").is_some() && properties.get("age").is_none(), "{}", resolved.json_schema);
}

#[test]
fn the_public_selection_finds_an_open_buffer_at_its_first_candidate_and_probes_no_further() {
    let fixture = Fixture::new();
    fixture.write("lib/schema.yaml", "$schema:\n  age: number\n");
    let open = fixture.open("doc/lib/schema.yaml", "$schema:\n  name: string\n");
    let local = fixture.doc.join("lib/schema.yaml");

    let selection =
        buffers::with(&open, || select_schema_file("lib/schema.yaml", &fixture.doc, &fixture.roots(), &fixture.repository_context()));

    match &selection.outcome {
        SelectionOutcome::Found(found) => assert_eq!(normalize(found), local),
        other => panic!("expected the open buffer, got {other:?}"),
    }
    assert_eq!(normalized(&selection.attempted), vec![local]);
}

#[test]
fn an_explicit_returned_import_selects_its_open_buffer() {
    let fixture = Fixture::new();
    let open = fixture.open("doc/lib/types.yaml", PERSON);
    let local = fixture.doc.join("lib/types.yaml");
    let local_only = Gate::granting(std::slice::from_ref(&fixture.doc));

    let (resolved, _) = fixture.resolve_open_template("Person@./lib/types.yaml", &local_only, &open);

    assert_eq!(normalized(&resolved.imports), vec![local.clone()]);
    assert_eq!(local_only.probes_asked(), vec![local]);
}

#[test]
fn a_returned_bare_name_import_stops_at_an_open_buffer_in_the_first_root() {
    let fixture = Fixture::new();
    fixture.write("second/types.yaml", AGED_PERSON);
    let open = fixture.open("first/types.yaml", PERSON);
    let first_types = fixture.first.join("types.yaml");
    let only_first = Gate::granting(std::slice::from_ref(&fixture.first));

    let (resolved, _) = fixture.resolve_open_template("Person@types.yaml", &only_first, &open);

    assert_eq!(normalized(&resolved.imports), vec![first_types.clone()]);
    assert_eq!(only_first.probes_asked(), vec![first_types]);
    assert_buffer_person(&resolved, &fixture.repository_context());
}

#[test]
fn a_returned_import_found_only_as_an_open_buffer_records_no_probe_after_it() {
    let fixture = Fixture::new();
    let open = fixture.open("doc/lib/types.yaml", PERSON);
    let local = fixture.doc.join("lib/types.yaml");
    let everything = Gate::granting(std::slice::from_ref(&fixture.root));

    let (resolved, observed) = fixture.resolve_open_template("Person@lib/types.yaml", &everything, &open);

    assert_eq!(normalized(&resolved.imports), vec![local.clone()]);
    assert_eq!(everything.probes_asked(), vec![local.clone()]);
    assert_eq!(everything.probes_observed(), vec![(local.clone(), true)]);
    assert_eq!(normalized(&resolved.searched), vec![local.clone()]);
    assert_eq!(normalized(&observed.searched), vec![local]);
}

#[test]
fn a_returned_import_reads_the_unsaved_edit_of_its_first_file() {
    let fixture = Fixture::new();
    fixture.write("doc/lib/types.yaml", AGED_PERSON);
    fixture.write("lib/types.yaml", AGED_PERSON);
    let open = fixture.open("doc/lib/types.yaml", PERSON);
    let local = fixture.doc.join("lib/types.yaml");
    let local_only = Gate::granting(std::slice::from_ref(&fixture.doc));

    let (resolved, _) = fixture.resolve_open_template("Person@lib/types.yaml", &local_only, &open);

    assert_eq!(normalized(&resolved.imports), vec![local.clone()]);
    assert_eq!(local_only.probes_asked(), vec![local]);
    assert_buffer_person(&resolved, &fixture.repository_context());
}
