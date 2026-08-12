use std::fs;
use std::process::Command;

use tempfile::TempDir;

use super::*;

fn init_repo(root: &Path) {
    Command::new("git")
        .args(["init", "--initial-branch=main"])
        .current_dir(root)
        .output()
        .expect("initialize git fixture");
}

fn write_workspace(root: &Path) {
    fs::create_dir_all(root.join("area/pkg")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"area/pkg\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    fs::write(
        root.join("area/pkg/Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
}

struct ProcessStateGuard {
    cwd: PathBuf,
    home: Option<std::ffi::OsString>,
    marker: Option<std::ffi::OsString>,
}

impl Drop for ProcessStateGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.cwd);
        unsafe {
            match self.home.take() {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match self.marker.take() {
                Some(value) => std::env::set_var("CLAUDINE_INVOCATION_CONTEXT_TEST", value),
                None => std::env::remove_var("CLAUDINE_INVOCATION_CONTEXT_TEST"),
            }
        }
    }
}

#[test]
fn one_launch_observation_projects_every_existing_context() {
    let fixture = TempDir::new().unwrap();
    init_repo(fixture.path());
    write_workspace(fixture.path());

    let invocation = InvocationContext::capture_at(fixture.path());
    let before = invocation.work_snapshot();
    let file_resolution = invocation.launch_file_resolution_context();
    let launch = invocation.launch_context();
    let workspace = invocation.launch_workspace_context(None);
    let environment = invocation.environment_context();
    let after = invocation.work_snapshot();

    assert_eq!(file_resolution.base_dir(), fixture.path());
    // `LaunchContext` canonicalizes every path it projects so search-dir
    // dedup compares one form; the authored roots below are unaffected.
    let canonical_fixture = fixture.path().canonicalize().unwrap();
    assert_eq!(launch.repo_root.as_deref(), Some(canonical_fixture.as_path()));
    assert_eq!(workspace.repo_root.as_deref(), Some(fixture.path()));
    assert_eq!(environment.git.as_ref().map(|git| git.repo_root.as_path()), Some(fixture.path()));
    assert_eq!(after.git_root_discoveries, before.git_root_discoveries);
    assert_eq!(after.topology_probes, before.topology_probes);
}

#[test]
#[serial_test::serial(cwd, env)]
fn captured_process_state_is_immutable_for_later_projections() {
    let launch = TempDir::new().unwrap();
    let elsewhere = TempDir::new().unwrap();
    let source = launch.path().join("prompt.md");
    fs::write(&source, "prompt").unwrap();
    let guard = ProcessStateGuard {
        cwd: std::env::current_dir().unwrap(),
        home: std::env::var_os("HOME"),
        marker: std::env::var_os("CLAUDINE_INVOCATION_CONTEXT_TEST"),
    };
    unsafe {
        std::env::set_var("CLAUDINE_INVOCATION_CONTEXT_TEST", "captured");
    }
    let invocation = InvocationContext::capture_at(launch.path());
    let captured_home = invocation.home_dir().map(Path::to_path_buf);

    std::env::set_current_dir(elsewhere.path()).unwrap();
    unsafe {
        std::env::set_var("HOME", elsewhere.path());
        std::env::set_var("CLAUDINE_INVOCATION_CONTEXT_TEST", "mutated");
    }
    let derived = invocation.derive_source(Path::new("prompt.md")).unwrap();

    assert_eq!(derived.source_path(), source);
    assert_eq!(invocation.home_dir(), captured_home.as_deref());
    assert_eq!(
        invocation.environment().get("CLAUDINE_INVOCATION_CONTEXT_TEST").map(String::as_str),
        Some("captured")
    );
    assert_eq!(
        derived
            .file_resolution_context()
            .env()
            .get("CLAUDINE_INVOCATION_CONTEXT_TEST")
            .map(String::as_str),
        Some("captured")
    );
    assert_eq!(
        derived.file_resolution_context().home_dir(),
        captured_home.as_deref()
    );
    drop(guard);
}

#[test]
fn launch_and_same_repository_source_share_one_topology_probe() {
    let fixture = TempDir::new().unwrap();
    init_repo(fixture.path());
    write_workspace(fixture.path());
    let source = fixture.path().join("area/pkg/prompt.md");
    fs::write(&source, "prompt").unwrap();

    let invocation = InvocationContext::capture_at(fixture.path());
    let source_context = invocation.derive_source(&source).unwrap();

    assert_eq!(source_context.repository_root(), Some(fixture.path()));
    let expected_area = fixture.path().join("area");
    assert_eq!(source_context.package_area_root(), Some(expected_area.as_path()));
    let work = invocation.work_snapshot();
    assert_eq!(work.git_root_discoveries, 1);
    assert_eq!(work.topology_probes, 1);
    assert!(work.topology_reuses >= 1);
}

#[test]
fn same_repository_serial_sources_stay_within_launch_work_bounds() {
    let fixture = TempDir::new().unwrap();
    init_repo(fixture.path());
    write_workspace(fixture.path());
    let first = fixture.path().join("area/pkg/first.md");
    let second = fixture.path().join("docs/second.md");
    fs::create_dir_all(second.parent().unwrap()).unwrap();
    fs::write(&first, "first").unwrap();
    fs::write(&second, "second").unwrap();

    let invocation = InvocationContext::capture_at(fixture.path());
    let first_context = invocation.derive_source(&first).unwrap();
    let second_context = invocation.derive_source(&second).unwrap();

    assert_eq!(first_context.repository_root(), Some(fixture.path()));
    assert_eq!(second_context.repository_root(), Some(fixture.path()));
    let work = invocation.work_snapshot();
    assert_eq!(work.git_root_discoveries, 1);
    assert_eq!(work.topology_probes, 1);
    assert!(work.topology_reuses >= 2);
    assert_eq!(work.ambient_fallbacks, 0);
}

#[test]
fn same_repository_parallel_sources_stay_within_launch_work_bounds() {
    let fixture = TempDir::new().unwrap();
    init_repo(fixture.path());
    write_workspace(fixture.path());
    let first = fixture.path().join("area/pkg/first.md");
    let second = fixture.path().join("docs/second.md");
    fs::create_dir_all(second.parent().unwrap()).unwrap();
    fs::write(&first, "first").unwrap();
    fs::write(&second, "second").unwrap();

    let invocation = InvocationContext::capture_at(fixture.path());
    std::thread::scope(|scope| {
        let left = invocation.clone();
        let right = invocation.clone();
        scope.spawn(move || left.derive_source(&first).unwrap());
        scope.spawn(move || right.derive_source(&second).unwrap());
    });

    let work = invocation.work_snapshot();
    assert_eq!(work.git_root_discoveries, 1);
    assert_eq!(work.topology_probes, 1);
    assert!(work.topology_reuses >= 2);
    assert_eq!(work.ambient_fallbacks, 0);
}

#[test]
fn sibling_repository_serial_sources_add_one_repository_observation() {
    let fixture = TempDir::new().unwrap();
    let launch = fixture.path().join("launch");
    let sibling = fixture.path().join("sibling");
    fs::create_dir_all(&launch).unwrap();
    fs::create_dir_all(&sibling).unwrap();
    init_repo(&launch);
    init_repo(&sibling);
    write_workspace(&launch);
    write_workspace(&sibling);
    let first = sibling.join("area/pkg/first.md");
    let second = sibling.join("docs/second.md");
    fs::create_dir_all(second.parent().unwrap()).unwrap();
    fs::write(&first, "first").unwrap();
    fs::write(&second, "second").unwrap();

    let invocation = InvocationContext::capture_at(&launch);
    let first_context = invocation.derive_source(&first).unwrap();
    let second_context = invocation.derive_source(&second).unwrap();

    assert_eq!(first_context.repository_root(), Some(sibling.as_path()));
    assert_eq!(second_context.repository_root(), Some(sibling.as_path()));
    let work = invocation.work_snapshot();
    assert_eq!(work.git_root_discoveries, 2);
    assert_eq!(work.topology_probes, 2);
    assert!(work.topology_reuses >= 1);
    assert_eq!(work.ambient_fallbacks, 0);
}

#[test]
fn repeated_derivation_for_retry_resume_and_jit_reuses_invocation_evidence() {
    let fixture = TempDir::new().unwrap();
    init_repo(fixture.path());
    write_workspace(fixture.path());
    let source = fixture.path().join("area/pkg/prompt.md");
    fs::write(&source, "first revision").unwrap();
    let requirements =
        darkmatter::markdown::compose::ContextRequirements::for_content("{{ ctx.repo_root }}");

    let invocation = InvocationContext::capture_at(fixture.path());
    for revision in 0..6 {
        fs::write(&source, format!("revision {revision}")).unwrap();
        let source_context = invocation.derive_source(&source).unwrap();
        let evidence = invocation.runtime_evidence(&source_context, &requirements);
        let context = darkmatter::markdown::compose::ComposeContext::capture_with_evidence(
            source_context.base_dir(),
            &requirements,
            &evidence,
        );
        assert_eq!(
            context.get("repo_root").and_then(serde_json::Value::as_str),
            Some(fixture.path().to_string_lossy().as_ref())
        );
    }

    assert_eq!(fs::read_to_string(&source).unwrap(), "revision 5");
    let work = invocation.work_snapshot();
    assert_eq!(work.git_root_discoveries, 1);
    assert_eq!(work.topology_probes, 1);
    assert!(work.topology_reuses >= 6);
    assert_eq!(work.ambient_fallbacks, 0);
    // `repo` clones what the repository entry already holds, so no iteration
    // may be charged as capture work; all six are reuses.
    assert_eq!(work.runtime_evidence_captures.get("repo"), None);
    assert_eq!(work.runtime_evidence_reuses.get("repo"), Some(&6));
}

/// Phase 1 work-accounting baseline for the launch/source relocation matrix.
///
/// This intentionally exercises the retained source-owned compatibility API.
/// Canonical preparation uses one launch-context construction per document
/// epoch; these counters keep extra Git, topology, host, environment, or
/// unrecorded ambient work visible without timing.
#[test]
#[serial_test::serial(ctx_launch_anchor_work)]
fn relocation_matrix_reuses_request_evidence_without_ambient_fallbacks() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path();
    init_repo(root);
    for (area, package) in [("alpha", "alpha-lib"), ("beta", "beta-lib")] {
        fs::create_dir_all(root.join(area).join("lib/src")).unwrap();
        fs::write(
            root.join(area).join("lib/Cargo.toml"),
            format!(
                "[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
            ),
        )
        .unwrap();
        fs::write(root.join(area).join("lib/src/lib.rs"), "").unwrap();
    }
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"alpha/lib\", \"beta/lib\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    let external = root.join("external-repository");
    fs::create_dir_all(&external).unwrap();
    init_repo(&external);

    let documents = [
        root.join("prompt.md"),
        root.join("alpha/lib/prompt.md"),
        root.join("beta/lib/prompt.md"),
        external.join("prompt.md"),
    ];
    for document in &documents {
        fs::write(document, "{{ ctx.area }} {{ ctx.repo_root }} {{ ctx.os }} {{ ctx.agent }}")
            .unwrap();
    }

    let _captured_agent =
        test_toolkit::EnvGuard::set_safe("AGENT", "baseline-captured-agent");
    let invocation = InvocationContext::capture_at(&root.join("alpha/lib"));
    let _later_agent = test_toolkit::EnvGuard::set_safe("AGENT", "ambient-mutated-agent");
    let requirements = darkmatter::markdown::compose::ContextRequirements::for_content(
        "{{ ctx.area }} {{ ctx.repo_root }} {{ ctx.os }} {{ ctx.agent }}",
    );

    for document in &documents {
        let source_context = invocation.derive_source(document).unwrap();
        let evidence = invocation.runtime_evidence(&source_context, &requirements);
        let context = darkmatter::markdown::compose::ComposeContext::capture_with_evidence(
            source_context.base_dir(),
            &requirements,
            &evidence,
        );
        assert_eq!(
            context.get("agent").and_then(serde_json::Value::as_str),
            Some("baseline-captured-agent"),
            "every route must consume the invocation's one environment snapshot"
        );
    }

    let work = invocation.work_snapshot();
    assert_eq!(work.git_root_discoveries, 2);
    assert_eq!(work.topology_probes, 2);
    assert_eq!(work.topology_reuses, 3);
    assert_eq!(work.runtime_evidence_captures.get("os"), Some(&1));
    assert_eq!(work.runtime_evidence_reuses.get("os"), Some(&3));
    assert_eq!(work.runtime_evidence_captures.get("repo"), None);
    assert_eq!(work.runtime_evidence_reuses.get("repo"), Some(&4));
    assert_eq!(work.runtime_evidence_captures.get("agent"), None);
    assert_eq!(work.runtime_evidence_reuses.get("agent"), Some(&4));
    assert_eq!(work.ambient_fallbacks, 0);
}

/// A group whose evidence costs real work must be computed once per source
/// directory no matter how many documents or derivations ask for it.
#[test]
fn repeated_requests_for_one_source_capture_costly_evidence_once() {
    let fixture = TempDir::new().unwrap();
    init_repo(fixture.path());
    write_workspace(fixture.path());
    let first = fixture.path().join("area/pkg/first.md");
    let second = fixture.path().join("area/pkg/second.md");
    fs::write(&first, "first").unwrap();
    fs::write(&second, "second").unwrap();
    let requirements =
        darkmatter::markdown::compose::ContextRequirements::for_content("{{ ctx.dirty_files }}");

    let invocation = InvocationContext::capture_at(fixture.path());
    let first_context = invocation.derive_source(&first).unwrap();
    let _ = invocation.runtime_evidence(&first_context, &requirements);
    let _ = invocation.runtime_evidence(&first_context, &requirements);
    let second_context = invocation.derive_source(&second).unwrap();
    let _ = invocation.runtime_evidence(&second_context, &requirements);

    assert_eq!(first_context.base_dir(), second_context.base_dir());
    let work = invocation.work_snapshot();
    assert_eq!(work.runtime_evidence_captures.get("file_changes"), Some(&1));
    assert_eq!(work.runtime_evidence_reuses.get("file_changes"), Some(&2));
    // `datetime` holds no evidence at all and must never look like work.
    assert_eq!(work.runtime_evidence_captures.get("datetime"), None);
    assert_eq!(work.runtime_evidence_reuses.get("datetime"), Some(&3));
    assert_eq!(work.git_root_discoveries, 1);
}

/// The supplied-evidence OS request must stay the request Darkmatter's own
/// ambient capture issues, so that replacing ambient capture with request-owned
/// evidence changes neither the cost of `ctx.os*` nor its value.
#[test]
fn supplied_os_evidence_matches_ambient_os_capture() {
    let fixture = TempDir::new().unwrap();
    let content =
        "{{ ctx.os }} {{ ctx.os_distro }} {{ ctx.os_version }} {{ ctx.os_package_manager }}";
    let source = fixture.path().join("prompt.md");
    fs::write(&source, content).unwrap();
    let requirements = darkmatter::markdown::compose::ContextRequirements::for_content(content);

    let invocation = InvocationContext::capture_at(fixture.path());
    let source_context = invocation.derive_source(&source).unwrap();
    let evidence = invocation.runtime_evidence(&source_context, &requirements);
    let supplied = darkmatter::markdown::compose::ComposeContext::capture_with_evidence(
        source_context.base_dir(),
        &requirements,
        &evidence,
    );
    let ambient = darkmatter::markdown::compose::ComposeContext::capture_for_content(
        source_context.base_dir(),
        content,
    );

    let os_keys = |context: &darkmatter::markdown::compose::ComposeContext| {
        context
            .keys()
            .filter(|key| *key == "os" || key.starts_with("os_"))
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let supplied_keys = os_keys(&supplied);
    assert!(
        !supplied_keys.is_empty(),
        "supplied capture projected no `ctx.os*` key for a document that references them"
    );
    assert_eq!(
        supplied_keys,
        os_keys(&ambient),
        "supplied and ambient capture must project the same `ctx.os*` keys"
    );
    for key in supplied_keys {
        assert_eq!(
            supplied.get(&key),
            ambient.get(&key),
            "supplied and ambient capture must agree on `ctx.{key}`"
        );
    }
}

fn write_two_area_workspace(root: &Path) {
    for (area, package) in [("alpha", "alpha-lib"), ("beta", "beta-lib")] {
        let package_root = root.join(area).join("lib");
        fs::create_dir_all(package_root.join("src")).unwrap();
        fs::write(
            package_root.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
            ),
        )
        .unwrap();
        fs::write(package_root.join("src/lib.rs"), "").unwrap();
    }
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"alpha/lib\", \"beta/lib\"]\nresolver = \"2\"\n",
    )
    .unwrap();
}

#[test]
fn launch_capture_ignores_same_opposing_and_external_document_sources() {
    let fixture = TempDir::new().unwrap();
    let launch_repo = fixture.path().join("launch-repository");
    let external_repo = fixture.path().join("external-repository");
    fs::create_dir_all(&launch_repo).unwrap();
    fs::create_dir_all(&external_repo).unwrap();
    init_repo(&launch_repo);
    init_repo(&external_repo);
    write_two_area_workspace(&launch_repo);

    let documents = [
        launch_repo.join("alpha/lib/prompt.md"),
        launch_repo.join("beta/lib/prompt.md"),
        external_repo.join("prompt.md"),
    ];
    for document in &documents {
        fs::write(document, "{{ ctx.area }} {{ ctx.repo_root }}").unwrap();
    }

    let launch_dir = launch_repo.join("alpha/lib");
    let invocation = InvocationContext::capture_at(&launch_dir);
    let requirements = darkmatter::markdown::compose::ContextRequirements::for_content(
        "{{ ctx.area }} {{ ctx.repo_root }}",
    );

    for document in &documents {
        let source = invocation.derive_source(document).unwrap();
        assert_eq!(source.source_path(), document);
        let before = invocation.work_snapshot();
        let context = invocation.capture_launch_context(&requirements);
        let after = invocation.work_snapshot();

        assert_eq!(
            context.get("area").and_then(serde_json::Value::as_str),
            Some("alpha-lib")
        );
        assert_eq!(
            context.get("repo_root").and_then(serde_json::Value::as_str),
            Some(launch_repo.to_string_lossy().as_ref())
        );
        assert_eq!(after.git_root_discoveries, before.git_root_discoveries);
        assert_eq!(after.topology_probes, before.topology_probes);
    }

    let work = invocation.work_snapshot();
    assert_eq!(work.launch_context_constructions, documents.len());
    assert_eq!(work.ambient_fallbacks, 0);
}

#[test]
fn launch_capture_reports_no_repository_for_an_outside_launch() {
    let fixture = TempDir::new().unwrap();
    let outside = fixture.path().join("outside");
    let source_repo = fixture.path().join("source-repository");
    fs::create_dir_all(&outside).unwrap();
    fs::create_dir_all(&source_repo).unwrap();
    init_repo(&source_repo);
    write_two_area_workspace(&source_repo);
    let document = source_repo.join("alpha/lib/prompt.md");
    fs::write(&document, "{{ ctx.area }} {{ ctx.repo_root }}").unwrap();

    let invocation = InvocationContext::capture_at(&outside);
    let source = invocation.derive_source(&document).unwrap();
    assert_eq!(source.repository_root(), Some(source_repo.as_path()));
    let before = invocation.work_snapshot();
    let requirements = darkmatter::markdown::compose::ContextRequirements::for_content(
        "{{ ctx.area }} {{ ctx.repo_root }}",
    );
    let context = invocation.capture_launch_context(&requirements);
    let after = invocation.work_snapshot();

    assert_eq!(
        context.get("area").and_then(serde_json::Value::as_str),
        Some("")
    );
    assert_eq!(context.get("repo_root"), Some(&serde_json::Value::Null));
    assert_eq!(after.git_root_discoveries, before.git_root_discoveries);
    assert_eq!(after.topology_probes, before.topology_probes);
    assert_eq!(after.launch_context_constructions, 1);
    assert_eq!(after.ambient_fallbacks, 0);
}

#[test]
#[serial_test::serial(cwd, env, ctx_launch_anchor_work)]
fn repeated_launch_capture_reuses_retained_evidence_after_ambient_state_changes() {
    let fixture = TempDir::new().unwrap();
    let launch_repo = fixture.path().join("launch-repository");
    let elsewhere = fixture.path().join("elsewhere");
    fs::create_dir_all(&launch_repo).unwrap();
    fs::create_dir_all(&elsewhere).unwrap();
    init_repo(&launch_repo);
    write_two_area_workspace(&launch_repo);
    fs::write(launch_repo.join("alpha/lib/README.md"), "# Fixture\n").unwrap();

    let guard = ProcessStateGuard {
        cwd: std::env::current_dir().unwrap(),
        home: std::env::var_os("HOME"),
        marker: std::env::var_os("CLAUDINE_INVOCATION_CONTEXT_TEST"),
    };
    unsafe {
        std::env::set_var("CLAUDINE_INVOCATION_CONTEXT_TEST", "captured");
    }
    let invocation = InvocationContext::capture_at(&launch_repo.join("alpha/lib"));
    let captured_home = invocation.home_dir().map(Path::to_path_buf);

    std::env::set_current_dir(&elsewhere).unwrap();
    unsafe {
        std::env::set_var("HOME", &elsewhere);
        std::env::set_var("CLAUDINE_INVOCATION_CONTEXT_TEST", "mutated");
    }
    let requirements = darkmatter::markdown::compose::ContextRequirements::for_content(
        "{{ ctx.area }} {{ ctx.repo_root }} {{ ctx.dirty_files }} \
         {{ ctx.programming_languages_in_repo }} {{ ctx.docs_readme }} \
         {{ ctx.os }} {{ ctx.cpu_cores }} {{ ctx.gpu }}",
    );

    for _ in 0..2 {
        let context = invocation.capture_launch_context(&requirements);
        assert_eq!(
            context.get("area").and_then(serde_json::Value::as_str),
            Some("alpha-lib")
        );
        assert_eq!(
            context
                .env()
                .get("CLAUDINE_INVOCATION_CONTEXT_TEST")
                .map(String::as_str),
            Some("captured")
        );
        assert_eq!(context.env().get("HOME").map(PathBuf::from), captured_home);
    }

    let work = invocation.work_snapshot();
    assert_eq!(work.git_root_discoveries, 1);
    assert_eq!(work.topology_probes, 1);
    assert_eq!(work.launch_context_constructions, 2);
    for group in ["file_changes", "languages", "documents", "os", "hardware", "gpu"] {
        assert_eq!(work.runtime_evidence_captures.get(group), Some(&1), "{group}");
        assert_eq!(work.runtime_evidence_reuses.get(group), Some(&1), "{group}");
    }
    assert_eq!(work.ambient_fallbacks, 0);
    drop(guard);
}

#[test]
#[serial_test::serial(env, ctx_launch_anchor_work)]
fn launch_context_extension_projects_only_missing_groups_and_preserves_overrides() {
    let fixture = TempDir::new().unwrap();
    init_repo(fixture.path());
    write_two_area_workspace(fixture.path());
    let _agent = test_toolkit::EnvGuard::set_safe("AGENT", "invocation-agent");
    let invocation = InvocationContext::capture_at(&fixture.path().join("alpha/lib"));
    let mut captured = darkmatter::markdown::compose::ContextRequirements::for_content(
        "{{ ctx.area }} {{ ctx.agent }}",
    );
    let mut context = invocation.capture_launch_context(&captured);
    context
        .env_mut()
        .insert("AGENT".to_string(), "target-agent".to_string());
    context
        .env_mut()
        .insert("MODEL".to_string(), "target-model".to_string());
    let original_now = context.now().to_string();

    let expanded = darkmatter::markdown::compose::ContextRequirements::for_content(
        "{{ ctx.area }} {{ ctx.agent }} {{ ctx.os }} {{ ctx.cpu_cores }}",
    );
    invocation.extend_launch_context(&mut context, &mut captured, &expanded);
    invocation.extend_launch_context(&mut context, &mut captured, &expanded);

    assert_eq!(context.now(), original_now);
    assert!(context.get("os").is_some());
    assert!(context.get("cpu_cores").is_some());
    let effective = context.as_object();
    assert_eq!(effective.get("agent"), Some(&serde_json::json!("target-agent")));
    assert_eq!(effective.get("model"), Some(&serde_json::json!("target-model")));
    assert_eq!(context.env().get("AGENT").map(String::as_str), Some("target-agent"));
    assert_eq!(context.env().get("MODEL").map(String::as_str), Some("target-model"));

    let work = invocation.work_snapshot();
    assert_eq!(work.launch_context_constructions, 1);
    assert_eq!(work.launch_context_extensions, 1);
    assert_eq!(work.runtime_evidence_reuses.get("repo"), Some(&1));
    assert_eq!(work.runtime_evidence_reuses.get("agent"), Some(&1));
    assert_eq!(work.runtime_evidence_captures.get("os"), Some(&1));
    assert_eq!(work.runtime_evidence_captures.get("hardware"), Some(&1));
    assert_eq!(work.git_root_discoveries, 1);
    assert_eq!(work.topology_probes, 1);
    assert_eq!(work.ambient_fallbacks, 0);
}

#[test]
fn non_repository_sources_cache_exact_absence_without_topology() {
    let fixture = TempDir::new().unwrap();
    let source = fixture.path().join("prompt.md");
    fs::write(&source, "prompt").unwrap();

    let invocation = InvocationContext::capture_at(fixture.path());
    let first = invocation.derive_source(&source).unwrap();
    let second = invocation.derive_source(&source).unwrap();

    assert!(first.repository().is_absent());
    assert!(second.repository().is_absent());
    let work = invocation.work_snapshot();
    assert_eq!(work.git_root_discoveries, 1);
    assert_eq!(work.topology_probes, 0);
}

#[test]
fn nested_repository_gets_a_distinct_observation() {
    let fixture = TempDir::new().unwrap();
    init_repo(fixture.path());
    write_workspace(fixture.path());
    let nested = fixture.path().join("nested");
    fs::create_dir_all(&nested).unwrap();
    init_repo(&nested);
    fs::write(
        nested.join("Cargo.toml"),
        "[package]\nname = \"nested\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    let source = nested.join("prompt.md");
    fs::write(&source, "prompt").unwrap();

    let invocation = InvocationContext::capture_at(fixture.path());
    let source_context = invocation.derive_source(&source).unwrap();

    assert_eq!(source_context.repository_root(), Some(nested.as_path()));
    let work = invocation.work_snapshot();
    assert_eq!(work.git_root_discoveries, 2);
    assert_eq!(work.topology_probes, 2);
}

#[test]
fn linked_worktrees_keep_distinct_repository_keys() {
    let fixture = TempDir::new().unwrap();
    let main = fixture.path().join("main");
    let linked = fixture.path().join("linked");
    fs::create_dir_all(&main).unwrap();
    init_repo(&main);
    fs::write(main.join("tracked.txt"), "tracked").unwrap();
    let commit = Command::new("git")
        .args([
            "-c",
            "user.name=Claudine Test",
            "-c",
            "user.email=claudine@example.invalid",
            "add",
            "tracked.txt",
        ])
        .current_dir(&main)
        .status()
        .unwrap();
    assert!(commit.success());
    let commit = Command::new("git")
        .args([
            "-c",
            "user.name=Claudine Test",
            "-c",
            "user.email=claudine@example.invalid",
            "commit",
            "-q",
            "-m",
            "fixture",
        ])
        .current_dir(&main)
        .status()
        .unwrap();
    assert!(commit.success());
    let worktree = Command::new("git")
        .args(["worktree", "add", "-q", "-b", "linked", linked.to_str().unwrap()])
        .current_dir(&main)
        .status()
        .unwrap();
    assert!(worktree.success());
    let source = linked.join("prompt.md");
    fs::write(&source, "prompt").unwrap();

    let invocation = InvocationContext::capture_at(&main);
    let source_context = invocation.derive_source(&source).unwrap();
    let launch_identity = invocation
        .launch_repository()
        .filesystem_observation()
        .repository_identity()
        .unwrap()
        .unwrap()
        .clone();
    let linked_identity = source_context
        .repository()
        .filesystem_observation()
        .repository_identity()
        .unwrap()
        .unwrap()
        .clone();

    assert_ne!(
        RepositoryKey::from_identity(&launch_identity),
        RepositoryKey::from_identity(&linked_identity)
    );
    assert_eq!(invocation.work_snapshot().git_root_discoveries, 2);
}

#[test]
fn retained_launch_failure_projects_without_retry() {
    let fixture = TempDir::new().unwrap();
    init_repo(fixture.path());
    fs::write(fixture.path().join(".git/config"), "[core\n").unwrap();

    let invocation = InvocationContext::capture_at(fixture.path());
    let first = invocation.launch_repository();
    let first_message = first.failure().expect("typed failure").to_string();
    assert!(first.diagnostic().is_some());
    let _ = invocation.launch_context();
    let _ = invocation.launch_workspace_context(None);
    let _ = invocation.environment_context();
    let second = invocation.launch_repository();

    assert_eq!(second.failure().unwrap().to_string(), first_message);
    assert!(second.diagnostic().is_some());
    assert_eq!(invocation.work_snapshot().git_root_discoveries, 1);
}

#[test]
fn parallel_sources_in_one_unseen_repository_are_single_flight() {
    let fixture = TempDir::new().unwrap();
    let launch = fixture.path().join("launch");
    let repository = fixture.path().join("sibling");
    fs::create_dir_all(&launch).unwrap();
    fs::create_dir_all(repository.join("docs")).unwrap();
    init_repo(&repository);
    write_workspace(&repository);
    let first = repository.join("area/pkg/first.md");
    let second = repository.join("docs/second.md");
    fs::write(&first, "first").unwrap();
    fs::write(&second, "second").unwrap();

    let invocation = InvocationContext::capture_at(&launch);
    std::thread::scope(|scope| {
        let left = invocation.clone();
        let right = invocation.clone();
        scope.spawn(move || left.derive_source(&first).unwrap());
        scope.spawn(move || right.derive_source(&second).unwrap());
    });

    let work = invocation.work_snapshot();
    assert_eq!(work.git_root_discoveries, 2);
    assert_eq!(work.topology_probes, 1);
    assert!(work.topology_reuses >= 1);
}

/// A launch reached through a symlinked ancestor must not re-discover Git for
/// sources inside it, and must keep projecting the authored root it was given.
///
/// Unix-gated: creating a directory symlink on Windows needs Developer Mode or
/// elevation, so the guarantee is pinned on the platforms that can express it.
#[test]
#[cfg(unix)]
fn symlinked_launch_ancestor_reuses_one_observation_and_authored_root() {
    let fixture = TempDir::new().unwrap();
    let real = fixture.path().join("real");
    fs::create_dir_all(&real).unwrap();
    init_repo(&real);
    write_workspace(&real);
    let aliased = fixture.path().join("aliased");
    std::os::unix::fs::symlink(&real, &aliased).unwrap();
    let source = aliased.join("area/pkg/prompt.md");
    fs::write(&source, "prompt").unwrap();

    let invocation = InvocationContext::capture_at(&aliased);
    let source_context = invocation.derive_source(&source).unwrap();

    assert_eq!(source_context.repository_root(), Some(aliased.as_path()));
    assert_eq!(
        invocation.launch_repository().repo_root(),
        Some(aliased.as_path())
    );
    assert_eq!(
        source_context.package_area_root(),
        Some(aliased.join("area").as_path())
    );
    let work = invocation.work_snapshot();
    assert_eq!(work.git_root_discoveries, 1);
    assert_eq!(work.topology_probes, 1);
    assert!(work.topology_reuses >= 1);
}

/// A repository nested under a symlinked launch still earns its own entry.
#[test]
#[cfg(unix)]
fn symlinked_launch_ancestor_still_separates_nested_repositories() {
    let fixture = TempDir::new().unwrap();
    let real = fixture.path().join("real");
    fs::create_dir_all(&real).unwrap();
    init_repo(&real);
    write_workspace(&real);
    let nested = real.join("nested");
    fs::create_dir_all(&nested).unwrap();
    init_repo(&nested);
    let aliased = fixture.path().join("aliased");
    std::os::unix::fs::symlink(&real, &aliased).unwrap();
    let source = aliased.join("nested/prompt.md");
    fs::write(&source, "prompt").unwrap();

    let invocation = InvocationContext::capture_at(&aliased);
    let source_context = invocation.derive_source(&source).unwrap();

    assert_eq!(
        source_context.repository_root(),
        Some(aliased.join("nested").as_path())
    );
    assert_eq!(invocation.work_snapshot().git_root_discoveries, 2);
}

#[test]
fn path_containment_is_component_bounded() {
    let root = Path::new("/repo");
    assert!(Path::new("/repo/docs").starts_with(root));
    assert!(!Path::new("/repository/docs").starts_with(root));
}

#[test]
fn repository_keys_preserve_windows_drive_and_unc_shapes() {
    let drive = RepositoryKey {
        worktree_root: PathBuf::from(r"C:\Work\repo"),
        git_dir: PathBuf::from(r"C:\Work\repo\.git"),
    };
    let drive_prefix_collision = RepositoryKey {
        worktree_root: PathBuf::from(r"C:\Work\repository"),
        git_dir: PathBuf::from(r"C:\Work\repository\.git"),
    };
    let unc = RepositoryKey {
        worktree_root: PathBuf::from(r"\\server\share\repo"),
        git_dir: PathBuf::from(r"\\server\share\repo\.git"),
    };

    assert_ne!(drive, drive_prefix_collision);
    assert_ne!(drive, unc);
}
