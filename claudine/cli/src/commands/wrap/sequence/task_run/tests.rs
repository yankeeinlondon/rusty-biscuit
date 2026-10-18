use std::path::{Path, PathBuf};
use std::process::Command;

use claudine::composition::sequence::preflight::{GroupExecution, PreflightAction};
use serde_json::{Map, json};

use super::*;

/// A `ctx.*` reference whose group (`Repo`) is never captured by default, so
/// its presence in the composed context proves the field was scanned.
const CTX_REF: &str = "{{ ctx.repo_root }}";

/// A side effect with no authored order — like every task built here, it is
/// synthesized in memory rather than walked out of an authored document.
fn side_effect(action: Value) -> PreflightAction {
    PreflightAction::SideEffect {
        action,
        authored_set_order: None,
    }
}

fn init_git_repo(path: &Path) {
    let status = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(path)
        .status()
        .expect("git must be available for repository-context tests");
    assert!(status.success());
}

/// A sequence launched in one repository authoring its task in another.
struct Fixture {
    _root: tempfile::TempDir,
    launch_repo: PathBuf,
    launch_dir: PathBuf,
    task_repo: PathBuf,
    task_dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let launch_repo = root.path().join("launch");
        let launch_dir = launch_repo.join("alpha/lib");
        let task_repo = root.path().join("tasks");
        let task_dir = task_repo.join("nested");
        std::fs::create_dir_all(&launch_dir).unwrap();
        std::fs::create_dir_all(&task_dir).unwrap();
        init_git_repo(&launch_repo);
        init_git_repo(&task_repo);
        std::fs::write(
            launch_repo.join("Cargo.toml"),
            "[workspace]\nmembers = [\"alpha/lib\", \"sibling\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        std::fs::write(
            launch_dir.join("Cargo.toml"),
            "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(launch_repo.join("sibling")).unwrap();
        std::fs::write(
            launch_repo.join("sibling/Cargo.toml"),
            "[package]\nname = \"sibling\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        Self {
            _root: root,
            launch_repo,
            launch_dir,
            task_repo,
            task_dir,
        }
    }

    fn origin_path(&self) -> PathBuf {
        self.task_dir.join("task.yaml")
    }

    /// A task carrying no expression at all, for a caller to plant one in.
    fn task(&self) -> PreflightTask {
        PreflightTask {
            name: Some("external".to_string()),
            label: "external".to_string(),
            action: PreflightAction::Shell { commands: vec!["true".to_string()] },
            with: Map::new(),
            timeout: None,
            when: None,
            operation: None,
            flow: None,
            setup: None,
            teardown: None,
            origin_dir: self.task_dir.clone(),
            origin_path: self.origin_path(),
            authored: claudine::composition::AuthoredOrder::default(),
            diagnostic: claudine::composition::sequence::preflight::TaskDiagnosticProvenance {
                source_path: self.origin_path(),
                action_property: "shell".to_string(),
                task_property: String::new(),
            },
            approved_stack_commands: Vec::new(),
            preflight_approvals: Vec::new(),
            position: claudine::composition::TaskPosition::First,
            shell_sites: Vec::new(),
            approved_set_values: Vec::new(),
            identity: Default::default(),
            provider: None,
        }
    }
}

#[test]
fn task_source_belongs_to_the_authoring_repo_context_to_the_launch_repo() {
    let fixture = Fixture::new();
    let origin_path = fixture.origin_path();
    let mut task = fixture.task();
    task.action = side_effect(json!({"stderr": CTX_REF}));
    let invocation = InvocationContext::capture_at(crate::request::snapshot(), &fixture.launch_dir).unwrap();
    let env = BTreeMap::from([("TASK_MARKER".to_string(), "owned".to_string())]);

    let (source, context, _) = prepare_task_context(&invocation, &task, &env, &RunEvidence::default());

    // File resolution stays source-relative: the task document's own
    // repository drives references and provenance.
    assert_eq!(source.base_dir(), fixture.task_dir);
    assert_eq!(source.repository_root(), Some(fixture.task_repo.as_path()));
    assert_eq!(
        source.file_resolution_context().source_path(),
        Some(origin_path.as_path())
    );
    assert_eq!(
        source.file_resolution_context().repository_root(),
        Some(fixture.task_repo.as_path())
    );
    // Prepared `ctx.*` is launch-anchored: the task may be authored in a
    // different repository, but `ctx.repo_root` still reports the one the
    // caller launched from (D1/AC7).
    assert_eq!(
        context.get("repo_root").and_then(Value::as_str),
        Some(biscuit_file::to_portable_string(&fixture.launch_repo).as_str())
    );
    assert_eq!(context.env().get("TASK_MARKER").map(String::as_str), Some("owned"));
}

#[test]
fn distributed_task_uses_launch_area_and_selected_target_identity() {
    let fixture = Fixture::new();
    std::fs::write(
        fixture.origin_path(),
        "kind: task\naction: {stderr: task}\n",
    )
    .unwrap();
    let mut task = fixture.task();
    task.action = side_effect(json!({
        "stderr": "{{ ctx.repo_root }}|{{ ctx.area }}|{{ ctx.agent }}|{{ ctx.model }}|{{ env.AGENT }}|{{ env.MODEL }}"
    }));
    let invocation = InvocationContext::capture_at(crate::request::snapshot(), &fixture.launch_dir).unwrap();
    let env = BTreeMap::from([
        ("AGENT".to_string(), "codex".to_string()),
        ("MODEL".to_string(), "gpt-5".to_string()),
    ]);

    let (source, context, _) = prepare_task_context(&invocation, &task, &env, &RunEvidence::default());

    assert_eq!(source.repository_root(), Some(fixture.task_repo.as_path()));
    assert_eq!(
        context.get("repo_root").and_then(Value::as_str),
        Some(biscuit_file::to_portable_string(&fixture.launch_repo).as_str())
    );
    assert_eq!(context.get("area").and_then(Value::as_str), Some("alpha"));
    let effective = context.as_object();
    assert_eq!(effective.get("agent").and_then(Value::as_str), Some("codex"));
    assert_eq!(effective.get("model").and_then(Value::as_str), Some("gpt-5"));
    assert_eq!(context.env().get("AGENT").map(String::as_str), Some("codex"));
    assert_eq!(context.env().get("MODEL").map(String::as_str), Some("gpt-5"));
}

/// The control the per-field cases are read against: without a reference the
/// group is genuinely absent, so `repo_root` being present below can only
/// have come from the planted expression.
#[test]
fn a_task_with_no_ctx_reference_captures_no_repository_group() {
    let fixture = Fixture::new();
    let invocation = InvocationContext::capture_at(crate::request::snapshot(), &fixture.launch_dir).unwrap();

    let (_, context, _) =
        prepare_task_context(&invocation, &fixture.task(), &BTreeMap::new(), &RunEvidence::default());

    assert_eq!(context.get("repo_root"), None);
}

#[test]
fn every_expression_bearing_field_is_scanned_for_ctx_references() {
    let fixture = Fixture::new();
    let invocation = InvocationContext::capture_at(crate::request::snapshot(), &fixture.launch_dir).unwrap();

    #[allow(clippy::type_complexity)]
    let cases: Vec<(&str, Box<dyn Fn(&mut PreflightTask)>)> = vec![
        (
            "action (side_effect)",
            Box::new(|task| task.action = side_effect(json!({"stderr": CTX_REF}))),
        ),
        (
            "action (shell)",
            Box::new(|task| {
                task.action = PreflightAction::Shell {
                    commands: vec![format!("echo {CTX_REF}")],
                };
            }),
        ),
        (
            "action (prompt reference)",
            Box::new(|task| {
                task.action = PreflightAction::Prompt {
                    path: PathBuf::from("/resolved/prompt.md"),
                    reference: format!("{CTX_REF}/prompt.md"),
                };
            }),
        ),
        (
            "with",
            Box::new(|task| {
                task.with.insert("target".to_string(), json!(CTX_REF));
            }),
        ),
        ("timeout", Box::new(|task| task.timeout = Some(json!(CTX_REF)))),
        (
            "operation",
            Box::new(|task| task.operation = Some(CTX_REF.to_string())),
        ),
        ("flow", Box::new(|task| task.flow = Some(CTX_REF.to_string()))),
        (
            "setup",
            Box::new(|task| task.setup = Some(json!([{"shell": format!("echo {CTX_REF}")}]))),
        ),
        (
            "teardown",
            Box::new(|task| {
                task.teardown = Some(json!([{"shell": format!("echo {CTX_REF}")}]));
            }),
        ),
        (
            "name",
            Box::new(|task| task.name = Some(CTX_REF.to_string())),
        ),
        ("label", Box::new(|task| task.label = CTX_REF.to_string())),
    ];

    for (field, plant) in cases {
        let mut task = fixture.task();
        plant(&mut task);

        let (_, context, _) = prepare_task_context(&invocation, &task, &BTreeMap::new(), &RunEvidence::default());

        assert_eq!(
            context.get("repo_root").and_then(Value::as_str),
            Some(biscuit_file::to_portable_string(&fixture.launch_repo).as_str()),
            "`{field}` was not scanned for runtime-context references"
        );
    }
}

#[test]
fn group_variables_and_member_tasks_are_scanned_for_ctx_references() {
    let fixture = Fixture::new();
    let invocation = InvocationContext::capture_at(crate::request::snapshot(), &fixture.launch_dir).unwrap();

    let group_of = |variables: Map<String, Value>, member: PreflightTask| PreflightAction::Group(
        PreflightGroup {
            name: "batch".to_string(),
            execution: GroupExecution::Serial,
            fail_fast: true,
            lifecycle: Default::default(),
            max_parallel: None,
            variables,
            tasks: vec![member],
            origin_path: fixture.origin_path(),
            loop_config: None,
        },
    );

    let mut member = fixture.task();
    member.teardown = Some(json!([{"shell": format!("echo {CTX_REF}")}]));

    for (label, action) in [
        (
            "group variables",
            group_of(
                Map::from_iter([("target".to_string(), json!(CTX_REF))]),
                fixture.task(),
            ),
        ),
        ("group member task", group_of(Map::new(), member)),
    ] {
        let mut task = fixture.task();
        task.action = action;

        let (_, context, _) = prepare_task_context(&invocation, &task, &BTreeMap::new(), &RunEvidence::default());

        assert_eq!(
            context.get("repo_root").and_then(Value::as_str),
            Some(biscuit_file::to_portable_string(&fixture.launch_repo).as_str()),
            "`{label}` was not scanned for runtime-context references"
        );
    }
}
