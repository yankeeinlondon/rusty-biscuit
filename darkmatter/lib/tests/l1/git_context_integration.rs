use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::expression::is_truthy;
use darkmatter::markdown::compose::context::context_variable_descriptors;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::{Markdown, schemas::SimplifiedType};
use gix::bstr::ByteSlice;
use serde_json::{Value, json};
use tempfile::TempDir;

fn init_repo(branch: &str, with_commit: bool) -> TempDir {
    let dir = tempfile::tempdir().expect("temporary repository root");
    gix::init(dir.path()).expect("initialize repository");
    fs::write(
        dir.path().join(".git/HEAD"),
        format!("ref: refs/heads/{branch}\n"),
    )
    .expect("set fixture branch");

    if with_commit {
        let mut config = OpenOptions::new()
            .append(true)
            .open(dir.path().join(".git/config"))
            .expect("open repository config");
        writeln!(
            config,
            "[user]\n\tname = Darkmatter Test\n\temail = darkmatter@example.com"
        )
        .expect("configure fixture identity");
        let repo = gix::open(dir.path()).expect("open initialized repository");
        let tree = repo
            .write_object(gix::objs::Tree::default())
            .expect("write empty tree")
            .detach();
        repo.commit(
            "HEAD",
            "fixture commit",
            tree,
            std::iter::empty::<gix::ObjectId>(),
        )
        .expect("create fixture commit");
    }

    dir
}

fn capture(base_dir: &Path, content: &str) -> ComposeContext {
    ComposeContext::capture_for_content(base_dir, content)
}

fn write_conflict_index(repo_root: &Path, paths: &[(&str, gix::index::entry::Stage)]) {
    let mut state = gix::index::State::new(gix::hash::Kind::Sha1);
    for (path, stage) in paths {
        state.dangerously_push_entry(
            gix::index::entry::Stat::default(),
            gix::ObjectId::empty_blob(gix::hash::Kind::Sha1),
            gix::index::entry::Flags::from_stage(*stage),
            gix::index::entry::Mode::FILE,
            path.as_bytes().as_bstr(),
        );
    }
    state.sort_entries();
    let mut index = gix::index::File::from_state(state, repo_root.join(".git/index"));
    index
        .write(gix::index::write::Options::default())
        .expect("write conflicted fixture index");
}

/// Bare repository whose symbolic HEAD points at a committed local branch.
fn init_bare_repo(branch: &str) -> TempDir {
    let dir = tempfile::tempdir().expect("bare repository root");
    gix::init_bare(dir.path()).expect("initialize bare repository");
    fs::write(dir.path().join("HEAD"), format!("ref: refs/heads/{branch}\n"))
        .expect("set fixture branch");

    let mut config = OpenOptions::new()
        .append(true)
        .open(dir.path().join("config"))
        .expect("open repository config");
    writeln!(
        config,
        "[user]\n\tname = Darkmatter Test\n\temail = darkmatter@example.com"
    )
    .expect("configure fixture identity");

    let repo = gix::open(dir.path()).expect("open bare repository");
    let tree = repo
        .write_object(gix::objs::Tree::default())
        .expect("write empty tree")
        .detach();
    repo.commit(
        "HEAD",
        "fixture commit",
        tree,
        std::iter::empty::<gix::ObjectId>(),
    )
    .expect("create fixture commit");

    dir
}

fn create_linked_worktree(main: &Path, name: &str) -> PathBuf {
    let linked = main.join(name);
    fs::create_dir_all(&linked).expect("create linked worktree root");
    let metadata = main.join(".git/worktrees").join(name);
    fs::create_dir_all(&metadata).expect("create linked worktree metadata");
    let metadata = fs::canonicalize(metadata).expect("canonical worktree metadata");
    let dot_git = linked.join(".git");
    fs::write(&dot_git, format!("gitdir: {}\n", metadata.display()))
        .expect("write linked worktree gitfile");
    fs::write(metadata.join("gitdir"), format!("{}\n", dot_git.display()))
        .expect("write worktree backlink");
    fs::write(metadata.join("commondir"), "../..\n").expect("write common-dir link");
    fs::write(metadata.join("HEAD"), "ref: refs/heads/fixture/branch\n")
        .expect("write linked worktree HEAD");
    linked
}

#[test]
fn shipped_schema_exposes_exact_git_context_descriptors_in_declaration_order() {
    let descriptors = context_variable_descriptors();
    let selected: Vec<_> = descriptors
        .iter()
        .filter(|descriptor| {
            matches!(descriptor.name, "branch" | "worktree" | "merge_conflicts")
        })
        .collect();

    assert_eq!(
        selected.iter().map(|descriptor| descriptor.name).collect::<Vec<_>>(),
        ["branch", "worktree", "merge_conflicts"]
    );
    assert_eq!(selected[0].display_type.base, SimplifiedType::String);
    assert!(!selected[0].display_type.is_array);
    assert!(!selected[0].required);
    assert_eq!(selected[0].category, "Repository");
    assert_eq!(selected[0].subsection, "Git");
    assert_eq!(
        selected[0].description,
        "Current local Git branch name, or null outside a repository or at detached HEAD."
    );
    assert_eq!(selected[1].display_type.base, SimplifiedType::String);
    assert!(!selected[1].display_type.is_array);
    assert!(!selected[1].required);
    assert_eq!(selected[1].category, "Repository");
    assert_eq!(selected[1].subsection, "Git");
    assert_eq!(
        selected[1].description,
        "Current linked Git worktree name, or null in the main worktree or outside a repository."
    );
    assert_eq!(selected[2].display_type.base, SimplifiedType::String);
    assert!(selected[2].display_type.is_array);
    assert!(selected[2].required);
    assert_eq!(selected[2].category, "File Changes");
    assert_eq!(selected[2].subsection, "Conflicts");
    assert_eq!(
        selected[2].description,
        "Repository-relative paths currently in an unresolved Git index state."
    );
}

#[test]
fn git_group_is_demand_driven_and_populates_all_three_values() {
    let repo = init_repo("fixture/branch", true);
    let context = capture(repo.path(), "{{ ctx.branch }}");

    assert_eq!(context.values().get("branch"), Some(&json!("fixture/branch")));
    assert_eq!(context.values().get("worktree"), Some(&Value::Null));
    assert_eq!(context.values().get("merge_conflicts"), Some(&json!([])));
    for unrelated in [
        "repo",
        "is_monorepo",
        "dirty_files",
        "docs_readme",
        "os",
        "cpu_cores",
        "gpu",
    ] {
        assert!(
            !context.values().contains_key(unrelated),
            "Git-only capture unexpectedly populated {unrelated}"
        );
    }
    assert_eq!(
        context
            .capture_timings()
            .iter()
            .map(|(area, _)| area.as_str())
            .collect::<Vec<_>>(),
        ["git"]
    );
}

#[test]
fn branch_projects_attached_outside_unborn_and_detached_states() {
    let attached = init_repo("feature/original-input", true);
    assert_eq!(
        capture(attached.path(), "{{ ctx.branch }}").values().get("branch"),
        Some(&json!("feature/original-input"))
    );

    let outside = tempfile::tempdir().expect("non-repository directory");
    let outside_context = capture(outside.path(), "{{ ctx.branch }}");
    assert_eq!(outside_context.values().get("branch"), Some(&Value::Null));
    assert!(outside_context.diagnostics().is_empty());

    let unborn = init_repo("feature/unborn", false);
    assert_eq!(
        capture(unborn.path(), "{{ ctx.branch }}").values().get("branch"),
        Some(&Value::Null)
    );

    let detached = init_repo("feature/detached-source", true);
    let head_id = gix::open(detached.path())
        .expect("open detached fixture")
        .head_id()
        .expect("fixture HEAD id")
        .detach();
    fs::write(detached.path().join(".git/HEAD"), format!("{head_id}\n"))
        .expect("detach fixture HEAD");
    assert_eq!(
        capture(detached.path(), "{{ ctx.branch }}").values().get("branch"),
        Some(&Value::Null)
    );
}

#[test]
fn worktree_projects_linked_main_bare_and_non_repository_states() {
    let main = init_repo("fixture/branch", true);
    assert_eq!(
        capture(main.path(), "{{ ctx.worktree }}").values().get("worktree"),
        Some(&Value::Null),
        "main worktree must not substitute the branch name"
    );

    let linked = create_linked_worktree(main.path(), "linked-original-input");
    assert_eq!(
        capture(&linked, "{{ ctx.worktree }}").values().get("worktree"),
        Some(&json!("linked-original-input"))
    );

    let bare = tempfile::tempdir().expect("bare repository directory");
    gix::init_bare(bare.path()).expect("initialize bare repository");
    assert_eq!(
        capture(bare.path(), "{{ ctx.worktree }}").values().get("worktree"),
        Some(&Value::Null)
    );

    let outside = tempfile::tempdir().expect("non-repository directory");
    assert_eq!(
        capture(outside.path(), "{{ ctx.worktree }}").values().get("worktree"),
        Some(&Value::Null)
    );
}

/// A bare repository is a valid repository: only worktree- and index-derived
/// fields are neutral there. Discovery must not degrade the whole Git group.
#[test]
fn bare_repository_keeps_branch_and_reports_no_capture_diagnostic() {
    let bare = init_bare_repo("fixture/bare-branch");
    let context = capture(
        bare.path(),
        "{{ ctx.branch }} {{ ctx.worktree }} {{ ctx.merge_conflicts }}",
    );

    assert_eq!(
        context.values().get("branch"),
        Some(&json!("fixture/bare-branch")),
        "bare repo with symbolic HEAD must yield its attached branch"
    );
    assert_eq!(
        context.values().get("worktree"),
        Some(&Value::Null),
        "bare repo has no worktree and must not substitute the branch name"
    );
    assert_eq!(
        context.values().get("merge_conflicts"),
        Some(&json!([])),
        "bare repo has no index and therefore no conflicts"
    );
    assert!(
        context.diagnostics().is_empty(),
        "bare repo discovery must not record a partial-capture diagnostic, got {:?}",
        context.diagnostics()
    );
}

#[test]
fn merge_conflicts_are_native_sorted_deduped_portable_arrays_and_empty_is_falsy() {
    use gix::index::entry::Stage;

    let repo = init_repo("fixture/branch", true);
    write_conflict_index(
        repo.path(),
        &[
            ("zeta.txt", Stage::Ours),
            ("nested/original-input.txt", Stage::Theirs),
            ("zeta.txt", Stage::Theirs),
        ],
    );
    let conflicts = capture(repo.path(), "{{ ctx.merge_conflicts }}")
        .values()
        .get("merge_conflicts")
        .cloned()
        .expect("merge_conflicts value");
    assert_eq!(conflicts, json!(["nested/original-input.txt", "zeta.txt"]));
    assert!(is_truthy(&conflicts));

    let clean = init_repo("fixture/clean", true);
    let clean_value = capture(clean.path(), "{{ ctx.merge_conflicts }}")
        .values()
        .get("merge_conflicts")
        .cloned()
        .expect("clean merge_conflicts value");
    assert_eq!(clean_value, json!([]));
    assert!(!is_truthy(&clean_value));

    let outside = tempfile::tempdir().expect("non-repository directory");
    let absent_value = capture(outside.path(), "{{ ctx.merge_conflicts }}")
        .values()
        .get("merge_conflicts")
        .cloned()
        .expect("absent merge_conflicts value");
    assert_eq!(absent_value, json!([]));
    assert!(!is_truthy(&absent_value));
}

#[test]
fn corrupt_index_degrades_only_merge_conflicts_and_preserves_siblings() {
    let repo = init_repo("fixture/preserved", true);
    fs::write(repo.path().join(".git/index"), b"corrupt index bytes")
        .expect("corrupt fixture index");

    let context = capture(
        repo.path(),
        "{{ ctx.branch }} {{ ctx.worktree }} {{ ctx.merge_conflicts }}",
    );
    assert_eq!(context.values().get("branch"), Some(&json!("fixture/preserved")));
    assert_eq!(context.values().get("worktree"), Some(&Value::Null));
    assert_eq!(context.values().get("merge_conflicts"), Some(&json!([])));
    assert_eq!(context.diagnostics().len(), 1);
    assert!(
        format!("{:?}", context.diagnostics()[0]).contains("merge_conflicts"),
        "field diagnostic must name merge_conflicts: {:?}",
        context.diagnostics()
    );
}

#[test]
fn normal_compose_path_renders_all_git_context_values_from_one_snapshot() {
    use gix::index::entry::Stage;

    let repo = init_repo("feature/compose-original-input", true);
    write_conflict_index(
        repo.path(),
        &[("nested/conflict.md", Stage::Ours), ("nested/conflict.md", Stage::Theirs)],
    );
    let document: Markdown = "Branch: {{ ctx.branch }}\n\nWorktree: {{ ctx.worktree ? ctx.worktree : \"main\" }}\n\nConflicts:\n\n{{ ctx.merge_conflicts }}\n".into();
    let context = ComposeContext::capture_for_document(repo.path(), &document);
    let (composed, report) = document
        .compose_with(ComposeOptions::new_with_context(context))
        .expect("Git-context document composes");

    assert!(report.warnings.is_empty(), "unexpected compose warnings: {report:?}");
    assert!(
        composed.content().contains("Branch: feature/compose-original-input"),
        "branch output: {}",
        composed.content()
    );
    assert!(
        composed.content().contains("Worktree: main"),
        "worktree output: {}",
        composed.content()
    );
    assert!(
        composed.content().contains("nested/conflict.md"),
        "conflict output: {}",
        composed.content()
    );
}
