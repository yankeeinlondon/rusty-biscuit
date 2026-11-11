//! The `ci-push` binary over a real repository: real Git (gix reads, git2
//! notes, the Git CLI transport to a bare remote), the real Python
//! interpreter `ci-push` selects, and a compiled `just` stub as the gates.
//!
//! The reviewed revision commits stand-in planner and evidence helpers under
//! `scripts/ci/`, as every revision of this repository commits the real
//! ones; they record their argv so the test can see what the review handed
//! each step.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

use crate::fixture::Fixture;

const PLAN: &str = include_str!("../../ci/fixtures/pre-push/plans/plan-macos-executing.json");

/// The planner (`--event ...`) and `--apply-to`.
const AFFECTED_SCOPE: &str = r#"import json, os, sys
args = sys.argv[1:]
def value(flag):
    return args[args.index(flag) + 1]
record = {"script": "affected_scope.py", "argv": args, "python": sys.executable}
if "--apply-to" in args:
    with open(value("--apply-to")) as source, open(value("--plan-out"), "w") as target:
        target.write(source.read())
else:
    if "--arguments-file" in args:
        with open(value("--arguments-file"), "rb") as arguments:
            record["arguments"] = arguments.read().decode("utf-8").split("\0")
    with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "plan.json")) as plan, open(value("--plan-out"), "w") as out:
        out.write(plan.read())
    print('{"scope": "document"}')
with open(os.environ["CI_PUSH_STANDIN_LOG"], "a") as log:
    log.write(json.dumps(record) + "\n")
"#;

/// `verify` and `scope-record`.
const LOCAL_EVIDENCE: &str = r#"import json, os, sys
args = sys.argv[1:]
def value(flag):
    return args[args.index(flag) + 1]
if args[0] == "verify":
    with open(value("--rejections"), "w") as rejections:
        rejections.write("[]")
    print("[]")
elif args[0] == "scope-record":
    print(json.dumps({"receipt": "scope", "base": value("--base"), "head": value("--head")}))
else:
    sys.exit("unexpected " + args[0])
with open(os.environ["CI_PUSH_STANDIN_LOG"], "a") as log:
    log.write(json.dumps({"script": "local_evidence.py", "argv": args, "python": sys.executable}) + "\n")
"#;

struct Repository {
    _root: tempfile::TempDir,
    checkout: PathBuf,
    remote: PathBuf,
    home: PathBuf,
    log: PathBuf,
}

fn commit(repository: &git2::Repository, message: &str) -> git2::Oid {
    let mut index = repository.index().unwrap();
    index.add_all(["*"], git2::IndexAddOption::DEFAULT, None).unwrap();
    index.write().unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = repository.signature().unwrap();
    let parents: Vec<git2::Commit<'_>> = repository
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .into_iter()
        .collect();
    let parents: Vec<&git2::Commit<'_>> = parents.iter().collect();
    repository
        .commit(Some("HEAD"), &signature, &signature, message, &tree, &parents)
        .unwrap()
}

/// A checkout whose `main` is on a bare remote at one commit and locally one
/// commit ahead (README edited). Returns the repository and both commits.
fn repository() -> (Repository, git2::Oid, git2::Oid) {
    let root = tempfile::tempdir().unwrap();
    let checkout = root.path().join("checkout with space");
    let remote = root.path().join("remote.git");
    let home = root.path().join("home");
    std::fs::create_dir_all(checkout.join("scripts").join("ci")).unwrap();
    std::fs::create_dir_all(&home).unwrap();
    let scripts = checkout.join("scripts").join("ci");
    std::fs::write(scripts.join("affected_scope.py"), AFFECTED_SCOPE).unwrap();
    std::fs::write(scripts.join("local_evidence.py"), LOCAL_EVIDENCE).unwrap();
    std::fs::write(scripts.join("plan.json"), PLAN).unwrap();
    std::fs::write(checkout.join("README.md"), "first\n").unwrap();

    let git = git2::Repository::init(&checkout).unwrap();
    git.set_head("refs/heads/main").unwrap();
    let mut config = git.config().unwrap();
    config.set_str("user.name", "Fixture").unwrap();
    config.set_str("user.email", "fixture@example.com").unwrap();
    let first = commit(&git, "first");
    git2::Repository::init_bare(&remote).unwrap();
    let url = remote.to_str().unwrap();
    git.remote("origin", url)
        .unwrap()
        .push(&["refs/heads/main:refs/heads/main"], None)
        .unwrap();
    std::fs::write(checkout.join("README.md"), "second\n").unwrap();
    let second = commit(&git, "second");
    let log = root.path().join("standins.jsonl");
    (
        Repository {
            _root: root,
            checkout,
            remote,
            home,
            log,
        },
        first,
        second,
    )
}

#[test]
fn a_strict_push_to_main_reviews_publishes_scope_and_hands_the_gates_the_reviewed_plan() {
    let (repository, first, second) = repository();
    let gates = Fixture::new(
        &["just"],
        json!({
            "just": {
                "record_env": [
                    "BISCUIT_CI_PLAN_IN",
                    "BISCUIT_CI_PYTHON",
                    "BISCUIT_CI_PYTHON_ARGS_FILE",
                    "BISCUIT_CI_REPORTS_OUT",
                    "GIT_DIR",
                ]
            }
        }),
    );
    let mut path = vec![gates.bin()];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()));
    let url = repository.remote.to_str().unwrap();
    let mut child = Command::new(biscuit_test_harness::bin_exe!("ci-push"))
        .args(["origin", url])
        .current_dir(&repository.checkout)
        .env("PATH", std::env::join_paths(path).unwrap())
        .env("HOME", &repository.home)
        .env("USERPROFILE", &repository.home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_DIR", repository.home.join("not-this-repository"))
        .env("BISCUIT_CI_CONSTRAINTS_DIR", repository.home.join("constraints"))
        .env("BISCUIT_CI_EVIDENCE_DIR", repository.home.join("evidence"))
        .env("BISCUIT_CI_PLAN_IN", repository.home.join("inherited-plan.json"))
        .env("CI_PUSH_STANDIN_LOG", &repository.log)
        .env("RUSTY_BISCUIT_PRE_PUSH", "strict")
        .env_remove("RUSTY_BISCUIT_PRE_PUSH_AREAS")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("refs/heads/main {second} refs/heads/main {first}\r\n").as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");

    // Every helper ran under one interpreter, against the committed diff.
    let standins: Vec<Value> = std::fs::read_to_string(&repository.log)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let steps: Vec<String> = standins
        .iter()
        .map(|record| format!("{} {}", record["script"].as_str().unwrap(), record["argv"][0].as_str().unwrap()))
        .collect();
    assert_eq!(
        steps,
        [
            "affected_scope.py --event",
            "local_evidence.py verify",
            "affected_scope.py --apply-to",
            "local_evidence.py scope-record",
        ]
    );
    let planner = &standins[0];
    let argv: Vec<&str> = planner["argv"].as_array().unwrap().iter().map(|arg| arg.as_str().unwrap()).collect();
    assert_eq!(argv[..2], ["--event", "push"]);
    assert!(argv.windows(2).any(|pair| pair == ["--base", &first.to_string()]));
    assert!(argv.windows(2).any(|pair| pair == ["--head", &second.to_string()]));
    assert_eq!(planner["arguments"], json!(["--", "README.md", ""]));

    // The scope receipt reached the remote as a note on the pushed commit.
    let remote = git2::Repository::open_bare(&repository.remote).unwrap();
    let note = remote
        .find_note(Some("refs/notes/ci-local/scope"), second)
        .expect("the scope note was pushed");
    let receipt: Value = serde_json::from_str(note.message().unwrap()).unwrap();
    assert_eq!(receipt, json!({"receipt": "scope", "base": first.to_string(), "head": second.to_string()}));
    assert!(stderr.contains("Published scope evidence for"), "{stderr}");

    // The gates got the reviewed plan (not the inherited one), the selected
    // interpreter, and no repository-pinning variable.
    let gate = gates.only_invocation();
    assert_eq!(gate["argv"], json!(["pre-push"]));
    let plan_in = gate["env"]["BISCUIT_CI_PLAN_IN"].as_str().unwrap();
    assert!(plan_in.ends_with("review.json"), "{plan_in}");
    let python = gate["env"]["BISCUIT_CI_PYTHON"].as_str().unwrap();
    assert!(Path::new(python).is_absolute(), "{python}");
    // `sys.executable` is what a launcher or shim resolved to, so compare
    // the helpers with each other: one interpreter ran them all.
    let interpreters: std::collections::BTreeSet<&str> =
        standins.iter().filter_map(|record| record["python"].as_str()).collect();
    assert_eq!(interpreters.len(), 1, "{interpreters:?}");
    assert!(gate["env"]["BISCUIT_CI_PYTHON_ARGS_FILE"].is_string());
    assert_eq!(gate["env"]["GIT_DIR"], Value::Null);

    // The stub staged no plan or report, so no validation receipt.
    assert!(stderr.contains("No validation receipt: the local run wrote no plan"), "{stderr}");
    assert!(stderr.contains("Pre-push validation passed."), "{stderr}");
    // No temporary review tree was needed or left behind.
    let worktrees = git2::Repository::open(&repository.checkout).unwrap().worktrees().unwrap();
    assert_eq!(worktrees.len(), 0);
}
