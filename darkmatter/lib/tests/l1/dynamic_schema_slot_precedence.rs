//! Only the effective schema's slots resolve: a trigger slot under a
//! top-level property that a higher layer (the document `$schema`, or a
//! later matched trigger) redeclares is never read or asked about.
//!
//! Each case is a Git repository in a temp dir whose `schemas/` folder holds
//! the trigger files and whose `data/os.yaml` is untracked (so no automatic
//! grant applies), with an empty home (no persisted grant) and an explicit
//! data runtime whose observation log is read afterwards. `a.trigger.yaml`
//! selects `topic: sandbox` and declares `os` as an enum read from that
//! file; every document's value is `Windows`, which the file does not list.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::data::{ObservationKind, ReadMark};
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::shell_expansion::{ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest};
use darkmatter::markdown::compose::{
    ComposeFailure, ComposeOptions, ComposeRequest, ComposeSource, DataRuntime, RequestSnapshot, ShellExpansionError,
};
use darkmatter::markdown::reference::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::ReferenceValidationOptions;
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::slots::SlotSession;
use darkmatter::markdown::schemas::slots::grants::{SlotGrantDecision, SlotGrantRequest};
use simplified_schema::SchemaError;

use crate::dynamic_slots_fixtures::git;

const APPROVAL_REQUIRED: &str = "dm.schema.slot_approval_required";
const OS_DATA: &str = "data/os.yaml";
const ARCH_DATA: &str = "data/arch.yaml";
const SANDBOX_TRIGGER: &str = "kind: schema-trigger\nmatch:\n  topic: literal(sandbox; required)\n$schema:\n    os: enum({{ load_data(\"../data/os.yaml\", \"string[]\") }}; required)\n";

/// A repository holding the given files (committed) and the untracked data
/// files, a temp home, and the runtime the work observes into.
struct Case {
    _temp: tempfile::TempDir,
    root: PathBuf,
    home: tempfile::TempDir,
    runtime: DataRuntime,
}

impl Case {
    fn new(files: &[(&str, &str)]) -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(temp.path()).expect("canonical temp dir").join("repo");
        let case = Self { _temp: temp, root, home: tempfile::tempdir().expect("temp home"), runtime: DataRuntime::new() };
        for (name, text) in files {
            case.write(name, text);
        }
        git(&case.root, &["init", "-q", "-b", "main"]);
        git(&case.root, &["add", "-A"]);
        git(&case.root, &["commit", "-q", "-m", "fixture"]);
        // Untracked, so only an explicit grant could allow the reads.
        case.write(OS_DATA, "- macOS\n- Linux\n");
        case.write(ARCH_DATA, "- arm64\n");
        case
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn write(&self, name: &str, text: &str) {
        let path = self.path(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, text).expect("write");
    }

    /// Persists `allow file <name>` for each of `names` in the home's store.
    fn grant_files(&self, names: &[&str]) {
        let lines: String = names
            .iter()
            .map(|name| format!("allow file {}\n", biscuit_file::to_portable_string(&self.path(name))))
            .collect();
        let store = self.home.path().join(".config/darkmatter/schema-read-policy");
        std::fs::create_dir_all(store.parent().expect("store folder")).expect("store folder");
        std::fs::write(&store, lines).expect("grant store");
    }

    fn request(&self, document: &Path) -> ComposeRequest {
        let snapshot = RequestSnapshot::new(&self.root).with_home(Some(self.home.path().to_path_buf())).with_env(HashMap::new());
        let options = ComposeOptions::new()
            .with_source_file(document)
            .with_trigger_schemas(true)
            .with_data_runtime(self.runtime.clone());
        ComposeRequest::prepare(options, &snapshot).expect("request")
    }

    /// Runs `doc` as `md compose` does: the passive reference check, then
    /// the composition.
    fn compose(&self, doc: &str) -> Result<(), Box<ComposeFailure>> {
        // A trigger skipped as invalid would make every case pass vacuously.
        let (schemas, _) = self.schemas(doc, None);
        assert!(schemas.discovery_findings().is_empty(), "{:?}", schemas.discovery_findings());
        let document = self.path(doc);
        let request = self.request(&document);
        let markdown = Markdown::try_from(document.as_path()).expect("document");
        let passive = request.clone().map_options(ComposeOptions::for_reference_check);
        markdown
            .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&passive)))
            .unwrap_or_else(|error| panic!("the reference check passes: {error}"));
        markdown.compose_with_warnings(&request).map(|_| ())
    }

    fn compose_needing_approval(&self, doc: &str) {
        let failure = self.compose(doc).expect_err("an unapproved slot read fails the composition");
        let codes: Vec<String> = error_diagnostics(&failure.error).into_iter().map(|diagnostic| diagnostic.code).collect();
        assert_eq!(codes, vec![APPROVAL_REQUIRED.to_string()], "{}", failure.error);
    }

    /// The reads pre-flight asks a handler about for `doc`.
    fn preflight_questions(&self, doc: &str) -> Vec<SlotGrantRequest> {
        let document = self.path(doc);
        let handler = Arc::new(Recording::default());
        Markdown::try_from(document.as_path())
            .expect("document")
            .compose_slot_approvals(&self.request(&document), Some(handler.clone()))
            .expect("approvals");
        handler.asked()
    }

    /// The schemas `md schema validate` prepares for `doc`: the shipped
    /// baseline, discovered triggers, and a slot session asking `handler`.
    fn schemas(&self, doc: &str, handler: Option<Arc<Recording>>) -> (DarkmatterSchemas, Markdown) {
        let document = self.path(doc);
        let request = self.request(&document);
        let mut session = SlotSession::for_request(&request);
        if let Some(handler) = handler {
            session = session.with_handler(handler);
        }
        let schemas = DarkmatterSchemas::new(request.resolution_context().clone())
            .with_darkmatter_baseline_json_schema()
            .expect("baseline")
            .with_trigger_discovery()
            .expect("triggers")
            .with_slot_session(Arc::new(session));
        let markdown = Markdown::try_from(document.as_path()).expect("document").with_source(ComposeSource::File(document));
        (schemas, markdown)
    }

    /// Asserts nothing observed `name`: no read of its bytes, no entry in
    /// the read set.
    fn assert_unread(&self, name: &str) {
        let entry = biscuit_file::to_portable_string(&self.path(name));
        let observations = self.runtime.observations_since(ReadMark::default());
        let read = observations.iter().find(|observation| {
            observation.entry == entry && matches!(observation.kind, ObservationKind::File | ObservationKind::Url)
        });
        assert!(read.is_none(), "{name} was read: {observations:?}");
        assert!(!self.runtime.read_set().contains(&entry), "{name} is in the read set: {:?}", self.runtime.read_set());
    }

    fn assert_read(&self, name: &str) {
        let entry = biscuit_file::to_portable_string(&self.path(name));
        assert!(self.runtime.read_set().contains(&entry), "{name} is read: {:?}", self.runtime.read_set());
    }
}

/// Records every slot read it is asked about and answers none.
#[derive(Default)]
struct Recording {
    asked: Mutex<Vec<SlotGrantRequest>>,
}

impl Recording {
    fn asked(&self) -> Vec<SlotGrantRequest> {
        self.asked.lock().expect("asked").clone()
    }
}

impl ShellApprovalHandler for Recording {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(ShellApprovalDecision::Deny)
    }

    fn approve_slot_reads(&self, requests: &[SlotGrantRequest]) -> Result<Vec<SlotGrantDecision>, ShellExpansionError> {
        self.asked.lock().expect("asked").extend_from_slice(requests);
        Ok(requests.iter().map(|_| SlotGrantDecision::Unanswered).collect())
    }
}

/// R1-F2.1: the document's inline `os: string` replaces the trigger's.
fn document_override() -> Case {
    Case::new(&[
        ("schemas/a.trigger.yaml", SANDBOX_TRIGGER),
        ("docs/doc.md", "---\n$schema:\n    os: string\ntopic: sandbox\nos: Windows\n---\nBody.\n"),
    ])
}

/// R1-F2.2: `b.trigger.yaml`, layered above `a.trigger.yaml`, replaces it.
fn trigger_override() -> Case {
    Case::new(&[
        ("schemas/a.trigger.yaml", SANDBOX_TRIGGER),
        ("schemas/b.trigger.yaml", "kind: schema-trigger\nmatch:\n  topic: literal(sandbox; required)\n$schema:\n    os: string\n"),
        ("docs/doc.md", "---\ntopic: sandbox\nos: Windows\n---\nBody.\n"),
    ])
}

/// R1-F2.4: the trigger is selected and nothing replaces `os`.
fn no_override() -> Case {
    Case::new(&[("schemas/a.trigger.yaml", SANDBOX_TRIGGER), ("docs/doc.md", "---\ntopic: sandbox\nos: Windows\n---\nBody.\n")])
}

mod compose {
    use super::*;

    #[test]
    fn a_document_redeclaring_the_trigger_property_composes_without_reading_its_data() {
        let case = document_override();
        case.compose("docs/doc.md").unwrap_or_else(|failure| panic!("composes: {}", failure.error));
        case.assert_unread(OS_DATA);
    }

    #[test]
    fn a_later_trigger_redeclaring_the_property_discards_the_earlier_slot() {
        let case = trigger_override();
        case.compose("docs/doc.md").unwrap_or_else(|failure| panic!("composes: {}", failure.error));
        case.assert_unread(OS_DATA);
    }

    /// The order is the registry's (filename order within a root): an
    /// earlier trigger's `os: string` is itself replaced by the later
    /// dynamic one, whose slot still needs approval.
    #[test]
    fn an_earlier_trigger_never_discards_a_later_one() {
        let case = Case::new(&[
            ("schemas/0.trigger.yaml", "kind: schema-trigger\nmatch:\n  topic: literal(sandbox; required)\n$schema:\n    os: string\n"),
            ("schemas/a.trigger.yaml", SANDBOX_TRIGGER),
            ("docs/doc.md", "---\ntopic: sandbox\nos: Windows\n---\nBody.\n"),
        ]);
        case.compose_needing_approval("docs/doc.md");
        case.assert_unread(OS_DATA);
    }

    #[test]
    fn an_unmatched_template_trigger_reads_nothing() {
        let case = Case::new(&[
            ("schemas/a.trigger.yaml", SANDBOX_TRIGGER),
            ("docs/doc.md", "---\ntopic: other\nos: Windows\n---\nBody.\n"),
        ]);
        case.compose("docs/doc.md").unwrap_or_else(|failure| panic!("composes: {}", failure.error));
        case.assert_unread(OS_DATA);
    }

    #[test]
    fn a_selected_trigger_slot_with_no_override_still_needs_approval() {
        let case = no_override();
        case.compose_needing_approval("docs/doc.md");
        case.assert_unread(OS_DATA);
    }

    /// Only the redeclared property is discarded: the payload's other slot
    /// still resolves, so its unapproved read still fails the run.
    #[test]
    fn a_surviving_sibling_slot_of_a_discarded_property_still_needs_approval() {
        let case = Case::new(&[
            (
                "schemas/a.trigger.yaml",
                "kind: schema-trigger\nmatch:\n  topic: literal(sandbox; required)\n$schema:\n    os: enum({{ load_data(\"../data/os.yaml\", \"string[]\") }})\n    arch: enum({{ load_data(\"../data/arch.yaml\", \"string[]\") }})\n",
            ),
            ("docs/doc.md", "---\n$schema:\n    os: string\ntopic: sandbox\nos: Windows\narch: arm64\n---\nBody.\n"),
        ]);
        case.compose_needing_approval("docs/doc.md");
        case.assert_unread(OS_DATA);
        case.assert_unread(ARCH_DATA);
    }

    /// A root-union document replaces a trigger property only when every
    /// arm declares it; an arm that leaves `os` out keeps the trigger's.
    #[test]
    fn a_root_union_document_discards_a_trigger_property_only_when_every_arm_redeclares_it() {
        let every_arm = Case::new(&[
            ("schemas/a.trigger.yaml", SANDBOX_TRIGGER),
            (
                "docs/doc.md",
                "---\n$schema:\n  - os: literal(Windows; required)\n  - os: literal(BeOS; required)\ntopic: sandbox\nos: Windows\n---\nBody.\n",
            ),
        ]);
        every_arm.compose("docs/doc.md").unwrap_or_else(|failure| panic!("composes: {}", failure.error));
        every_arm.assert_unread(OS_DATA);

        let one_arm = Case::new(&[
            ("schemas/a.trigger.yaml", SANDBOX_TRIGGER),
            (
                "docs/doc.md",
                "---\n$schema:\n  - os: literal(Windows; required)\n  - title: string(required)\ntopic: sandbox\nos: Windows\n---\nBody.\n",
            ),
        ]);
        one_arm.compose_needing_approval("docs/doc.md");
        one_arm.assert_unread(OS_DATA);
    }
}

/// Named types a payload imports: a stored definition's slot belongs to the
/// properties that reach it.
mod named_types {
    use super::*;

    /// `Platform` is an object used by two properties, so it is stored once
    /// and both properties reference it.
    const PLATFORM_TYPES: &str = "kind: schema\ntypes:\n    Platform: \"{ os: enum({{ load_data('../data/os.yaml', 'string[]') }}) }\"\n";
    const SHARED_TRIGGER: &str = "kind: schema-trigger\nmatch:\n  topic: literal(sandbox; required)\n$schema:\n    host: Platform@./types.yaml\n    guest: Platform@./types.yaml\n";

    #[test]
    fn a_definition_reached_only_by_discarded_properties_is_never_read() {
        let case = Case::new(&[
            ("schemas/types.yaml", PLATFORM_TYPES),
            ("schemas/a.trigger.yaml", SHARED_TRIGGER),
            (
                "docs/doc.md",
                "---\n$schema:\n    host: unknown\n    guest: unknown\ntopic: sandbox\nhost:\n  os: Windows\nguest:\n  os: Windows\n---\nBody.\n",
            ),
        ]);
        case.compose("docs/doc.md").unwrap_or_else(|failure| panic!("composes: {}", failure.error));
        case.assert_unread(OS_DATA);
    }

    #[test]
    fn the_shared_definition_slot_needs_approval_without_an_override() {
        let case = Case::new(&[
            ("schemas/types.yaml", PLATFORM_TYPES),
            ("schemas/a.trigger.yaml", SHARED_TRIGGER),
            ("docs/doc.md", "---\ntopic: sandbox\nhost:\n  os: Windows\nguest:\n  os: Windows\n---\nBody.\n"),
        ]);
        case.compose_needing_approval("docs/doc.md");
        case.assert_unread(OS_DATA);
    }

    #[test]
    fn a_definition_a_surviving_property_reaches_still_resolves() {
        let case = Case::new(&[
            ("schemas/types.yaml", PLATFORM_TYPES),
            ("schemas/a.trigger.yaml", SHARED_TRIGGER),
            (
                "docs/doc.md",
                "---\n$schema:\n    host: unknown\ntopic: sandbox\nhost:\n  os: Windows\nguest:\n  os: Windows\n---\nBody.\n",
            ),
        ]);
        case.compose_needing_approval("docs/doc.md");
        case.assert_unread(OS_DATA);
    }
}

/// The one selection pre-flight and runtime share.
mod preflight {
    use super::*;

    #[test]
    fn pre_flight_asks_nothing_about_a_slot_the_document_discards() {
        assert_eq!(document_override().preflight_questions("docs/doc.md"), Vec::new());
    }

    #[test]
    fn pre_flight_asks_nothing_about_a_slot_a_later_trigger_discards() {
        assert_eq!(trigger_override().preflight_questions("docs/doc.md"), Vec::new());
    }

    #[test]
    fn pre_flight_asks_about_a_selected_trigger_slot_with_no_override() {
        let case = no_override();
        let asked = case.preflight_questions("docs/doc.md");
        assert_eq!(asked.len(), 1, "{asked:?}");
    }

    /// `pending_templates_for` lists no template for a payload left with no
    /// slot, so an editor never treats the schema as templated.
    #[test]
    fn a_payload_left_without_slots_is_not_a_pending_template() {
        for case in [document_override(), trigger_override()] {
            let (schemas, markdown) = case.schemas("docs/doc.md", None);
            assert!(schemas.pending_templates_for(&markdown).is_empty(), "{:?}", schemas.pending_templates_for(&markdown));
        }
        let (schemas, markdown) = no_override().schemas("docs/doc.md", None);
        assert_eq!(schemas.pending_templates_for(&markdown).len(), 1);
    }
}

/// Validation (`md schema validate`, `md clean`, DMLS) prepares the
/// effective schema the same way.
mod validation {
    use super::*;

    #[test]
    fn validation_with_a_discarded_slot_is_valid_and_asks_nothing() {
        for case in [document_override(), trigger_override()] {
            let handler = Arc::new(Recording::default());
            let (schemas, markdown) = case.schemas("docs/doc.md", Some(handler.clone()));
            assert_eq!(schemas.approve_slot_reads(&markdown).expect("approval step"), 0);
            let report = schemas.validate(&markdown).expect("the effective schema prepares");
            assert!(report.problems.is_empty(), "{:?}", report.problems);
            assert_eq!(handler.asked(), Vec::new());
            case.assert_unread(OS_DATA);
        }
    }

    /// Without any slot session, a schema whose only slot was discarded is
    /// concrete, never pending.
    #[test]
    fn a_discarded_slot_never_leaves_the_effective_schema_pending() {
        for case in [document_override(), trigger_override()] {
            let document = case.path("docs/doc.md");
            let request = case.request(&document);
            let markdown =
                Markdown::try_from(document.as_path()).expect("document").with_source(ComposeSource::File(document.clone()));
            let schemas = DarkmatterSchemas::new(request.resolution_context().clone())
                .with_trigger_discovery()
                .expect("triggers");
            let effective = schemas.effective_for(&markdown).expect("not pending").expect("an effective schema");
            let os = effective.json_schema["properties"]["os"].to_string();
            assert!(os.contains("\"string\"") && !os.contains("enum"), "the document's `os: string` applies: {os}");
        }
        let case = no_override();
        let document = case.path("docs/doc.md");
        let markdown =
            Markdown::try_from(document.as_path()).expect("document").with_source(ComposeSource::File(document.clone()));
        let schemas = DarkmatterSchemas::new(case.request(&document).resolution_context().clone())
            .with_trigger_discovery()
            .expect("triggers");
        assert!(matches!(schemas.effective_for(&markdown), Err(SchemaError::UnresolvedSlots(_))));
    }
}

/// A process-cached resolution of a payload without a discarded property is
/// never served once the document stops redeclaring it, even when that
/// property held no slot (so both resolutions evaluate the same slots).
#[test]
fn a_trigger_resolved_without_a_property_is_cached_apart_from_its_whole_payload() {
    let case = Case::new(&[
        (
            "schemas/a.trigger.yaml",
            "kind: schema-trigger\nmatch:\n  topic: literal(sandbox; required)\n$schema:\n    os: enum({{ load_data(\"../data/os.yaml\", \"string[]\") }})\n    title: string(required)\n",
        ),
        ("docs/doc.md", "---\n$schema:\n    title: unknown\ntopic: sandbox\nos: Linux\n---\nBody.\n"),
    ]);
    case.grant_files(&[OS_DATA]);
    case.compose("docs/doc.md").unwrap_or_else(|failure| panic!("the override composes: {}", failure.error));
    case.assert_read(OS_DATA);
    // The same document, edited to drop its `$schema`.
    case.write("docs/doc.md", "---\ntopic: sandbox\nos: Linux\n---\nBody.\n");
    let failure = case.compose("docs/doc.md").expect_err("the trigger's `title` is required");
    let messages: Vec<String> = error_diagnostics(&failure.error).into_iter().map(|diagnostic| diagnostic.message).collect();
    assert!(messages.iter().any(|message| message.contains("title")), "{messages:?}");
}

/// R1-F2.12: a root-union document `$schema` that is itself a template (an
/// arm holds a pure slot, so no grant is needed) exposes every arm's
/// declarations before it resolves, so pre-flight discards what runtime
/// discards.
mod template_union_document {
    use super::*;

    fn every_arm_redeclares() -> Case {
        Case::new(&[
            ("schemas/a.trigger.yaml", SANDBOX_TRIGGER),
            (
                "docs/doc.md",
                "---\n$schema:\n  - os: \"enum({{ ['Linux', 'Windows'] }}; required)\"\n  - os: literal(BeOS; required)\ntopic: sandbox\nos: Windows\n---\nBody.\n",
            ),
        ])
    }

    fn one_arm_redeclares() -> Case {
        Case::new(&[
            ("schemas/a.trigger.yaml", SANDBOX_TRIGGER),
            (
                "docs/doc.md",
                "---\n$schema:\n  - os: \"enum({{ ['Linux', 'Windows'] }}; required)\"\n  - title: string(required)\ntopic: sandbox\nos: Windows\n---\nBody.\n",
            ),
        ])
    }

    #[test]
    fn pre_flight_asks_nothing_when_every_arm_redeclares_the_trigger_property() {
        let case = every_arm_redeclares();
        assert_eq!(case.preflight_questions("docs/doc.md"), Vec::new());
        case.assert_unread(OS_DATA);
    }

    #[test]
    fn only_the_document_template_is_pending_when_every_arm_redeclares_it() {
        let (schemas, markdown) = every_arm_redeclares().schemas("docs/doc.md", None);
        let templates = schemas.pending_templates_for(&markdown);
        assert_eq!(templates.len(), 1, "{templates:?}");
        assert!(templates[0].pending.is_union_arm(), "{templates:?}");
        assert_eq!(templates[0].pending.declared_properties(), ["os"]);
    }

    #[test]
    fn composition_succeeds_without_reading_the_trigger_data() {
        let case = every_arm_redeclares();
        case.compose("docs/doc.md").unwrap_or_else(|failure| panic!("composes: {}", failure.error));
        case.assert_unread(OS_DATA);
    }

    #[test]
    fn pre_flight_still_asks_when_only_some_arms_redeclare_it() {
        let case = one_arm_redeclares();
        let asked = case.preflight_questions("docs/doc.md");
        assert_eq!(asked.len(), 1, "{asked:?}");
        assert_eq!(asked[0].schema.as_deref(), Some(case.path("schemas/a.trigger.yaml").as_path()), "{asked:?}");
        case.assert_unread(OS_DATA);
    }
}
