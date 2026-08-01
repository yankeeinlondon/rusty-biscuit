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
    assert_eq!(launch.repo_root.as_deref(), Some(fixture.path()));
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
    assert_eq!(work.runtime_evidence_captures.get("repo"), Some(&1));
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
