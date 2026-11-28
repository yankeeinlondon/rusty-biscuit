//! A registered trigger's template resolves against its registration's
//! verified bundle: a named type a slot returns is read from the listed
//! member's verified bytes, and a slot that names an unlisted file or one
//! outside the bundle is refused at the slot, even when the read itself was
//! granted and the target exists on the disk. Slot reads still need their
//! grants.

use std::path::Path;
use std::sync::Arc;

use crate::host_fixtures::*;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, DataRuntime};
use darkmatter::markdown::compose::report_diagnostics::template_diagnostics;
use darkmatter::markdown::schemas::slots::SlotSession;
use darkmatter::markdown::schemas::slots::grants::{Grant, GrantDecision, GrantScope, GrantTarget};
use darkmatter::markdown::schemas::{DarkmatterSchemas, frontmatter_as_json};
use simplified_schema::SchemaError;

const HOST: &str = "../host/darkmatter-host.yaml";
const DOCUMENT: &str = "---\nwidget:\n    size: three\n---\nBody.\n";

/// A widget type whose `size` is a string, the opposite of the listed
/// library's number: a schema built from this file accepts `size: three`.
const STRAY: &str = "kind: schema\ntypes:\n    widget:\n        size: string\n";

struct Case {
    fixture: Fixture,
    hosts: Arc<darkmatter::config::registration::RegisteredHosts>,
    session: Arc<SlotSession>,
    schemas: DarkmatterSchemas,
    markdown: darkmatter::markdown::Markdown,
}

impl Case {
    /// The document composed as `md compose` would, with the same hosts
    /// and session.
    fn compose(&self) -> Result<darkmatter::markdown::Markdown, darkmatter::markdown::MarkdownError> {
        let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&self.fixture.path("work"), ""))
            .with_source_file(self.fixture.path("work/doc.md"))
            .with_trigger_schemas(true)
            .with_registered_hosts(Arc::clone(&self.hosts))
            .with_slot_session(Arc::clone(&self.session));
        let request = ComposeRequest::with_context(options, self.fixture.context().for_source(self.fixture.path("work/doc.md")))
            .expect("a request");
        self.markdown.clone().compose_with(&request).map(|(composed, _)| composed)
    }
}

/// The `acme` bundle whose widget trigger types `widget` with the slot
/// returning `reference`, installed with an unlisted `schemas/stray.yaml`
/// and an `outside.yaml` above the bundle, every slot read granted.
fn case(reference: &str) -> Case {
    let fixture = Fixture::new();
    let trigger = format!("kind: schema-trigger\nmatch:\n    widget: object(required)\n$schema:\n    widget: \"{{{{ '{reference}' }}}}\"\n");
    fixture.install_bundle("host", &Bundle::acme().with_file("schemas/widget.yaml", &trigger));
    fixture.write("host/schemas/stray.yaml", STRAY);
    fixture.write("outside.yaml", STRAY);
    let hosts = fixture.hosts(&[HOST]);
    assert!(hosts.diagnostics().is_empty(), "the bundle loads: {:#?}", hosts.diagnostics());
    let grants = ["host/schemas/library.yaml", "host/schemas/stray.yaml", "outside.yaml"].map(|relative| Grant {
        decision: GrantDecision::Allow,
        target: GrantTarget::File(fixture.path(relative)),
        scope: GrantScope::Once,
    });
    let session = Arc::new(SlotSession::new(DataRuntime::new()).with_run_grants(grants));
    let (markdown, context) = fixture.document(&fixture.context(), "work/doc.md", DOCUMENT);
    let schemas = DarkmatterSchemas::new(context).with_registered_hosts(Arc::clone(&hosts)).with_slot_session(Arc::clone(&session));
    Case { fixture, hosts, session, schemas, markdown }
}

/// The refusal `validate` reports, checked to be located at the slot in the
/// trigger file.
fn refused_at_the_slot(case: &Case) -> String {
    let error = case.schemas.validate(&case.markdown).expect_err("the slot's reference is refused");
    assert!(matches!(error, SchemaError::SlotResolution(_)), "{error:?}");
    let trigger = case.fixture.path("host/schemas/widget.yaml");
    let records = template_diagnostics(&error, Path::new("unknown")).expect("a template failure");
    let [record] = records.as_slice() else { panic!("one record: {records:#?}") };
    let source = record.source.as_ref().expect("a located record");
    assert_eq!(source.file.as_deref(), Some(trigger.as_path()), "{record:#?}");
    assert_eq!(source.line, Some(5), "the slot's line: {record:#?}");
    error.to_string()
}

#[test]
fn a_slot_naming_a_listed_member_resolves_from_its_verified_bytes() {
    let case = case("widget@./library.yaml");
    // The disk copy differs from the verified bytes; only the latter count.
    case.fixture.write("host/schemas/library.yaml", STRAY);
    let report = case.schemas.validate(&case.markdown).expect("the template resolves");
    let paths: Vec<&str> = report.problems.iter().map(|problem| problem.path.as_str()).collect();
    assert_eq!(paths, ["/widget/size"], "the listed library's number judges: {:#?}", report.problems);
    let effective = case.schemas.effective_for(&case.markdown).expect("assembles").expect("a schema");
    assert!(!effective.validate(&frontmatter_as_json(&case.markdown)).valid);
    let error = case.compose().expect_err("composition judges with the same schema");
    let darkmatter::markdown::MarkdownError::SchemaValidationFailed { problems, .. } = &error else {
        panic!("a schema failure: {error:?}")
    };
    let paths: Vec<&str> = problems.iter().map(|problem| problem.path.as_str()).collect();
    assert_eq!(paths, ["/widget/size"]);
}

#[test]
fn a_slot_naming_an_unlisted_sibling_is_refused_at_the_slot() {
    let case = case("widget@./stray.yaml");
    let message = refused_at_the_slot(&case);
    assert!(message.contains("stray.yaml") && message.contains("no listed member"), "{message}");
    let error = case.compose().expect_err("composition refuses the same reference");
    assert!(error.to_string().contains("no listed member"), "{error}");
}

#[test]
fn a_slot_naming_a_file_outside_the_bundle_is_refused_at_the_slot() {
    let case = case("widget@../../outside.yaml");
    let message = refused_at_the_slot(&case);
    assert!(message.contains("outside the bundle"), "{message}");
}

/// A resolution is reused only under the revision whose bytes it imported:
/// republishing the listed library changes what the same template means,
/// for the same session and the same trigger text.
#[test]
fn a_republished_member_is_not_served_from_an_earlier_resolution() {
    let case = case("widget@./library.yaml");
    assert!(!case.schemas.validate(&case.markdown).expect("resolves").valid, "the published number judges");

    let republished = LIBRARY.replace("\"number -> The widget's size.\"", "string");
    case.fixture.write_member("host", "schemas/library.yaml", &republished);
    let hosts = case.fixture.hosts(&[HOST]);
    assert!(hosts.diagnostics().is_empty(), "the republished bundle loads: {:#?}", hosts.diagnostics());
    let context = case.fixture.context().for_source(case.fixture.path("work/doc.md"));
    let schemas = DarkmatterSchemas::new(context).with_registered_hosts(hosts).with_slot_session(Arc::clone(&case.session));
    let report = schemas.validate(&case.markdown).expect("resolves");
    assert!(report.valid, "the republished string judges: {:#?}", report.problems);
}

/// The refusal a load of a bundle with `path` holding `text` reports at that
/// member; nothing of the host is published.
fn registration_refusal(path: &str, text: &str) -> String {
    let fixture = Fixture::new();
    fixture.install_bundle("host", &Bundle::acme().with_file(path, text));
    let loaded = fixture.load();
    assert!(loaded.profiles.is_empty(), "nothing is published: {:?}", loaded.profiles);
    let file = fixture.path(&format!("host/{path}"));
    let diagnostic = loaded
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.file == file)
        .unwrap_or_else(|| panic!("a refusal at {}: {:#?}", file.display(), loaded.diagnostics));
    diagnostic.message.clone()
}

/// A YAML kind and a namespace need complete definitions when the
/// registration loads; a slot there is refused, never resolved later.
#[test]
fn a_template_in_a_yaml_kind_or_namespace_is_refused_at_registration() {
    let kind = "kind: yaml-schema\n$schema:\n    kind: literal(roster; required)\n    platforms: \"{{ 'widget@./library.yaml' }}\"\n";
    let message = registration_refusal("schemas/roster.yaml", kind);
    assert!(message.contains("YAML kind definition does not load") && message.contains("slot"), "{message}");

    let namespace = "kind: namespace-schema\nnamespace: run\ndescription: The current pass.\n$schema:\n    count: \"{{ 'widget@./library.yaml' }}\"\n";
    let message = registration_refusal("schemas/run.yaml", namespace);
    assert!(message.contains("does not resolve") && message.contains("unresolved `{{ … }}` slot"), "{message}");
}

/// An embedded registration's template keeps its virtual bundle: its
/// pending payload is bound to `<acme>` at the snapshot's revision, and it
/// never resolves through a host file (no disk fallback for `<acme>/…`).
#[test]
fn an_embedded_template_keeps_its_embedded_boundary() {
    use darkmatter::config::registration::bundle::VerifiedSnapshot;
    use darkmatter::config::registration::{LoadedRegistrations, RegisteredHosts, load_embedded, md_owned_names};

    let trigger = "kind: schema-trigger\nmatch:\n    widget: object(required)\n$schema:\n    widget: \"{{ 'widget@./library.yaml' }}\"\n";
    let bundle = Bundle::acme().with_file("schemas/widget.yaml", trigger);
    let members = bundle.members.iter().map(|(path, text)| (path.clone(), Arc::from(text.as_bytes()))).collect();
    let snapshot = VerifiedSnapshot::from_embedded("acme", MANIFEST_NAME, bundle.manifest().as_bytes(), &members).expect("verifies");
    let revision = snapshot.revision().to_string();
    let profile = load_embedded(snapshot, md_owned_names(), &|_| None).expect("loads");
    let widget = profile.triggers().iter().find(|trigger| trigger.bundle_path == "schemas/widget.yaml").expect("widget");
    let pending = widget.payload.pending.as_ref().expect("a template payload");
    let scope = pending.verified_bundle().expect("bound to its bundle");
    assert_eq!((scope.root(), scope.revision()), (Path::new("<acme>"), revision.as_str()));

    let fixture = Fixture::new();
    let loaded = LoadedRegistrations { profiles: vec![Arc::new(profile)], diagnostics: Vec::new(), watches: Vec::new() };
    let hosts = Arc::new(RegisteredHosts::new(Arc::new(loaded)));
    let (markdown, context) = fixture.document(&fixture.context(), "work/doc.md", DOCUMENT);
    let schemas = DarkmatterSchemas::new(context)
        .with_registered_hosts(hosts)
        .with_slot_session(Arc::new(SlotSession::new(DataRuntime::new())));
    assert!(matches!(schemas.validate(&markdown), Err(SchemaError::UnresolvedSlots(_))), "no file origin, no resolution");
}
