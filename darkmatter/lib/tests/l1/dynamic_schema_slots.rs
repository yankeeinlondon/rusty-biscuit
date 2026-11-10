//! Schema templates end to end through `Markdown::compose`.
//!
//! Every case builds a real Git repository in a temp dir (from the
//! sandboxing-shaped fixture, or written inline), anchors the request there
//! with a temp home (so the user's real grant store is never read), and
//! composes through the public API. The session, cache, grants, trigger
//! payloads, and contract recursion are the real ones.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
#[cfg(unix)]
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionOptions,
};
use darkmatter::markdown::compose::{ComposeFailure, ComposeOptions, ComposeReport, ComposeRequest, DataRuntime, RequestSnapshot};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::slots::SlotSession;
use serde_json::{Value, json};

use crate::dynamic_slots_fixtures::{DATA_FILE, DOCUMENT_FILE, RepoState, SCHEMA_FILE, SlotRepo, git};

const APPROVAL_REQUIRED: &str = "dm.schema.slot_approval_required";

/// A home with no grant store, so only the run's own grants apply.
fn empty_home() -> tempfile::TempDir {
    tempfile::tempdir().expect("temp home")
}

fn request(document: &Path, home: &Path, options: ComposeOptions) -> ComposeRequest {
    let snapshot = RequestSnapshot::new(document.parent().expect("document folder"))
        .with_home(Some(home.to_path_buf()))
        .with_env(HashMap::new());
    ComposeRequest::prepare(options.with_source_file(document), &snapshot).expect("request")
}

fn compose(document: &Path, home: &Path, options: ComposeOptions) -> Result<(Markdown, ComposeReport), Box<ComposeFailure>> {
    Markdown::try_from(document).expect("document").compose_with_warnings(&request(document, home, options))
}

fn codes(failure: &ComposeFailure) -> Vec<String> {
    error_diagnostics(&failure.error).into_iter().map(|diagnostic| diagnostic.code).collect()
}

/// Every diagnostic message of `failure`, one per line.
fn messages(failure: &ComposeFailure) -> String {
    error_diagnostics(&failure.error).into_iter().map(|diagnostic| diagnostic.message).collect::<Vec<_>>().join("\n")
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
    std::fs::write(path, text).expect("write");
}

/// The fixture repository's document, rewritten with `frontmatter`.
fn document_with(repo: &SlotRepo, frontmatter: &str) -> PathBuf {
    let path = repo.path(DOCUMENT_FILE);
    write(&path, &format!("---\n$schema: ./sandboxing.yaml\n{frontmatter}---\n\n# Provider\n"));
    path
}

#[test]
fn a_document_valid_under_the_resolved_list_composes_and_reads_the_data_file() {
    let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
    let repo = SlotRepo::build(&source, RepoState::Clean);
    let home = empty_home();
    let (_, report) = compose(&repo.path(DOCUMENT_FILE), home.path(), ComposeOptions::new())
        .unwrap_or_else(|failure| panic!("composes: {}", failure.error));
    assert!(
        report.read_set.contains(&biscuit_file::to_portable_string(&repo.path(DATA_FILE))),
        "the slot's read is in the read set: {:?}",
        report.read_set
    );
}

#[test]
fn a_value_outside_the_resolved_list_is_a_validation_problem() {
    let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
    let repo = SlotRepo::build(&source, RepoState::Clean);
    let document = document_with(&repo, "schema_revision: 5\nhas_native_support: true\nos_support:\n  - BeOS\n");
    let failure = compose(&document, empty_home().path(), ComposeOptions::new()).expect_err("BeOS is not listed");
    let messages = messages(&failure);
    assert!(messages.contains("\"BeOS\" is not one of \"macOS\", \"Linux\" or \"Windows\""), "{messages}");
    assert!(!codes(&failure).iter().any(|code| code.starts_with("dm.schema.slot_")), "{:?}", codes(&failure));
}

#[test]
fn a_dirty_data_file_needs_approval_and_a_library_caller_is_never_asked() {
    let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
    let repo = SlotRepo::build(&source, RepoState::DirtyData);
    let failure = compose(&repo.path(DOCUMENT_FILE), empty_home().path(), ComposeOptions::new())
        .expect_err("a dirty file is not read without a grant");
    assert_eq!(codes(&failure), vec![APPROVAL_REQUIRED.to_string()]);
    let diagnostic = &error_diagnostics(&failure.error)[0];
    let at = diagnostic.source.as_ref().expect("positioned");
    assert_eq!(at.file.as_deref(), Some(repo.path(SCHEMA_FILE).as_path()), "{diagnostic:?}");
    assert!(at.line.is_some(), "{diagnostic:?}");
}

/// A template constraint: a slot-supplied `default` lowers exactly as the same
/// literal default does, and the document composes under it.
#[test]
fn a_slot_supplied_default_lowers_like_the_same_literal_default() {
    let temp = tempfile::tempdir().unwrap();
    let root = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
    let home = empty_home();
    let document = root.join("doc.md");
    write(&document, "---\n$schema: ./schema.yaml\n---\n\nBody.\n");
    let lowered = |default: &str| {
        write(&root.join("schema.yaml"), &format!("kind: schema\n$schema:\n    retries: number(default({default}))\n"));
        compose(&document, home.path(), ComposeOptions::new())
            .unwrap_or_else(|failure| panic!("{default} composes: {}", failure.error));
        let markdown = Markdown::try_from(document.as_path())
            .unwrap()
            .with_source(darkmatter::markdown::compose::ComposeSource::File(document.clone()));
        let context = biscuit_file::FileResolutionContext::new(&root).with_env(HashMap::new());
        let session = Arc::new(SlotSession::new(DataRuntime::new()));
        let effective = DarkmatterSchemas::new(context).with_slot_session(session).effective_for(&markdown);
        effective.expect("resolves").expect("a schema").json_schema["properties"]["retries"].clone()
    };
    let literal = lowered("3");
    assert_eq!(literal["default"], json!(3), "{literal}");
    assert_eq!(lowered("{{ 1 + 2 }}"), literal);
}

#[test]
fn one_session_keeps_its_pinned_snapshot_for_every_pass() {
    let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
    let repo = SlotRepo::build(&source, RepoState::Clean);
    let home = empty_home();
    let data = DataRuntime::new();
    let session = Arc::new(SlotSession::new(data.clone()).with_repository_root(Some(repo.root().to_path_buf())));
    let options = || ComposeOptions::new().with_data_runtime(data.clone()).with_slot_session(Arc::clone(&session));
    compose(&repo.path(DOCUMENT_FILE), home.path(), options()).unwrap_or_else(|failure| panic!("{}", failure.error));

    // Drop `Linux` from the data. The command's pin still holds the list it
    // resolved (nothing the command ran changed its inputs' generation)...
    write(&repo.path(DATA_FILE), "- macOS\n- Windows\n");
    compose(&repo.path(DOCUMENT_FILE), home.path(), options())
        .unwrap_or_else(|failure| panic!("the pinned snapshot applies: {}", failure.error));
    // ...while a new command judges the edited (now dirty) file afresh.
    let failure = compose(&repo.path(DOCUMENT_FILE), home.path(), ComposeOptions::new()).expect_err("dirty");
    assert_eq!(codes(&failure), vec![APPROVAL_REQUIRED.to_string()]);
}

#[cfg(unix)]
struct AllowAll;

#[cfg(unix)]
impl ShellApprovalHandler for AllowAll {
    fn approve(
        &self,
        _request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, darkmatter::markdown::compose::ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

/// Q7/Q17: a step the run approved rewrites the data file; the next
/// validation in the same command re-resolves (the memo generation moved and
/// the pin's input changed) under the automatic grant decided earlier, even
/// though the file is now dirty.
#[cfg(unix)]
#[test]
fn a_rewrite_by_the_runs_own_shell_step_re_resolves_with_the_grant_decided_earlier() {
    let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
    let repo = SlotRepo::build(&source, RepoState::Clean);
    let home = empty_home();
    let data = DataRuntime::new();
    let session = Arc::new(SlotSession::new(data.clone()).with_repository_root(Some(repo.root().to_path_buf())));
    let folder = repo.path(DOCUMENT_FILE).parent().unwrap().to_path_buf();
    let shell_document = folder.join("rewrite.md");
    write(&folder.join("next-os.yaml"), "- macOS\n- Linux\n- Windows\n- FreeBSD\n");
    write(&shell_document, "Rewrite:\n\n::shell cp next-os.yaml ../../../os.yaml\n");
    let options = || ComposeOptions::new().with_data_runtime(data.clone()).with_slot_session(Arc::clone(&session));
    compose(&repo.path(DOCUMENT_FILE), home.path(), options()).unwrap_or_else(|failure| panic!("{}", failure.error));
    compose(
        &shell_document,
        home.path(),
        options()
            .with_shell(ShellExpansionOptions { policy_root: Some(folder.clone()), ..Default::default() })
            .with_shell_working_directory(&folder)
            .with_shell_approval_handler(Arc::new(AllowAll)),
    )
    .unwrap_or_else(|failure| panic!("the shell step runs: {}", failure.error));
    assert!(std::fs::read_to_string(repo.path(DATA_FILE)).unwrap().contains("FreeBSD"));

    let freebsd = document_with(&repo, "schema_revision: 5\nhas_native_support: true\nos_support:\n  - FreeBSD\n");
    compose(&freebsd, home.path(), options())
        .unwrap_or_else(|failure| panic!("re-resolved under the run's grant: {}", failure.error));
}

/// A repository with a schema trigger whose payload is a template.
fn trigger_repo() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = biscuit_file::canonicalize_simplified(temp.path()).unwrap().join("repo");
    write(
        &root.join("schemas/sandbox.trigger.yaml"),
        "kind: schema-trigger\nmatch:\n  topic: literal(sandbox; required)\n$schema:\n    os: enum({{ load_data(\"../data/os.yaml\", \"string[]\") }}; required)\n",
    );
    write(&root.join("data/os.yaml"), "- macOS\n- Linux\n");
    write(&root.join("docs/selected.md"), "---\ntopic: sandbox\nos: Windows\n---\nBody.\n");
    write(&root.join("docs/unselected.md"), "---\ntopic: other\nos: Windows\n---\nBody.\n");
    git(&root, &["init", "-q", "-b", "main"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "fixture"]);
    (temp, root)
}

#[test]
fn a_matched_template_trigger_payload_resolves_and_applies() {
    let (_temp, root) = trigger_repo();
    let failure = compose(&root.join("docs/selected.md"), empty_home().path(), ComposeOptions::new().with_trigger_schemas(true))
        .expect_err("Windows is not in the resolved list");
    let messages = messages(&failure);
    assert!(messages.contains("Windows"), "{messages}");
    assert!(failure.read_set.contains(&biscuit_file::to_portable_string(&root.join("data/os.yaml"))), "{:?}", failure.read_set);
}

#[test]
fn an_unselected_template_trigger_payload_reads_nothing() {
    let (_temp, root) = trigger_repo();
    let (_, report) = compose(&root.join("docs/unselected.md"), empty_home().path(), ComposeOptions::new().with_trigger_schemas(true))
        .unwrap_or_else(|failure| panic!("an unmatched trigger does not apply: {}", failure.error));
    assert!(report.read_set.is_empty(), "{:?}", report.read_set);
    assert!(report.warnings.is_empty(), "a template payload is not an invalid trigger: {:?}", report.warnings);
}

#[test]
fn the_effective_schema_holds_only_resolved_constraints() {
    let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
    let repo = SlotRepo::build(&source, RepoState::Clean);
    let document = repo.path(DOCUMENT_FILE);
    let context = biscuit_file::FileResolutionContext::new(document.parent().unwrap())
        .with_env(HashMap::new())
        .with_repository_root(repo.root());
    let markdown = Markdown::try_from(document.as_path()).unwrap()
        .with_source(darkmatter::markdown::compose::ComposeSource::File(document.clone()));

    let without = DarkmatterSchemas::new(context.clone()).effective_for(&markdown);
    assert!(
        matches!(without, Err(simplified_schema::SchemaError::UnresolvedSlots(_))),
        "without a session the template is unresolved: {:?}",
        without.err()
    );

    let session = Arc::new(SlotSession::new(DataRuntime::new()).with_repository_root(Some(repo.root().to_path_buf())));
    let effective = DarkmatterSchemas::new(context)
        .with_slot_session(session)
        .effective_for(&markdown)
        .expect("resolves")
        .expect("a schema applies");
    let root = effective.simplified.as_ref().expect("a SimplifiedSchema document layer");
    assert!(!simplified_schema::slots::has_dynamic(root, effective.definitions()));
    let os: Value = effective.json_schema.pointer("/properties/os_support/items").cloned().unwrap_or(Value::Null);
    let rendered = serde_json::to_string(&effective.json_schema).unwrap();
    assert!(rendered.contains("\"Windows\""), "{os} / {rendered}");
}

/// A repository where `a.yaml`'s slot loads data under a contract defined in
/// `b.yaml`, whose own slot loads data under a contract defined in `a.yaml`.
fn cycle_repo() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = biscuit_file::canonicalize_simplified(temp.path()).unwrap().join("repo");
    write(
        &root.join("a.yaml"),
        "kind: schema\n$schema:\n    os: enum({{ load_data(\"./data.yaml\", \"Os@./b.yaml\") }}; required)\ntypes:\n    Items: enum({{ load_data(\"./data.yaml\", \"Os@./b.yaml\") }})[]\n",
    );
    write(&root.join("b.yaml"), "kind: schema\ntypes:\n    Os: enum({{ load_data(\"./data.yaml\", \"Items@./a.yaml\") }})[]\n");
    write(&root.join("data.yaml"), "- macOS\n");
    write(&root.join("doc.md"), "---\n$schema: ./a.yaml\nos: macOS\n---\nBody.\n");
    write(&root.join("plain.md"), "Os: {{ load_data(\"./data.yaml\", \"Os@./b.yaml\") }}\n");
    git(&root, &["init", "-q", "-b", "main"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "fixture"]);
    (temp, root)
}

#[test]
fn two_schemas_whose_slots_load_each_other_are_a_cycle_at_the_outer_slot() {
    let (_temp, root) = cycle_repo();
    let failure = compose(&root.join("doc.md"), empty_home().path(), ComposeOptions::new()).expect_err("a cycle");
    let diagnostics = error_diagnostics(&failure.error);
    assert_eq!(diagnostics.iter().map(|diagnostic| diagnostic.code.as_str()).collect::<Vec<_>>(), ["dm.schema.slot_cycle"]);
}

#[test]
fn outside_a_slot_a_contract_naming_a_template_is_unresolved() {
    let (_temp, root) = cycle_repo();
    let failure = compose(&root.join("plain.md"), empty_home().path(), ComposeOptions::new()).expect_err("unresolved");
    let message = format!("{}", failure.error);
    assert!(message.contains("unresolved") || message.contains("slot"), "{message}");
}

mod site_schemas {
    use super::*;
    use darkmatter::markdown::compose::ComposeSource;
    use darkmatter::markdown::compose::expression::{
        ArgumentCategory, DeclaredTypes, Shape, TypeScope, check_expression_calls, expression_member_findings, parse,
        parse_spanned,
    };
    use darkmatter::markdown::schemas::SiteSchema;
    use simplified_schema::TypeExpr;

    /// A pending document schema: `os` waits on a pure literal slot beside a
    /// literal member, `server` is a closed object.
    const PENDING: &str = "---\n$schema:\n  os: enum(Windows, {{ 123 }}; required)\n  server: '{ name: string }'\nos: Windows\n---\nBody.\n";
    /// The same schema with the slot dropped.
    const CONCRETE: &str = "---\n$schema:\n  os: enum(Windows; required)\n  server: '{ name: string }'\nos: Windows\n---\nBody.\n";

    fn schemas_at(dir: &Path) -> DarkmatterSchemas {
        DarkmatterSchemas::new(
            biscuit_file::FileResolutionContext::new(dir)
                .with_env(HashMap::new())
                .with_repository_root(dir),
        )
    }

    fn markdown(dir: &Path, name: &str, text: &str) -> Markdown {
        let path = dir.join(name);
        write(&path, text);
        Markdown::try_from(path.as_path()).unwrap().with_source(ComposeSource::File(path))
    }

    #[cfg(feature = "work-counters")]
    fn slot_runs() -> u64 {
        darkmatter::work_counters::snapshot().get(darkmatter::work_counters::Work::SlotResolution)
    }

    #[test]
    fn a_pending_document_schema_types_its_sites_from_the_template() {
        let temp = tempfile::tempdir().unwrap();
        let dir = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
        let schemas = schemas_at(&dir);
        let pending = markdown(&dir, "pending.md", PENDING);

        #[cfg(feature = "work-counters")]
        let before = slot_runs();
        let site = schemas.site_schema_for(&pending).expect("a pending template is not an error");
        #[cfg(feature = "work-counters")]
        assert_eq!(slot_runs(), before, "no slot is evaluated");

        let SiteSchema::Template { template: Some(template), unresolved } = &site else {
            panic!("a pending document schema is a template");
        };
        assert_eq!(unresolved.slots.len(), 1);
        assert!(template.definitions.is_empty());
        let declared = DeclaredTypes::for_site(&site);
        let scope = TypeScope::unguarded(&declared);
        let server = scope.type_of_expr(&parse("server").unwrap()).expect("a document property").shape().clone();
        let Shape::Arms { arms, .. } = &server else { panic!("server is typed: {server:?}") };
        let TypeExpr::InlineObject(shape) = &arms[0].ty else { panic!("server is an object: {server:?}") };
        assert_eq!(shape.properties.keys().collect::<Vec<_>>(), ["name"]);
        assert!(shape.pattern_keys.is_empty());
        let text = "server.nmae";
        let findings = expression_member_findings(&parse_spanned(text).unwrap(), text, &scope);
        assert_eq!(findings.len(), 1, "{findings:?}");

        // `effective_for` keeps reporting the template as unresolved.
        assert!(matches!(schemas.effective_for(&pending), Err(simplified_schema::SchemaError::UnresolvedSlots(_))));

        let concrete = schemas.site_schema_for(&markdown(&dir, "concrete.md", CONCRETE)).expect("resolves");
        assert!(matches!(concrete, SiteSchema::Effective(Some(_))), "a concrete schema is effective");
        let none = schemas.site_schema_for(&markdown(&dir, "plain.md", "---\ntitle: x\n---\nBody.\n")).unwrap();
        assert!(matches!(none, SiteSchema::Effective(None)));
    }

    /// A constraint waiting on a slot is unknown to the call check, never
    /// the literal members written beside it: the concrete `enum(Windows)`
    /// is judged as a string, the pending one is not judged at all.
    #[test]
    fn a_pending_enum_is_not_judged_by_its_literal_members() {
        let temp = tempfile::tempdir().unwrap();
        let dir = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
        let schemas = schemas_at(&dir);
        let categories = |site: &SiteSchema| -> Vec<ArgumentCategory> {
            let declared = DeclaredTypes::for_site(site);
            let scope = TypeScope::unguarded(&declared);
            ["upper(os)", "min(os, 1)", "os == 'Linux' ? upper(os) : ''"]
                .into_iter()
                .map(|text| check_expression_calls(&parse_spanned(text).unwrap(), &scope)[0].category())
                .collect()
        };
        let concrete = schemas.site_schema_for(&markdown(&dir, "concrete.md", CONCRETE)).unwrap();
        assert_eq!(
            categories(&concrete),
            [ArgumentCategory::Compatible, ArgumentCategory::Uncertain, ArgumentCategory::Compatible],
            "control: the literal list types the argument"
        );
        let pending = schemas.site_schema_for(&markdown(&dir, "pending.md", PENDING)).unwrap();
        assert!(matches!(pending, SiteSchema::Template { .. }));
        assert_eq!(categories(&pending), [ArgumentCategory::Unknown; 3]);
    }
}
