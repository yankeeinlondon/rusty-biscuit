//! Phase 5 — recursive task graph and static preflight.
//!
//! Every test drives the public seam ([`build_preflight_graph`]) against real
//! files on disk, because the contract under test *is* filesystem-shaped:
//! references resolve from the directory of the document that authored them,
//! and cycle detection compares canonical paths.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};
use tempfile::TempDir;

use super::*;
use crate::composition::error::CompositionError;
use crate::composition::sequence::resolve_sequence_plan;

// -- fixtures --------------------------------------------------------------

/// Write a Markdown source with the given frontmatter pairs and body.
///
/// Values are emitted as JSON, which is YAML flow syntax — the fixtures stay
/// readable without pulling a YAML serializer into the library's dev-deps.
fn write_source(dir: &Path, name: &str, frontmatter: &[(&str, Value)], body: &str) -> String {
    let mut text = String::from("---\n");
    for (key, value) in frontmatter {
        text.push_str(&format!("{key}: {}\n", serde_json::to_string(value).unwrap()));
    }
    text.push_str("---\n\n");
    text.push_str(body);
    let path = dir.join(name);
    fs::write(&path, &text).unwrap();
    path.display().to_string()
}

fn write_yaml(dir: &Path, name: &str, value: &Value) {
    fs::write(
        dir.join(name),
        serde_json::to_string_pretty(value).unwrap(),
    )
    .unwrap();
}

/// Resolve a written Markdown source and build its preflight graph.
fn graph_for(path: &str) -> Result<PreflightGraph, CompositionError> {
    let source = crate::composition::resolve_composition_source(path)?;
    let plan = resolve_sequence_plan(&source)?.expect("fixture declares a sequence");
    build_preflight_graph(&plan, &source)
}

fn err_for(path: &str) -> CompositionError {
    graph_for(path).expect_err("expected preflight to reject this fixture")
}

// -- transitive loading and origin-relative resolution ---------------------

mod loading {
    use super::*;

    fn init_git_repo(path: &Path) {
        let status = std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(path)
            .status()
            .expect("git must be available for repository-context tests");
        assert!(status.success(), "git init failed for {}", path.display());
    }

    #[test]
    fn cross_repo_task_nested_reference_uses_its_own_repository_context() {
        let fixture = TempDir::new().unwrap();
        let launch_repo = fixture.path().join("launch");
        let task_repo = fixture.path().join("tasks");
        let nested = task_repo.join("nested");
        fs::create_dir_all(&launch_repo).unwrap();
        fs::create_dir_all(&nested).unwrap();
        init_git_repo(&launch_repo);
        init_git_repo(&task_repo);

        write_source(&task_repo, "repo-prompt.md", &[], "Repository prompt.\n");
        write_yaml(
            &nested,
            "task.yaml",
            &json!({ "kind": "task", "prompt": "repo-prompt.md" }),
        );
        let task_ref = nested.join("task.yaml").display().to_string();
        let source_path = write_source(
            &launch_repo,
            "sequence.md",
            &[("sequence", json!([{ "name": "external", "task": task_ref }]))],
            "Sequence body.\n",
        );
        let source = crate::composition::resolve_composition_source(&source_path).unwrap();
        let plan = resolve_sequence_plan(&source)
            .unwrap()
            .expect("fixture declares a sequence");
        let invocation = crate::invocation_context::InvocationContext::capture_at(&launch_repo);
        let source_context = invocation.derive_source(&source.resolved_path).unwrap();
        let requirements =
            darkmatter::markdown::compose::ContextRequirements::for_document(&source.markdown);
        let evidence = invocation.runtime_evidence(&source_context, &requirements);
        let context = darkmatter::markdown::compose::ComposeContext::capture_with_evidence(
            source_context.base_dir(),
            &requirements,
            &evidence,
        );

        let graph = build_preflight_graph_with_invocation(
            &plan,
            &source,
            context,
            &invocation,
            &source_context,
        )
        .unwrap();

        assert_eq!(graph.prompt_documents.len(), 1);
        assert_eq!(
            graph.prompt_documents[0].path,
            fs::canonicalize(task_repo.join("repo-prompt.md")).unwrap(),
            "the nested bare reference must search the task document's repository root",
        );
        assert_eq!(invocation.work_snapshot().topology_probes, 2);
    }

    #[test]
    #[serial_test::serial(file_resolution_snapshot)]
    fn nested_references_reuse_all_request_resolution_inputs() {
        let request = TempDir::new().unwrap();
        let request_root = dunce::canonicalize(request.path()).unwrap();
        let source_dir = request_root.join("sequences");
        let home = request_root.join("home");
        let env_root = request_root.join("env");
        let magic = request_root.join("magic");
        let package = request_root.join("package");
        for dir in [&source_dir, &home, &env_root, &magic, &package] {
            fs::create_dir_all(dir).unwrap();
        }

        write_source(&home, "home.md", &[], "home\n");
        write_yaml(
            &home,
            "home-task.yaml",
            &json!({ "kind": "task", "prompt": "./home.md" }),
        );
        write_source(&env_root, "env.md", &[], "env\n");
        write_yaml(
            &env_root,
            "env-task.yaml",
            &json!({ "kind": "task", "prompt": "./env.md" }),
        );
        write_source(&package, "package.md", &[], "package\n");
        write_yaml(
            &package,
            "package-task.yaml",
            &json!({ "kind": "task", "prompt": "./package.md" }),
        );

        let nested = magic.join("nested");
        fs::create_dir_all(&nested).unwrap();
        write_source(&nested, "magic.md", &[], "magic\n");
        write_yaml(
            &nested,
            "magic-task.yaml",
            &json!({ "kind": "task", "prompt": "./magic.md" }),
        );
        write_yaml(
            &magic,
            "group.yaml",
            &json!({
                "kind": "group",
                "tasks": [
                    { "task": "nested/magic-task.yaml" },
                    { "task": "!package-task.yaml" },
                ],
            }),
        );

        let source = write_source(
            &source_dir,
            "seq.md",
            &[(
                "sequence",
                json!([
                    { "name": "home", "task": "~/home-task.yaml" },
                    { "name": "env", "task": "{{CLAUDINE_SEQUENCE_SNAPSHOT_ROOT}}/env-task.yaml" },
                    { "name": "magic-and-package", "group": "@group.yaml" },
                ]),
            )],
            "Body.\n",
        );
        let resolved = crate::composition::resolve_composition_source(&source).unwrap();
        let plan = resolve_sequence_plan(&resolved)
            .unwrap()
            .expect("fixture declares a sequence");
        let snapshot = biscuit_file::FileResolutionContext::from_snapshot(
            &request_root,
            Some(home),
            std::collections::HashMap::from([(
                "CLAUDINE_SEQUENCE_SNAPSHOT_ROOT".to_string(),
                env_root.display().to_string(),
            )]),
        )
        .with_repository_root(request_root)
        .with_package_area(package)
        .add_magic_path(magic, biscuit_file::PathPosition::Start);

        let ambient = TempDir::new().unwrap();
        let _home = test_toolkit::EnvGuard::set_safe("HOME", ambient.path());
        let _env = test_toolkit::EnvGuard::set_safe(
            "CLAUDINE_SEQUENCE_SNAPSHOT_ROOT",
            ambient.path(),
        );
        let graph = build_preflight_graph_with_context_and_resolution(
            &plan,
            &resolved,
            darkmatter::markdown::compose::ComposeContext::capture(),
            Some(&snapshot),
        )
        .unwrap();

        let mut prompts: Vec<_> = graph
            .prompt_documents
            .iter()
            .map(|document| document.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        prompts.sort();
        assert_eq!(prompts, ["env.md", "home.md", "magic.md", "package.md"]);

        let PreflightAction::Group(group) = &graph.steps[2].task.as_ref().unwrap().action else {
            panic!("the magic reference must load the external group");
        };
        assert_eq!(group.tasks.len(), 2);
        assert!(
            graph
                .prompt_documents
                .iter()
                .any(|document| document.path == fs::canonicalize(nested.join("magic.md")).unwrap()),
            "the nested task's prompt must resolve from the external group/task authoring base",
        );
    }

    /// A `task:` file, a `group:` file, and every `prompt:` under them are all
    /// loaded transitively — one preflight walk sees work three hops down.
    #[test]
    fn loads_tasks_groups_and_prompts_transitively() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        write_source(root, "leaf.md", &[("title", json!("Leaf"))], "Do the thing.\n");
        write_source(root, "review.md", &[("title", json!("Review"))], "Review.\n");
        write_yaml(
            root,
            "outer.yaml",
            &json!({ "kind": "task", "prompt": "leaf.md" }),
        );
        write_yaml(
            root,
            "group.yaml",
            &json!({
                "kind": "group",
                "name": "bundle",
                "tasks": [ { "task": "outer.yaml" }, { "prompt": "review.md" } ],
            }),
        );
        let source = write_source(
            root,
            "seq.md",
            &[("sequence", json!([{ "name": "one", "group": "group.yaml" }]))],
            "Body.\n",
        );

        let graph = graph_for(&source).unwrap();
        let mut names: Vec<String> = graph
            .prompt_documents
            .iter()
            .map(|d| d.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec!["leaf.md".to_string(), "review.md".to_string()],
            "both prompts under the group must be discovered transitively",
        );

        let PreflightAction::Group(group) = &graph.steps[0].task.as_ref().unwrap().action else {
            panic!("step 1 must expand to a group");
        };
        assert_eq!(group.name, "bundle");
        assert_eq!(group.tasks.len(), 2);
        assert_eq!(
            group.execution,
            GroupExecution::Serial,
            "serial is the default"
        );
    }

    /// A reference inside a referenced document resolves from *that* document's
    /// directory, not the sequence source's and not the process CWD.
    #[test]
    fn nested_references_resolve_from_their_authoring_directory() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let nested = root.join("nested");
        fs::create_dir(&nested).unwrap();

        // Two prompts named `target.md`: one beside the sequence, one beside the
        // task file. The task file must reach its own sibling.
        write_source(root, "target.md", &[("title", json!("Wrong"))], "wrong\n");
        write_source(&nested, "target.md", &[("title", json!("Right"))], "right\n");
        write_yaml(
            &nested,
            "task.yaml",
            &json!({ "kind": "task", "prompt": "target.md" }),
        );
        let source = write_source(
            root,
            "seq.md",
            &[(
                "sequence",
                json!([{ "name": "one", "task": "nested/task.yaml" }]),
            )],
            "Body.\n",
        );

        let graph = graph_for(&source).unwrap();
        let document = &graph.prompt_documents[0];
        assert!(
            document.path.starts_with(fs::canonicalize(&nested).unwrap()),
            "the task file's sibling must win, got {}",
            document.path.display(),
        );
    }

    /// Independent branches may reach the same document; it is parsed once and
    /// appears once in the graph rather than being rejected as a cycle.
    #[test]
    fn shared_document_across_branches_is_not_a_cycle() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_source(root, "shared.md", &[("title", json!("Shared"))], "x\n");
        let source = write_source(
            root,
            "seq.md",
            &[(
                "sequence",
                json!([
                    { "name": "one", "prompt": "shared.md" },
                    { "name": "two", "prompt": "shared.md" },
                ]),
            )],
            "Body.\n",
        );

        let graph = graph_for(&source).unwrap();
        assert_eq!(
            graph.prompt_documents.len(),
            1,
            "the shared document is deduplicated, not duplicated or rejected",
        );
        assert_eq!(graph.steps.len(), 2);
    }
}

// -- group catalogs --------------------------------------------------------

mod catalogs {
    use super::*;

    fn catalog_fixture(root: &Path, reference: &str) -> String {
        write_source(root, "impl.md", &[("title", json!("Impl"))], "x\n");
        write_yaml(
            root,
            "catalog.yaml",
            &json!({
                "kind": "group-catalog",
                "groups": [
                    { "name": "alpha", "tasks": [ { "prompt": "impl.md" } ] },
                    { "name": "beta", "tasks": [ { "shell": "echo beta" } ] },
                ],
            }),
        );
        write_source(
            root,
            "seq.md",
            &[("sequence", json!([{ "name": "one", "group": reference }]))],
            "Body.\n",
        )
    }

    #[test]
    fn named_catalog_entry_resolves() {
        let dir = TempDir::new().unwrap();
        let source = catalog_fixture(dir.path(), "beta@catalog.yaml");
        let graph = graph_for(&source).unwrap();
        let PreflightAction::Group(group) = &graph.steps[0].task.as_ref().unwrap().action else {
            panic!("expected a group");
        };
        assert_eq!(group.name, "beta");
        assert_eq!(
            graph.shell_commands[0].command, "echo beta",
            "the catalog entry's shell task joins the approval set",
        );
    }

    /// A magic catalog reference composes: the split is on the *first* `@`, so
    /// the second one stays with the file reference.
    #[test]
    fn magic_catalog_reference_splits_on_first_at() {
        let (name, reference) =
            super::super::split_catalog_reference("build-group@@catalogs/all.yaml");
        assert_eq!(name.as_deref(), Some("build-group"));
        assert_eq!(reference, "@catalogs/all.yaml");
    }

    /// A leading `@` is a magic *file* reference to a `kind: group` document,
    /// not a nameless catalog lookup.
    #[test]
    fn leading_at_is_a_file_reference_not_a_catalog_name() {
        let (name, reference) = super::super::split_catalog_reference("@groups/my.yaml");
        assert_eq!(name, None);
        assert_eq!(reference, "@groups/my.yaml");
    }

    #[test]
    fn missing_catalog_entry_lists_what_is_available() {
        let dir = TempDir::new().unwrap();
        let source = catalog_fixture(dir.path(), "gamma@catalog.yaml");
        let error = err_for(&source);
        let CompositionError::SequenceGroupCatalogLookup {
            name,
            problem,
            available,
            ..
        } = &error
        else {
            panic!("expected a catalog lookup failure, got: {error}");
        };
        assert_eq!(name, "gamma");
        assert_eq!(problem, "is not defined");
        assert_eq!(available, &vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn duplicate_catalog_entries_are_ambiguous() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_yaml(
            root,
            "catalog.yaml",
            &json!({
                "kind": "group-catalog",
                "groups": [
                    { "name": "dup", "tasks": [ { "shell": "echo a" } ] },
                    { "name": "dup", "tasks": [ { "shell": "echo b" } ] },
                ],
            }),
        );
        let source = write_source(
            root,
            "seq.md",
            &[(
                "sequence",
                json!([{ "name": "one", "group": "dup@catalog.yaml" }]),
            )],
            "Body.\n",
        );
        let error = err_for(&source);
        assert!(
            matches!(
                &error,
                CompositionError::SequenceGroupCatalogLookup { problem, .. }
                    if problem == "is defined more than once"
            ),
            "a duplicated group name must be ambiguous, not first-wins; got: {error}",
        );
    }
}

// -- cycles ----------------------------------------------------------------

mod cycles {
    use super::*;

    /// The reported chain is the whole path back to the repeat, so the author
    /// can see which hop closed the loop.
    #[test]
    fn task_cycle_reports_the_full_chain() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_yaml(root, "a.yaml", &json!({ "kind": "task", "task": "b.yaml" }));
        write_yaml(root, "b.yaml", &json!({ "kind": "task", "task": "a.yaml" }));
        let source = write_source(
            root,
            "seq.md",
            &[("sequence", json!([{ "name": "one", "task": "a.yaml" }]))],
            "Body.\n",
        );

        let error = err_for(&source);
        let CompositionError::SequenceReferenceCycle { chain } = &error else {
            panic!("expected a cycle, got: {error}");
        };
        let names: Vec<String> = chain
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            vec![
                "seq.md".to_string(),
                "a.yaml".to_string(),
                "b.yaml".to_string(),
                "a.yaml".to_string(),
            ],
            "the chain runs from the entry document to the repeated hop",
        );
    }

    /// A prompt task pointing back at the sequence document is a cycle, not a
    /// silent re-entry.
    #[test]
    fn prompt_back_to_the_sequence_source_is_a_cycle() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let source = write_source(
            root,
            "seq.md",
            &[("sequence", json!([{ "name": "one", "prompt": "seq.md" }]))],
            "Body.\n",
        );
        assert!(
            matches!(
                err_for(&source),
                CompositionError::SequenceReferenceCycle { .. }
            ),
            "a self-referencing prompt task must be a typed cycle",
        );
    }
}

// -- blocked constructs ----------------------------------------------------

mod blocked {
    use super::*;

    #[test]
    fn nested_sequence_prompt_document_is_rejected() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_source(
            root,
            "inner.md",
            &[("sequence", json!(["a", "b"]))],
            "Inner body.\n",
        );
        let source = write_source(
            root,
            "seq.md",
            &[("sequence", json!([{ "name": "one", "prompt": "inner.md" }]))],
            "Body.\n",
        );
        let error = err_for(&source);
        let CompositionError::SequenceNestedSequence { path, .. } = &error else {
            panic!("expected a nested-sequence rejection, got: {error}");
        };
        assert!(path.ends_with("inner.md"));
    }

    #[test]
    fn nested_group_task_is_rejected() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let source = write_source(
            root,
            "seq.md",
            &[(
                "sequence",
                json!([{
                    "name": "one",
                    "group": {
                        "name": "outer",
                        "tasks": [ { "group": { "name": "inner", "tasks": [ { "shell": "echo x" } ] } } ],
                    },
                }]),
            )],
            "Body.\n",
        );
        let error = err_for(&source);
        assert!(
            matches!(
                &error,
                CompositionError::SequenceUnsupportedConstruct { construct, .. }
                    if construct.contains("nested group")
            ),
            "groups must not nest in v1; got: {error}",
        );
    }

    /// Group `loop` commit semantics are unratified (spec → Open Questions), so
    /// a group carrying `loop` is rejected rather than executed with invented
    /// semantics. This is the Phase 1 clean-break guard, now reachable.
    #[test]
    fn group_loop_is_rejected() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let source = write_source(
            root,
            "seq.md",
            &[(
                "sequence",
                json!([{
                    "name": "one",
                    "group": {
                        "name": "looper",
                        "loop": { "while": "true" },
                        "tasks": [ { "shell": "echo x" } ],
                    },
                }]),
            )],
            "Body.\n",
        );
        let error = err_for(&source);
        assert!(
            matches!(
                &error,
                CompositionError::SequenceUnsupportedConstruct { construct, .. }
                    if construct.contains("group `loop`")
            ),
            "group `loop` must be blocked with a typed error; got: {error}",
        );
    }

    /// A group document is never directly executable — it runs only as a task.
    #[test]
    fn direct_group_execution_is_rejected() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let mut frontmatter = darkmatter::markdown::Frontmatter::new();
        frontmatter.insert("kind", json!("group")).unwrap();
        let source = crate::composition::ResolvedCompositionSource {
            original_ref: "group.yaml".to_string(),
            resolved_path: root.join("group.yaml"),
            original_text: String::new(),
            markdown: darkmatter::markdown::Markdown::with_frontmatter(frontmatter, ""),
        };

        let error = reject_non_sequence_kind(&source).expect_err("a group is not executable");
        assert!(
            matches!(
                &error,
                CompositionError::SequenceUnsupportedConstruct { detail, .. }
                    if detail.contains("only as a sequence task")
            ),
            "the rejection must point the author at `group:`; got: {error}",
        );
    }

    /// A plain sequence document passes the same gate untouched.
    #[test]
    fn sequence_documents_pass_the_kind_gate() {
        let dir = TempDir::new().unwrap();
        let source = write_source(
            dir.path(),
            "seq.md",
            &[("kind", json!("sequence")), ("sequence", json!(["a"]))],
            "Body.\n",
        );
        let resolved = crate::composition::resolve_composition_source(&source).unwrap();
        assert!(reject_non_sequence_kind(&resolved).is_ok());
    }
}

// -- group schema ----------------------------------------------------------

mod group_schema {
    use super::*;

    fn group_error(group: Value) -> CompositionError {
        let dir = TempDir::new().unwrap();
        let source = write_source(
            dir.path(),
            "seq.md",
            &[("sequence", json!([{ "name": "one", "group": group }]))],
            "Body.\n",
        );
        err_for(&source)
    }

    #[test]
    fn empty_task_list_is_rejected() {
        let error = group_error(json!({ "name": "g", "tasks": [] }));
        assert!(
            matches!(
                &error,
                CompositionError::SequenceGroupInvalid { problem, .. }
                    if problem.contains("at least one task")
            ),
            "got: {error}",
        );
    }

    #[test]
    fn invalid_execution_mode_is_rejected() {
        let error = group_error(json!({
            "name": "g", "execution": "concurrent", "tasks": [ { "shell": "echo x" } ],
        }));
        assert!(
            matches!(
                &error,
                CompositionError::SequenceGroupInvalid { problem, .. }
                    if problem.contains("`serial` or `parallel`")
            ),
            "got: {error}",
        );
    }

    #[test]
    fn max_parallel_on_a_serial_group_is_rejected() {
        let error = group_error(json!({
            "name": "g", "max_parallel": 2, "tasks": [ { "shell": "echo x" } ],
        }));
        assert!(
            matches!(
                &error,
                CompositionError::SequenceGroupInvalid { problem, .. }
                    if problem.contains("only valid on a parallel group")
            ),
            "got: {error}",
        );
    }

    /// `min(1)` boundary: a one-task group is legal, and `max_parallel: 1` is
    /// the smallest legal cap.
    #[test]
    fn single_task_parallel_group_with_cap_one_is_accepted() {
        let dir = TempDir::new().unwrap();
        let source = write_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([{
                    "name": "one",
                    "group": {
                        "name": "g",
                        "execution": "parallel",
                        "max_parallel": 1,
                        "tasks": [ { "shell": "echo x" } ],
                    },
                }]),
            )],
            "Body.\n",
        );
        let graph = graph_for(&source).unwrap();
        let PreflightAction::Group(group) = &graph.steps[0].task.as_ref().unwrap().action else {
            panic!("expected a group");
        };
        assert_eq!(group.max_parallel, Some(1));
        assert_eq!(group.execution, GroupExecution::Parallel);
    }

    #[test]
    fn zero_max_parallel_is_rejected() {
        let error = group_error(json!({
            "name": "g",
            "execution": "parallel",
            "max_parallel": 0,
            "tasks": [ { "shell": "echo x" } ],
        }));
        assert!(
            matches!(
                &error,
                CompositionError::SequenceGroupInvalid { problem, .. }
                    if problem.contains(">= 1")
            ),
            "got: {error}",
        );
    }

    /// Group `operation`/`flow` are defaults; a task-level value wins, and the
    /// default never lands on a task type it cannot affect.
    #[test]
    fn group_defaults_apply_to_prompts_and_yield_to_task_overrides() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_source(root, "a.md", &[("title", json!("A"))], "a\n");
        write_source(root, "b.md", &[("title", json!("B"))], "b\n");
        let source = write_source(
            root,
            "seq.md",
            &[(
                "sequence",
                json!([{
                    "name": "one",
                    "group": {
                        "name": "g",
                        "operation": "group-op",
                        "flow": "group-flow",
                        "tasks": [
                            { "prompt": "a.md" },
                            { "prompt": "b.md", "operation": "task-op" },
                            { "shell": "echo x" },
                        ],
                    },
                }]),
            )],
            "Body.\n",
        );

        let graph = graph_for(&source).unwrap();
        let PreflightAction::Group(group) = &graph.steps[0].task.as_ref().unwrap().action else {
            panic!("expected a group");
        };
        assert_eq!(group.tasks[0].operation.as_deref(), Some("group-op"));
        assert_eq!(group.tasks[0].flow.as_deref(), Some("group-flow"));
        assert_eq!(
            group.tasks[1].operation.as_deref(),
            Some("task-op"),
            "a task-level value overrides the group default",
        );
        assert_eq!(
            group.tasks[2].operation, None,
            "a shell task never receives an operation it cannot act on",
        );
    }

    /// A field that cannot affect the chosen task type is a typed error rather
    /// than a silently ignored key.
    #[test]
    fn meaningless_task_field_is_rejected() {
        let error = group_error(json!({
            "name": "g",
            "tasks": [ { "shell": "echo x", "params": { "a": 1 } } ],
        }));
        assert!(
            matches!(
                &error,
                CompositionError::SequenceReferenceInvalid { problem, .. }
                    if problem.contains("meaningless for a `shell` task")
            ),
            "got: {error}",
        );
    }
}

// -- write-back collisions -------------------------------------------------

mod write_back {
    use super::*;

    fn inline_prompt(root: &Path, name: &str) {
        write_source(root, name, &[("prompt", json!("Write something."))], "old\n");
    }

    #[test]
    fn parallel_siblings_targeting_one_document_collide() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        inline_prompt(root, "target.md");
        let source = write_source(
            root,
            "seq.md",
            &[(
                "sequence",
                json!([{
                    "name": "one",
                    "group": {
                        "name": "racers",
                        "execution": "parallel",
                        "tasks": [
                            { "prompt": "target.md", "name": "first" },
                            { "prompt": "target.md", "name": "second" },
                        ],
                    },
                }]),
            )],
            "Body.\n",
        );

        let error = err_for(&source);
        let CompositionError::SequenceWriteBackCollision {
            group,
            target,
            first,
            second,
        } = &error
        else {
            panic!("expected a write-back collision, got: {error}");
        };
        assert_eq!(group, "racers");
        assert!(target.ends_with("target.md"));
        assert_eq!(first, "first");
        assert_eq!(second, "second");
    }

    /// Racing the sequence source itself is caught: an inline-compose sequence
    /// rewrites its own body between steps.
    #[test]
    fn collision_with_the_sequence_source_is_caught() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let source = write_source(
            root,
            "seq.md",
            &[
                ("prompt", json!("Compose the body.")),
                (
                    "sequence",
                    json!([{
                        "name": "one",
                        "group": {
                            "name": "racers",
                            "execution": "parallel",
                            "tasks": [ { "prompt": "seq.md", "name": "racer" } ],
                        },
                    }]),
                ),
            ],
            "Body.\n",
        );
        // The self-reference closes an ancestry cycle before the collision map
        // is consulted; either rejection is abort-all and names the document.
        let error = err_for(&source);
        assert!(
            matches!(
                &error,
                CompositionError::SequenceReferenceCycle { .. }
                    | CompositionError::SequenceWriteBackCollision { .. }
            ),
            "racing the sequence source must be rejected; got: {error}",
        );
    }

    /// Distinct targets, and non-inline-compose prompts sharing a target, are
    /// both legal — only racing *write-backs* are illegal.
    #[test]
    fn plain_compose_siblings_do_not_collide() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_source(root, "plain.md", &[("title", json!("Plain"))], "x\n");
        let source = write_source(
            root,
            "seq.md",
            &[(
                "sequence",
                json!([{
                    "name": "one",
                    "group": {
                        "name": "racers",
                        "execution": "parallel",
                        "tasks": [
                            { "prompt": "plain.md", "name": "first" },
                            { "prompt": "plain.md", "name": "second" },
                        ],
                    },
                }]),
            )],
            "Body.\n",
        );
        assert!(
            graph_for(&source).is_ok(),
            "two plain compose tasks write nothing back and may share a document",
        );
    }

    /// Serial groups are exempt: they never run two tasks at once.
    #[test]
    fn serial_group_may_reuse_an_inline_compose_target() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        inline_prompt(root, "target.md");
        let source = write_source(
            root,
            "seq.md",
            &[(
                "sequence",
                json!([{
                    "name": "one",
                    "group": {
                        "name": "chain",
                        "tasks": [
                            { "prompt": "target.md", "name": "first" },
                            { "prompt": "target.md", "name": "second" },
                        ],
                    },
                }]),
            )],
            "Body.\n",
        );
        assert!(graph_for(&source).is_ok());
    }
}

// -- shell byte parity -----------------------------------------------------

mod shell {
    use super::*;

    /// The stored command is the *resolved* string, so what preflight approves
    /// is exactly what runs.
    #[test]
    fn shell_bytes_are_resolved_once_and_stored() {
        let dir = TempDir::new().unwrap();
        let source = write_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([{ "name": "alpha", "shell": "echo {{ state.name }}" }]),
            )],
            "Body.\n",
        );

        let graph = graph_for(&source).unwrap();
        let PreflightAction::Shell { commands } = &graph.steps[0].task.as_ref().unwrap().action
        else {
            panic!("expected a shell task");
        };
        assert_eq!(commands, &vec!["echo alpha".to_string()]);
        assert_eq!(
            graph.shell_commands[0].command, "echo alpha",
            "the approval set carries the resolved bytes, not the template",
        );
    }

    /// Each step resolves against its own state, so a per-step command is
    /// approved once per distinct byte string.
    #[test]
    fn per_step_state_produces_per_step_bytes() {
        let dir = TempDir::new().unwrap();
        let source = write_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([
                    { "name": "alpha", "shell": "echo {{ state.name }}" },
                    { "name": "beta", "shell": "echo {{ state.name }}" },
                ]),
            )],
            "Body.\n",
        );
        let graph = graph_for(&source).unwrap();
        let commands: Vec<&str> = graph
            .shell_commands
            .iter()
            .map(|c| c.command.as_str())
            .collect();
        assert_eq!(
            commands,
            vec!["echo alpha", "echo beta"],
            "one approval entry per step, because the bytes differ per step",
        );
    }

    /// A command list preserves declaration order, which is also the order the
    /// concatenated stdout will follow.
    #[test]
    fn command_lists_keep_declaration_order() {
        let dir = TempDir::new().unwrap();
        let source = write_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([{ "name": "one", "shell": ["echo a", "echo b", "echo c"] }]),
            )],
            "Body.\n",
        );
        let graph = graph_for(&source).unwrap();
        let PreflightAction::Shell { commands } = &graph.steps[0].task.as_ref().unwrap().action
        else {
            panic!("expected a shell task");
        };
        assert_eq!(commands, &vec!["echo a", "echo b", "echo c"]);
    }

    /// `outputs` is the accumulator later tasks push onto; a shell string
    /// depending on it could never be approved byte-for-byte up front.
    #[test]
    fn outputs_in_a_shell_command_is_rejected() {
        let dir = TempDir::new().unwrap();
        let source = write_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([{ "name": "one", "shell": "echo {{ last(outputs) }}" }]),
            )],
            "Body.\n",
        );
        let error = err_for(&source);
        let CompositionError::SequenceShellLateBinding { root, .. } = &error else {
            panic!("expected a late-binding rejection, got: {error}");
        };
        assert_eq!(root, "outputs");
    }

    /// The lifecycle late-binding globals are rejected for the same reason.
    #[test]
    fn lifecycle_late_binding_in_a_shell_command_is_rejected() {
        for root_name in ["err", "timing", "current"] {
            let dir = TempDir::new().unwrap();
            let command = format!("echo {{{{ {root_name}.message }}}}");
            let source = write_source(
                dir.path(),
                "seq.md",
                &[(
                    "sequence",
                    json!([{ "name": "one", "shell": command }]),
                )],
                "Body.\n",
            );
            let error = err_for(&source);
            assert!(
                matches!(
                    &error,
                    CompositionError::SequenceShellLateBinding { root, .. } if root == root_name
                ),
                "`{root_name}` must be rejected in a shell command; got: {error}",
            );
        }
    }

    /// Graph-phase shell bytes are resolved before per-task target selection,
    /// so a target-identity reference would expand a pre-selection value that
    /// execution then runs verbatim. Each root is rejected with the typed
    /// target-identity error naming the offending path (D4/AC7).
    #[test]
    fn target_identity_in_a_graph_phase_shell_command_is_rejected() {
        for root in ["ctx.agent", "ctx.model", "env.AGENT", "env.MODEL"] {
            let dir = TempDir::new().unwrap();
            let command = format!("echo {{{{ {root} }}}}");
            let source = write_source(
                dir.path(),
                "seq.md",
                &[(
                    "sequence",
                    json!([{ "name": "one", "shell": command }]),
                )],
                "Body.\n",
            );
            let error = err_for(&source);
            assert!(
                matches!(
                    &error,
                    CompositionError::SequenceShellTargetIdentity { root: offending, .. }
                        if offending == root
                ),
                "`{root}` must be rejected with the typed target-identity error at \
                 graph preflight; got: {error}",
            );
        }
    }

    /// A task's `teardown:` stack resolves through the same graph phase, so a
    /// target-identity reference there is rejected identically — the bytes it
    /// would bake in are the bytes execution runs.
    #[test]
    fn target_identity_in_a_teardown_shell_command_is_rejected() {
        let dir = TempDir::new().unwrap();
        let source = write_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([{
                    "name": "one",
                    "shell": "echo primary",
                    "teardown": [ { "action": [ { "shell": "echo {{ env.AGENT }}" } ] } ],
                }]),
            )],
            "Body.\n",
        );
        let error = err_for(&source);
        assert!(
            matches!(
                &error,
                CompositionError::SequenceShellTargetIdentity { root, .. } if root == "env.AGENT"
            ),
            "a teardown target-identity reference must be rejected; got: {error}",
        );
    }

    /// `ctx.*` and `env.*` as namespaces stay legal in graph-phase commands —
    /// only the agent/model identity leaves are target-dependent.
    #[test]
    fn non_identity_ctx_and_env_references_stay_legal_in_shell_commands() {
        let dir = TempDir::new().unwrap();
        let source = write_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([{
                    "name": "one",
                    "shell": [
                        "echo {{ state.name }}",
                        "echo {{ env.HOME }}",
                    ],
                }]),
            )],
            "Body.\n",
        );
        let graph = graph_for(&source).unwrap();
        assert!(
            !graph.shell_commands.is_empty(),
            "non-identity references must resolve, not reject"
        );
    }

    /// Conditional branches are potential work: a `when:`-guarded setup action
    /// contributes its command whether or not the guard reads true today.
    #[test]
    fn guarded_setup_and_teardown_commands_are_collected() {
        let dir = TempDir::new().unwrap();
        let source = write_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([{
                    "name": "one",
                    "shell": "echo primary",
                    "setup": [ { "when": "false", "action": [ { "shell": "echo setup" } ] } ],
                    "teardown": [ { "action": [ { "shell": "echo teardown" } ] } ],
                }]),
            )],
            "Body.\n",
        );
        let graph = graph_for(&source).unwrap();
        let commands: Vec<&str> = graph
            .shell_commands
            .iter()
            .map(|c| c.command.as_str())
            .collect();
        assert!(
            commands.contains(&"echo setup"),
            "a false `when:` guard must not hide a command from approval: {commands:?}",
        );
        assert!(commands.contains(&"echo teardown"), "{commands:?}");
        assert!(commands.contains(&"echo primary"), "{commands:?}");
    }

    /// A key/value shell action and its `on_error:` companion both count.
    #[test]
    fn key_value_shell_actions_and_on_error_are_collected() {
        let dir = TempDir::new().unwrap();
        let source = write_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([{
                    "name": "one",
                    "side_effect": { "set": ["ready", "{{ true }}"] },
                    "setup": [ {
                        "action": [ {
                            "action": "shell",
                            "command": "echo main",
                            "on_error": "echo recover",
                        } ],
                    } ],
                }]),
            )],
            "Body.\n",
        );
        let graph = graph_for(&source).unwrap();
        let commands: Vec<&str> = graph
            .shell_commands
            .iter()
            .map(|c| c.command.as_str())
            .collect();
        assert_eq!(commands, vec!["echo main", "echo recover"]);
    }
}

// -- step shape ------------------------------------------------------------

mod steps {
    use super::*;

    /// A step with no executable runs the source document body, so it carries
    /// no task node at all.
    #[test]
    fn bodyless_step_has_no_task() {
        let dir = TempDir::new().unwrap();
        let source = write_source(
            dir.path(),
            "seq.md",
            &[("sequence", json!(["alpha", "beta"]))],
            "Work on {{ state }}.\n",
        );
        let graph = graph_for(&source).unwrap();
        assert_eq!(graph.steps.len(), 2);
        assert!(graph.steps.iter().all(|s| s.task.is_none()));
        assert!(graph.prompt_documents.is_empty());
        assert!(graph.shell_commands.is_empty());
    }

    /// Mixed steps keep their sequence identity: the node carries the generated
    /// id and one-based ordering stays intact.
    #[test]
    fn mixed_steps_retain_identity_and_order() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_source(root, "review.md", &[("title", json!("Review"))], "r\n");
        let source = write_source(
            root,
            "seq.md",
            &[(
                "sequence",
                json!([
                    { "name": "alpha", "topic": "parsing" },
                    { "name": "run tests", "shell": "just test" },
                    { "name": "review", "prompt": "review.md", "params": { "topic": "x" } },
                ]),
            )],
            "Work on {{ state.topic }}.\n",
        );

        let graph = graph_for(&source).unwrap();
        assert_eq!(
            graph.steps.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            vec!["alpha", "run-tests", "review"],
        );
        assert!(graph.steps[0].task.is_none());
        assert!(matches!(
            graph.steps[1].task.as_ref().unwrap().action,
            PreflightAction::Shell { .. }
        ));
        let review = graph.steps[2].task.as_ref().unwrap();
        assert!(matches!(review.action, PreflightAction::Prompt { .. }));
        assert_eq!(
            review.params.get("topic").and_then(Value::as_str),
            Some("x"),
            "params travel with the node unevaluated; they bind just in time",
        );
    }

    /// An external `task:` file supplies the whole task; the referencing site
    /// contributes nothing but the reference.
    #[test]
    fn external_task_expands_to_the_referenced_file() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_yaml(
            root,
            "task.yaml",
            &json!({ "kind": "task", "shell": "echo external", "name": "outside" }),
        );
        let source = write_source(
            root,
            "seq.md",
            &[("sequence", json!([{ "name": "one", "task": "task.yaml" }]))],
            "Body.\n",
        );
        let graph = graph_for(&source).unwrap();
        let task = graph.steps[0].task.as_ref().unwrap();
        assert_eq!(task.name.as_deref(), Some("outside"));
        assert!(task.origin_path.ends_with("task.yaml"));
        assert!(
            matches!(&task.action, PreflightAction::Shell { commands } if commands == &["echo external"])
        );
    }

    /// A `kind: task` file must actually declare that kind.
    #[test]
    fn task_file_with_the_wrong_kind_is_rejected() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_yaml(root, "task.yaml", &json!({ "shell": "echo x" }));
        let source = write_source(
            root,
            "seq.md",
            &[("sequence", json!([{ "name": "one", "task": "task.yaml" }]))],
            "Body.\n",
        );
        let error = err_for(&source);
        assert!(
            matches!(
                &error,
                CompositionError::SequenceReferenceInvalid { problem, .. }
                    if problem.contains("does not declare `kind: task`")
            ),
            "got: {error}",
        );
    }
}
