//! Phase 5 — recursive task graph and static preflight.
//!
//! Grammar matrices use the public preflight seam against files on disk.
//! Retained approval and runner fixtures construct legacy inert task models so
//! their contracts remain covered while authored stack grammar is removed.
//! References still resolve from their authoring directory, and cycle
//! detection compares canonical paths.

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
fn write_legacy_source(dir: &Path, name: &str, frontmatter: &[(&str, Value)], body: &str) -> String {
    let frontmatter: Vec<_> = frontmatter.iter().map(|(key, value)| {
        let mut value = value.clone(); name_fixture_legs(&mut value); (*key, value)
    }).collect();
    write_source(dir, name, &frontmatter, body)
}

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

fn write_legacy_yaml(dir: &Path, name: &str, value: &Value) {
    let mut value = value.clone(); name_fixture_legs(&mut value);
    write_yaml(dir, name, &value);
}

// Existing preflight fixtures exercise loading and approval, with authored leg
// names supplied here so those tests do not duplicate the name-validation matrix.
fn name_fixture_legs(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if let Some(Value::Array(tasks)) = map.get_mut("tasks") {
                for (index, task) in tasks.iter_mut().enumerate() {
                    if let Some(task) = task.as_object_mut() { task.entry("name").or_insert(json!(format!("leg-{index}"))); }
                }
            }
            for value in map.values_mut() { name_fixture_legs(value); }
        }
        Value::Array(items) => for value in items { name_fixture_legs(value); },
        _ => {}
    }
}

fn write_yaml(dir: &Path, name: &str, value: &Value) {
    fs::write(
        dir.join(name),
        serde_json::to_string_pretty(value).unwrap(),
    )
    .unwrap();
}

/// Resolve a written Markdown source and build its preflight graph.
fn legacy_graph_for(path: &str) -> Result<PreflightGraph, CompositionError> {
    let source = crate::composition::resolve_fixture_source(path)?;
    let fixture = super::runtime_fixture::RuntimeFixture::new(&source, source.resolved_path.parent().unwrap());
    let resolution = crate::test_support::context_for(&source.resolved_path);
    let plan = resolve_sequence_plan(&fixture.source, &resolution)?.expect("fixture declares a sequence");
    let context = darkmatter::markdown::compose::ComposeContext::capture_for_content(source.resolved_path.parent().unwrap(), "");
    fixture.build(&plan, context, &resolution)
}

fn graph_for(path: &str) -> Result<PreflightGraph, CompositionError> {
    let source = crate::composition::resolve_fixture_source(path)?;
    let resolution = crate::test_support::context_for(&source.resolved_path);
    let plan = resolve_sequence_plan(&source, &resolution)?.expect("fixture declares a sequence");
    build_preflight_graph(&plan, &source, &resolution)
}

fn err_for(path: &str) -> CompositionError {
    graph_for(path).expect_err("expected public preflight to reject this fixture")
}

fn legacy_err_for(path: &str) -> CompositionError {
    legacy_graph_for(path).expect_err("expected preflight to reject this fixture")
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

        write_legacy_source(&task_repo, "repo-prompt.md", &[], "Repository prompt.\n");
        write_legacy_yaml(
            &nested,
            "task.yaml",
            &json!({ "kind": "task", "prompt": "repo-prompt.md" }),
        );
        let task_ref = nested.join("task.yaml").display().to_string();
        let source_path = write_legacy_source(
            &launch_repo,
            "sequence.md",
            &[("sequence", json!([{ "name": "external", "task": task_ref }]))],
            "Sequence body.\n",
        );
        let source = crate::composition::resolve_fixture_source(&source_path).unwrap();
        let plan = resolve_sequence_plan(&source, &crate::test_support::context_for(&source.resolved_path))
            .unwrap()
            .expect("fixture declares a sequence");
        let invocation = crate::invocation_context::InvocationContext::capture_at(&crate::test_support::snapshot(), &launch_repo).unwrap();
        let source_context = invocation.derive_source(&source.resolved_path).unwrap();
        let requirements =
            darkmatter::markdown::compose::ContextRequirements::for_document(&source.markdown);
        let evidence = invocation.runtime_evidence(
            &crate::invocation_context::RunEvidence::default(),
            &source_context,
            &requirements,
        );
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
            None,
        )
        .unwrap();

        assert_eq!(graph.prompt_documents.len(), 1);
        assert_eq!(
            graph.prompt_documents[0].path,
            dunce::canonicalize(task_repo.join("repo-prompt.md")).unwrap(),
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

        write_legacy_source(&home, "home.md", &[], "home\n");
        write_legacy_yaml(
            &home,
            "home-task.yaml",
            &json!({ "kind": "task", "prompt": "./home.md" }),
        );
        write_legacy_source(&env_root, "env.md", &[], "env\n");
        write_legacy_yaml(
            &env_root,
            "env-task.yaml",
            &json!({ "kind": "task", "prompt": "./env.md" }),
        );
        write_legacy_source(&package, "package.md", &[], "package\n");
        write_legacy_yaml(
            &package,
            "package-task.yaml",
            &json!({ "kind": "task", "prompt": "./package.md" }),
        );

        let nested = magic.join("nested");
        fs::create_dir_all(&nested).unwrap();
        write_legacy_source(&nested, "magic.md", &[], "magic\n");
        write_legacy_yaml(
            &nested,
            "magic-task.yaml",
            &json!({ "kind": "task", "prompt": "./magic.md" }),
        );
        write_legacy_yaml(
            &magic,
            "group.yaml",
            &json!({
                "kind": "group",
                "tasks": [
                    { "task": "nested/magic-task.yaml" },
                    { "task": "^package-task.yaml" },
                ],
            }),
        );

        let source = write_legacy_source(
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
        let resolved = crate::composition::resolve_fixture_source(&source).unwrap();
        let plan = resolve_sequence_plan(&resolved, &crate::test_support::context_for(&resolved.resolved_path))
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
        let graph = build_preflight_graph_with_context(
            &plan,
            &resolved,
            darkmatter::markdown::compose::ComposeContext::capture_for_dir(&source_dir),
            &snapshot,
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
                .any(|document| document.path == dunce::canonicalize(nested.join("magic.md")).unwrap()),
            "the nested task's prompt must resolve from the external group/task authoring base",
        );
    }

    /// A `task:` file, a `group:` file, and every `prompt:` under them are all
    /// loaded transitively — one preflight walk sees work three hops down.
    #[test]
    fn loads_tasks_groups_and_prompts_transitively() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        write_legacy_source(root, "leaf.md", &[("title", json!("Leaf"))], "Do the thing.\n");
        write_legacy_source(root, "review.md", &[("title", json!("Review"))], "Review.\n");
        write_legacy_yaml(
            root,
            "outer.yaml",
            &json!({ "kind": "task", "prompt": "leaf.md" }),
        );
        write_legacy_yaml(
            root,
            "group.yaml",
            &json!({
                "kind": "group",
                "name": "bundle",
                "tasks": [ { "task": "outer.yaml" }, { "prompt": "review.md" } ],
            }),
        );
        let source = write_legacy_source(
            root,
            "seq.md",
            &[("sequence", json!([{ "name": "one", "group": "group.yaml" }]))],
            "Body.\n",
        );

        let graph = legacy_graph_for(&source).unwrap();
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
        write_legacy_source(root, "target.md", &[("title", json!("Wrong"))], "wrong\n");
        write_legacy_source(&nested, "target.md", &[("title", json!("Right"))], "right\n");
        write_legacy_yaml(
            &nested,
            "task.yaml",
            &json!({ "kind": "task", "prompt": "target.md" }),
        );
        let source = write_legacy_source(
            root,
            "seq.md",
            &[(
                "sequence",
                json!([{ "name": "one", "task": "nested/task.yaml" }]),
            )],
            "Body.\n",
        );

        let graph = legacy_graph_for(&source).unwrap();
        let document = &graph.prompt_documents[0];
        assert!(
            document.path.starts_with(dunce::canonicalize(&nested).unwrap()),
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
        write_legacy_source(root, "shared.md", &[("title", json!("Shared"))], "x\n");
        let source = write_legacy_source(
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

        let graph = legacy_graph_for(&source).unwrap();
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
        write_legacy_source(root, "impl.md", &[("title", json!("Impl"))], "x\n");
        write_legacy_yaml(
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
        write_legacy_source(
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
        let graph = legacy_graph_for(&source).unwrap();
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
        let error = legacy_err_for(&source);
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
        write_legacy_yaml(
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
        let source = write_legacy_source(
            root,
            "seq.md",
            &[(
                "sequence",
                json!([{ "name": "one", "group": "dup@catalog.yaml" }]),
            )],
            "Body.\n",
        );
        let error = legacy_err_for(&source);
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
        write_legacy_yaml(root, "a.yaml", &json!({ "kind": "task", "task": "b.yaml" }));
        write_legacy_yaml(root, "b.yaml", &json!({ "kind": "task", "task": "a.yaml" }));
        let source = write_legacy_source(
            root,
            "seq.md",
            &[("sequence", json!([{ "name": "one", "task": "a.yaml" }]))],
            "Body.\n",
        );

        let error = legacy_err_for(&source);
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
        let source = write_legacy_source(
            root,
            "seq.md",
            &[("sequence", json!([{ "name": "one", "prompt": "seq.md" }]))],
            "Body.\n",
        );
        assert!(
            matches!(
                legacy_err_for(&source),
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
        write_legacy_source(
            root,
            "inner.md",
            &[("sequence", json!(["a", "b"]))],
            "Inner body.\n",
        );
        let source = write_legacy_source(
            root,
            "seq.md",
            &[("sequence", json!([{ "name": "one", "prompt": "inner.md" }]))],
            "Body.\n",
        );
        let error = legacy_err_for(&source);
        let CompositionError::SequenceNestedSequence { path, .. } = &error else {
            panic!("expected a nested-sequence rejection, got: {error}");
        };
        assert!(path.ends_with("inner.md"));
    }

    #[test]
    fn nested_group_task_is_accepted() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let source = write_legacy_source(
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
        let graph = graph_for(&source).unwrap();
        let PreflightAction::Group(outer) = &graph.steps[0].task.as_ref().unwrap().action else { panic!("expected outer group") };
        assert!(matches!(outer.tasks[0].action, PreflightAction::Group(_)));
    }

    /// A valid group `loop` is frozen on the preflight group, its gate on
    /// the group's `loop` event and its location kept for diagnostics.
    #[test]
    fn group_loop_is_carried_on_the_preflight_group() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let source = write_legacy_source(
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
        let graph = graph_for(&source).unwrap();
        let PreflightAction::Group(group) = &graph.steps[0].task.as_ref().unwrap().action else { panic!("expected a group") };
        let looping = group.loop_config.as_ref().expect("the group's loop is carried");
        // Runtime diagnostics locate a root step as `tasks[N]`, as every other
        // preflight task diagnostic does.
        assert_eq!(looping.location.property, "tasks[0].group.loop");
        assert!(matches!(&looping.config.condition, crate::composition::LoopCondition::While(text) if text == "true"));
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
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[("kind", json!("sequence")), ("sequence", json!(["a"]))],
            "Body.\n",
        );
        let resolved = crate::composition::resolve_fixture_source(&source).unwrap();
        assert!(reject_non_sequence_kind(&resolved).is_ok());
    }
}

// -- group schema ----------------------------------------------------------

mod group_schema {
    use super::*;

    fn group_error(group: Value) -> CompositionError {
        let dir = TempDir::new().unwrap();
        let source = write_legacy_source(
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
                CompositionError::RemovedGrammar { removed, .. }
                    if removed == "execution"
            ),
            "got: {error}",
        );
    }

    #[test]
    fn serial_execution_with_a_cap_is_removed() {
        let error = group_error(json!({
            "name": "g", "execution": "serial", "max_parallel": 2, "tasks": [ { "shell": "echo x" } ],
        }));
        assert!(
            matches!(
                &error,
                CompositionError::RemovedGrammar { removed, .. }
                    if removed == "execution"
            ),
            "got: {error}",
        );
    }

    /// `min(1)` boundary: a one-task group is legal, and `max_parallel: 1` is
    /// the smallest legal cap.
    #[test]
    fn single_task_parallel_group_with_cap_one_is_accepted() {
        let dir = TempDir::new().unwrap();
        let source = write_legacy_source(
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
        let graph = legacy_graph_for(&source).unwrap();
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
        write_legacy_source(root, "a.md", &[("title", json!("A"))], "a\n");
        write_legacy_source(root, "b.md", &[("title", json!("B"))], "b\n");
        let source = write_legacy_source(
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

        let graph = legacy_graph_for(&source).unwrap();
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
        write_legacy_source(root, name, &[("prompt", json!("Write something."))], "old\n");
    }

    #[test]
    fn parallel_siblings_targeting_one_document_collide() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        inline_prompt(root, "target.md");
        let source = write_legacy_source(
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

        let error = legacy_err_for(&source);
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
        let source = write_legacy_source(
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
        let error = legacy_err_for(&source);
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
        write_legacy_source(root, "plain.md", &[("title", json!("Plain"))], "x\n");
        let source = write_legacy_source(
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
            legacy_graph_for(&source).is_ok(),
            "two plain compose tasks write nothing back and may share a document",
        );
    }

    /// Groups default to concurrency, so their write-back targets must be disjoint.
    #[test]
    fn default_concurrent_group_rejects_reused_inline_target() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        inline_prompt(root, "target.md");
        let source = write_legacy_source(
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
        assert!(matches!(err_for(&source), CompositionError::SequenceWriteBackCollision {..}));
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
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[("label", json!("alpha")), (
                "sequence",
                json!([{ "name": "alpha", "shell": "echo {{ label }}" }]),
            )],
            "Body.\n",
        );

        let graph = legacy_graph_for(&source).unwrap();
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

    /// A stored literal token resolves to the text it holds, exactly as
    /// composition will read it, so the approved bytes match what runs; the
    /// inserted text is data and is not scanned again.
    #[test]
    fn a_stored_token_resolves_to_its_text_in_approved_bytes() {
        let dir = TempDir::new().unwrap();
        let token = darkmatter::markdown::literal_token::encode("see {{ title }}");
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[
                ("title", json!("t")),
                ("note", json!(token)),
                ("sequence", json!([{ "name": "alpha", "shell": "echo '{{ note }}'" }])),
            ],
            "Body.\n",
        );

        let graph = legacy_graph_for(&source).unwrap();
        assert_eq!(graph.shell_commands[0].command, "echo 'see {{ title }}'");
    }

    /// Each step resolves against its own state. The first step's command is
    /// approved as its bytes; the second step's, a later task, only by its
    /// shape, though its approval-time values still resolve per step.
    #[test]
    fn per_step_state_produces_per_step_bytes() {
        let dir = TempDir::new().unwrap();
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([
                    { "name": "alpha", "shell": "echo {{ seq.name }}" },
                    { "name": "beta", "shell": "echo {{ seq.name }}" },
                ]),
            )],
            "Body.\n",
        );
        let graph = legacy_graph_for(&source).unwrap();
        let commands: Vec<&str> = graph
            .shell_commands
            .iter()
            .map(|c| c.command.as_str())
            .collect();
        assert!(commands.is_empty(), "runtime seq reads require templates");
        assert_eq!(step_commands(&graph, 0), vec!["echo {{ seq.name }}"]);
        assert_eq!(step_commands(&graph, 1), vec!["echo {{ seq.name }}"]);
        let shapes: Vec<&str> = graph
            .template_candidates
            .iter()
            .filter(|candidate| candidate.requirement == ModeRequirement::TemplateRequired)
            .filter_map(|candidate| candidate.authored())
            .collect();
        assert_eq!(shapes, vec!["echo {{ seq.name }}", "echo {{ seq.name }}"]);
    }

    fn step_commands(graph: &PreflightGraph, index: usize) -> Vec<String> {
        let PreflightAction::Shell { commands } = &graph.steps[index].task.as_ref().unwrap().action
        else {
            panic!("expected a shell task");
        };
        commands.clone()
    }

    /// The sequence-owned properties are frontmatter-root properties of each
    /// step, so the `fm` namespace reads the same value as the bare name.
    #[test]
    fn sequence_owned_properties_are_approved_as_runtime_templates() {
        let dir = TempDir::new().unwrap();
        let command = "echo {{ seq.name }}={{ seq.name }} \
                       {{ seq.index }}={{ seq.index }} \
                       {{ seq.run_id }}={{ seq.run_id }} \
                       {{ seq.previous.name || 'none' }}={{ seq.previous.name || 'none' }} \
                       {{ seq.next.name || 'none' }}={{ seq.next.name || 'none' }}";
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([
                    { "name": "alpha", "shell": command },
                    { "name": "beta", "shell": command },
                ]),
            )],
            "Body.\n",
        );

        let graph = legacy_graph_for(&source).unwrap();
        // A later task retains authored text; only structured dispatch fills it.
        let commands: Vec<String> =
            [step_commands(&graph, 0), step_commands(&graph, 1)].concat();
        assert_eq!(commands.len(), 2, "{commands:?}");
        assert!(graph.shell_commands.is_empty());
        assert_eq!(commands, vec![command.to_string(), command.to_string()]);
        assert_eq!(graph.template_candidates.len(), 2);
    }

    /// A command list preserves declaration order, which is also the order the
    /// concatenated stdout will follow.
    #[test]
    fn command_lists_keep_declaration_order() {
        let dir = TempDir::new().unwrap();
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([{ "name": "one", "shell": ["echo a", "echo b", "echo c"] }]),
            )],
            "Body.\n",
        );
        let graph = legacy_graph_for(&source).unwrap();
        let PreflightAction::Shell { commands } = &graph.steps[0].task.as_ref().unwrap().action
        else {
            panic!("expected a shell task");
        };
        assert_eq!(commands, &vec!["echo a", "echo b", "echo c"]);
    }

    /// `outputs` is the accumulator later tasks push onto, so a command
    /// reading it has no bytes up front: it is approved as a template, its
    /// placeholder late, never evaluated here — even in the first task.
    #[test]
    fn outputs_in_a_shell_command_is_a_late_template() {
        let dir = TempDir::new().unwrap();
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([{ "name": "one", "shell": "echo {{ last(seq.outputs) }}" }]),
            )],
            "Body.\n",
        );
        let graph = legacy_graph_for(&source).unwrap();
        assert!(graph.shell_commands.is_empty(), "{:?}", graph.shell_commands);
        let task = graph.steps[0].task.as_ref().unwrap();
        assert_eq!(task.position, TaskPosition::First);
        let site = &task.shell_sites[0];
        assert_eq!(site.mode, SiteMode::Template);
        assert!(site.reads_late_value());
        assert_eq!(graph.template_candidates.len(), 1);
    }

    /// The lifecycle late-binding roots are rejected for the same reason.
    ///
    /// `current` belongs here even though Darkmatter owns it: it observes a
    /// fact when the reference is reached, and a shell command is resolved at
    /// preflight before any event fires (R33).
    #[test]
    fn lifecycle_late_binding_in_a_shell_command_is_rejected() {
        for root_name in ["err", "timing", "current"] {
            let dir = TempDir::new().unwrap();
            let command = format!("echo {{{{ {root_name}.message }}}}");
            let source = write_legacy_source(
                dir.path(),
                "seq.md",
                &[(
                    "sequence",
                    json!([{ "name": "one", "shell": command }]),
                )],
                "Body.\n",
            );
            let error = legacy_err_for(&source);
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
            let source = write_legacy_source(
                dir.path(),
                "seq.md",
                &[(
                    "sequence",
                    json!([{ "name": "one", "shell": command }]),
                )],
                "Body.\n",
            );
            let error = legacy_err_for(&source);
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

    #[test]
    fn bracket_target_identity_is_rejected_on_every_graph_shell_surface() {
        let spellings = [
            (r#"ctx["agent"]"#, "ctx.agent"),
            (r#"ctx["model"]"#, "ctx.model"),
            (r#"env["AGENT"]"#, "env.AGENT"),
            (r#"env["MODEL"]"#, "env.MODEL"),
        ];

        for (access, canonical_root) in spellings {
            for position in ["primary", "setup", "teardown", "nested expression"] {
                let command = if position == "nested expression" {
                    format!("echo {{{{ [{access}] }}}}")
                } else {
                    format!("echo {{{{ {access} }}}}")
                };
                let task = match position {
                    "primary" | "nested expression" => json!({
                        "name": "one",
                        "shell": command,
                    }),
                    "setup" => json!({
                        "name": "one",
                        "shell": "echo primary",
                        "setup": [{ "shell": command }],
                    }),
                    "teardown" => json!({
                        "name": "one",
                        "shell": "echo primary",
                        "teardown": [{ "shell": command }],
                    }),
                    _ => unreachable!(),
                };
                let dir = TempDir::new().unwrap();
                let source = write_legacy_source(
                    dir.path(),
                    "seq.md",
                    &[("sequence", json!([task]))],
                    "Body.\n",
                );

                let error = legacy_err_for(&source);
                assert!(
                    matches!(
                        &error,
                        CompositionError::SequenceShellTargetIdentity { root, .. }
                            if root == canonical_root
                    ),
                    "`{access}` in the {position} position must be rejected as \
                     `{canonical_root}`; got: {error}",
                );
            }
        }
    }

    #[test]
    fn computed_ctx_and_env_indexes_fail_closed_at_graph_preflight() {
        for access in [r#"ctx[identity_key]"#, r#"env[lower("MODEL")]"#] {
            let dir = TempDir::new().unwrap();
            let command = format!("echo {{{{ {access} }}}}");
            let source = write_legacy_source(
                dir.path(),
                "seq.md",
                &[(
                    "sequence",
                    json!([{ "name": "one", "shell": command }]),
                )],
                "Body.\n",
            );

            let error = legacy_err_for(&source);
            assert!(
                matches!(
                    &error,
                    CompositionError::SequenceShellTargetIdentity { root, .. }
                        if root == access
                ),
                "computed access `{access}` could select a target-identity leaf and must \
                 fail closed; got: {error}",
            );
        }

        assert_eq!(
            shape::first_target_identity_root("echo {{ seq[identity_key] }}"),
            None,
            "the conservative computed-index rule is limited to ctx/env namespaces",
        );
    }

    /// A task's `teardown:` stack resolves through the same graph phase, so a
    /// target-identity reference there is rejected identically — the bytes it
    /// would bake in are the bytes execution runs.
    #[test]
    fn target_identity_in_a_teardown_shell_command_is_rejected() {
        let dir = TempDir::new().unwrap();
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([
                    {
                        "name": "one",
                        "shell": "echo primary",
                        "teardown": [{ "shell": "echo {{ env.AGENT }}" }],
                    },
                ]),
            )],
            "Body.\n",
        );
        let error = legacy_err_for(&source);
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
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([{
                    "name": "one",
                    // A fallback keeps an absent variable present: a shell
                    // command never runs with a missing value.
                    "shell": [
                        "echo {{ seq.name }}",
                        "echo {{ env.HOME || 'none' }}",
                        "echo {{ ctx[\"area\"] || 'none' }}",
                        "echo {{ env[\"HOME\"] || 'none' }}",
                    ],
                }]),
            )],
            "Body.\n",
        );
        let graph = legacy_graph_for(&source).unwrap();
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
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([
                    {
                        "name": "one",
                        "shell": "echo primary",
                        "setup": [{ "when": "false", "then": [{ "shell": "echo setup" }] }],
                        "teardown": [{ "shell": "echo teardown" }],
                    },
                ]),
            )],
            "Body.\n",
        );
        let graph = legacy_graph_for(&source).unwrap();
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
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([
                    {
                        "name": "one",
                        "side_effect": { "set": { "ready": "{{ true }}" } },
                        "setup": [{ "shell": { "command": "echo main", "on_error": "echo recover" } }],
                    },
                ]),
            )],
            "Body.\n",
        );
        let graph = legacy_graph_for(&source).unwrap();
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
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[("sequence", json!(["alpha", "beta"]))],
            "Work on {{ seq }}.\n",
        );
        let graph = legacy_graph_for(&source).unwrap();
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
        write_legacy_source(root, "review.md", &[("title", json!("Review"))], "r\n");
        let source = write_legacy_source(
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
            "Work on {{ seq.topic }}.\n",
        );

        let graph = legacy_graph_for(&source).unwrap();
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
        write_legacy_yaml(
            root,
            "task.yaml",
            &json!({ "kind": "task", "shell": "echo external", "name": "outside" }),
        );
        let source = write_legacy_source(
            root,
            "seq.md",
            &[("sequence", json!([{ "name": "one", "task": "task.yaml" }]))],
            "Body.\n",
        );
        let graph = legacy_graph_for(&source).unwrap();
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
        write_legacy_yaml(root, "task.yaml", &json!({ "shell": "echo x" }));
        let source = write_legacy_source(
            root,
            "seq.md",
            &[("sequence", json!([{ "name": "one", "task": "task.yaml" }]))],
            "Body.\n",
        );
        let error = legacy_err_for(&source);
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

// -- referenced-document lifecycle pre-scan --------------------------------

mod lifecycle_literals {
    use super::*;

    const DEFECT_SAY: &str = "{{ ok ? 'done in {{area}}' : 'failed' }}";

    /// One sequence per reference spelling, each naming a prompt whose
    /// `success.say` nests a span in a literal. The graph resolves the
    /// reference through the request snapshot, so the pre-scan sees exactly
    /// the document execution would compose.
    #[test]
    #[serial_test::serial(file_resolution_snapshot)]
    fn every_reference_spelling_reaches_the_referenced_document() {
        let request = TempDir::new().unwrap();
        let root = dunce::canonicalize(request.path()).unwrap();
        let sequences = root.join("sequences");
        let magic = root.join("magic");
        let package = root.join("package");
        let env_root = root.join("env");
        let nested = sequences.join("nested");
        for dir in [&sequences, &magic, &package, &env_root, &nested] {
            fs::create_dir_all(dir).unwrap();
        }
        let success = json!({ "say": DEFECT_SAY });
        let cases: [(&str, &Path, &str); 6] = [
            ("./nested/explicit.md", &nested, "explicit.md"),
            ("nested/implicit.md", &nested, "implicit.md"),
            ("@magic.md", &magic, "magic.md"),
            ("&root.md", &root, "root.md"),
            ("^package.md", &package, "package.md"),
            ("{{CLAUDINE_PRESCAN_ROOT}}/env.md", &env_root, "env.md"),
        ];
        let snapshot = biscuit_file::FileResolutionContext::from_snapshot(
            &root,
            None,
            std::collections::HashMap::from([(
                "CLAUDINE_PRESCAN_ROOT".to_string(),
                env_root.display().to_string(),
            )]),
        )
        .with_repository_root(root.clone())
        .with_package_area(package.clone())
        .add_magic_path(magic.clone(), biscuit_file::PathPosition::Start);
        // One capture for all six spellings: a full capture probes the host,
        // and the context is not what varies between the cases.
        let context = darkmatter::markdown::compose::ComposeContext::capture_for_dir(&sequences);

        for (reference, dir, file) in cases {
            write_legacy_source(dir, file, &[("success", success.clone())], "Prompt.\n");
            let source = write_legacy_source(
                &sequences,
                "seq.md",
                &[(
                    "sequence",
                    json!([
                        { "name": "clean", "shell": "true" },
                        { "name": "defect", "prompt": reference },
                    ]),
                )],
                "Body.\n",
            );
            let resolved = crate::composition::resolve_fixture_source(&source).unwrap();
            let plan = resolve_sequence_plan(&resolved, &crate::test_support::context_for(&resolved.resolved_path)).unwrap().unwrap();
            let graph = build_preflight_graph_with_context(
                &plan,
                &resolved,
                context.clone(),
                &snapshot,
            )
            .unwrap_or_else(|error| panic!("{reference}: graph failed: {error}"));

            match graph.validate_prompt_lifecycle_literals() {
                Err(CompositionError::LifecycleNestedSpanInLiteral {
                    source_path,
                    property,
                    nested,
                    ..
                }) => {
                    assert_eq!(
                        source_path,
                        dunce::canonicalize(dir.join(file)).unwrap(),
                        "{reference}"
                    );
                    assert_eq!(property, "success.say.text", "{reference}");
                    assert_eq!(nested, "{{area}}", "{reference}");
                }
                other => panic!("{reference}: expected a nested-span rejection, got {other:?}"),
            }
            fs::remove_file(dir.join(file)).unwrap();
        }
    }

    #[test]
    fn clean_mixed_and_unparseable_documents_are_left_to_their_turn() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_legacy_source(
            root,
            "clean.md",
            &[("success", json!({ "say": "{{ 'done in ' + area }}", "info": "a {{ x ? 'in {{x}}' : 'y' }} b" }))],
            "Prompt.\n",
        );
        // An unparseable lifecycle block is its own turn's error, not the
        // pre-scan's; the nested span beside it is caught there by prepare.
        write_legacy_source(
            root,
            "broken.md",
            &[("success", json!({ "speak": "typo", "say": DEFECT_SAY }))],
            "Prompt.\n",
        );
        let source = write_legacy_source(
            root,
            "seq.md",
            &[(
                "sequence",
                json!([
                    { "name": "clean", "prompt": "clean.md" },
                    { "name": "broken", "prompt": "broken.md" },
                ]),
            )],
            "Body.\n",
        );
        let graph = legacy_graph_for(&source).unwrap();
        assert_eq!(graph.prompt_documents.len(), 2);
        graph
            .validate_prompt_lifecycle_literals()
            .expect("only a parseable single-pass defect is rejected up front");
    }
}

// -- task position and template approval -----------------------------------

mod template_position {
    use super::*;
    use crate::composition::preflight::resolve_graph_shell_approvals;
    use crate::composition::error::ShellApprovalFailure;
    use crate::harness::shell::ShellApprovalOptions;
    use darkmatter::markdown::compose::shell_expansion::template::{
        ModeRequirement, ShellPreApprovals, TemplateShape,
    };
    use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, ShellExpansionError};
    use darkmatter::markdown::MarkdownError;

    fn task(graph: &PreflightGraph, step: usize) -> &PreflightTask {
        graph.steps[step].task.as_ref().expect("the step declares a task")
    }

    /// The spec's `build`/`clean` sequence, with `out: dist`.
    fn build_clean(dir: &Path) -> String {
        write_legacy_source(
            dir,
            "seq.md",
            &[
                ("$schema", json!({ "out": "string -> the build directory" })),
                ("out", json!("dist")),
                (
                    "sequence",
                    json!([
                        { "name": "build", "shell": "just build --out {{ out }}" },
                        { "name": "clean", "shell": "just clean --out {{ out }}/build" },
                    ]),
                ),
            ],
            "Body.\n",
        )
    }

    /// Approve `graph` against the policy files in `policy`, with no handler:
    /// whatever no rule covers fails as it does without a terminal.
    fn approve(graph: &PreflightGraph, policy: &TempDir) -> Result<ShellPreApprovals, CompositionError> {
        let options = ShellApprovalOptions {
            policy_root: Some(policy.path().to_path_buf()),
            ..Default::default()
        };
        resolve_graph_shell_approvals(graph, &options, &|reference| {
            let options = ComposeOptions::new()
                .with_source_file(&reference.path)
                .with_deferred_schema_verdict(true)
                .with_deferred_shell_roots(reference.deferred_shell_roots())
                .with_data_overrides(Value::Object(reference.params.clone()));
            Ok(ComposeRequest::with_context(options, crate::test_support::context_for(&reference.path))
                .unwrap())
        })
        .map(|result| result.approved_commands)
    }

    fn unavailable_command(error: CompositionError) -> String {
        match error {
            CompositionError::ShellApprovalUnavailable {
                command,
                failure: ShellApprovalFailure::NoHandler,
                ..
            } => command,
            other => panic!("expected a no-handler approval failure, got {other:?}"),
        }
    }

    /// The first task's command is approved as its resolved bytes; the second
    /// task's, though `out` is known now, only as its typed shape.
    #[test]
    fn the_first_task_is_resolved_and_a_later_task_is_a_template() {
        let dir = TempDir::new().unwrap();
        let graph = legacy_graph_for(&build_clean(dir.path())).unwrap();

        let build = task(&graph, 0);
        assert_eq!(build.position, TaskPosition::First);
        assert_eq!(
            build.shell_sites[0].mode,
            SiteMode::ResolvedBytes { bytes: "just build --out dist".to_string() }
        );
        let clean = task(&graph, 1);
        assert_eq!(clean.position, TaskPosition::Later);
        assert_eq!(clean.shell_sites[0].mode, SiteMode::Template);
        assert!(!clean.shell_sites[0].reads_late_value(), "`out` is an approval-time value");

        let approved: Vec<&str> = graph.shell_commands.iter().map(|c| c.command.as_str()).collect();
        assert_eq!(approved, vec!["just build --out dist"], "a later task's bytes are never approved");
        let required: Vec<_> = graph.template_candidates.iter().filter(|candidate| candidate.requirement == ModeRequirement::TemplateRequired).collect();
        let [candidate] = required.as_slice() else {
            panic!("one template site: {:?}", graph.template_candidates);
        };
        assert_eq!(candidate.requirement, ModeRequirement::TemplateRequired);
        assert!(!candidate.offered().exact, "a later task is never offered its bytes");
        let view = candidate.approval_view().unwrap();
        assert_eq!(view.display, "just clean --out {{ out: string }}/build");
        assert_eq!(view.shape.canonical_text(), "just clean --out {{ string }}/build");
    }

    /// An `exact` rule for the later task's current bytes does not cover it; a
    /// `template` rule for its shape does, without asking.
    #[test]
    fn an_exact_rule_never_covers_a_later_task_and_a_template_rule_does() {
        let dir = TempDir::new().unwrap();
        let graph = legacy_graph_for(&build_clean(dir.path())).unwrap();

        let policy = TempDir::new().unwrap();
        fs::write(
            policy.path().join(".darkmatter-shell-whitelist"),
            "exact just build --out dist\nexact just clean --out dist/build\n",
        )
        .unwrap();
        let error = approve(&graph, &policy).expect_err("the exact rule must not cover the later task");
        assert_eq!(unavailable_command(error), "just clean --out {{ out }}/build");

        fs::write(
            policy.path().join(".darkmatter-shell-whitelist"),
            "exact just build --out dist\ntemplate just clean --out {{ string }}/build\n",
        )
        .unwrap();
        let approved = approve(&graph, &policy).unwrap();
        assert!(approved.contains_exact("just build --out dist"));
        assert!(!approved.contains_exact("just clean --out dist/build"));
        let shape = TemplateShape::parse_rule("just clean --out {{ string }}/build").unwrap();
        assert!(approved.contains_template(&shape));
    }

    /// A value that exists at approval is still checked for presence in a
    /// later task: only late values skip the check.
    #[test]
    fn a_missing_approval_time_value_fails_preflight_in_any_task() {
        for step in [0, 1] {
            let dir = TempDir::new().unwrap();
            let mut steps = vec![json!({ "name": "one", "shell": "echo one" })];
            steps.insert(step, json!({ "name": "reader", "shell": "echo {{ absent }}" }));
            let source = write_legacy_source(dir.path(), "seq.md", &[("sequence", json!(steps))], "Body.\n");
            let error = legacy_err_for(&source);
            let CompositionError::SequenceShellResolution { source: Some(cause), .. } = &error else {
                panic!("step {step}: expected a typed resolution failure, got {error:?}");
            };
            let MarkdownError::ShellExpansion(shell) = cause.as_ref() else {
                panic!("step {step}: expected a shell cause, got {cause:?}");
            };
            assert!(
                matches!(shell.as_ref(), ShellExpansionError::MissingValue { expression, .. } if expression == "absent"),
                "step {step}: {shell:?}"
            );
        }
    }

    /// `err`, `timing`, and `current` are not late values: a command reading
    /// one stays refused, even when it also reads an allowed late root.
    #[test]
    fn forbidden_roots_stay_refused_even_beside_a_late_root() {
        for (command, root) in [
            ("echo {{ err.msg }}", "err"),
            ("echo {{ timing.total }}", "timing"),
            ("echo {{ current.branch }}", "current"),
            ("echo {{ last(seq.outputs) }} {{ err.msg }}", "err"),
            ("echo {{ last(seq.outputs) || timing.total }}", "timing"),
            ("echo {{ loop.count + current.branch }}", "current"),
        ] {
            let dir = TempDir::new().unwrap();
            let source = write_legacy_source(
                dir.path(),
                "seq.md",
                &[("sequence", json!([{ "name": "one", "shell": "echo hi" }, { "name": "two", "shell": command }]))],
                "Body.\n",
            );
            let error = legacy_err_for(&source);
            assert!(
                matches!(&error, CompositionError::SequenceShellLateBinding { root: found, .. } if found == root),
                "`{command}` must be refused naming `{root}`; got {error:?}"
            );
        }
    }

    /// Every leg of a first group is first; a `group.*` reader is a template
    /// even there; every leg of a later group is later.
    #[test]
    fn a_first_group_qualifies_with_every_leg_and_group_readers_are_templates() {
        let dir = TempDir::new().unwrap();
        let group = |name: &str| {
            json!({ "name": name, "group": {
                "name": name,
                "execution": "parallel",
                "variables": { "label": "g" },
                "tasks": [
                    { "name": "a", "shell": "echo {{ word }} a" },
                    { "name": "b", "shell": "echo {{ word }} b" },
                    { "name": "reader", "shell": "echo {{ group.label }}" },
                ],
            }})
        };
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[("word", json!("w")), ("sequence", json!([group("first"), group("second")]))],
            "Body.\n",
        );
        let graph = legacy_graph_for(&source).unwrap();
        let legs = |step: usize| match &task(&graph, step).action {
            PreflightAction::Group(group) => group.tasks.clone(),
            other => panic!("expected a group, got {other:?}"),
        };

        let first = legs(0);
        for leg in &first[..2] {
            assert_eq!(leg.position, TaskPosition::First);
            assert!(matches!(leg.shell_sites[0].mode, SiteMode::ResolvedBytes { .. }), "{leg:?}");
        }
        assert_eq!(first[2].position, TaskPosition::First);
        assert_eq!(first[2].shell_sites[0].mode, SiteMode::Template);
        assert!(first[2].shell_sites[0].reads_late_value());

        for leg in legs(1) {
            assert_eq!(leg.position, TaskPosition::Later);
            assert_eq!(leg.shell_sites[0].mode, SiteMode::Template, "{leg:?}");
        }
        let approved: Vec<&str> = graph.shell_commands.iter().map(|c| c.command.as_str()).collect();
        assert_eq!(approved, vec!["echo w a", "echo w b"]);
    }

    /// Position is static: a task after work that runs no shell, or that a
    /// run skips, is still a later task.
    #[test]
    fn a_task_is_later_whatever_runs_before_it() {
        let dir = TempDir::new().unwrap();
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[
                ("word", json!("w")),
                (
                    "sequence",
                    json!([
                        { "name": "quiet", "side_effect": { "set": { "seen": true } } },
                        { "name": "speaks", "shell": "echo {{ word }}" },
                    ]),
                ),
            ],
            "Body.\n",
        );
        let graph = legacy_graph_for(&source).unwrap();
        assert_eq!(task(&graph, 0).position, TaskPosition::First);
        let speaks = task(&graph, 1);
        assert_eq!(speaks.position, TaskPosition::Later);
        assert_eq!(speaks.shell_sites[0].mode, SiteMode::Template);
        assert!(graph.shell_commands.is_empty());
    }

    /// A step value derived from a late root is late itself, not a missing
    /// approval-time value.
    #[test]
    fn a_value_derived_from_a_late_root_is_late_not_missing() {
        let dir = TempDir::new().unwrap();
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[
                ("latest", json!("{{ last(seq.outputs) }}")),
                ("again", json!("{{ fm.latest }}")),
                ("sequence", json!([{ "name": "one", "shell": "echo {{ again }}" }])),
            ],
            "Body.\n",
        );
        let graph = legacy_graph_for(&source).unwrap();
        let site = &task(&graph, 0).shell_sites[0];
        assert_eq!(site.mode, SiteMode::Template);
        assert!(site.reads_late_value());
    }

    /// A document property named like a late root, read through `fm.`, is the
    /// document's value and resolves now. (`group`, not `loop`: a sequence
    /// document's root `loop:` is its own loop, not data.)
    #[test]
    fn an_fm_read_of_a_late_root_name_is_a_document_value() {
        let dir = TempDir::new().unwrap();
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[
                ("outputs_dir", json!("o")),
                ("group", json!({ "max": 1 })),
                ("sequence", json!([{ "name": "one", "shell": "echo {{ fm.group.max }}" }])),
            ],
            "Body.\n",
        );
        let graph = legacy_graph_for(&source).unwrap();
        assert_eq!(
            task(&graph, 0).shell_sites[0].mode,
            SiteMode::ResolvedBytes { bytes: "echo 1".to_string() }
        );
    }

    /// A prompt task's parameter derived from `last(seq.outputs)` is late inside
    /// its document; an approval-time parameter is carried as data, so the
    /// document's command is neither missing a value nor approvable as bytes.
    #[test]
    fn a_late_task_parameter_is_late_in_its_document() {
        let dir = TempDir::new().unwrap();
        write_legacy_source(dir.path(), "doc.md", &[], "::shell echo {{ pkg }} {{ topic }}\n");
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[
                ("title", json!("t")),
                (
                    "sequence",
                    json!([{
                        "name": "one",
                        "prompt": "doc.md",
                        "params": { "pkg": "{{ last(seq.outputs) }}", "topic": "{{ title }}" },
                    }]),
                ),
            ],
            "Body.\n",
        );
        let graph = legacy_graph_for(&source).unwrap();
        let [reference] = graph.prompt_references.as_slice() else {
            panic!("one reference: {:?}", graph.prompt_references);
        };
        assert_eq!(reference.position, TaskPosition::First);
        assert_eq!(reference.deferred_params, vec!["pkg".to_string()]);
        assert_eq!(reference.params.get("topic"), Some(&json!("t")));

        let policy = TempDir::new().unwrap();
        let error = approve(&graph, &policy).expect_err("the late command needs a template approval");
        assert_eq!(unavailable_command(error), "echo {{ pkg }} {{ topic }}");

        fs::write(
            policy.path().join(".darkmatter-shell-whitelist"),
            "template echo {{ string }} {{ string }}\n",
        )
        .unwrap();
        approve(&graph, &policy).expect("the template rule covers the document's command");
    }

    /// A later task's document has every command with a placeholder approved
    /// as a template, though its values are known now; the same document as
    /// the first task may run its resolved bytes.
    #[test]
    fn a_later_tasks_document_is_approved_only_by_shape() {
        let dir = TempDir::new().unwrap();
        write_legacy_source(dir.path(), "doc.md", &[], "::shell echo {{ topic }}\n");
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[(
                "sequence",
                json!([
                    { "name": "one", "prompt": "doc.md", "params": { "topic": "t" } },
                    { "name": "two", "prompt": "doc.md", "params": { "topic": "t" } },
                ]),
            )],
            "Body.\n",
        );
        let graph = legacy_graph_for(&source).unwrap();
        let positions: Vec<TaskPosition> = graph.prompt_references.iter().map(|r| r.position).collect();
        assert_eq!(positions, vec![TaskPosition::First, TaskPosition::Later]);

        let policy = TempDir::new().unwrap();
        fs::write(policy.path().join(".darkmatter-shell-whitelist"), "exact echo t\n").unwrap();
        let error = approve(&graph, &policy).expect_err("the exact rule covers only the first reference");
        assert_eq!(unavailable_command(error), "echo {{ topic }}");
    }

    /// A stack `set` value written as `$( … )` is collected and approved: the
    /// first task's as its resolved bytes, a later task's by its shape.
    #[test]
    fn stack_set_shell_values_are_collected_and_approved() {
        let dir = TempDir::new().unwrap();
        let source = write_legacy_source(
            dir.path(),
            "seq.md",
            &[
                ("word", json!("w")),
                (
                    "sequence",
                    json!([
                        {
                            "name": "one",
                            "shell": "echo one",
                            "setup": [{ "when": "false", "then": [{ "set": { "v": "$(git --version {{ word }})" } }] }],
                        },
                        {
                            "name": "two",
                            "shell": "echo two",
                            "teardown": [{ "set": { "v": "$(git --version {{ word }})::ok" } }],
                        },
                    ]),
                ),
            ],
            "Body.\n",
        );
        let graph = legacy_graph_for(&source).unwrap();
        let one = task(&graph, 0);
        assert_eq!(one.approved_set_values.len(), 1);
        assert_eq!(one.approved_set_values[0].key, "v");
        let mut approved: Vec<&str> = graph.shell_commands.iter().map(|c| c.command.as_str()).collect();
        approved.sort_unstable();
        assert_eq!(approved, vec!["echo one", "echo two", "git --version w"]);

        let two = task(&graph, 1);
        assert!(two.approved_set_values.is_empty());
        let site = &two.shell_sites[1];
        assert_eq!(site.kind, ShellSiteKind::SetValue { key: "v".to_string() });
        assert_eq!(site.mode, SiteMode::Template);
        assert_eq!(site.authored, "git --version {{ word }}");
        assert_eq!(
            graph.template_candidates[0].approval_view().unwrap().shape.canonical_text(),
            "git --version {{ string }}"
        );
    }
}

#[test]
fn first_task_shell_template_scripted_choices_persist_and_settle() {
    use darkmatter::markdown::compose::shell_expansion::types::{ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest};
    use crate::harness::shell::ShellApprovalOptions;
    struct Handler(ShellApprovalDecision);
    impl ShellApprovalHandler for Handler {
        fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
            assert!(request.offered.exact && request.offered.shape && request.offered.prefix);
            assert_eq!(request.normalized_exact, "git --version hello");
            assert_eq!(request.template.unwrap().display, "git --version {{ word: string }}");
            Ok(self.0.clone())
        }
    }
    for (decision, row, template_mode) in [
        (ShellApprovalDecision::AllowExactPersist, "exact git --version hello\n", false),
        (ShellApprovalDecision::AllowTemplatePersist, "template git --version {{ string }}\n", true),
        (ShellApprovalDecision::AllowCommandPersist, "prefix git\n", false),
    ] {
        let dir = TempDir::new().unwrap();
        let path = write_legacy_source(dir.path(), "seq.md", &[("word", json!("hello")), ("sequence", json!([{ "name": "probe", "shell": "git --version {{ word }}" }]))], "Body.\n");
        let graph = legacy_graph_for(&path).unwrap();
        let options = ShellApprovalOptions { policy_root: Some(dir.path().to_path_buf()), approval_handler: Some(Arc::new(Handler(decision))), ..Default::default() };
        crate::composition::preflight::resolve_graph_shell_approvals(&graph, &options, &|_| unreachable!()).unwrap();
        assert_eq!(fs::read_to_string(dir.path().join(".darkmatter-shell-whitelist")).unwrap(), row);
        let mut task = graph.steps[0].task.as_ref().unwrap().clone();
        graph.settle_task_shell_modes(&mut task);
        assert_eq!(task.shell_sites[0].mode == SiteMode::Template, template_mode);
    }
}

mod shell_template_program_approval {
    use super::*;
    use crate::composition::preflight::{resolve_graph_shell_approvals, resolve_shell_approvals, resolve_lifecycle_shell_commands};
    use crate::harness::shell::ShellApprovalOptions;
    use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest};
    use darkmatter::markdown::compose::shell_expansion::{ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest};
    use std::sync::Mutex;

    struct Handler(ShellApprovalDecision, Mutex<Vec<ShellApprovalRequest>>);
    impl ShellApprovalHandler for Handler {
        fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
            assert!(request.offered.exact, "resolved bytes stay approvable");
            assert!(!request.offered.shape && !request.offered.prefix, "program holes cannot grant a shape or prefix: {request:?}");
            self.1.lock().unwrap().push(request);
            Ok(self.0.clone())
        }
    }

    fn check(adapter: &str, event: &str, long: bool, chain: bool) {
        for decision in [ShellApprovalDecision::AllowCommandPersist, ShellApprovalDecision::AllowTemplatePersist,
            ShellApprovalDecision::AllowExactPersist, ShellApprovalDecision::AllowOnce, ShellApprovalDecision::Deny, ShellApprovalDecision::BlacklistPersist] {
            let dir = TempDir::new().unwrap();
            let command = if chain { "git --version && {{ tool }} probe" } else { "{{ tool }} probe" };
            let action = if long { json!({"shell": {"command": command}}) } else { json!({"shell": command}) };
            let event_value = if event == "loop" { json!({"gate": [action]}) } else { json!([action]) };
            let mut fm = vec![("tool", json!("argv-recorder"))];
            if adapter == "standalone" {
                fm.push((event, event_value));
            } else {
                let task = if adapter.contains("prompt") {
                    let prompt = write_legacy_source(dir.path(), "prompt.md", &[("tool", json!("argv-recorder")), (event, event_value)], "Prompt.");
                    json!({"name": "probe", "prompt": prompt})
                } else {
                    match event {
                        "primary" => json!({"name": "probe", "shell": command}),
                        stage => json!({"name": "probe", "shell": "echo baseline", stage: [action]}),
                    }
                };
                let step = if adapter.starts_with("group") { json!({"name": "bundle", "group": {"name": "group", "tasks": [task]}}) } else { task };
                fm.push(("sequence", json!([step])));
                fs::write(dir.path().join(".darkmatter-shell-whitelist"), "exact echo baseline\n").unwrap();
            }
            let path = write_legacy_source(dir.path(), "seq.md", &fm, "Body.");
            let handler = Arc::new(Handler(decision.clone(), Mutex::new(Vec::new())));
            let options = ShellApprovalOptions { policy_root: Some(dir.path().to_path_buf()), approval_handler: Some(handler.clone()), ..Default::default() };
            let result = if adapter == "standalone" {
                let source = crate::composition::resolve_fixture_source(&path).unwrap();
                let frontmatter = crate::composition::prepare::frontmatter_to_value(source.markdown.frontmatter());
                let mut lifecycle = crate::composition::lifecycle::parse_lifecycle_config(&frontmatter, &source.resolved_path).unwrap();
                let request = ComposeRequest::with_context(ComposeOptions::new(), crate::test_support::context_for(&source.resolved_path)).unwrap();
                resolve_lifecycle_shell_commands(&mut lifecycle, &frontmatter, request.context(), &source.resolved_path,
                    request.resolution_context(), None, crate::composition::lifecycle::bindings::EnclosingLoop::Outside, None).unwrap();
                resolve_lifecycle_shell_commands(&mut lifecycle, &frontmatter, request.context(), &source.resolved_path,
                    request.resolution_context(), None, crate::composition::lifecycle::bindings::EnclosingLoop::Outside, None).unwrap();
                resolve_shell_approvals(None, None, &options, Some(&lifecycle), Some(&source.resolved_path))
            } else {
                let graph = legacy_graph_for(&path).unwrap();
                resolve_graph_shell_approvals(&graph, &options, &|reference| {
                    Ok(ComposeRequest::with_context(ComposeOptions::new().with_source_file(&reference.path)
                        .with_deferred_schema_verdict(true).with_deferred_shell_roots(reference.deferred_shell_roots())
                        .with_data_overrides(Value::Object(reference.params.clone())), crate::test_support::context_for(&reference.path)).unwrap())
                })
            };
            let requests = handler.1.lock().unwrap();
            assert_eq!(requests.len(), 1, "{adapter}/{event}/{long}/{chain}/{decision:?}: {result:?}");
            let resolved = command.replace("{{ tool }}", "argv-recorder");
            assert_eq!(requests[0].normalized_exact, resolved);
            if let Ok(approved) = &result {
                assert!(approved.approved_commands.contains_exact(&resolved));
                assert_eq!(approved.approved_commands.templates().count(), 0);
            }
            let whitelist = fs::read_to_string(dir.path().join(".darkmatter-shell-whitelist")).unwrap_or_default();
            let blacklist = fs::read_to_string(dir.path().join(".darkmatter-shell-blacklist")).unwrap_or_default();
            assert!(!whitelist.lines().any(|line| line.starts_with("prefix ")));
            match decision {
                ShellApprovalDecision::AllowExactPersist => { assert!(result.is_ok(), "{result:?}"); assert!(whitelist.contains("argv-recorder")); }
                ShellApprovalDecision::AllowOnce => { assert!(result.is_ok(), "{result:?}"); assert!(!whitelist.contains("argv-recorder")); }
                ShellApprovalDecision::BlacklistPersist => { assert!(result.is_err()); assert!(blacklist.contains("argv-recorder")); assert!(!whitelist.contains("argv-recorder")); }
                _ => { assert!(result.is_err()); assert!(!whitelist.contains("argv-recorder")); assert!(blacklist.is_empty()); }
            }
        }
    }

    fn boundary_choice(command: &str, shape: bool) {
        struct BoundaryHandler(bool, Mutex<Vec<ShellApprovalRequest>>);
        impl ShellApprovalHandler for BoundaryHandler {
            fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
                assert!(request.offered.exact);
                assert_eq!(request.offered.shape, self.0);
                assert!(!request.offered.prefix, "variable native-shell program: {request:?}");
                self.1.lock().unwrap().push(request);
                Ok(ShellApprovalDecision::AllowCommandPersist)
            }
        }
        let dir = TempDir::new().unwrap();
        let path = write_legacy_source(dir.path(), "prompt.md", &[
            ("tool", json!("argv-recorder")),
            ("start", json!([{"shell": command}])),
        ], "Body.");
        let source = crate::composition::resolve_fixture_source(&path).unwrap();
        let frontmatter = crate::composition::prepare::frontmatter_to_value(source.markdown.frontmatter());
        let mut lifecycle = crate::composition::lifecycle::parse_lifecycle_config(&frontmatter, &source.resolved_path).unwrap();
        let request = ComposeRequest::with_context(ComposeOptions::new(), crate::test_support::context_for(&source.resolved_path)).unwrap();
        resolve_lifecycle_shell_commands(&mut lifecycle, &frontmatter, request.context(), &source.resolved_path,
            request.resolution_context(), None, crate::composition::lifecycle::bindings::EnclosingLoop::Outside, None).unwrap();
        let handler = Arc::new(BoundaryHandler(shape, Mutex::new(Vec::new())));
        let options = ShellApprovalOptions { policy_root: Some(dir.path().to_path_buf()), approval_handler: Some(handler.clone()), ..Default::default() };
        let result = resolve_shell_approvals(None, None, &options, Some(&lifecycle), Some(&source.resolved_path));
        assert!(result.is_err(), "unoffered program approval must fail: {result:?}");
        assert_eq!(handler.1.lock().unwrap().len(), 1, "must reach authored site approval");
        assert!(!dir.path().join(".darkmatter-shell-whitelist").exists(), "no grant persisted");
    }

    #[test]
    fn leading_curated_redirects_withdraw_program_choice() {
        for command in ["> /dev/null {{ tool }} probe", "2> /dev/null {{ tool }} probe", ">&2 {{ tool }} probe", "2>&1 {{ tool }} probe"] {
            boundary_choice(command, false);
        }
    }

    #[test]
    fn native_shell_line_boundaries_withdraw_program_choice() {
        for command in ["git --version\n{{ tool }} probe", "git --version\r\n{{ tool }} probe",
            "git --version\n> /dev/null {{ tool }} probe", "git --version\r\n2> /dev/null {{ tool }} probe",
            "git --version\n>&2 {{ tool }} probe", "git --version\r\n2>&1 {{ tool }} probe"] {
            boundary_choice(command, true);
        }
    }

    macro_rules! cases {
        ($name:ident, $adapter:literal, $event:literal) => {
            mod $name {
                use super::*;
                #[test] fn short_program() { check($adapter, $event, false, false); }
                #[test] fn long_program() { check($adapter, $event, true, false); }
                #[test] fn short_chain() { check($adapter, $event, false, true); }
                #[test] fn long_chain() { check($adapter, $event, true, true); }
            }
        };
    }
    cases!(first_primary, "first", "primary");
    cases!(first_setup, "first", "setup");
    cases!(first_teardown, "first", "teardown");
    cases!(group_primary, "group", "primary");
    cases!(group_setup, "group", "setup");
    cases!(group_teardown, "group", "teardown");
    cases!(prompt_start, "prompt", "start");
    cases!(prompt_success, "prompt", "success");
    cases!(prompt_finalize, "prompt", "finalize");
    cases!(prompt_failure, "prompt", "failure");
    cases!(prompt_blocked, "prompt", "blocked");
    cases!(prompt_loop, "prompt", "loop");
    cases!(group_prompt_start, "group_prompt", "start");
    cases!(group_prompt_success, "group_prompt", "success");
    cases!(group_prompt_finalize, "group_prompt", "finalize");
    cases!(group_prompt_failure, "group_prompt", "failure");
    cases!(group_prompt_blocked, "group_prompt", "blocked");
    cases!(group_prompt_loop, "group_prompt", "loop");
    cases!(standalone_start, "standalone", "start");
    cases!(standalone_success, "standalone", "success");
    cases!(standalone_finalize, "standalone", "finalize");
    cases!(standalone_failure, "standalone", "failure");
    cases!(standalone_blocked, "standalone", "blocked");
    cases!(standalone_loop, "standalone", "loop");
}

mod container_grammar {
    use super::*;

    #[test]
    fn group_fields_matrix() {
        let dir = TempDir::new().unwrap();
        let control = json!({"name":"work", "tasks":[{"name":"leg", "shell":"echo untouched"}]});
        let cases = [
            ("tasks", Value::Null, false), ("tasks", json!(123), false),
            ("tasks", json!([{"name":"leg", "shell":"echo untouched"}, 123]), false),
            ("tasks", json!([123]), false), ("tasks", json!([]), false),
            ("fail_fast", json!(true), true), ("fail_fast", json!(false), true),
            ("fail_fast", Value::Null, false), ("fail_fast", json!("true"), false),
            ("fail_fast", json!(123), false), ("variables", json!({}), true),
            ("variables", Value::Null, false), ("variables", json!(123), false),
        ];
        for (key, value, valid) in cases {
            let mut group = control.clone(); group[key] = value.clone();
            let path = write_source(dir.path(), "seq.md", &[("sequence", json!([{"name":"work", "group": group}]))], "");
            assert_eq!(graph_for(&path).is_ok(), valid, "{key}={value}");
        }
        for key in ["legs", "succeeded", "failed", "cancelled"] {
            let mut group = control.clone(); group["variables"] = json!({key: "reserved"});
            let path = write_source(dir.path(), "seq.md", &[("sequence", json!([{"name":"work", "group": group}]))], "");
            assert!(err_for(&path).to_string().contains(key));
        }
    }

    #[test]
    fn group_legs_require_unique_nonempty_names_and_execution_is_removed() {
        let dir = TempDir::new().unwrap();
        for tasks in [json!([{"shell":"untouched"}]), json!([{"name":"", "shell":"untouched"}]), json!([{"name":null, "shell":"untouched"}]), json!([{"name":123, "shell":"untouched"}]), json!([{"name":"same", "shell":"untouched"}, {"name":"same", "shell":"untouched"}])] {
            let path = write_source(dir.path(), "seq.md", &[("sequence", json!([{"name":"work", "group":{"name":"bundle", "tasks":tasks}}]))], "");
            assert!(err_for(&path).to_string().contains("bundle"));
        }
        for execution in [Value::Null, json!("parallel"), json!("serial"), json!(123)] {
            let path = write_source(dir.path(), "seq.md", &[("sequence", json!([{"name":"work", "group":{"name":"bundle", "execution":execution, "tasks":[{"name":"leg", "shell":"untouched"}]}}]))], "");
            assert!(matches!(err_for(&path), CompositionError::RemovedGrammar {..}));
        }
    }
}

mod container_yaml_matrix {
    use super::*;

    #[test]
    fn group_and_catalog_file_fields_matrix() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let control = "tasks: [{name: leg, shell: echo untouched}]\n";
        // Optional fields are absent in the control. Variables deliberately
        // allow arbitrary JSON values, so element-type rejection is inapplicable.
        let fields = [
            ("tasks", "null", false), ("tasks", "123", false),
            ("tasks", "[{name: leg, shell: echo untouched}, 123]", false),
            ("tasks", "[123]", false), ("tasks", "[]", false),
            ("tasks", "[", false),
            ("fail_fast", "true", true), ("fail_fast", "false", true),
            ("fail_fast", "null", false), ("fail_fast", "'false'", false),
            ("fail_fast", "'true'", false), ("fail_fast", "123", false),
            ("fail_fast", "[]", false), ("fail_fast", "[", false),
            ("variables", "{}", true), ("variables", "null", false),
            ("variables", "123", false), ("variables", "[]", false),
            ("variables", "{", false),
            ("variables", "{sample: [a, 123]}", true),
            ("variables", "{sample: [123]}", true),
            ("variables", "{sample: null}", true),
            ("variables", "{sample: '123'}", true),
            ("variables", "{legs: reserved}", false),
            ("variables", "{succeeded: reserved}", false),
            ("variables", "{failed: reserved}", false),
            ("variables", "{cancelled: reserved}", false),
            ("cancelled", "[]", true), ("cancelled", "null", false),
            ("cancelled", "123", false), ("cancelled", "[123]", false),
            ("cancelled", "[{warn: good}, 123]", false),
            ("cancelled", "[", false),
        ];
        let mut cases = vec![("control".to_owned(), control.to_owned(), true),
            ("tasks absent".into(), String::new(), false)];
        for (field, value, valid) in fields {
            let replacement = format!("{field}: {value}\n");
            let yaml = if field == "tasks" {replacement} else {format!("{control}{replacement}")};
            cases.push((format!("{field}={value}"), yaml, valid));
        }
        for (field, value) in [("tasks", "[]"), ("variables", "{}"), ("fail_fast", "true"), ("cancelled", "[]")] {
            let yaml = if field == "tasks" {format!("tasks: []\n{control}")}
                else {format!("{control}{field}: {value}\n{field}: {value}\n")};
            cases.push((format!("duplicate {field}"), yaml, false));
        }
        for (label, from, to) in [
            ("leg name absent", "name: leg, ", ""),
            ("leg name empty", "name: leg", "name: ''"),
            ("leg name null", "name: leg", "name: null"),
            ("leg name wrong type", "name: leg", "name: 123"),
            ("leg name duplicate", "name: leg", "name: leg, name: other"),
            ("leg name invalid", "name: leg", "name: ["),
            ("leg name nonunique", "{name: leg, shell: echo untouched}", "{name: leg, shell: echo untouched}, {name: leg, shell: echo untouched}"),
        ] {
            cases.push((label.into(), control.replacen(from, to, 1), false));
        }
        cases.push(("invalid trailing content".into(), format!("{control}invalid: [\n"), false));
        for catalog in [false, true] {
            for (label, yaml, valid) in &cases {
                let document = if catalog {
                    format!("kind: group-catalog\ngroups:\n  - name: bundle\n{}", yaml.lines().map(|line| format!("    {line}\n")).collect::<String>())
                } else { format!("kind: group\nname: bundle\n{yaml}") };
                fs::write(root.join("group.yaml"), document).unwrap();
                let reference = if catalog {"bundle@group.yaml"} else {"group.yaml"};
                let path = write_source(root, "seq.md", &[("sequence", json!([{"name":"work", "group":reference}]))], "");
                let result = graph_for(&path);
                assert_eq!(result.is_ok(), *valid, "catalog={catalog}: {label}: {yaml}: {result:?}");
                if let Ok(graph) = result {
                    let PreflightAction::Group(group) = &graph.steps[0].task.as_ref().unwrap().action else { panic!("expected group") };
                    assert_eq!(group.execution, GroupExecution::Parallel);
                    assert_eq!(group.tasks.len(), 1);
                    assert_eq!(group.fail_fast, label != "fail_fast=false");
                    assert_eq!(group.lifecycle.cancelled_declared, label == "cancelled=[]");
                    assert_eq!(group.lifecycle.clone().cancelled_declared, group.lifecycle.cancelled_declared);
                    assert_eq!(group.tasks[0].name.as_deref(), Some("leg"));
                    if label.starts_with("variables={sample:") {
                        assert!(group.variables.contains_key("sample"));
                    } else { assert!(group.variables.is_empty()); }
                    assert_eq!(graph.steps.len(), 1);
                }
            }
        }
    }
}

mod nesting {
    use super::*;

    #[test]
    fn inline_sequence_preflight_preserves_nested_positions_and_duplicates() {
        let dir = TempDir::new().unwrap();
        let path = write_source(dir.path(), "root.md", &[("sequence", json!([
            {"name":"outer","sequence":[
                {"name":"same","shell":"echo first"},
                {"name":"same","shell":"echo second"}
            ]}
        ]))], "body");
        let graph = graph_for(&path).unwrap();
        let PreflightAction::Sequence(inner) = &graph.steps[0].task.as_ref().unwrap().action else { panic!("nested sequence") };
        assert_eq!(inner.steps.iter().map(|step| step.id.as_str()).collect::<Vec<_>>(), ["same", "same-2"]);
        assert_eq!(inner.steps[0].task.as_ref().unwrap().position, TaskPosition::First);
        assert_eq!(inner.steps[1].task.as_ref().unwrap().position, TaskPosition::Later);
        assert_eq!(graph.shell_commands.iter().map(|command| command.command.as_str()).collect::<Vec<_>>(), ["echo first", "echo second"]);
    }

    #[test]
    fn referenced_sequence_suffix_and_origin_are_preserved() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("nested")).unwrap();
        write_yaml(&dir.path().join("nested"), "steps.json", &json!({"rows":[{"title":"child","shell":"echo child"}]}));
        let path = write_source(dir.path(), "root.md", &[("sequence", json!([
            {"name":"outer","sequence":"nested/steps.json -> rows ::name(title)"}
        ]))], "body");
        let graph = graph_for(&path).unwrap();
        let PreflightAction::Sequence(inner) = &graph.steps[0].task.as_ref().unwrap().action else { panic!("nested sequence") };
        assert_eq!(inner.steps[0].name, "child");
        assert_eq!(inner.steps[0].task.as_ref().unwrap().origin_path, biscuit_file::canonicalize_simplified(&dir.path().join("nested/steps.json")).unwrap());
    }

    #[test]
    fn sequence_reference_cycle_reports_the_active_chain() {
        let dir = TempDir::new().unwrap();
        write_yaml(dir.path(), "a.json", &json!({"sequence":[{"name":"b","sequence":"b.json"}]}));
        write_yaml(dir.path(), "b.json", &json!({"sequence":[{"name":"a","sequence":"a.json"}]}));
        let path = write_source(dir.path(), "root.md", &[("sequence", json!([{"name":"a","sequence":"a.json"}]))], "body");
        let CompositionError::SequenceReferenceCycle { chain } = err_for(&path) else { panic!("cycle diagnostic") };
        assert_eq!(chain.iter().map(|path| path.file_name().unwrap().to_str().unwrap()).collect::<Vec<_>>(), ["root.md", "a.json", "b.json", "a.json"]);
    }
}

mod phase7 {
    use super::*;

    #[test]
    fn distinct_groups_in_one_active_catalog_are_legal() {
        let dir = TempDir::new().unwrap();
        write_yaml(dir.path(), "catalog.json", &json!({"kind":"group-catalog", "groups":[
            {"name":"outer","tasks":[{"name":"inner","group":"inner@catalog.json"}]},
            {"name":"inner","tasks":[{"name":"leaf","shell":"echo safe"}]}
        ]}));
        let path = write_source(dir.path(), "root.md", &[("sequence", json!([{ "name":"root", "group":"outer@catalog.json" }]))], "");
        let graph = graph_for(&path).unwrap();
        assert_eq!(graph.shell_commands[0].command, "echo safe");
    }

    #[test]
    fn mixed_reference_cycle_names_every_active_reference() {
        let dir = TempDir::new().unwrap();
        write_yaml(dir.path(), "inner.json", &json!({"sequence":[{"name":"task","task":"task.json"}]}));
        write_yaml(dir.path(), "task.json", &json!({"kind":"task","group":"checks@catalog.json"}));
        write_yaml(dir.path(), "catalog.json", &json!({"kind":"group-catalog","groups":[
            {"name":"checks","tasks":[{"name":"back","sequence":"inner.json"}]}
        ]}));
        let path = write_source(dir.path(), "root.md", &[("sequence", json!([{ "name":"inner", "sequence":"inner.json" }]))], "");
        let CompositionError::SequenceReferenceCycle { chain } = err_for(&path) else { panic!("cycle diagnostic") };
        assert_eq!(chain.iter().map(|path| path.file_name().unwrap().to_str().unwrap()).collect::<Vec<_>>(),
            ["root.md", "inner.json", "task.json", "catalog.json#checks", "inner.json"]);
    }

    #[test]
    fn sequence_reuse_is_legal_and_only_all_first_descendants_offer_bytes() {
        let dir = TempDir::new().unwrap();
        // The referenced document declares the value its commands read.
        write_yaml(dir.path(), "inner.json", &json!({"value":"early","sequence":[
            {"name":"first","sequence":[{"name":"leaf","shell":"echo {{ value }}"}]},
            {"name":"second","shell":"echo {{ value }}"}
        ]}));
        let path = write_source(dir.path(), "root.md", &[("sequence", json!([
            {"name":"one","sequence":"inner.json"}, {"name":"two","sequence":"inner.json"}
        ]))], "");
        let graph = graph_for(&path).unwrap();
        assert_eq!(graph.template_candidates.iter().map(|candidate| candidate.resolved.is_some()).collect::<Vec<_>>(),
            [true, false, false, false]);
        assert_eq!(graph.steps.len(), 2);
    }

    #[test]
    fn referenced_sequence_shells_resolve_against_the_referenced_document() {
        let dir = TempDir::new().unwrap();
        write_yaml(dir.path(), "child.json", &json!({"value":"child","start":[{"shell":"echo {{ value }}"}],
            "sequence":[{"name":"leaf","shell":"echo {{ value }}-leaf"}]}));
        let path = write_source(dir.path(), "root.md", &[("value", json!("caller")), ("sequence", json!([
            {"name":"child","sequence":"child.json"}, {"name":"after","shell":"echo {{ value }}-after"}
        ]))], "");
        let graph = graph_for(&path).unwrap();
        let commands = graph.shell_commands.iter().map(|command| command.command.as_str()).collect::<Vec<_>>();
        assert_eq!(commands, ["echo child", "echo child-leaf"], "the root's later step offers only a template");
        assert_eq!(graph.steps[0].task.as_ref().map(|task| matches!(&task.action,
            PreflightAction::Sequence(sequence) if sequence.frontmatter.as_ref().and_then(|fm| fm.get("value")) == Some(&json!("child")))),
            Some(true));
    }

    #[test]
    fn nested_inline_write_back_collision_is_rejected() {
        let dir = TempDir::new().unwrap();
        write_source(dir.path(), "write.md", &[("prompt", json!("write"))], "body");
        let path = write_source(dir.path(), "root.md", &[("sequence", json!([
            {"name":"parallel","group":{"tasks":[
                {"name":"a","sequence":[{"name":"write","prompt":"write.md"}]},
                {"name":"b","sequence":[{"name":"nested","sequence":[{"name":"write","prompt":"write.md"}]}]}
            ]}}
        ]))], "");
        let CompositionError::SequenceWriteBackCollision { first, second, target, .. } = err_for(&path) else { panic!("collision diagnostic") };
        assert_eq!((first.as_str(), second.as_str()), ("a", "b"));
        assert_eq!(target.file_name().unwrap(), "write.md");
    }

    #[test]
    fn container_shell_discovery_walks_both_branches_and_shell_values() {
        let dir = TempDir::new().unwrap();
        let path = write_source(dir.path(), "root.md", &[("sequence", json!([
            {"name":"nested","sequence":[], "success":[
                {"when":"false","then":[{"shell":{"command":"echo then","on_error":"warning"}}],"else":[{"shell":"echo else"}]},
                {"set":{"captured":"$(echo captured)"}}
            ]}
        ]))], "");
        let graph = graph_for(&path).unwrap();
        let commands: Vec<_> = graph.shell_commands.iter().map(|command| command.command.as_str()).collect();
        assert_eq!(commands, ["echo then", "echo else", "echo captured"]);
    }

    #[test]
    fn container_shells_refuse_unavailable_roots_even_beside_late_values() {
        let dir = TempDir::new().unwrap();
        for root in ["err.msg", "timing.total", "current.model"] {
            let path = write_source(dir.path(), "root.md", &[("sequence", json!([
                {"name":"nested","sequence":[],"finalize":[{"shell":format!("echo {{{{ {root} }}}} {{{{ seq.outputs }}}}")}]}
            ]))], "");
            assert!(matches!(err_for(&path), CompositionError::SequenceShellLateBinding {..}), "{root}");
        }
    }

    #[test]
    fn container_events_are_discovered_and_terminal_commands_require_templates() {
        let dir = TempDir::new().unwrap();
        let path = write_source(dir.path(), "root.md", &[("value", json!("early")), ("sequence", json!([
            {"name":"group","group":{"name":"checks", "finalize":[{"shell":"echo {{ value }}"}],
                "tasks":[{"name":"a","shell":"echo {{ value }}"},{"name":"b","shell":"echo {{ group.label }}"}],
                "variables":{"label":"label"}}}
        ]))], "");
        let graph = graph_for(&path).unwrap();
        let candidates = &graph.template_candidates;
        assert_eq!(candidates.len(), 3);
        let terminal = candidates.iter().find(|candidate| format!("{:?}", candidate.site.origin).contains("finalize")).unwrap();
        assert_eq!(terminal.requirement, ModeRequirement::TemplateRequired);
        assert!(candidates.iter().any(|candidate| candidate.authored() == Some("echo {{ value }}") && candidate.resolved.is_some()));
        assert!(candidates.iter().any(|candidate| candidate.authored() == Some("echo {{ group.label }}") && candidate.resolved.is_none()));
    }
}
