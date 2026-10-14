//! One environment per document: every consumer of a document entered under a
//! supplied map reads that map, a second document reads its own, and
//! re-preparing a document captures nothing new.
//!
//! The maps are supplied; no test here touches the process environment. The
//! probe variable is asserted absent from the process so a pass cannot come
//! from an ambient value.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use darkmatter::markdown::compose::{ContextRequirements, RequestSnapshot};
use darkmatter::markdown::schemas::{SchemaRootKind, SchemaRoots};
use tempfile::TempDir;

use super::{DocumentEnvironment, EnvironmentSource, InvocationContext};
use crate::composition::{
    PrepareOptions, PreparedComposition, ResolvedCompositionSource, approve_document_shell,
    prepare_direct, resolve_fail_fast_from_env, resolve_max_iterations_from_env,
    resolve_pause_reset_margin_from_env,
};
use crate::harness::ShellApprovalOptions;

const PROBE: &str = "CLAUDINE_DOCUMENT_ENVIRONMENT_PROBE";

const DOCUMENT: &str = "---\ntitle: probe\n---\n\
env=[{{ env.CLAUDINE_DOCUMENT_ENVIRONMENT_PROBE }}] \
agent=[{{ ctx.agent }}] lazy=[{{ current.agent }}] \
ref=[{{ markdown_title(\"{{CLAUDINE_DOCUMENT_ENVIRONMENT_PROBE}}/target.md\") }}]\n\n\
::file ./child.md\n\n\
::shell echo shell={{ env.CLAUDINE_DOCUMENT_ENVIRONMENT_PROBE }}\n";

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    home: PathBuf,
    document: PathBuf,
    approval: ShellApprovalOptions,
}

impl Fixture {
    fn new() -> Self {
        assert!(std::env::var_os(PROBE).is_none(), "the process must not carry {PROBE}");
        let temp = TempDir::new().unwrap();
        // Canonical so macOS `/var` and `/private/var` spellings agree.
        let root = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
        let home = root.join("home");
        for (dir, title) in [("a", "Title A"), ("b", "Title B")] {
            fs::create_dir_all(root.join(dir).join("schemas")).unwrap();
            fs::write(root.join(dir).join("target.md"), format!("# {title}\n")).unwrap();
        }
        let work = root.join("work");
        fs::create_dir_all(&work).unwrap();
        fs::create_dir_all(&home).unwrap();
        let document = work.join("doc.md");
        fs::write(&document, DOCUMENT).unwrap();
        fs::write(work.join("child.md"), format!("child=[{{{{ env.{PROBE} }}}}]\n")).unwrap();
        let policy = root.join("policy");
        fs::create_dir_all(&policy).unwrap();
        fs::write(policy.join(".darkmatter-shell-whitelist"), "prefix echo\n").unwrap();
        let approval = ShellApprovalOptions {
            policy_root: Some(policy),
            approval_handler: None,
            ..Default::default()
        };
        Self { _temp: temp, root, home, document, approval }
    }

    /// The map a document named `dir` is entered under: every value differs
    /// from the other document's and from anything ambient.
    fn map(&self, dir: &str) -> DocumentEnvironment {
        DocumentEnvironment::new(HashMap::from([
            (PROBE.to_string(), biscuit_file::to_portable_string(&self.root.join(dir))),
            ("AGENT".to_string(), format!("agent-{dir}")),
            ("MODEL".to_string(), format!("model-{dir}")),
            ("SCHEMAS_DIR".to_string(), self.schemas(dir).display().to_string()),
            ("CLAUDINE_MAX_ITERATIONS".to_string(), if dir == "a" { "3" } else { "7" }.to_string()),
            ("CLAUDINE_FAIL_FAST".to_string(), (dir == "a").to_string()),
            ("CLAUDINE_PAUSE_RESET_MARGIN".to_string(), if dir == "a" { "1s" } else { "2s" }.to_string()),
        ]))
    }

    fn schemas(&self, dir: &str) -> PathBuf {
        self.root.join(dir).join("schemas")
    }

    /// The invocation the CLI boundary captures, launched under map `a`.
    fn launch(&self) -> InvocationContext {
        let snapshot = RequestSnapshot::new(self.document.parent().unwrap())
            .with_home(Some(self.home.clone()))
            .with_env(self.map("a").vars().clone());
        InvocationContext::capture(&snapshot).unwrap()
    }

    fn expected(&self, dir: &str) -> String {
        let value = biscuit_file::to_portable_string(&self.root.join(dir));
        let title = if dir == "a" { "Title A" } else { "Title B" };
        format!("env=[{value}] agent=[agent-{dir}] lazy=[agent-{dir}] ref=[{title}]")
    }
}

struct Prepared {
    composition: PreparedComposition,
    approved: HashSet<String>,
    options: PrepareOptions,
}

/// Prepare the fixture document the way the canonical command path does:
/// a source and an epoch derived from one invocation view, an audit, then
/// the preparation that runs the audited commands.
fn prepare_under(fixture: &Fixture, invocation: &InvocationContext) -> Prepared {
    let text = fs::read_to_string(&fixture.document).unwrap();
    let source = ResolvedCompositionSource {
        original_ref: fixture.document.display().to_string(),
        resolved_path: fixture.document.clone(),
        original_text: text.clone(),
        markdown: text.into(),
    };
    let source_context = invocation.derive_source(&fixture.document).unwrap();
    let epoch = invocation.begin_document_epoch();
    let prepared_context =
        epoch.capture_launch_context(&ContextRequirements::for_document(&source.markdown));
    let mut options = PrepareOptions {
        invocation_context: Some(invocation.clone()),
        document_epoch: Some(epoch),
        prepared_context: Some(prepared_context),
        file_ref_fallback_dir: Some(invocation.launch_cwd().to_path_buf()),
        ..PrepareOptions::new(source_context.file_resolution_context().clone())
    };
    let approved = approve_document_shell(&source, &options, &fixture.approval)
        .unwrap()
        .approved_commands;
    options.pre_approved_commands = Some(approved.clone());
    let composition = prepare_direct(&source, options.clone()).unwrap();
    Prepared { composition, approved, options }
}

fn schemas_dir_root(context: &biscuit_file::FileResolutionContext) -> Option<PathBuf> {
    SchemaRoots::for_document(context)
        .unwrap()
        .searched()
        .iter()
        .find(|root| root.kind == SchemaRootKind::SchemasDir)
        .map(|root| root.path.clone())
}

/// Assert that every consumer of one prepared document read `dir`'s map.
fn assert_document_reads(fixture: &Fixture, prepared: &Prepared, dir: &str) {
    let value = biscuit_file::to_portable_string(&fixture.root.join(dir));
    let prompt = &prepared.composition.prompt;
    assert!(prompt.contains(&fixture.expected(dir)), "{dir}: {prompt}");
    assert!(prompt.contains(&format!("child=[{value}]")), "the transclusion reads {dir}: {prompt}");
    assert!(prompt.contains(&format!("shell={value}")), "the executed command reads {dir}: {prompt}");
    assert_eq!(
        prepared.approved,
        HashSet::from([format!("echo shell={value}")]),
        "the approved command text reads {dir}",
    );
    let environment = prepared.options.document_epoch.as_ref().unwrap().environment();
    assert_eq!(environment, &fixture.map(dir));
    assert_eq!(prepared.options.file_resolution_context.env(), fixture.map(dir).vars());
    assert_eq!(
        prepared.composition.compose_context.env().get(PROBE),
        Some(&value),
        "ctx environment values read {dir}",
    );
    assert_eq!(
        schemas_dir_root(&prepared.options.file_resolution_context).as_deref(),
        Some(fixture.schemas(dir).as_path()),
        "`SCHEMAS_DIR` selects {dir}'s schema root",
    );
    let (model, _) = crate::composition::resolve_model_with_hints(
        crate::provider::Provider::Claude,
        &prepared.composition.selection_hints,
        None,
        None,
        environment,
    );
    assert_eq!(model.as_deref(), Some(format!("model-{dir}").as_str()), "selector input");
    assert_eq!(
        crate::composition::prepared_environment(&prepared.composition),
        fixture.map(dir),
    );
    assert_eq!(resolve_max_iterations_from_env(environment), Some(if dir == "a" { 3 } else { 7 }));
    assert_eq!(resolve_fail_fast_from_env(environment), Some(dir == "a"));
    assert_eq!(
        resolve_pause_reset_margin_from_env(environment),
        Some(std::time::Duration::from_secs(if dir == "a" { 1 } else { 2 })),
    );
}

#[test]
fn every_consumer_of_the_launch_document_reads_its_captured_map() {
    let fixture = Fixture::new();
    let launch = fixture.launch();

    let prepared = prepare_under(&fixture, &launch);

    assert_document_reads(&fixture, &prepared, "a");
}

#[test]
fn a_second_document_reads_its_own_map_and_the_first_keeps_its_own() {
    let fixture = Fixture::new();
    let first = fixture.launch();
    let first_prepared = prepare_under(&fixture, &first);

    let second = first.enter_document(fixture.map("b"));
    let second_prepared = prepare_under(&fixture, &second);

    assert_document_reads(&fixture, &second_prepared, "b");
    assert_document_reads(&fixture, &first_prepared, "a");
    // Re-preparing the first document after the second was entered still
    // reads the first document's map.
    assert_document_reads(&fixture, &prepare_under(&fixture, &first), "a");
    // Only the environment moved: home, launch directory, the launch `@`
    // scope, and the repository evidence are the invocation's.
    assert_eq!(second.home_dir(), first.home_dir());
    assert_eq!(second.launch_cwd(), first.launch_cwd());
    assert_eq!(
        second.launch_file_resolution_context().launch_magic_scope(),
        first.launch_file_resolution_context().launch_magic_scope(),
    );
    assert_eq!(
        &second
            .launch_file_resolution_context()
            .clone()
            .with_env(fixture.map("a").vars().clone()),
        first.launch_file_resolution_context(),
    );
}

#[test]
fn re_preparing_a_document_captures_no_new_environment() {
    let fixture = Fixture::new();
    let launch = fixture.launch();
    assert_eq!(launch.work_snapshot().document_environment_captures, 1);

    let task = launch.enter_document(fixture.map("b"));
    assert_eq!(task.work_snapshot().document_environment_captures, 2);
    let prepared = prepare_under(&fixture, &task);
    // A re-preparation of the same document (a loop iteration, a retry, the
    // stabilized reread) opens new epochs and extends their snapshots from
    // the same view.
    let again = prepare_under(&fixture, &task);
    let mut context = prepared.composition.compose_context.clone();
    prepared
        .options
        .document_epoch
        .as_ref()
        .unwrap()
        .extend_launch_context(&mut context, &ContextRequirements::from_groups([
            darkmatter::markdown::compose::ContextGroup::Os,
        ]));

    assert_document_reads(&fixture, &again, "b");
    assert_eq!(context.env().get(PROBE), prepared.composition.compose_context.env().get(PROBE));
    assert_eq!(task.work_snapshot().document_environment_captures, 2);
}

#[test]
fn a_captured_document_reads_the_installed_source_or_else_the_launch_map() {
    let fixture = Fixture::new();
    let launch = fixture.launch();

    let unsourced = launch.capture_document();
    let b = fixture.map("b");
    let sourced = launch
        .clone()
        .with_environment_source(EnvironmentSource::fixed(b.clone()))
        .capture_document();

    assert_eq!(unsourced.document_environment(), &fixture.map("a"));
    assert_eq!(sourced.document_environment(), &b);
    assert_eq!(sourced.request_snapshot().env(), b.vars());
    assert_eq!(launch.document_environment(), &fixture.map("a"));
}

#[test]
fn a_child_process_starts_from_the_document_map_with_the_launch_home() {
    let fixture = Fixture::new();
    let launch = fixture.launch();
    let mut changed = fixture.map("b").vars().clone();
    changed.insert("HOME".to_string(), "/not/the/home".to_string());

    let document = launch.enter_document(DocumentEnvironment::new(changed));

    let baseline = document.env_baseline();
    assert_eq!(
        baseline.get(PROBE),
        Some(std::ffi::OsStr::new(&biscuit_file::to_portable_string(&fixture.root.join("b")))),
    );
    // A raw variable the launch map never named is the document's too.
    let launch_baseline = launch.env_baseline();
    for (name, value) in launch_baseline.iter() {
        if name.to_str().is_none_or(|name| !launch.environment().contains_key(name)) {
            assert_eq!(baseline.get(name), Some(value), "{name:?} is kept");
        }
    }
    assert_eq!(baseline.get("HOME"), launch_baseline.get("HOME"), "home stays request-owned");
    assert_eq!(document.home_baseline(), launch.home_baseline());
}

/// A library caller that supplies only a resolution context (no prepared
/// snapshot, so Darkmatter captures `ctx` from the host) still composes
/// under that context's map: a process value of the same name does not
/// outrank it.
#[test]
fn a_preparation_without_a_snapshot_reads_its_resolution_context_map() {
    let fixture = Fixture::new();
    assert!(std::env::var_os("PATH").is_some(), "the process carries a PATH to outrank");
    let document = fixture.document.with_file_name("plain.md");
    fs::write(&document, "---\ntitle: plain\n---\npath=[{{ env.PATH }}] agent=[{{ ctx.agent }}]\n").unwrap();
    let text = fs::read_to_string(&document).unwrap();
    let source = ResolvedCompositionSource {
        original_ref: document.display().to_string(),
        resolved_path: document.clone(),
        original_text: text.clone(),
        markdown: text.into(),
    };
    let mut map = fixture.map("b").vars().clone();
    map.insert("PATH".to_string(), "/document/path".to_string());
    let launch = fixture.launch().enter_document(DocumentEnvironment::new(map));
    let context = launch.derive_source(&document).unwrap().file_resolution_context().clone();

    let prepared = prepare_direct(&source, PrepareOptions::new(context)).unwrap();

    assert!(prepared.prompt.contains("path=[/document/path]"), "{}", prepared.prompt);
}
