use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use darkmatter::markdown::compose::expression::{
    EvaluationLookup, ResolutionContext, evaluate, expression_function_descriptors, parse,
};
use darkmatter::markdown::compose::{ComposeOperation, ComposeOptions};
use darkmatter::markdown::Markdown;
use git2::{
    IndexAddOption, Oid, Repository, RepositoryInitOptions, ResetType,
    build::CheckoutBuilder,
};
use pulldown_cmark::{Event, Parser};
use serde_json::{Value, json};
use tempfile::TempDir;

#[derive(Clone, Copy)]
enum Change<'a> {
    Write(&'a str, &'a str),
    Remove(&'a str),
}

struct Fixture {
    _dir: TempDir,
    repo: Repository,
}

impl Fixture {
    fn new(files: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().expect("temporary repository");
        let mut options = RepositoryInitOptions::new();
        options.initial_head("main");
        let repo = Repository::init_opts(dir.path(), &options).expect("initialize repository");
        let fixture = Self { _dir: dir, repo };
        let changes = files
            .iter()
            .map(|(path, content)| Change::Write(path, content))
            .collect::<Vec<_>>();
        fixture.commit("base", &changes, &[]);
        fixture
    }

    fn unborn() -> Self {
        let dir = tempfile::tempdir().expect("temporary repository");
        let mut options = RepositoryInitOptions::new();
        options.initial_head("main");
        let repo = Repository::init_opts(dir.path(), &options).expect("initialize repository");
        Self { _dir: dir, repo }
    }

    fn path(&self) -> &Path {
        self.repo.workdir().expect("non-bare fixture")
    }

    fn head(&self) -> Oid {
        self.repo
            .head()
            .expect("HEAD")
            .peel_to_commit()
            .expect("HEAD commit")
            .id()
    }

    fn create_branch(&self, name: &str, target: Oid) {
        let commit = self.repo.find_commit(target).expect("branch target");
        self.repo.branch(name, &commit, false).expect("create branch");
    }

    fn checkout(&self, name: &str) {
        self.repo
            .set_head(&format!("refs/heads/{name}"))
            .expect("attach HEAD");
        self.repo
            .checkout_head(Some(CheckoutBuilder::new().force()))
            .expect("checkout branch");
    }

    fn detach(&self) {
        self.repo.set_head_detached(self.head()).expect("detach HEAD");
    }

    fn commit(&self, message: &str, changes: &[Change<'_>], extra_parents: &[Oid]) -> Oid {
        let mut index = self.repo.index().expect("repository index");
        for change in changes {
            match *change {
                Change::Write(path, content) => {
                    let absolute = self.path().join(path);
                    if let Some(parent) = absolute.parent() {
                        fs::create_dir_all(parent).expect("create parent directory");
                    }
                    fs::write(&absolute, content).expect("write fixture file");
                    index.add_path(Path::new(path)).expect("stage fixture file");
                }
                Change::Remove(path) => {
                    let absolute = self.path().join(path);
                    if absolute.is_dir() {
                        fs::remove_dir_all(&absolute).expect("remove fixture directory");
                    } else if absolute.exists() {
                        fs::remove_file(&absolute).expect("remove fixture file");
                    }
                    index.remove_path(Path::new(path)).expect("stage removal");
                }
            }
        }
        index
            .add_all(["*"], IndexAddOption::DEFAULT, None)
            .expect("stage fixture tree");
        index.write().expect("write fixture index");
        let tree_id = index.write_tree().expect("write fixture tree");
        let tree = self.repo.find_tree(tree_id).expect("fixture tree");
        let signature = git2::Signature::now("Test", "test@example.com").expect("signature");
        let head_parent = self.repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let mut parents = Vec::new();
        if let Some(parent) = head_parent.as_ref() {
            parents.push(parent);
        }
        let extra = extra_parents
            .iter()
            .map(|id| self.repo.find_commit(*id).expect("extra parent"))
            .collect::<Vec<_>>();
        parents.extend(extra.iter());
        self.repo
            .commit(Some("HEAD"), &signature, &signature, message, &tree, &parents)
            .expect("commit fixture")
    }

    fn diverged(
        files: &[(&str, &str)],
        ours: &[Change<'_>],
        theirs: &[Change<'_>],
    ) -> Self {
        let fixture = Self::new(files);
        let base = fixture.head();
        fixture.create_branch("incoming", base);
        fixture.commit("ours", ours, &[]);
        fixture.checkout("incoming");
        fixture.commit("theirs", theirs, &[]);
        fixture.checkout("main");
        fixture
    }
}

struct Lookup {
    context: ResolutionContext,
}

impl EvaluationLookup for Lookup {
    fn get(&self, _path: &str) -> Option<Value> {
        None
    }

    fn resolution_context_ref(&self) -> Option<&ResolutionContext> {
        Some(&self.context)
    }
}

fn evaluate_at(path: &Path, expression: &str) -> Result<Value, String> {
    let parsed = parse(expression).map_err(|error| error.to_string())?;
    evaluate(
        &parsed,
        &Lookup {
            context: ResolutionContext::new(path.to_path_buf()),
        },
    )
    .map_err(|error| error.to_string())
}

fn conflict_fixture() -> Fixture {
    Fixture::diverged(
        &[("shared.txt", "base\n")],
        &[Change::Write("shared.txt", "ours\n")],
        &[Change::Write("shared.txt", "theirs\n")],
    )
}

#[test]
fn shipped_catalog_describes_predict_conflicts_exactly() {
    let descriptors = expression_function_descriptors();
    let matches = descriptors
        .iter()
        .filter(|descriptor| descriptor.signature == "predict_conflicts(branch)")
        .collect::<Vec<_>>();

    assert_eq!(matches.len(), 1);
    let descriptor = matches[0];
    assert_eq!(descriptor.category, "Git");
    assert_eq!(descriptor.order, 1);
    assert_eq!(
        descriptor.typed_signature(),
        "predict_conflicts(branch: string) -> string[] | error"
    );
    assert_eq!(
        descriptor.description,
        "Returns the repository-relative paths that would conflict if the named local branch were merged into the caller's current branch."
    );
    let example = descriptor.example.as_ref().expect("catalog example");
    assert_eq!(example.invocation, "predict_conflicts(\"feature/example\")");
    assert_eq!(example.result, "src/config.rs");

    let authored: serde_yaml_ng::Value = serde_yaml_ng::from_str(include_str!(
        "../../docs/schemas/expression-functions.yaml"
    ))
    .expect("shipped catalog parses as YAML");
    let authored_function = authored["functions"]
        .as_sequence()
        .expect("functions sequence")
        .iter()
        .find(|function| function["name"].as_str() == Some("predict_conflicts"))
        .expect("authored predict_conflicts entry");
    assert_eq!(authored_function["order"].as_i64(), Some(90));
}

#[test]
fn evaluator_returns_native_conflict_array_and_alias_matches() {
    let fixture = conflict_fixture();
    assert_eq!(
        evaluate_at(fixture.path(), "predict_conflicts(\"incoming\")").unwrap(),
        json!(["shared.txt"])
    );
    assert_eq!(
        evaluate_at(fixture.path(), "predictconflicts(\"incoming\")").unwrap(),
        json!(["shared.txt"])
    );
    assert_eq!(
        evaluate_at(fixture.path(), "predict_conflicts(\"refs/heads/incoming\")").unwrap(),
        json!(["shared.txt"])
    );
}

#[test]
fn conflict_paths_preserve_sniff_ordering_and_merge_direction() {
    let sorted = Fixture::diverged(
        &[("zeta.txt", "base\n"), ("alpha.txt", "base\n")],
        &[
            Change::Write("zeta.txt", "ours\n"),
            Change::Write("alpha.txt", "ours\n"),
        ],
        &[
            Change::Write("zeta.txt", "theirs\n"),
            Change::Write("alpha.txt", "theirs\n"),
        ],
    );
    assert_eq!(
        evaluate_at(sorted.path(), "predict_conflicts(\"incoming\")").unwrap(),
        json!(["alpha.txt", "zeta.txt"])
    );

    let directional = Fixture::diverged(
        &[],
        &[Change::Write("node/child.txt", "ours child\n")],
        &[Change::Write("node", "theirs file\n")],
    );
    let theirs_into_ours =
        evaluate_at(directional.path(), "predict_conflicts(\"incoming\")").unwrap();
    directional.checkout("incoming");
    let ours_into_theirs =
        evaluate_at(directional.path(), "predict_conflicts(\"main\")").unwrap();
    assert_eq!(theirs_into_ours, json!(["node~MERGE_HEAD"]));
    assert_eq!(ours_into_theirs, json!(["node"]));
    assert_ne!(theirs_into_ours, ours_into_theirs);
}

#[test]
fn argument_contract_covers_null_native_types_and_empty_boundaries() {
    let fixture = conflict_fixture();
    assert_eq!(
        evaluate_at(fixture.path(), "predict_conflicts(missing)").unwrap(),
        Value::Null
    );

    for expression in [
        "predict_conflicts(true)",
        "predict_conflicts(7)",
    ] {
        let error = evaluate_at(fixture.path(), expression).unwrap_err();
        assert!(error.contains("expected string"), "{expression}: {error}");
    }

    for expression in [
        "predict_conflicts(\"\")",
        "predict_conflicts(\"   \")",
    ] {
        let error = evaluate_at(fixture.path(), expression).unwrap_err();
        assert!(error.contains("branch name must not be empty"), "{expression}: {error}");
    }

    for expression in ["predict_conflicts()", "predict_conflicts(\"main\", \"incoming\")"] {
        let error = evaluate_at(fixture.path(), expression).unwrap_err();
        assert!(error.contains("requires 1 argument"), "{expression}: {error}");
    }
}

#[test]
fn clean_merge_variants_return_empty_arrays() {
    let same_branch = Fixture::new(&[("file.txt", "base\n")]);
    assert_eq!(
        evaluate_at(same_branch.path(), "predict_conflicts(\"main\")").unwrap(),
        json!([])
    );

    let ancestor = Fixture::new(&[("file.txt", "base\n")]);
    let base = ancestor.head();
    ancestor.create_branch("ancestor", base);
    ancestor.commit("main ahead", &[Change::Write("file.txt", "ahead\n")], &[]);
    assert_eq!(
        evaluate_at(ancestor.path(), "predict_conflicts(\"ancestor\")").unwrap(),
        json!([])
    );
    ancestor.checkout("ancestor");
    assert_eq!(
        evaluate_at(ancestor.path(), "predict_conflicts(\"main\")").unwrap(),
        json!([])
    );

    let clean = Fixture::diverged(
        &[("ours.txt", "base\n"), ("theirs.txt", "base\n")],
        &[Change::Write("ours.txt", "ours\n")],
        &[Change::Write("theirs.txt", "theirs\n")],
    );
    assert_eq!(
        evaluate_at(clean.path(), "predict_conflicts(\"incoming\")").unwrap(),
        json!([])
    );
}

#[test]
fn errors_preserve_branch_and_caller_anchor() {
    let fixture = conflict_fixture();
    for branch in ["missing", "refs/heads/../main"] {
        let error = evaluate_at(
            fixture.path(),
            &format!("predict_conflicts({branch:?})"),
        )
        .unwrap_err();
        assert!(error.contains(branch), "branch missing from error: {error}");
        assert!(
            error.contains(&fixture.path().display().to_string()),
            "caller anchor missing from error: {error}"
        );
    }

    let outside = tempfile::tempdir().expect("non-repository directory");
    let error = evaluate_at(outside.path(), "predict_conflicts(\"main\")").unwrap_err();
    assert!(error.contains("main"));
    assert!(error.contains(&outside.path().display().to_string()));

    let unborn = Fixture::unborn();
    let error = evaluate_at(unborn.path(), "predict_conflicts(\"main\")").unwrap_err();
    assert!(error.contains("main"));
    assert!(error.contains(&unborn.path().display().to_string()));

    let detached = conflict_fixture();
    detached.detach();
    let error = evaluate_at(detached.path(), "predict_conflicts(\"incoming\")").unwrap_err();
    assert!(error.contains("incoming"));
    assert!(error.contains(&detached.path().display().to_string()));

    let unrelated = Fixture::new(&[("file.txt", "base\n")]);
    let mut index = unrelated.repo.index().expect("repository index");
    fs::write(unrelated.path().join("orphan.txt"), "orphan\n").expect("orphan file");
    index.add_path(Path::new("orphan.txt")).expect("stage orphan");
    let tree_id = index.write_tree().expect("orphan tree");
    let tree = unrelated.repo.find_tree(tree_id).expect("orphan tree object");
    let signature = git2::Signature::now("Test", "test@example.com").expect("signature");
    let orphan = unrelated
        .repo
        .commit(None, &signature, &signature, "orphan", &tree, &[])
        .expect("orphan commit");
    unrelated.create_branch("unrelated", orphan);
    let error = evaluate_at(
        unrelated.path(),
        "predict_conflicts(\"unrelated\")",
    )
    .unwrap_err();
    assert!(error.contains("unrelated"));
    assert!(error.contains(&unrelated.path().display().to_string()));

    let corrupt = conflict_fixture();
    let incoming = corrupt
        .repo
        .find_reference("refs/heads/incoming")
        .expect("incoming reference")
        .peel_to_commit()
        .expect("incoming commit")
        .id()
        .to_string();
    let object_path = corrupt
        .repo
        .path()
        .join("objects")
        .join(&incoming[..2])
        .join(&incoming[2..]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&object_path)
            .expect("incoming object metadata")
            .permissions();
        permissions.set_mode(permissions.mode() | 0o200);
        fs::set_permissions(&object_path, permissions).expect("make incoming object writable");
    }
    #[cfg(windows)]
    {
        let mut permissions = fs::metadata(&object_path)
            .expect("incoming object metadata")
            .permissions();
        // This branch is Windows-only, where clearing the read-only file
        // attribute does not broaden Unix mode bits.
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(&object_path, permissions).expect("make incoming object writable");
    }
    fs::write(object_path, b"corrupt").expect("corrupt incoming object");
    let error = evaluate_at(corrupt.path(), "predict_conflicts(\"incoming\")").unwrap_err();
    assert!(error.contains("incoming"));
    assert!(error.contains(&corrupt.path().display().to_string()));
}

#[cfg(unix)]
#[test]
fn git_access_failure_preserves_branch_and_caller_anchor() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = conflict_fixture();
    let git_dir = fixture.repo.path();
    let original = fs::metadata(git_dir).expect("Git metadata").permissions();
    let mut denied = original.clone();
    denied.set_mode(0o000);
    fs::set_permissions(git_dir, denied).expect("deny Git directory access");
    let result = evaluate_at(fixture.path(), "predict_conflicts(\"incoming\")");
    fs::set_permissions(git_dir, original).expect("restore Git directory access");

    let error = result.unwrap_err();
    assert!(error.contains("incoming"));
    assert!(error.contains(&fixture.path().display().to_string()));
}

#[test]
fn caller_anchor_is_shared_by_body_frontmatter_and_shell_ternary() {
    let caller_repo = conflict_fixture();
    let document_repo = Fixture::new(&[("prompt.md", "stored elsewhere\n")]);
    let prompt = document_repo.path().join("prompt.md");
    let document: Markdown = r#"---
frontmatter_result: '{{ predict_conflicts("incoming") }}'
shell_result: $(predict_conflicts("incoming") ? 'conflicts' : echo should-not-run)
---
Body:

{{ predict_conflicts("incoming") }}

Frontmatter: {{ frontmatter_result }}

Shell: {{ shell_result }}
"#
    .into();
    let options = ComposeOptions::new()
        .with_source_file(&prompt)
        .with_file_ref_fallback_dir(caller_repo.path())
        .with_pre_approved_commands(std::collections::HashSet::from([
            "echo should-not-run".to_string(),
        ]))
        .only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::FrontmatterShellExpansion,
            ComposeOperation::Interpolation,
        ]);
    let (composed, report) = document.compose_with(options).expect("compose Git expression");

    assert!(report.warnings.is_empty(), "unexpected warnings: {report:?}");
    assert_eq!(
        composed.frontmatter().as_map().get("frontmatter_result"),
        Some(&json!(["shared.txt"]))
    );
    assert_eq!(
        composed.frontmatter().as_map().get("shell_result"),
        Some(&json!("conflicts"))
    );
    let parsed_text = Parser::new(composed.content())
        .filter_map(|event| match event {
            Event::Text(text) | Event::Code(text) => Some(text.into_string()),
            Event::SoftBreak | Event::HardBreak => Some("\n".to_string()),
            _ => None,
        })
        .collect::<String>();
    assert_eq!(parsed_text.matches("shared.txt").count(), 2);
    assert!(parsed_text.contains("Shell: conflicts"));
}

#[derive(Debug, PartialEq, Eq)]
struct RepositorySnapshot {
    head: Vec<u8>,
    refs: BTreeMap<String, String>,
    index: Option<Vec<u8>>,
    files: BTreeMap<String, Vec<u8>>,
    objects: BTreeSet<String>,
}

fn snapshot(fixture: &Fixture) -> RepositorySnapshot {
    let git_dir = fixture.repo.path();
    let head = fs::read(git_dir.join("HEAD")).expect("HEAD bytes");
    let refs = fixture
        .repo
        .references()
        .expect("references")
        .map(|reference| {
            let reference = reference.expect("reference");
            let name = reference.name().expect("UTF-8 reference").to_string();
            let target = reference
                .resolve()
                .expect("resolve reference")
                .target()
                .expect("direct reference")
                .to_string();
            (name, target)
        })
        .collect();
    let index = fs::read(git_dir.join("index")).ok();
    let mut files = BTreeMap::new();
    collect_files(fixture.path(), fixture.path(), &mut files);
    let mut objects = BTreeSet::new();
    collect_object_paths(&git_dir.join("objects"), &git_dir.join("objects"), &mut objects);
    RepositorySnapshot { head, refs, index, files, objects }
}

fn collect_files(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
    for entry in fs::read_dir(dir).expect("read worktree") {
        let entry = entry.expect("worktree entry");
        let path = entry.path();
        if path == root.join(".git") {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, out);
        } else {
            out.insert(
                path.strip_prefix(root)
                    .expect("relative worktree path")
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/"),
                fs::read(path).expect("worktree file bytes"),
            );
        }
    }
}

fn collect_object_paths(root: &Path, dir: &Path, out: &mut BTreeSet<String>) {
    for entry in fs::read_dir(dir).expect("read object directory") {
        let entry = entry.expect("object entry");
        let path = entry.path();
        if path.is_dir() {
            collect_object_paths(root, &path, out);
        } else {
            out.insert(
                path.strip_prefix(root)
                    .expect("relative object path")
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/"),
            );
        }
    }
}

#[test]
fn prediction_ignores_live_state_and_is_read_only_on_repeated_reads() {
    let fixture = conflict_fixture();
    let expected = json!(["shared.txt"]);

    assert_eq!(
        evaluate_at(fixture.path(), "predict_conflicts(\"incoming\")").unwrap(),
        expected
    );
    let incoming = fixture
        .repo
        .find_reference("refs/heads/incoming")
        .expect("incoming reference")
        .peel_to_commit()
        .expect("incoming commit")
        .id();
    let incoming = fixture
        .repo
        .find_annotated_commit(incoming)
        .expect("annotated incoming commit");
    fixture
        .repo
        .merge(&[&incoming], None, None)
        .expect("create conflicted live index");
    assert!(
        fixture.repo.index().expect("conflicted index").has_conflicts(),
        "fixture must contain unresolved live-index stages"
    );
    let before = snapshot(&fixture);
    assert_eq!(
        evaluate_at(fixture.path(), "predict_conflicts(\"incoming\")").unwrap(),
        expected
    );
    assert_eq!(snapshot(&fixture), before);

    let ours = fixture
        .repo
        .find_object(fixture.head(), None)
        .expect("ours object");
    fixture
        .repo
        .reset(&ours, ResetType::Hard, None)
        .expect("restore live index and worktree");
    fixture.repo.cleanup_state().expect("clear merge state");

    fs::write(fixture.path().join("shared.txt"), "unstaged\n").expect("dirty tracked file");
    fs::write(fixture.path().join("untracked.txt"), "untracked\n").expect("untracked file");
    fs::write(
        fixture.path().join(".gitattributes"),
        "shared.txt merge=unsafe filter=unsafe\n",
    )
    .expect("working attributes");
    let mut index = fixture.repo.index().expect("live index");
    index.add_path(Path::new(".gitattributes")).expect("stage attributes");
    index.write().expect("write staged attributes");

    for _ in 0..2 {
        let before = snapshot(&fixture);
        assert_eq!(
            evaluate_at(fixture.path(), "predict_conflicts(\"incoming\")").unwrap(),
            expected
        );
        assert_eq!(snapshot(&fixture), before);
    }

    fs::write(fixture.repo.path().join("index"), b"corrupt live index")
        .expect("corrupt live index");
    let before = snapshot(&fixture);
    assert_eq!(
        evaluate_at(fixture.path(), "predict_conflicts(\"incoming\")").unwrap(),
        expected
    );
    assert_eq!(snapshot(&fixture), before);

    fs::remove_file(fixture.repo.path().join("index")).expect("remove live index");
    let before = snapshot(&fixture);
    assert_eq!(
        evaluate_at(fixture.path(), "predict_conflicts(\"incoming\")").unwrap(),
        expected
    );
    assert_eq!(snapshot(&fixture), before);
}

#[test]
fn committed_unsafe_driver_is_an_error_and_never_launches() {
    let fixture = Fixture::diverged(
        &[(".gitattributes", "shared.txt merge=unsafe\n"), ("shared.txt", "base\n")],
        &[Change::Write("shared.txt", "ours\n")],
        &[Change::Write("shared.txt", "theirs\n")],
    );
    let sentinel = fixture.path().join("driver-launched");
    let mut config = fixture.repo.config().expect("repository config");
    config
        .set_str(
            "merge.unsafe.driver",
            &format!("touch {}", sentinel.display()),
        )
        .expect("configure unsafe driver");

    let error = evaluate_at(fixture.path(), "predict_conflicts(\"incoming\")").unwrap_err();
    assert!(error.contains("incoming"));
    assert!(error.contains(&fixture.path().display().to_string()));
    assert!(error.contains("unsupported"));
    assert!(!sentinel.exists(), "merge driver must never be launched");
}

#[test]
fn remove_change_fixture_is_exercised() {
    let fixture = Fixture::diverged(
        &[("shared.txt", "base\n")],
        &[Change::Remove("shared.txt")],
        &[Change::Write("shared.txt", "theirs\n")],
    );
    assert_eq!(
        evaluate_at(fixture.path(), "predict_conflicts(\"incoming\")").unwrap(),
        json!(["shared.txt"])
    );
}
