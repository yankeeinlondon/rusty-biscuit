//! A dry run never runs `initialize`, yet a real run's `initialize` can
//! create a file a later read needs. `lifecycle::created_files` lists the
//! files a stack creates with a literal target, `CreatedFile::target`
//! resolves each one as its effect does, and a passive session told about
//! them (`PassiveSession::defer_created`) leaves a read of a still-absent
//! one unchecked instead of missing. Unrelated and existing files are read
//! as they are. A templated target is listed too: a host resolves it
//! (`CreatedFile::target_as`) or bounds it (`CreatedFile::pattern`). An
//! append to an existing file changes what it holds, never whether it
//! exists (`PassiveSession::initializer_changes`).

use std::path::{Path, PathBuf};

use darkmatter::lifecycle::{LifecycleSignal, created_files, md_host, parse_lifecycle_config_with};
use darkmatter::markdown::compose::passive::{DeferralReason, DeferredConstruct, InitializerChange, PassiveSession};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::json;
use tempfile::TempDir;

fn initialize(stack: serde_json::Value) -> Vec<darkmatter::lifecycle::CreatedFile> {
    let options = md_host::parse_options(None);
    let config = parse_lifecycle_config_with(&json!({ "initialize": stack }), Path::new("doc.md"), &options).unwrap();
    created_files(config.stack(LifecycleSignal::Initialize))
}

fn created(path: &str) -> DeferralReason {
    DeferralReason::CreatedAtLaunch { path: path.to_string() }
}

#[test]
fn every_file_creating_effect_with_a_literal_target_is_listed_in_authored_order() {
    let found = initialize(json!([
        { "ensure_file": "a.md" },
        { "ensure_file": ["b.md", "content"] },
        { "append_line": ["c.yaml", "kind: task"] },
        { "append_jsonl": ["d.jsonl", "{{ record }}"] },
        { "when": "flag", "then": [{ "ensure_file": "e.md" }], "else": [{ "append_line": ["f.md", "x"] }] },
        // Not a file a read can find, or a target known only when it runs.
        { "ensure_dir": "folder" },
        { "set_frontmatter": ["g.md", "k", "v"] },
        { "ensure_file": "{{ folder }}/h.md" },
    ]));
    let listed: Vec<(&str, &str, bool, bool)> =
        found.iter().map(|file| (file.verb.as_str(), file.reference.as_str(), file.conditional, file.template)).collect();
    assert_eq!(
        listed,
        [
            ("ensure_file", "a.md", false, false),
            ("ensure_file", "b.md", false, false),
            ("append_line", "c.yaml", false, false),
            ("append_jsonl", "d.jsonl", false, false),
            ("ensure_file", "e.md", true, false),
            ("append_line", "f.md", true, false),
            ("ensure_file", "{{ folder }}/h.md", false, true),
        ]
    );
    assert_eq!(found[0].change(), InitializerChange::CreatesIfAbsent);
    assert_eq!(found[2].change(), InitializerChange::CreatesOrAppends);
    assert_eq!(found[3].change(), InitializerChange::CreatesOrAppends);
    assert_eq!(found[0].path.to_string(), "initialize[0].ensure_file");
}

#[test]
fn a_target_resolves_beside_the_document_as_its_effect_resolves_it() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("docs")).unwrap();
    std::fs::write(dir.path().join("docs/existing.md"), "").unwrap();
    let source = dir.path().join("docs/doc.md");
    let context = biscuit_file::FileResolutionContext::new(dir.path()).with_repository_root(dir.path());
    let found = initialize(json!([{ "ensure_file": "notes.md" }, { "ensure_file": "existing.md" }, { "ensure_file": "https://example.invalid/x.md" }]));

    assert_eq!(found[0].target(&source, &context, dir.path()), Some(dir.path().join("docs/notes.md")));
    assert_eq!(found[1].target(&source, &context, dir.path()), Some(dir.path().join("docs/existing.md")));
    assert_eq!(found[2].target(&source, &context, dir.path()), None, "a URL has no local target");
}

/// The effect refuses to write outside its engine's mutation root, so a
/// target there is no file the run creates.
#[test]
fn a_target_outside_the_mutation_root_is_not_created() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("repo");
    std::fs::create_dir_all(root.join("docs")).unwrap();
    let source = root.join("docs/doc.md");
    let context = biscuit_file::FileResolutionContext::new(&root).with_repository_root(&root);
    let found = initialize(json!([{ "ensure_file": "../../outside.md" }, { "ensure_file": "../inside.md" }]));

    assert_eq!(found[0].target(&source, &context, &root), None);
    assert_eq!(found[1].target(&source, &context, &root), Some(root.join("inside.md")));
}

#[test]
fn a_declared_file_is_matched_by_the_entry_it_names_not_by_its_spelling() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("present.md"), "").unwrap();
    let session = PassiveSession::new();
    session.defer_created(&dir.path().join("gen/notes.md"), created("gen/notes.md"));
    session.defer_created(&dir.path().join("present.md"), created("present.md"));

    assert_eq!(session.created_file(&dir.path().join("sub/../gen/./notes.md")), Some(created("gen/notes.md")));
    assert_eq!(session.created_file(&dir.path().join("gen/other.md")), None);
    assert_eq!(session.created_file(&dir.path().join("present.md")), None, "an existing file is read as it is");
    let pass = session.sharing_observations();
    assert_eq!(pass.created_among([dir.path().join("x.md"), dir.path().join("gen/notes.md")]), Some(created("gen/notes.md")));

    // A folder reached through a symlink is the same folder.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(dir.path(), dir.path().join("sub/link")).unwrap();
        assert_eq!(session.created_file(&dir.path().join("sub/link/gen/notes.md")), Some(created("gen/notes.md")));
    }

    // Another case names the same entry only where the volume opens it.
    std::fs::write(dir.path().join("CaseProbe"), "").unwrap();
    let insensitive = dir.path().join("caseprobe").exists();
    let other_case = session.created_file(&dir.path().join("GEN/Notes.md"));
    assert_eq!(other_case.is_some(), insensitive, "case-insensitive volume: {insensitive}");
}

fn dry_compose(dir: &Path, text: &str, declared: &[&str]) -> (Result<String, MarkdownError>, PassiveSession) {
    let path: PathBuf = dir.join("doc.md");
    std::fs::write(&path, text).unwrap();
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.clone())
        .with_dry_run();
    let session = options.passive_session().cloned().unwrap();
    for name in declared {
        session.defer_created(&dir.join(name), created(name));
    }
    let composed = Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request(options))
        .map(|(composed, _)| composed.content().to_string());
    (composed, session)
}

#[test]
fn a_read_of_a_declared_file_is_left_as_written_and_an_unrelated_miss_is_refused() {
    let dir = TempDir::new().unwrap();
    let text = "::file gen/notes.md\n\nData {{ load_data('gen/data.json').name }}.\n\n::block when=\"file_exists('gen/flag.md')\"\nFlag.\n::end-block\n";

    let (composed, session) = dry_compose(dir.path(), text, &["gen/notes.md", "gen/data.json", "gen/flag.md"]);
    let body = composed.unwrap();
    assert!(body.contains("::file gen/notes.md"), "{body}");
    assert!(body.contains("{{ load_data('gen/data.json').name }}"), "{body}");
    assert!(body.contains("::block when=\"file_exists('gen/flag.md')\""), "{body}");
    let deferred: Vec<(DeferredConstruct, DeferralReason)> =
        session.deferrals().into_iter().map(|deferral| (deferral.construct, deferral.reason)).collect();
    assert!(deferred.contains(&(DeferredConstruct::Transclusion, created("gen/notes.md"))), "{deferred:?}");
    assert!(deferred.contains(&(DeferredConstruct::Interpolation, created("gen/data.json"))), "{deferred:?}");
    assert!(deferred.contains(&(DeferredConstruct::BlockCondition, created("gen/flag.md"))), "{deferred:?}");

    let (unrelated, _) = dry_compose(dir.path(), "::file gen/notes.md\n", &["gen/other.md"]);
    assert!(unrelated.is_err(), "an undeclared missing file is still missing: {unrelated:?}");
}

fn changed(path: &str) -> DeferralReason {
    DeferralReason::ChangedAtLaunch { path: path.to_string() }
}

/// A template is never a file before it runs; resolved, it is placed as
/// its literal spelling is, and unresolved it bounds the locations its
/// fixed text allows.
#[test]
fn a_templated_target_is_placed_as_resolved_or_bounded_by_its_fixed_text() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("docs")).unwrap();
    let source = dir.path().join("docs/doc.md");
    let context = biscuit_file::FileResolutionContext::new(dir.path()).with_repository_root(dir.path());
    let found = initialize(json!([
        { "ensure_file": ["{{ target }}", "x"] },
        { "append_line": ["logs/{{ day }}.jsonl", "x"] },
        { "ensure_file": "literal.md" },
    ]));

    assert!(found[0].template && found[1].template && !found[2].template);
    assert_eq!(found[0].target(&source, &context, dir.path()), None, "a template names no file before it runs");
    assert_eq!(found[0].target_as("gen/notes.md", &source, &context, dir.path()), Some(dir.path().join("docs/gen/notes.md")));
    assert_eq!(found[0].target_as("../../outside.md", &source, &context, dir.path()), None);
    assert!(found[2].pattern(&source, &context, dir.path()).is_none(), "a literal is no pattern");

    let anywhere = found[0].pattern(&source, &context, dir.path()).unwrap();
    assert!(anywhere.matches(&dir.path().join("any/where.md")));
    assert!(!anywhere.matches(&dir.path().parent().unwrap().join("elsewhere.md")), "the effect writes only below its mutation root");
    let logs = found[1].pattern(&source, &context, dir.path()).unwrap();
    assert!(logs.matches(&dir.path().join("docs/logs/2026-11-27.jsonl")));
    assert!(!logs.matches(&dir.path().join("docs/logs/notes.md")));
    assert!(!logs.matches(&dir.path().join("docs/other.jsonl")));
    assert!(logs.may_lie_below(&dir.path().join("docs")));
    assert!(!logs.may_lie_below(&dir.path().join("gen")));
}

/// An append changes an existing file's content but never whether it
/// exists; `ensure_file` leaves an existing file as it is.
#[test]
fn an_append_changes_an_existing_files_content_but_not_its_existence() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("log.jsonl"), "").unwrap();
    std::fs::write(dir.path().join("kept.md"), "").unwrap();
    let session = PassiveSession::new();
    session.declare_initializer_write(&dir.path().join("log.jsonl"), InitializerChange::CreatesOrAppends, created("log.jsonl"));
    session.declare_initializer_write(&dir.path().join("kept.md"), InitializerChange::CreatesIfAbsent, created("kept.md"));
    session.declare_initializer_write(&dir.path().join("gen/new.md"), InitializerChange::CreatesIfAbsent, created("gen/new.md"));

    assert_eq!(session.initializer_changes(&dir.path().join("log.jsonl")), Some(changed("log.jsonl")));
    assert_eq!(session.created_file(&dir.path().join("log.jsonl")), None, "an existing file's existence is proven");
    assert_eq!(session.initializer_changes(&dir.path().join("kept.md")), None, "ensure_file leaves an existing file as it is");
    assert_eq!(session.initializer_changes(&dir.path().join("gen/new.md")), Some(created("gen/new.md")));

    // A listing changes only by a file that does not exist yet.
    let folder = [dir.path().join("gen")];
    assert_eq!(session.changes_listing(&folder, |path| path.extension().is_some_and(|ext| ext == "md")), Some(created("gen/new.md")));
    assert_eq!(session.changes_listing(&folder, |path| path.extension().is_some_and(|ext| ext == "txt")), None);
    assert_eq!(session.changes_listing(&[dir.path().to_path_buf()], |path| path.ends_with("log.jsonl")), None);
}

/// An unresolved destination may create any absent file its pattern
/// matches, and an unresolved append may change any existing one.
#[test]
fn an_unresolved_destination_defers_what_its_pattern_may_reach() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("logs")).unwrap();
    std::fs::write(dir.path().join("logs/old.jsonl"), "").unwrap();
    let source = dir.path().join("doc.md");
    let context = biscuit_file::FileResolutionContext::new(dir.path()).with_repository_root(dir.path());
    let found = initialize(json!([{ "append_line": ["logs/{{ day }}.jsonl", "x"] }]));
    let session = PassiveSession::new();
    let pattern = found[0].pattern(&source, &context, dir.path()).unwrap();
    session.declare_unresolved_write(pattern, found[0].change(), created(&found[0].reference));

    assert!(session.created_file(&dir.path().join("logs/new.jsonl")).is_some());
    assert!(session.created_file(&dir.path().join("logs/new.md")).is_none());
    assert!(session.created_file(&dir.path().join("logs/old.jsonl")).is_none(), "an existing file still exists");
    assert!(matches!(session.initializer_changes(&dir.path().join("logs/old.jsonl")), Some(DeferralReason::ChangedAtLaunch { .. })));
    assert!(session.changes_listing(&[dir.path().join("logs")], |_| true).is_some());
    assert!(session.changes_listing(&[dir.path().join("docs")], |_| true).is_none());
}

/// A dry composition in `dir` whose session declares each `(name, change)`.
fn dry_compose_writes(dir: &Path, text: &str, writes: &[(&str, InitializerChange)]) -> (Result<String, MarkdownError>, PassiveSession) {
    let path: PathBuf = dir.join("doc.md");
    std::fs::write(&path, text).unwrap();
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.clone())
        .with_dry_run();
    let session = options.passive_session().cloned().unwrap();
    for (name, change) in writes {
        session.declare_initializer_write(&dir.join(name), *change, created(name));
    }
    let composed = Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request(options))
        .map(|(composed, _)| composed.content().to_string());
    (composed, session)
}

fn deferred(session: &PassiveSession) -> Vec<(DeferredConstruct, DeferralReason)> {
    session.deferrals().into_iter().map(|deferral| (deferral.construct, deferral.reason)).collect()
}

const APPENDS: InitializerChange = InitializerChange::CreatesOrAppends;
const ENSURES: InitializerChange = InitializerChange::CreatesIfAbsent;

/// Every content reader of an existing file an append targets leaves its
/// verdict to launch; with the file only ensured, each reads it as it is.
macro_rules! existing_content_readers {
    ($($name:ident: $text:expr, $construct:expr;)*) => {$(
        mod $name {
            use super::*;

            fn fixture() -> TempDir {
                let dir = TempDir::new().unwrap();
                std::fs::write(dir.path().join("blank.md"), "").unwrap();
                std::fs::write(dir.path().join("rows.jsonl"), "").unwrap();
                dir
            }

            #[test]
            fn appended() {
                let dir = fixture();
                let name = if $text.contains("rows.jsonl") { "rows.jsonl" } else { "blank.md" };
                let (composed, session) = dry_compose_writes(dir.path(), $text, &[(name, APPENDS)]);
                composed.unwrap();
                assert!(deferred(&session).contains(&($construct, changed(name))), "{:?}", deferred(&session));
            }

            #[test]
            fn ensured() {
                let dir = fixture();
                let name = if $text.contains("rows.jsonl") { "rows.jsonl" } else { "blank.md" };
                let (_, session) = dry_compose_writes(dir.path(), $text, &[(name, ENSURES)]);
                assert!(deferred(&session).is_empty(), "{:?}", deferred(&session));
            }
        }
    )*};
}

existing_content_readers! {
    a_file_directive_on_an_appended_file: "Read:\n\n::file blank.md\n", DeferredConstruct::Transclusion;
    a_code_directive_on_an_appended_file: "Read:\n\n::code blank.md\n", DeferredConstruct::Transclusion;
    a_prologue_on_an_appended_file: "---\nprologue: blank.md\n---\n\nRead.\n", DeferredConstruct::Transclusion;
    a_body_empty_check_on_an_appended_file:
        "::block when=\"!markdown_body_empty('blank.md')\"\nRead.\n::end-block\n", DeferredConstruct::BlockCondition;
    a_title_read_of_an_appended_file: "Title {{ markdown_title('blank.md') }}.\n", DeferredConstruct::Interpolation;
    a_data_read_of_an_appended_file: "Rows {{ length(load_data('rows.jsonl')) }}.\n", DeferredConstruct::Interpolation;
}

/// `file_exists` of an existing file stays proven whatever appends to it;
/// a listing changes only when a declared file is still absent.
#[test]
fn existence_of_an_appended_file_is_proven_and_a_listing_waits_for_a_new_file() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("gen")).unwrap();
    std::fs::write(dir.path().join("gen/old.md"), "").unwrap();
    let text = "{{ file_exists('gen/old.md') }} {{ length(find_files('gen/*.md')) }}\n";

    let (composed, session) = dry_compose_writes(dir.path(), text, &[("gen/old.md", APPENDS)]);
    assert_eq!(composed.unwrap().trim(), "true 1", "{:?}", deferred(&session));

    let (composed, session) = dry_compose_writes(dir.path(), text, &[("gen/new.md", ENSURES)]);
    let body = composed.unwrap();
    assert!(body.contains("{{ length(find_files('gen/*.md')) }}"), "{body}");
    assert!(body.starts_with("true"), "{body}");
    assert!(deferred(&session).contains(&(DeferredConstruct::Interpolation, created("gen/new.md"))), "{:?}", deferred(&session));

    let (composed, _) = dry_compose_writes(dir.path(), text, &[("other/new.md", ENSURES)]);
    assert_eq!(composed.unwrap().trim(), "true 1", "an unrelated file changes no listing");
}

/// `validate_schema` reads the page's `$schema` file and its named-type
/// imports; one the run creates first decides the verdict only at launch.
macro_rules! indirect_schema_reads {
    ($($name:ident: $schema:expr, $declared:expr;)*) => {$(
        mod $name {
            use super::*;

            fn fixture() -> TempDir {
                let dir = TempDir::new().unwrap();
                std::fs::write(dir.path().join("page.md"), "---\n$schema: gen/schema.yaml\ntopic: hi\n---\n").unwrap();
                if let Some(schema) = $schema {
                    std::fs::create_dir_all(dir.path().join("gen")).unwrap();
                    std::fs::write(dir.path().join("gen/schema.yaml"), schema).unwrap();
                }
                dir
            }

            const TEXT: &str = "Valid {{ validate_schema('page.md') }}.\n";

            #[test]
            fn created() {
                let dir = fixture();
                let (composed, session) = dry_compose_writes(dir.path(), TEXT, &[($declared, ENSURES)]);
                composed.unwrap();
                assert!(
                    deferred(&session).iter().any(|(construct, _)| *construct == DeferredConstruct::Interpolation),
                    "{:?}",
                    deferred(&session)
                );
            }

            #[test]
            fn unrelated() {
                let dir = fixture();
                let (composed, _) = dry_compose_writes(dir.path(), TEXT, &[("gen/unrelated.yaml", ENSURES)]);
                assert!(composed.is_err(), "the missing schema file is still missing: {composed:?}");
            }
        }
    )*};
}

indirect_schema_reads! {
    a_created_schema_file_read_through_validate_schema: None::<&str>, "gen/schema.yaml";
    a_created_named_type_import_read_through_validate_schema:
        Some("kind: schema\n$schema:\n    topic: Topic@types.yaml\n"), "gen/types.yaml";
}

/// A link into an existing file an append targets is not judged against
/// the headings the file holds now; without the append, the missing
/// fragment is still an issue.
#[test]
fn a_fragment_link_into_an_appended_file_is_not_judged() {
    use darkmatter::markdown::reference::ReferenceGraphOptions;
    use darkmatter::markdown::reference::validate::ReferenceValidationOptions;

    let issues = |writes: &[(&str, InitializerChange)]| {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("notes.md"), "# Now\n").unwrap();
        let path = dir.path().join("doc.md");
        std::fs::write(&path, "See [later](notes.md#later).\n").unwrap();
        let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), ""))
            .with_source_file(path.clone())
            .with_dry_run();
        let session = options.passive_session().cloned().unwrap();
        for (name, change) in writes {
            session.declare_initializer_write(&dir.path().join(name), *change, created(name));
        }
        let request = crate::request_support::request(options);
        let mut validation = ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&request));
        validation.validate_fragments = true;
        let report = Markdown::try_from(path.as_path()).unwrap().validate_references(validation).unwrap();
        report.issues.len()
    };

    assert_eq!(issues(&[("notes.md", APPENDS)]), 0);
    assert_eq!(issues(&[("notes.md", ENSURES)]), 1, "ensure_file leaves the headings as they are");
}

/// The reference graph does not descend into an existing inclusion an
/// append targets: what it includes is known only at launch.
#[test]
fn the_reference_graph_does_not_walk_an_appended_inclusion() {
    use darkmatter::markdown::reference::ReferenceGraphOptions;
    use darkmatter::markdown::reference::validate::ReferenceValidationOptions;

    let issues = |change: InitializerChange| {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("part.md"), "::file missing.md\n").unwrap();
        let path = dir.path().join("doc.md");
        std::fs::write(&path, "::file part.md\n").unwrap();
        let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), ""))
            .with_source_file(path.clone())
            .with_dry_run();
        options.passive_session().unwrap().declare_initializer_write(&dir.path().join("part.md"), change, created("part.md"));
        let request = crate::request_support::request(options);
        let validation = ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&request));
        Markdown::try_from(path.as_path()).unwrap().validate_references(validation).unwrap().issues.len()
    };

    assert_eq!(issues(APPENDS), 0);
    assert_eq!(issues(ENSURES), 1, "an unchanged inclusion is walked");
}

/// The pre-flight walk does not read an existing inclusion or `prologue`
/// an append targets: what it holds is known only at launch.
macro_rules! preflight_walks {
    ($($name:ident: $text:expr;)*) => {$(
        #[test]
        fn $name() {
            let walk = |change: InitializerChange| {
                let dir = TempDir::new().unwrap();
                std::fs::write(dir.path().join("part.md"), "::file missing.md\n").unwrap();
                let path = dir.path().join("doc.md");
                std::fs::write(&path, $text).unwrap();
                let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), ""))
                    .with_source_file(path.clone())
                    .with_dry_run();
                options.passive_session().unwrap().declare_initializer_write(&dir.path().join("part.md"), change, created("part.md"));
                let request = crate::request_support::request(options);
                Markdown::try_from(path.as_path()).unwrap().compose_preflight(&request).map(|_| ())
            };
            walk(APPENDS).expect("an appended inclusion is not walked");
            assert!(walk(ENSURES).is_err(), "an unchanged inclusion is walked and its missing child refused");
        }
    )*};
}

preflight_walks! {
    the_preflight_walk_skips_an_appended_inclusion: "::file part.md\n";
    the_preflight_walk_skips_an_appended_prologue: "---\nprologue: part.md\n---\n\nBody.\n";
}
