//! The frontmatter-surface projection composes a document's effective
//! frontmatter without reading its body.
//!
//! A staged run reads the frontmatter that drives an initialization step, runs
//! that step (which may create a file the body includes), and only then
//! composes the whole document. These tests pin both halves of that contract:
//! the projection never dereferences body dependencies — including through the
//! pre-approval check a caller-supplied approval set turns on — and full
//! composition and condition-blind preflight still fail on the same missing
//! file, so the projection is an ordering tool rather than an error
//! suppression.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::{
    ComposeOperation, ComposeOptions, ShellExpansionError, collect_frontmatter_shell_commands,
};
use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::json;
use tempfile::TempDir;

/// The shape of the reported failure: a log path derived from a caller input,
/// guarded by nested conditions and included by the body, not yet on disk.
const GENERATED_LOG_DOCUMENT: &str = r#"---
phase: 1
log: "{{ dirname(spec) + '/implementation-log.md' }}"
has_log: "{{ file_exists(log) }}"
initialize:
  - ensure_file: "{{ log }}"
  - shell: "echo {{ log }}"
---
# Implement phase {{ phase }}

::block when="file_exists(log)"
::block when="phase > 1"
::file {{ log }}
::end-block
::end-block

::file ./always-missing.md
"#;

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    dir: PathBuf,
}

impl Fixture {
    fn new(content: &str) -> Self {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().to_path_buf();
        let root = dir.join("prompt.md");
        fs::write(&root, content).unwrap();
        Self {
            _temp: temp,
            root,
            dir,
        }
    }

    fn markdown(&self) -> Markdown {
        fs::read_to_string(&self.root).unwrap().into()
    }

    /// Options shaped like a staged caller's: source file, caller inputs, the
    /// lifecycle subtree excluded, a policy root, and a generous shell leash
    /// so parallel-suite load cannot masquerade as a timeout.
    fn options(&self) -> ComposeOptions {
        ComposeOptions::new()
            .with_source_file(&self.root)
            .with_shell_policy_root(&self.dir)
            .with_shell_timeout(std::time::Duration::from_secs(60))
            .with_set_overrides(json!({ "spec": "fixes/demo/spec.md" }))
            .with_exclude_keys(["initialize"])
    }
}

fn approved(commands: &[&str]) -> HashSet<String> {
    commands.iter().map(|command| (*command).to_string()).collect()
}

fn frontmatter_str<'a>(md: &'a Markdown, key: &str) -> &'a str {
    md.frontmatter()
        .as_map()
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or_else(|| panic!("frontmatter key `{key}` missing or not a string"))
}

fn is_not_pre_approved(err: &MarkdownError) -> bool {
    matches!(
        err,
        MarkdownError::ShellExpansion(inner)
            if matches!(**inner, ShellExpansionError::NotPreApproved { .. })
    )
}

fn portable(path: &Path) -> String {
    biscuit_file::to_portable_string(path)
}

#[test]
fn projection_reads_frontmatter_without_dereferencing_missing_includes() {
    let fixture = Fixture::new(GENERATED_LOG_DOCUMENT);
    let md = fixture.markdown();

    let (projected, report) = md
        .compose_with(fixture.options().only_frontmatter_surface())
        .expect("the projection must not dereference a missing include");

    assert_eq!(
        frontmatter_str(&projected, "log"),
        "fixes/demo/implementation-log.md",
        "a caller input flows through `dirname` into a derived key"
    );
    assert_eq!(
        projected.frontmatter().as_map()["has_log"],
        json!(false),
        "`file_exists` resolves against the source before the file exists"
    );

    let initialize = serde_json::to_string(&projected.frontmatter().as_map()["initialize"]).unwrap();
    assert!(
        initialize.contains("{{ log }}"),
        "an excluded lifecycle subtree keeps its spans verbatim: {initialize}"
    );
    assert_eq!(
        report.deferred_frontmatter_keys,
        HashSet::from(["initialize".to_string()])
    );

    // The body is exactly as authored: no transclusion, no page-block
    // evaluation, no body interpolation.
    assert_eq!(projected.content(), md.content());
    assert_eq!(report.shell_approvals_used, 0);
    assert!(
        !fixture.dir.join("fixes").exists(),
        "the projection runs no lifecycle effect"
    );
}

#[test]
fn projection_sees_the_generated_file_once_it_exists() {
    let fixture = Fixture::new(GENERATED_LOG_DOCUMENT);
    let log_dir = fixture.dir.join("fixes/demo");
    fs::create_dir_all(&log_dir).unwrap();
    fs::write(log_dir.join("implementation-log.md"), "## Phase 1\n").unwrap();

    let (projected, _) = fixture
        .markdown()
        .compose_with(fixture.options().only_frontmatter_surface())
        .unwrap();

    assert_eq!(projected.frontmatter().as_map()["has_log"], json!(true));
}

#[test]
fn full_compose_and_preflight_still_fail_on_the_missing_include() {
    let fixture = Fixture::new(GENERATED_LOG_DOCUMENT);
    let md = fixture.markdown();

    let full = md.compose_with(fixture.options());
    assert!(
        matches!(full, Err(MarkdownError::Transclusion(_))),
        "full composition must still fail on the missing include: {full:?}"
    );

    let full_with_approvals = md.compose_with(
        fixture
            .options()
            .with_pre_approved_commands(approved(&["echo fixes/demo/implementation-log.md"])),
    );
    assert!(
        full_with_approvals.is_err(),
        "the condition-blind approval walk must still dereference the include"
    );

    let preflight = md.compose_preflight(&fixture.options());
    assert!(
        preflight.is_err(),
        "condition-blind preflight must still resolve every include: {:?}",
        preflight.map(|report| report.approval_set())
    );
}

#[test]
fn projection_with_pre_approved_commands_skips_the_body_graph_walk() {
    let fixture = Fixture::new(GENERATED_LOG_DOCUMENT);
    let md = fixture.markdown();

    // An empty approval set is still an approval set: the up-front check runs,
    // and it must consult frontmatter commands only.
    let (projected, _) = md
        .compose_with(
            fixture
                .options()
                .with_pre_approved_commands(HashSet::new())
                .only_frontmatter_surface(),
        )
        .expect("an approval set must not turn the projection into a body walk");

    assert_eq!(frontmatter_str(&projected, "log"), "fixes/demo/implementation-log.md");
}

#[test]
fn projection_runs_approved_frontmatter_commands_but_never_body_commands() {
    let temp = TempDir::new().unwrap();
    let front = temp.path().join("front-ran");
    let body = temp.path().join("body-ran");
    let content = format!(
        "---\nfront: \"$(touch {front})\"\n---\n::shell touch {body}\n::file ./missing.md\n",
        front = portable(&front),
        body = portable(&body),
    );
    let fixture = Fixture::new(&content);

    // The body command is deliberately unapproved; the projection never needs it.
    let options = fixture
        .options()
        .with_pre_approved_commands(approved(&[&format!("touch {}", portable(&front))]))
        .only_frontmatter_surface();
    let (projected, _) = fixture.markdown().compose_with(options).unwrap();

    assert!(front.exists(), "the approved frontmatter command runs");
    assert!(!body.exists(), "a body `::shell` never runs through the projection");
    assert!(projected.content().contains("::shell touch"));
}

#[test]
fn projection_refuses_an_unapproved_frontmatter_command_before_any_runs() {
    let temp = TempDir::new().unwrap();
    let sentinel = temp.path().join("sentinel");
    let content = format!(
        "---\nfirst: \"$(touch {sentinel})\"\nsecond: \"$(echo unapproved)\"\n---\nbody\n",
        sentinel = portable(&sentinel),
    );
    let fixture = Fixture::new(&content);

    let options = fixture
        .options()
        .with_pre_approved_commands(approved(&[&format!("touch {}", portable(&sentinel))]))
        .only_frontmatter_surface();
    let err = fixture.markdown().compose_with(options).unwrap_err();

    assert!(is_not_pre_approved(&err), "unexpected error: {err:?}");
    assert!(
        !sentinel.exists(),
        "no frontmatter command may run before the whole frontmatter set is approved"
    );
}

#[test]
fn full_compose_still_refuses_an_unapproved_body_command() {
    let temp = TempDir::new().unwrap();
    let sentinel = temp.path().join("sentinel");
    let content = format!(
        "---\nfront: \"$(touch {sentinel})\"\n---\n::shell echo body-cmd\n",
        sentinel = portable(&sentinel),
    );
    let fixture = Fixture::new(&content);
    let approval = approved(&[&format!("touch {}", portable(&sentinel))]);

    let err = fixture
        .markdown()
        .compose_with(fixture.options().with_pre_approved_commands(approval.clone()))
        .unwrap_err();
    assert!(is_not_pre_approved(&err), "unexpected error: {err:?}");
    assert!(!sentinel.exists(), "full compose must refuse before any command runs");

    // The same approval set is sufficient for the projection.
    fixture
        .markdown()
        .compose_with(
            fixture
                .options()
                .with_pre_approved_commands(approval)
                .only_frontmatter_surface(),
        )
        .unwrap();
    assert!(sentinel.exists());
}

#[test]
fn projection_honors_the_schema_verdict_setting() {
    let content = "---\n$schema:\n  title: 'string(required)'\nlog: \"{{ 'a' + '/b' }}\"\n---\n::file ./missing.md\n";
    let fixture = Fixture::new(content);
    let md = fixture.markdown();

    let owned = md.compose_with(fixture.options().only_frontmatter_surface());
    assert!(
        matches!(owned, Err(MarkdownError::SchemaValidationFailed { .. })),
        "a projection that owns the verdict reports it: {owned:?}"
    );

    let (deferred, _) = md
        .compose_with(
            fixture
                .options()
                .with_deferred_schema_verdict(true)
                .only_frontmatter_surface(),
        )
        .expect("a deferred verdict is not reported by the projection");
    assert_eq!(frontmatter_str(&deferred, "log"), "a/b");
}

#[test]
fn projection_keeps_a_caller_disabled_frontmatter_operation_disabled() {
    let temp = TempDir::new().unwrap();
    let sentinel = temp.path().join("sentinel");
    let content = format!("---\nfront: \"$(touch {})\"\n---\nbody\n", portable(&sentinel));
    let fixture = Fixture::new(&content);

    let options = fixture
        .options()
        .disable(ComposeOperation::FrontmatterShellExpansion)
        .only_frontmatter_surface();
    assert!(options.is_frontmatter_surface_only());
    assert!(options.is_enabled(ComposeOperation::FrontmatterInterpolation));
    assert!(!options.is_enabled(ComposeOperation::FrontmatterShellExpansion));

    let (projected, _) = fixture.markdown().compose_with(options).unwrap();
    assert!(frontmatter_str(&projected, "front").starts_with("$(touch"));
    assert!(!sentinel.exists());
}

#[test]
fn frontmatter_surface_only_reflects_every_body_operation() {
    assert!(!ComposeOptions::new().is_frontmatter_surface_only());
    assert!(ComposeOptions::new().only_frontmatter_surface().is_frontmatter_surface_only());
    assert!(ComposeOptions::new().only(&[]).is_frontmatter_surface_only());

    for op in ComposeOperation::default_order() {
        let options = ComposeOptions::new().only(&[ComposeOperation::FrontmatterInterpolation, *op]);
        let body_op = !matches!(
            op,
            ComposeOperation::FrontmatterInterpolation | ComposeOperation::FrontmatterShellExpansion
        );
        assert_eq!(
            options.is_frontmatter_surface_only(),
            !body_op,
            "{op:?} must count as a body operation exactly when it is not a frontmatter one"
        );
    }
}

#[test]
fn frontmatter_collector_enumerates_frontmatter_commands_only() {
    let content = "\
---
fast: true
who: \"$(whoami)\"
mode: \"$(fast ? echo fast : echo slow)\"
initialize:
  shell: \"$(echo lifecycle)\"
---
::shell echo body
::shell-block
echo block
::end-block
::file ./missing.md
";
    let fixture = Fixture::new(content);
    let md = fixture.markdown();

    let entries = collect_frontmatter_shell_commands(&md, &fixture.options())
        .expect("the frontmatter collector must not read the body");
    let normalized: Vec<&str> = entries.iter().map(|entry| entry.normalized.as_str()).collect();

    assert!(normalized.contains(&"whoami"), "{normalized:?}");
    assert!(normalized.contains(&"echo fast"), "every ternary branch: {normalized:?}");
    assert!(normalized.contains(&"echo slow"), "every ternary branch: {normalized:?}");
    for absent in ["echo body", "echo block", "echo lifecycle"] {
        assert!(!normalized.contains(&absent), "{absent} collected: {normalized:?}");
    }

    // The condition-blind graph collector sees the body and fails on the include.
    assert!(md.compose_preflight(&fixture.options()).is_err());
}
