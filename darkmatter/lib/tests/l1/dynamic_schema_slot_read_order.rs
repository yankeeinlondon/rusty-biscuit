//! A slot's read is judged before it happens, on every path.
//!
//! Each case runs a document as `md compose` does (the passive reference
//! check, then the composition) in a temp folder with no repository, an
//! empty home (so no persisted grant applies unless the case writes one), no
//! approval handler, and an explicit data runtime whose observation log is
//! read after the failure. A content observation with a hash proves bytes
//! were read and a listing observation proves a folder was enumerated, so a
//! refused target must leave neither. `names.yaml` holds `- Linux` and the
//! document's value is `Linux` throughout.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::data::{Observation, ObservationKind, ReadMark};
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::{ComposeFailure, ComposeOptions, ComposeRequest, DataRuntime, RequestSnapshot};
use darkmatter::markdown::reference::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::ReferenceValidationOptions;

const APPROVAL_REQUIRED: &str = "dm.schema.slot_approval_required";
const NAMES: &str = "names.yaml";

/// A temp folder holding `names.yaml`, a temp home, and the runtime the
/// composition observes into.
struct Case {
    _temp: tempfile::TempDir,
    root: PathBuf,
    home: tempfile::TempDir,
    runtime: DataRuntime,
}

impl Case {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(temp.path()).expect("canonical temp dir");
        let case = Self { _temp: temp, root, home: tempfile::tempdir().expect("temp home"), runtime: DataRuntime::new() };
        case.write(NAMES, "- Linux\n");
        case
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn write(&self, name: &str, text: &str) {
        std::fs::write(self.path(name), text).expect("write");
    }

    /// Persists `allow file <name>` in the home's grant store.
    fn grant_file(&self, name: &str) {
        self.grant(&format!("allow file {}\n", biscuit_file::to_portable_string(&self.path(name))));
    }

    /// Persists `allow dir <root>` in the home's grant store.
    fn grant_folder(&self) {
        self.grant(&format!("allow dir {}\n", biscuit_file::to_portable_string(&self.root)));
    }

    fn grant(&self, line: &str) {
        let store = self.home.path().join(".config/darkmatter/schema-read-policy");
        std::fs::create_dir_all(store.parent().expect("store folder")).expect("store folder");
        std::fs::write(&store, line).expect("grant store");
    }

    /// `doc.md` under an inline `$schema` whose `os` property is `os_type`.
    fn inline_document(&self, os_type: &str) -> PathBuf {
        self.write("doc.md", &format!("---\n$schema:\n    os: \"{os_type}\"\nos: Linux\n---\n\nBody.\n"));
        self.path("doc.md")
    }

    /// `doc.md` naming `schema.yaml`, whose `os` property is `os_type`.
    fn referenced_document(&self, os_type: &str) -> PathBuf {
        self.write("schema.yaml", &format!("kind: schema\n$schema:\n    os: \"{os_type}\"\n"));
        self.write("doc.md", "---\n$schema: ./schema.yaml\nos: Linux\n---\n\nBody.\n");
        self.path("doc.md")
    }

    fn request(&self, document: &Path) -> ComposeRequest {
        let snapshot = RequestSnapshot::new(&self.root).with_home(Some(self.home.path().to_path_buf())).with_env(HashMap::new());
        let options = ComposeOptions::new().with_source_file(document).with_data_runtime(self.runtime.clone());
        ComposeRequest::prepare(options, &snapshot).expect("request")
    }

    /// Runs `document` as `md compose` does: the passive reference check
    /// (which defers an unapproved slot), then the composition.
    fn compose(&self, document: &Path) -> Result<(), Box<ComposeFailure>> {
        let request = self.request(document);
        let markdown = Markdown::try_from(document).expect("document");
        let passive = request.clone().map_options(ComposeOptions::for_reference_check);
        markdown
            .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&passive)))
            .unwrap_or_else(|error| panic!("the reference check defers the slot: {error}"));
        markdown.compose_with_warnings(&request).map(|_| ())
    }

    /// Composes `document`, which must fail needing approval.
    fn compose_needing_approval(&self, document: &Path) {
        let failure = self.compose(document).expect_err("an unapproved slot read fails the composition");
        let codes: Vec<String> = error_diagnostics(&failure.error).into_iter().map(|diagnostic| diagnostic.code).collect();
        assert_eq!(codes, vec![APPROVAL_REQUIRED.to_string()], "{}", failure.error);
    }

    fn observations(&self) -> Vec<Observation> {
        self.runtime.observations_since(ReadMark::default())
    }

    /// Asserts nothing read `name`'s bytes or listed this folder.
    fn assert_unread(&self, name: &str) {
        let entry = biscuit_file::to_portable_string(&self.path(name));
        let observations = self.observations();
        let read = observations.iter().find(|observation| {
            observation.entry == entry && matches!(observation.kind, ObservationKind::File | ObservationKind::Url)
        });
        assert!(read.is_none(), "{name} was read before approval: {observations:?}");
        assert!(!self.runtime.read_set().contains(&entry), "{name} is in the read set: {:?}", self.runtime.read_set());
    }

    fn assert_nothing_listed(&self) {
        let observations = self.observations();
        let listed = observations
            .iter()
            .find(|observation| matches!(observation.kind, ObservationKind::Listing | ObservationKind::Glob));
        assert!(listed.is_none(), "a folder was listed before approval: {observations:?}");
    }
}

const LOADED: &str = "load_data('./names.yaml', 'string[]')";

mod inline_slots_are_judged_before_their_data_is_read {
    use super::*;

    fn refused(os_type: &str) {
        let case = Case::new();
        let document = case.inline_document(os_type);
        case.compose_needing_approval(&document);
        case.assert_unread(NAMES);
    }

    #[test]
    fn an_enum() {
        refused(&format!("enum({{{{ {LOADED} }}}})"));
    }

    #[test]
    fn a_suggestion_list() {
        refused(&format!("string(suggest({{{{ {LOADED} }}}}))"));
    }

    #[test]
    fn a_literal() {
        refused(&format!("literal({{{{ {LOADED}[0] }}}})"));
    }

    #[test]
    fn a_default() {
        refused(&format!("string(default({{{{ {LOADED}[0] }}}}))"));
    }

    #[test]
    fn a_minimum() {
        refused(&format!("string(min({{{{ length({LOADED}) }}}}))"));
    }

    #[test]
    fn a_maximum() {
        refused(&format!("string(max({{{{ length({LOADED}) }}}}))"));
    }

    #[test]
    fn a_pattern() {
        refused(&format!("string(pattern({{{{ {LOADED}[0] }}}}))"));
    }

    /// Once granted, the slot (never ordinary interpolation) fills the
    /// list, so the members stay a list rather than becoming text.
    #[test]
    fn a_granted_enum_resolves_through_the_slot() {
        let case = Case::new();
        case.grant_file(NAMES);
        let document = case.inline_document(&format!("enum({{{{ {LOADED} }}}})"));
        case.compose(&document).unwrap_or_else(|failure| panic!("composes: {}", failure.error));
        let entry = biscuit_file::to_portable_string(&case.path(NAMES));
        assert!(case.runtime.read_set().contains(&entry), "{:?}", case.runtime.read_set());
    }
}

#[test]
fn a_contract_import_is_judged_before_it_is_opened() {
    let case = Case::new();
    case.write("private.yaml", "types:\n    Names: string[]\n");
    let document = case.referenced_document("enum({{ load_data('./names.yaml', 'Names@./private.yaml') }})");
    case.compose_needing_approval(&document);
    case.assert_unread("private.yaml");
    case.assert_unread(NAMES);
}

#[test]
fn validate_schema_reads_the_permitted_page_and_stops_before_its_schema() {
    let case = Case::new();
    case.write("private.yaml", "kind: schema\n$schema:\n    title: string\n");
    case.write("page.md", "---\n$schema: ./private.yaml\ntitle: Page\n---\n# Page\n");
    case.grant_file("page.md");
    let document = case.referenced_document("enum({{ validate_schema('./page.md') ? ['Linux'] : ['Linux'] }})");
    case.compose_needing_approval(&document);
    case.assert_unread("private.yaml");
    let page = biscuit_file::to_portable_string(&case.path("page.md"));
    assert!(
        case.observations().iter().any(|observation| observation.entry == page && observation.hash.is_some()),
        "the permitted page is read: {:?}",
        case.observations()
    );
}

mod listings_are_judged_before_the_folder_is_enumerated {
    use super::*;
    use std::sync::{Arc, Mutex};

    use darkmatter::markdown::compose::ShellExpansionError;
    use darkmatter::markdown::compose::shell_expansion::{
        ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
    };
    use darkmatter::markdown::schemas::slots::SlotSession;
    use darkmatter::markdown::schemas::slots::grants::{
        GrantDecision, GrantScope, GrantTarget, SlotGrantDecision, SlotGrantRequest,
    };

    fn refused(os_type: &str) {
        let case = Case::new();
        case.write("review-1.md", "");
        let document = case.referenced_document(os_type);
        case.compose_needing_approval(&document);
        case.assert_nothing_listed();
    }

    #[test]
    fn find_files() {
        refused("enum({{ length(find_files('./*.md')) > 0 ? ['Linux'] : ['Linux'] }})");
    }

    #[test]
    fn find_first_index() {
        refused("enum({{ find_first_index('./review-1.md') ? ['Linux'] : ['Linux'] }})");
    }

    #[test]
    fn find_last_index() {
        refused("enum({{ find_last_index('./review-1.md') ? ['Linux'] : ['Linux'] }})");
    }

    const LISTINGS: [&str; 3] = [
        "length(find_files('./*.md')) > 0",
        "find_first_index('./review-1.md')",
        "find_last_index('./review-1.md')",
    ];

    fn listing_document(case: &Case, condition: &str) -> PathBuf {
        case.write("review-1.md", "");
        case.referenced_document(&format!("enum({{{{ {condition} ? ['Linux'] : ['Linux'] }}}})"))
    }

    /// A persisted grant of the folder lets each listing run: the check
    /// before listing and the check of what was listed name the same folder.
    #[test]
    fn a_persisted_folder_grant_lets_each_listing_run() {
        for condition in LISTINGS {
            let case = Case::new();
            case.grant_folder();
            let document = listing_document(&case, condition);
            case.compose(&document).unwrap_or_else(|failure| panic!("{condition} composes: {}", failure.error));
        }
    }

    /// Pre-flight offers a grant of the folder each listing enumerates, and
    /// that grant covers the listing when the slot evaluates.
    #[test]
    fn the_folder_grant_pre_flight_offers_covers_each_listing() {
        for condition in LISTINGS {
            let case = Case::new();
            let document = listing_document(&case, condition);
            let request = case.request(&document);
            let handler = Arc::new(AllowFolderOnce::default());
            let grants = Markdown::try_from(document.as_path())
                .expect("document")
                .compose_slot_approvals(&request, Some(handler.clone()))
                .expect("approvals");
            assert_eq!(handler.asked(), vec![GrantTarget::Dir(case.root.clone())], "{condition}");
            let session = Arc::new(SlotSession::for_request(&request).with_run_grants(grants));
            let granted = request.map_options(|options| options.with_slot_session(session));
            Markdown::try_from(document.as_path())
                .expect("document")
                .compose_with_warnings(&granted)
                .unwrap_or_else(|failure| panic!("{condition} composes: {}", failure.error));
        }
    }

    /// Answers each slot request with its "this folder, once" choice and
    /// records the folders offered.
    #[derive(Default)]
    struct AllowFolderOnce {
        asked: Mutex<Vec<GrantTarget>>,
    }

    impl AllowFolderOnce {
        fn asked(&self) -> Vec<GrantTarget> {
            self.asked.lock().expect("asked").clone()
        }
    }

    impl ShellApprovalHandler for AllowFolderOnce {
        fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
            Ok(ShellApprovalDecision::Deny)
        }

        fn approve_slot_reads(&self, requests: &[SlotGrantRequest]) -> Result<Vec<SlotGrantDecision>, ShellExpansionError> {
            Ok(requests
                .iter()
                .map(|request| {
                    let folder = request.choices.iter().find(|grant| {
                        grant.decision == GrantDecision::Allow
                            && grant.scope == GrantScope::Once
                            && matches!(grant.target, GrantTarget::Dir(_))
                    });
                    match folder {
                        Some(grant) => {
                            self.asked.lock().expect("asked").push(grant.target.clone());
                            SlotGrantDecision::from_grant(grant)
                        }
                        None => SlotGrantDecision::Unanswered,
                    }
                })
                .collect())
        }
    }
}

/// The direct readers already authorize first; these keep them so.
mod direct_reads_stay_judged_before_reading {
    use super::*;

    fn refused(os_type: &str, target: &str) {
        let case = Case::new();
        case.write("page.md", "---\ntitle: Page\n---\n# Page\n");
        let document = case.referenced_document(os_type);
        case.compose_needing_approval(&document);
        case.assert_unread(target);
    }

    #[test]
    fn load_data() {
        refused(&format!("enum({{{{ {LOADED} }}}})"), NAMES);
    }

    #[test]
    fn frontmatter() {
        refused("enum({{ frontmatter('./page.md') ? ['Linux'] : ['Linux'] }})", "page.md");
    }

    #[test]
    fn try_frontmatter_keeps_the_refusal() {
        refused("enum({{ try_frontmatter('./page.md') ? ['Linux'] : ['Linux'] }})", "page.md");
    }

    #[test]
    fn markdown_title() {
        refused("enum({{ markdown_title('./page.md') ? ['Linux'] : ['Linux'] }})", "page.md");
    }

    #[test]
    fn markdown_body_empty() {
        refused("enum({{ markdown_body_empty('./page.md') ? ['Linux'] : ['Linux'] }})", "page.md");
    }
}


