//! A slot's read is judged before it happens, on every path.
//!
//! Each case runs a document as `md compose` does (the passive reference
//! check, then the composition) in a temp folder with no repository, a home
//! whose `permissions.file-read` pre-approves nothing unless the case writes
//! an entry, no approval handler, and an explicit data runtime whose
//! observation log is read after the failure. A content observation with a
//! hash proves bytes were read, so a refused target must leave none. A
//! listing needs no approval: it returns only the files the slot may read
//! without asking. `names.yaml` holds `- Linux` and the
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
        crate::slot_support::write_user_permissions(case.home.path(), &[]);
        case.write(NAMES, "- Linux\n");
        case
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn write(&self, name: &str, text: &str) {
        std::fs::write(self.path(name), text).expect("write");
    }

    /// Writes the home's `permissions.file-read` as `./<name>` (beside the
    /// reading document, which every case keeps in the root).
    fn grant_file(&self, name: &str) {
        crate::slot_support::write_user_permissions(self.home.path(), &[&format!("./{name}")]);
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
        let entry = biscuit_file::to_forward_slash_string(&self.path(name));
        let observations = self.observations();
        let read = observations.iter().find(|observation| {
            observation.entry == entry && matches!(observation.kind, ObservationKind::File | ObservationKind::Url)
        });
        assert!(read.is_none(), "{name} was read before approval: {observations:?}");
        assert!(!self.runtime.read_set().contains(&entry), "{name} is in the read set: {:?}", self.runtime.read_set());
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
        let entry = biscuit_file::to_forward_slash_string(&case.path(NAMES));
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
    let page = biscuit_file::to_forward_slash_string(&case.path("page.md"));
    assert!(
        case.observations().iter().any(|observation| observation.entry == page && observation.hash.is_some()),
        "the permitted page is read: {:?}",
        case.observations()
    );
}

mod listings_need_no_approval_and_return_only_permitted_files {
    use super::*;
    use std::sync::{Arc, Mutex};

    use darkmatter::markdown::compose::ShellExpansionError;
    use darkmatter::markdown::compose::shell_expansion::{
        ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
    };
    use darkmatter::markdown::schemas::slots::grants::{SlotGrantDecision, SlotGrantRequest};

    const LISTINGS: [&str; 3] = [
        "length(find_files('./*.md')) > 0",
        "find_first_index('./review-1.md')",
        "find_last_index('./review-1.md')",
    ];

    fn listing_document(case: &Case, condition: &str) -> PathBuf {
        case.write("review-1.md", "");
        case.referenced_document(&format!("enum({{{{ {condition} ? ['Linux'] : ['Linux'] }}}})"))
    }

    /// With nothing permitted, each listing still runs: no folder approval
    /// is needed, and pre-flight asks about nothing.
    #[test]
    fn each_listing_runs_without_approval_and_is_never_asked_about() {
        for condition in LISTINGS {
            let case = Case::new();
            let document = listing_document(&case, condition);
            let handler = Arc::new(RecordingHandler::default());
            let grants = Markdown::try_from(document.as_path())
                .expect("document")
                .compose_slot_approvals(&case.request(&document), Some(handler.clone()))
                .expect("approvals");
            assert!(grants.is_empty(), "{condition}: {grants:?}");
            assert!(handler.asked().is_empty(), "{condition}: a listing is never asked about: {:?}", handler.asked());
            case.compose(&document).unwrap_or_else(|failure| panic!("{condition} composes: {}", failure.error));
        }
    }

    /// `find_files` returns only the files the permissions admit: the
    /// document is valid exactly when one of the two listed files is
    /// permitted.
    #[test]
    fn find_files_returns_only_the_permitted_files() {
        let os_type = "enum({{ length(find_files('./review-*.md')) == 1 ? ['Linux'] : ['macOS'] }})";
        let case = Case::new();
        case.write("review-1.md", "");
        case.write("review-2.md", "");
        let document = case.referenced_document(os_type);
        let failure = case.compose(&document).expect_err("nothing is permitted, so nothing is listed");
        let codes: Vec<String> = error_diagnostics(&failure.error).into_iter().map(|diagnostic| diagnostic.code).collect();
        assert!(!codes.iter().any(|code| code.starts_with("dm.schema.slot_")), "{codes:?}");

        let case = Case::new();
        case.write("review-1.md", "");
        case.write("review-2.md", "");
        case.grant_file("review-2.md");
        let document = case.referenced_document(os_type);
        case.compose(&document).unwrap_or_else(|failure| panic!("only review-2.md is listed: {}", failure.error));
    }

    /// Records every slot request and answers none.
    #[derive(Default)]
    struct RecordingHandler {
        asked: Mutex<Vec<SlotGrantRequest>>,
    }

    impl RecordingHandler {
        fn asked(&self) -> Vec<SlotGrantRequest> {
            self.asked.lock().expect("asked").clone()
        }
    }

    impl ShellApprovalHandler for RecordingHandler {
        fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
            Ok(ShellApprovalDecision::Deny)
        }

        fn approve_slot_reads(&self, requests: &[SlotGrantRequest]) -> Result<Vec<SlotGrantDecision>, ShellExpansionError> {
            self.asked.lock().expect("asked").extend(requests.iter().cloned());
            Ok(requests.iter().map(|_| SlotGrantDecision::Unanswered).collect())
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


