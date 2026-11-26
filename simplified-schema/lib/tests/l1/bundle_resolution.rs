//! Resolution under a bundle scope (`resolve::bundle`): every schema source
//! a resolution selects or reads is decided against the bundle's verified
//! byte table before anything is probed or opened.
//!
//! Boundary: the crate's public resolution entry points, in-process. Each
//! test writes an installed bundle into a temporary directory, deliberately
//! giving the disk copy of a member different bytes from the table where it
//! matters, so a result built from the table cannot come from the disk.
//! Every request context is anchored in that directory.
#![cfg(feature = "file")]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use serde_json::{Value, json};
use simplified_schema::embedded::{EmbeddedDocument, ShippedSources};
use simplified_schema::resolve::bundle::{self, BundleReads, BundleRefusal, BundleSources};
use simplified_schema::resolve::{ResolvedSchema, check_declared_types, resolve_yaml_schema, resolve_yaml_schema_with_roots};
use simplified_schema::slots::{
    SchemaRead, SchemaReadOutcome, SlotAdmissionError, SlotFailure, SlotHost, SlotRequest, SlotResolutionError,
    SlotRunRequest, TemplateReads,
};
use simplified_schema::triggers::assemble::resolve_trigger_payload;
use simplified_schema::triggers::{LoadedTrigger, parse_trigger_envelope_from_str};
use simplified_schema::{SchemaError, Slot};

const NUMBER_OWNER: &str = "kind: schema\ntypes:\n  Owner: number\n";
const STRING_OWNER: &str = "kind: schema\ntypes:\n  Owner: string\n";
const VALID_EXAMPLE: &str = "kind: example\ninvocation: \"x\"\nreturns: \"val\"\ndescription: d\n";

static HOST_DOCUMENTS: &[EmbeddedDocument<'static>] =
    &[EmbeddedDocument { path: "host/partials/owner.yaml", text: "kind: schema\ntypes:\n  Owner: boolean\n" }];
const HOST: ShippedSources = ShippedSources::new("host:", "host/partials", HOST_DOCUMENTS);

/// An installed bundle under `dir/bundle`: `listed` are the table's members
/// (and written to the disk unless `disk` overrides them), `disk` extra or
/// differing disk files that are never members.
struct Fixture {
    dir: tempfile::TempDir,
    sources: BundleSources,
}

impl Fixture {
    fn new(listed: &[(&str, &str)], disk: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("bundle");
        for (path, text) in listed.iter().chain(disk) {
            write(&root, path, text);
        }
        let sources = BundleSources::on_disk(&root, members(listed)).expect("valid bundle");
        Self { dir, sources }
    }

    fn root(&self) -> PathBuf {
        self.sources.root().to_path_buf()
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::new(self.dir.path())
    }

    /// Resolves `schema` as if written in a file in the bundle's `schemas/`.
    fn resolve(&self, schema: &str) -> (Result<ResolvedSchema, SchemaError>, BundleReads) {
        let base = self.root().join("schemas");
        bundle::with(&self.sources, || resolve_yaml_schema(&yaml(schema), &base, &self.context(), HOST))
    }
}

fn members(entries: &[(&str, &str)]) -> BTreeMap<String, Arc<[u8]>> {
    entries.iter().map(|(path, text)| ((*path).to_string(), Arc::from(text.as_bytes()))).collect()
}

fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("fixture has a parent")).expect("create fixture dir");
    std::fs::write(path, text).expect("write fixture");
}

fn yaml(text: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::from_str(text).expect("test YAML parses")
}

/// The non-null type of the optional property `owner` of `object`.
fn property_type(object: &Value, name: &str) -> Value {
    let property = &object["properties"][name];
    property["anyOf"]
        .as_array()
        .and_then(|arms| arms.iter().find(|arm| arm["type"] != json!("null")))
        .unwrap_or(property)["type"]
        .clone()
}

fn owner_type(resolved: &ResolvedSchema) -> Value {
    property_type(&resolved.json_schema, "owner")
}

/// The refusal a bundle-refused resolution reports, looking through the
/// wrappers that locate it.
fn refusal(error: &SchemaError) -> Option<BundleRefusal> {
    match error {
        SchemaError::BundleBoundary { boundary, .. } => Some(boundary.refusal),
        SchemaError::ReferencedSchema { source, .. }
        | SchemaError::CompositionReference { source, .. }
        | SchemaError::ComposedMember { source, .. } => refusal(source),
        SchemaError::SlotResolution(failure) => match failure.error.as_ref() {
            simplified_schema::slots::SlotResolutionError::Schema(schema) => refusal(schema),
            _ => None,
        },
        _ => None,
    }
}

#[test]
fn a_listed_import_is_served_from_the_table_not_the_disk() {
    let fixture = Fixture::new(&[("schemas/types.yaml", NUMBER_OWNER)], &[]);
    write(&fixture.root(), "schemas/types.yaml", STRING_OWNER);

    let (resolved, reads) = fixture.resolve("owner: Owner@./types.yaml\n");

    assert_eq!(owner_type(&resolved.expect("listed import resolves")), json!("number"));
    assert_eq!(reads.served, vec!["schemas/types.yaml".to_string()]);
    assert_eq!(reads.attempted, vec![fixture.root().join("schemas/types.yaml")]);
    assert!(reads.missing.is_empty());
    let outside = resolve_yaml_schema(&yaml("owner: Owner@./types.yaml\n"), &fixture.root().join("schemas"), &fixture.context(), HOST)
        .expect("the disk copy resolves outside the scope");
    assert_eq!(owner_type(&outside), json!("string"), "outside the scope the disk is read again");
}

#[test]
fn nested_composition_extends_a_type_imported_from_another_member() {
    let fixture = Fixture::new(
        &[
            ("schemas/base.yaml", "kind: schema\ntypes:\n  base:\n    id: string(required)\n"),
            ("schemas/lib/derived.yaml", "kind: schema\ntypes:\n  derived:\n    $extends: base@../base.yaml\n    extra: number\n"),
        ],
        &[],
    );

    let (resolved, reads) = fixture.resolve("item: derived@./lib/derived.yaml\n");

    let resolved = resolved.expect("composition resolves");
    let item = resolved.json_schema["properties"]["item"].to_string();
    assert!(item.contains("\"id\"") && item.contains("\"extra\""), "{item}");
    let mut served = reads.served.clone();
    served.sort();
    assert_eq!(served, vec!["schemas/base.yaml".to_string(), "schemas/lib/derived.yaml".to_string()]);
}

#[test]
fn a_trigger_payload_indirection_resolves_through_the_table() {
    let trigger_text = "kind: schema-trigger\nmatch:\n  kind: string(required)\n$schema: ./payload.yaml\n";
    let fixture = Fixture::new(
        &[
            ("schemas/trigger.yaml", trigger_text),
            ("schemas/payload.yaml", "$schema:\n  owner: Owner@./types.yaml\n"),
            ("schemas/types.yaml", NUMBER_OWNER),
        ],
        &[],
    );
    write(&fixture.root(), "schemas/payload.yaml", "$schema:\n  owner: boolean\n");
    let path = fixture.sources.path_of("schemas/trigger.yaml").expect("listed trigger");
    let envelope = parse_trigger_envelope_from_str(trigger_text).expect("valid envelope").expect("a trigger");
    let trigger = LoadedTrigger::from_file(path, Arc::from(trigger_text), envelope, fixture.dir.path().to_path_buf());

    let (payload, reads) =
        bundle::with(&fixture.sources, || resolve_trigger_payload(&trigger, &[], &fixture.context(), HOST));

    let payload = payload.expect("payload resolves");
    assert_eq!(property_type(&payload.json_schema, "owner"), json!("number"));
    assert!(reads.served.contains(&"schemas/payload.yaml".to_string()), "{reads:?}");
}

#[test]
fn an_unlisted_target_inside_the_root_is_refused_unread_and_tracked_as_missing() {
    let fixture = Fixture::new(&[("schemas/main.yaml", "$schema:\n  x: string\n")], &[("schemas/types.yaml", STRING_OWNER)]);

    let (resolved, reads) = fixture.resolve("owner: Owner@./types.yaml\n");

    let error = resolved.expect_err("unlisted import is refused");
    assert_eq!(refusal(&error), Some(BundleRefusal::Unlisted), "{error:?}");
    assert!(error.to_string().contains("types.yaml"), "the error names the target: {error}");
    let target = fixture.root().join("schemas/types.yaml");
    assert_eq!(reads.attempted, vec![target.clone()]);
    assert_eq!(reads.missing, vec![target]);
    assert!(reads.served.is_empty(), "nothing was read");
}

#[test]
fn a_path_that_escapes_the_root_is_refused() {
    let fixture = Fixture::new(&[("schemas/main.yaml", "$schema:\n  x: string\n")], &[]);
    write(fixture.dir.path(), "outside.yaml", STRING_OWNER);

    let (resolved, reads) = fixture.resolve("owner: Owner@../../outside.yaml\n");

    assert_eq!(refusal(&resolved.expect_err("escape is refused")), Some(BundleRefusal::Escapes));
    assert_eq!(reads.attempted.len(), 1, "{reads:?}");
    assert!(reads.missing.is_empty() && reads.served.is_empty(), "{reads:?}");
}

#[test]
fn a_remote_reference_is_refused_without_fetching() {
    let fixture = Fixture::new(&[], &[]);

    let (resolved, reads) = fixture.resolve("owner: Owner@https://example.invalid/types.yaml\n");

    assert!(matches!(resolved, Err(SchemaError::RemoteUnsupported { .. })), "{resolved:?}");
    assert_eq!(reads, BundleReads::default());
}

#[test]
fn bare_names_and_rooted_references_are_refused_even_with_schema_roots() {
    let fixture = Fixture::new(&[("schemas/types.yaml", NUMBER_OWNER)], &[]);
    let roots = [fixture.root().join("schemas")];
    let absolute = biscuit_file::to_forward_slash_string(&roots[0].join("types.yaml"));
    for reference in ["types.yaml", "&bundle/schemas/types.yaml", "~/types.yaml", "%./types.yaml", absolute.as_str()] {
        let schema = yaml(&format!("owner: \"Owner@{reference}\"\n"));
        let (resolved, reads) = bundle::with(&fixture.sources, || {
            resolve_yaml_schema_with_roots(&schema, &roots[0], &roots, &fixture.context(), HOST)
        });
        assert_eq!(refusal(&resolved.expect_err(reference)), Some(BundleRefusal::NotRelative), "{reference}");
        assert_eq!(reads, BundleReads::default(), "{reference}: nothing probed");
    }
}

#[test]
fn a_built_in_reference_resolves_outside_the_table() {
    let fixture = Fixture::new(&[], &[]);

    let (resolved, reads) = fixture.resolve("owner: Owner@host:owner\n");

    assert_eq!(owner_type(&resolved.expect("built-in resolves")), json!("boolean"));
    assert_eq!(reads, BundleReads::default());
}

#[test]
fn file_backed_examples_face_the_boundary_and_their_envelope_check() {
    let fixture = Fixture::new(
        &[
            ("schemas/ex.yaml", VALID_EXAMPLE),
            ("schemas/types.yaml", NUMBER_OWNER),
            ("schemas/lib/named.yaml", "kind: schema\ntypes:\n  Named: string(example(../ex.yaml))\n"),
        ],
        &[("schemas/unlisted.yaml", VALID_EXAMPLE)],
    );

    let (member, _) = fixture.resolve("name: string(example(./ex.yaml))\n");
    assert_eq!(refusal(&member.expect_err("example member refused")), Some(BundleRefusal::ExampleMember));

    // An imported type's example is rewritten to the member's path and
    // selected again at lowering; the rewritten spelling keeps its verdict.
    let (imported, _) = fixture.resolve("name: Named@./lib/named.yaml\n");
    assert_eq!(refusal(&imported.expect_err("imported example member refused")), Some(BundleRefusal::ExampleMember));

    let (unlisted, reads) = fixture.resolve("name: string(example(./unlisted.yaml))\n");
    assert_eq!(refusal(&unlisted.expect_err("unlisted example refused")), Some(BundleRefusal::Unlisted));
    assert_eq!(reads.missing, vec![fixture.root().join("schemas/unlisted.yaml")]);
    assert!(reads.served.is_empty());

    let (not_example, _) = fixture.resolve("name: string(example(./types.yaml))\n");
    assert!(matches!(not_example, Err(SchemaError::InvalidExample { .. })), "{not_example:?}");
}

/// `check_declared_types` judges every declaration of a file's `types`
/// table against the bundle, including declarations nothing uses: each
/// import, `$extends`, spread, union arm, and `Name@this` chain, and each
/// file-backed example. Listed members and built-ins still resolve.
#[test]
fn every_declared_type_faces_the_boundary_used_or_not() {
    let fixture = Fixture::new(
        &[
            ("schemas/types.yaml", NUMBER_OWNER),
            ("schemas/base.yaml", "kind: schema\ntypes:\n  Base:\n    id: string\n"),
            ("schemas/lib.yaml", "kind: schema\ntypes:\n  a: string\n"),
        ],
        &[("schemas/unlisted.yaml", STRING_OWNER)],
    );
    write(fixture.dir.path(), "outside.yaml", STRING_OWNER);
    let path = fixture.root().join("schemas/lib.yaml");
    let check = |declarations: &str| {
        let text = format!("kind: schema\ntypes:\n  used: Owner@./types.yaml\n{declarations}");
        bundle::with(&fixture.sources, || check_declared_types(&text, &path, &fixture.context(), HOST))
    };

    for listed in [
        "  unused: Owner@./types.yaml\n",
        "  unused: Owner@host:owner\n",
        "  unused:\n    $extends: Base@./base.yaml\n",
        "  unused: Owner@this\n  Owner: string(example(this))\n",
    ] {
        let (checked, reads) = check(listed);
        checked.unwrap_or_else(|error| panic!("{listed}: {error}"));
        assert!(reads.missing.is_empty(), "{listed}: {reads:?}");
    }

    for (unused, expected) in [
        ("  unused: Owner@./unlisted.yaml\n", BundleRefusal::Unlisted),
        ("  unused: Owner@../../outside.yaml\n", BundleRefusal::Escapes),
        ("  unused:\n    $extends: Base@./unlisted.yaml\n", BundleRefusal::Unlisted),
        ("  unused:\n    ...: base\n  base:\n    o: Owner@./unlisted.yaml\n", BundleRefusal::Unlisted),
        ("  unused: string | Owner@./unlisted.yaml\n", BundleRefusal::Unlisted),
        ("  unused: Hidden@this\n  Hidden: Owner@./unlisted.yaml\n", BundleRefusal::Unlisted),
        ("  unused: string(example(./unlisted.yaml))\n", BundleRefusal::Unlisted),
        ("  unused:\n    nested: string(example(../../outside.yaml))\n", BundleRefusal::Escapes),
    ] {
        let (checked, _) = check(unused);
        let error = checked.expect_err(unused);
        assert_eq!(refusal(&error), Some(expected), "{unused}: {error:?}");
    }

    let (remote, reads) = check("  unused: Owner@https://example.invalid/types.yaml\n");
    assert!(matches!(remote, Err(SchemaError::RemoteUnsupported { .. })), "{remote:?}");
    assert!(reads.served.iter().all(|member| member == "schemas/types.yaml"), "nothing remote is read: {reads:?}");

    let (none, _) = bundle::with(&fixture.sources, || {
        check_declared_types("kind: schema-trigger\nmatch:\n  a: string(required)\n", &path, &fixture.context(), HOST)
    });
    assert_eq!(none.expect("a file without types checks nothing"), Vec::<PathBuf>::new());
}

/// A null `types` is the absent table the member envelope admitted for a
/// `yaml-schema` (blank or written `null`): the declaration check reads it
/// as no named types, never as a table, while a written table is still
/// judged.
#[test]
fn a_null_types_table_declares_nothing_to_check() {
    let fixture = Fixture::new(&[("schemas/types.yaml", NUMBER_OWNER)], &[]);
    let path = fixture.root().join("schemas/kind.yaml");
    let head = "kind: yaml-schema\n$schema:\n  kind: literal(roster; required)\n";
    let check = |types: &str| {
        let text = format!("{head}{types}");
        bundle::with(&fixture.sources, || check_declared_types(&text, &path, &fixture.context(), HOST)).0
    };
    for blank in ["types:\n", "types: null\n", "types: ~\n"] {
        assert_eq!(check(blank).unwrap_or_else(|error| panic!("{blank:?}: {error}")), Vec::<PathBuf>::new(), "{blank:?}");
    }
    check("types:\n  used: Owner@./types.yaml\n").expect("a listed import");
    let unlisted = check("types:\n  unused: Owner@./unlisted.yaml\n").expect_err("an unlisted import");
    assert_eq!(refusal(&unlisted), Some(BundleRefusal::Unlisted), "{unlisted:?}");
}

#[test]
fn an_embedded_bundle_resolves_like_the_installed_one_without_any_file() {
    let sources = BundleSources::embedded(
        "claudine",
        members(&[
            ("schemas/types.yaml", NUMBER_OWNER),
            ("schemas/lib/derived.yaml", "kind: schema\ntypes:\n  derived:\n    $extends: base@../base.yaml\n"),
            ("schemas/base.yaml", "kind: schema\ntypes:\n  base:\n    owner: Owner@./types.yaml\n"),
        ]),
    )
    .expect("valid bundle");
    let dir = tempfile::tempdir().expect("tempdir");
    let context = FileResolutionContext::new(dir.path());
    let base = sources.path_of("schemas/types.yaml").expect("listed").parent().expect("parent").to_path_buf();

    let (resolved, reads) = bundle::with(&sources, || {
        resolve_yaml_schema(&yaml("owner: Owner@./types.yaml\nitem: derived@./lib/derived.yaml\nother: Owner@./absent.yaml\n"), &base, &context, HOST)
    });
    assert_eq!(refusal(&resolved.expect_err("absent member refused")), Some(BundleRefusal::Unlisted));
    assert_eq!(reads.missing, vec![sources.root().join("schemas").join("absent.yaml")]);

    let (resolved, reads) = bundle::with(&sources, || {
        resolve_yaml_schema(&yaml("owner: Owner@./types.yaml\nitem: derived@./lib/derived.yaml\n"), &base, &context, HOST)
    });
    let resolved = resolved.expect("embedded bundle resolves");
    assert_eq!(owner_type(&resolved), json!("number"));
    let item = resolved.json_schema["properties"]["item"].to_string();
    assert!(item.contains("\"number\"") && !item.contains("\"string\""), "{item}");
    assert_eq!(reads.served.len(), 3, "{reads:?}");
    assert!(std::fs::read_dir(dir.path()).expect("tempdir").next().is_none(), "no file was created");
}

#[test]
fn the_scope_input_can_be_carried_to_a_worker_thread() {
    let fixture = Fixture::new(&[("schemas/types.yaml", NUMBER_OWNER)], &[]);
    let (sources, base, context) = (fixture.sources.clone(), fixture.root().join("schemas"), fixture.context());

    let owner = std::thread::spawn(move || {
        let (resolved, _) = bundle::with(&sources, || resolve_yaml_schema(&yaml("owner: Owner@./types.yaml\n"), &base, &context, HOST));
        owner_type(&resolved.expect("resolves on the worker"))
    })
    .join()
    .expect("worker");

    assert_eq!(owner, json!("number"));
}

#[test]
fn malformed_member_keys_and_relative_roots_are_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    for key in ["./schemas/a.yaml", "schemas//a.yaml", "schemas\\a.yaml", "../a.yaml"] {
        assert!(BundleSources::on_disk(dir.path(), members(&[(key, "")])).is_err(), "{key}");
    }
    assert!(BundleSources::on_disk(Path::new("relative/root"), BTreeMap::new()).is_err());
    assert!(BundleSources::embedded("a/b", BTreeMap::new()).is_err());
}

/// Answers every slot with `answer` and admits every template read,
/// recording each admission.
struct Host {
    answer: &'static str,
    gate: Rc<Gate>,
}

#[derive(Default)]
struct Gate {
    admitted: RefCell<Vec<PathBuf>>,
}

impl TemplateReads for Gate {
    fn admit(&self, _: &Slot, read: &SchemaRead<'_>) -> Result<(), SlotAdmissionError> {
        self.admitted.borrow_mut().push(read.path.to_path_buf());
        Ok(())
    }

    fn observe(&self, _: &Slot, _: &SchemaReadOutcome<'_>) {}
}

impl SlotHost for Host {
    fn admit(&self, _: &Slot) -> Result<(), SlotAdmissionError> {
        Ok(())
    }

    fn resolve(&self, _: &SlotRequest<'_>) -> Result<Value, SlotFailure> {
        Ok(json!(self.answer))
    }

    fn template_reads(&self) -> Rc<dyn TemplateReads> {
        self.gate.clone()
    }
}

#[test]
fn a_template_types_returned_import_faces_both_the_gate_and_the_bundle() {
    let fixture = Fixture::new(&[("schemas/types.yaml", NUMBER_OWNER)], &[("schemas/other.yaml", STRING_OWNER)]);
    let base = fixture.root().join("schemas");
    let run = |answer: &'static str| {
        let host = Host { answer, gate: Rc::default() };
        let (result, reads) = bundle::with(&fixture.sources, || {
            let pending = match resolve_yaml_schema(&yaml("owner: \"{{ t }}\"\n"), &base, &fixture.context(), HOST) {
                Err(SchemaError::UnresolvedSlots(unresolved)) => unresolved.pending.expect("pending schema"),
                other => panic!("expected a template, got {other:?}"),
            };
            pending.resolve(&host, &SlotRunRequest { context: fixture.context() })
        });
        (result, reads, host.gate.admitted.borrow().clone())
    };

    let (listed, reads, admitted) = run("Owner@./types.yaml");
    assert_eq!(owner_type(&listed.expect("listed import resolves")), json!("number"));
    assert!(admitted.contains(&base.join("types.yaml")), "the gate is still asked: {admitted:?}");
    assert_eq!(reads.served, vec!["schemas/types.yaml".to_string()]);

    let (unlisted, reads, admitted) = run("Owner@./other.yaml");
    match unlisted.expect_err("unlisted import refused") {
        SlotResolutionError::Refused(refused) => {
            let [(slot, refusal)] = refused.as_slice() else { panic!("one refused slot: {refused:?}") };
            assert_eq!(slot.expression, "t", "refused at the slot that returned the import");
            assert!(refusal.message.contains("no listed member"), "{refusal:?}");
        }
        other => panic!("expected a refusal at the slot, got {other:?}"),
    }
    assert!(admitted.is_empty(), "a refused target is never offered to the gate: {admitted:?}");
    assert!(reads.served.is_empty());
}

/// A template met under a scope carries it: resolved later, outside any
/// scope, its returned import still reads the table, not the disk, and the
/// bundle's revision identifies the bytes it read.
#[test]
fn a_template_resolved_after_its_scope_closed_still_reads_the_bundle() {
    let fixture = Fixture::new(&[("schemas/types.yaml", NUMBER_OWNER)], &[("schemas/other.yaml", STRING_OWNER)]);
    write(&fixture.root(), "schemas/types.yaml", STRING_OWNER);
    let sources = fixture.sources.clone().with_revision("rev-1");
    let base = fixture.root().join("schemas");
    let (met, _) = bundle::with(&sources, || resolve_yaml_schema(&yaml("owner: \"{{ t }}\"\n"), &base, &fixture.context(), HOST));
    let Err(SchemaError::UnresolvedSlots(unresolved)) = met else { panic!("expected a template, got {met:?}") };
    let pending = unresolved.pending.expect("pending schema");
    let carried = pending.verified_bundle().expect("the scope is carried");
    assert_eq!((carried.root(), carried.revision()), (fixture.root().as_path(), "rev-1"));

    let resolve = |answer: &'static str| {
        pending.resolve(&Host { answer, gate: Rc::default() }, &SlotRunRequest { context: fixture.context() })
    };
    assert_eq!(owner_type(&resolve("Owner@./types.yaml").expect("listed import resolves")), json!("number"));
    assert!(matches!(resolve("Owner@./other.yaml"), Err(SlotResolutionError::Refused(_))), "unlisted is refused");
    assert!(matches!(resolve("Owner@../../outside.yaml"), Err(SlotResolutionError::Refused(_))), "escaping is refused");

    let outside = match resolve_yaml_schema(&yaml("owner: \"{{ t }}\"\n"), &base, &fixture.context(), HOST) {
        Err(SchemaError::UnresolvedSlots(unresolved)) => unresolved.pending.expect("pending schema"),
        other => panic!("expected a template, got {other:?}"),
    };
    assert!(outside.verified_bundle().is_none(), "a template met outside a scope carries none");
}
