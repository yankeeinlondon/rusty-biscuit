//! The real `messenger research prepare|check-run|runs|promote|reject|cleanup`
//! binary against isolated fixture trees: required run limits, a run from
//! preparation to promotion with fake pass outputs, refusal exit statuses,
//! escape-free JSON, and dry-run-first cleanup. Nothing launches Claudine.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use messenger::research::generate::generate;
use messenger::research::model::{Date, PlatformId};
use messenger::research::publish::Options;
use messenger::research::{Loader, Workspace};
use serde_json::{Value, json};
use tempfile::TempDir;

fn cli_dir() -> PathBuf {
    biscuit_test_harness::manifest_dir!()
}

fn repo_root() -> PathBuf {
    cli_dir().parent().and_then(Path::parent).expect("repository root").to_path_buf()
}

const SHIPPED: &[&str] = &[
    "messenger/docs/platforms.yaml",
    "messenger/docs/platforms.schema.yaml",
    "messenger/docs/research/platforms/_schema.yaml",
    "messenger/docs/research/platforms/_types.yaml",
    "messenger/docs/research/platforms/_overrides.schema.yaml",
    "messenger/docs/research/platforms/_fleet.md",
    "messenger/docs/research/platforms/_rules.md",
    "messenger/docs/research/implementation/_schema.yaml",
];
const STATE: &str = "messenger/.research-state";

/// The shipped contract plus the accepted-fleet fixture, published.
struct Fleet {
    dir: TempDir,
    root: PathBuf,
}

impl Fleet {
    fn published() -> Self {
        let fleet = Self::unpublished_at(None);
        let loader = Loader::new(Workspace::new(fleet.root()).expect("absolute"));
        generate(&loader, &BTreeMap::new(), &Date::parse("2026-09-17").expect("date"), Options::default()).expect("publish");
        fleet
    }

    /// The fleet at its fixed paths with no snapshot, rooted at `name` inside
    /// the temporary directory when given, so the repository root can carry
    /// characters a shell would interpret.
    fn unpublished_at(name: Option<&str>) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = name.map_or_else(|| dir.path().to_path_buf(), |name| dir.path().join(name));
        for path in SHIPPED {
            let target = root.join(path);
            fs::create_dir_all(target.parent().expect("parent")).expect("mkdir");
            fs::copy(repo_root().join(path), target).expect("copy");
        }
        let fleet = Self { dir, root };
        for platform in PlatformId::ALL {
            fleet.write(&format!("messenger/docs/research/platforms/{platform}.md"), &fixture(*platform, None));
        }
        fleet
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn path(&self, relative: &str) -> PathBuf {
        relative.split('/').fold(self.root().to_path_buf(), |acc, segment| acc.join(segment))
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, content).expect("write");
    }

    fn research(&self, today: &str, args: &[&str]) -> Output {
        Command::new(biscuit_test_harness::bin_exe!("messenger"))
            .arg("research")
            .arg("--root")
            .arg(self.root())
            .args(["--today", today])
            .args(args)
            .env("NO_COLOR", "1")
            .output()
            .expect("run messenger")
    }

    /// Every file under `messenger/docs/`.
    fn docs(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(dir).expect("dir").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else {
                    out.insert(path.clone(), fs::read(&path).expect("read"));
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(&self.root().join("messenger/docs"), &mut out);
        out
    }

    fn curated(&self, platform: PlatformId) -> Vec<String> {
        let loader = Loader::new(Workspace::new(self.root()).expect("absolute"));
        let roster = loader.load_roster(&loader.workspace().roster()).expect("roster").record.expect("typed");
        roster.platform(platform).expect("platform").curated_sources.iter().map(|s| s.url.clone()).collect()
    }
}

/// A fleet fixture rebound to the accepted path, optionally re-observed on a date.
fn fixture(platform: PlatformId, on: Option<&str>) -> String {
    let source = repo_root().join(format!("messenger/lib/tests/fixtures/research/lifecycle/fleet/{platform}.md"));
    let text = fs::read_to_string(source)
        .expect("fleet fixture")
        .replace("$schema: ../../../../../../docs/research/platforms/_schema.yaml", "$schema: ./_schema.yaml");
    match on {
        Some(on) => text.replace("last_updated: 2026-09-10", &format!("last_updated: {on}")).replace("retrieved: 2026-09-10", &format!("retrieved: {on}")),
        None => text,
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("utf-8 stdout")
}

fn json_out(output: &Output) -> Value {
    let text = stdout(output);
    assert!(!text.contains('\u{1b}'), "JSON stdout carries an escape sequence: {text}");
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("stdout is not one JSON document ({error}): {text}"))
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("exit status")
}

/// Writes every pass's outputs for a Discord run with a raised content limit.
fn fake_passes(fleet: &Fleet, run_dir: &Path, on: &str) {
    let write = |relative: &str, text: &str| {
        let path = relative.split('/').fold(run_dir.to_path_buf(), |acc, s| acc.join(s));
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, text).expect("write");
    };
    write("outputs/discovery.md", "# Discovery\n");
    write(
        "outputs/suggested-sources.json",
        &json!({ "suggestions": [{ "url": "https://community.example.com/t/1", "questions": ["2"], "contribution": "Limit reports." }] }).to_string(),
    );
    let candidate = fixture(PlatformId::Discord, Some(on)).replacen("  value: 2000\n", "  value: 4000\n", 1).replacen(
        "changes: []\n",
        "changes:\n- id: chg.discord.content\n  kind: changed\n  facts:\n  - c.discord.content.max\n  evidence:\n  - src.discord.docs\n  summary: Re-checked.\n",
        1,
    );
    write("candidate/discord.md", &candidate);
    let mut urls = fleet.curated(PlatformId::Discord);
    urls.push("https://docs.example.com/discord/messages".to_string());
    let checks: Vec<Value> =
        urls.iter().map(|url| json!({ "url": url, "checked_on": on, "outcome": "checked", "finding": "Reviewed." })).collect();
    write("source-checks.json", &json!({ "checks": checks }).to_string());
    let retain: Vec<Value> = fleet
        .curated(PlatformId::Discord)
        .iter()
        .map(|url| json!({ "url": url, "interfaces": ["discord_bot_api"], "contribution": "Primary reference." }))
        .collect();
    write("outputs/source-proposal.json", &json!({ "retain": retain, "add": [], "remove": [] }).to_string());
}

fn fake_review(run_dir: &Path) {
    let delta: Value = serde_json::from_slice(&fs::read(run_dir.join("delta.json")).expect("delta")).expect("json");
    let mut conclusions = Vec::new();
    for (key, kind) in [("facts", "fact"), ("sources", "source"), ("gaps", "gap")] {
        for item in delta[key].as_array().expect("array") {
            conclusions.push(json!({ "kind": kind, "subject": item["id"], "verdict": "supported", "evidence": ["src.discord.docs"], "finding": "Matches." }));
        }
    }
    fs::write(
        run_dir.join("outputs").join("evidence-review.json"),
        json!({ "reviewer": "fake", "conclusions": conclusions }).to_string(),
    )
    .expect("write review");
}

#[test]
fn prepare_requires_both_limits_before_writing_anything() {
    let fleet = Fleet::published();
    for (args, expect) in [
        (vec!["prepare", "discord"], "elapsed-time"),
        (vec!["prepare", "discord", "--max-seconds", "600"], "agent-invocation"),
        (vec!["prepare", "discord", "--max-invocations", "8"], "elapsed-time"),
        (vec!["prepare", "discord", "--max-seconds", "0", "--max-invocations", "8"], "must be positive"),
    ] {
        let output = fleet.research("2026-09-18", &args);
        assert_eq!(code(&output), 2, "{args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expect), "{args:?}: {stderr}");
        assert!(!fleet.path(STATE).join("runs").exists(), "{args:?}: nothing is written before limits validate");
    }
    let refused = json_out(&fleet.research("2026-09-18", &["prepare", "--json"]));
    assert!(refused["refused"].as_str().expect("refusal").contains("no default"), "{refused}");
    // A dry run shows the selection without limits and writes nothing.
    let dry = fleet.research("2026-09-18", &["prepare", "--dry-run", "--json"]);
    assert_eq!(code(&dry), 0);
    let dry = json_out(&dry);
    assert_eq!(dry["selections"].as_array().expect("selections").len(), 5);
    assert_eq!(dry["selections"][0]["due"][0]["reason"], "no_review_record");
    assert!(!fleet.path(STATE).join("runs").exists());
}

#[test]
fn a_run_goes_from_preparation_to_promotion_through_the_binary() {
    let fleet = Fleet::published();
    let docs = fleet.docs();
    let prepared = fleet.research("2026-09-18", &["prepare", "discord", "--max-seconds", "600", "--max-invocations", "8", "--json"]);
    assert_eq!(code(&prepared), 0, "{}", String::from_utf8_lossy(&prepared.stderr));
    let prepared = json_out(&prepared);
    let run = &prepared["runs"][0];
    let run_id = run["run_id"].as_str().expect("run id").to_string();
    let run_dir = fleet.path(run["run_dir"].as_str().expect("run dir"));
    assert_eq!(run["commands"][0].as_array().expect("command")[..3], [json!("claudine"), json!("budget"), json!("init")]);
    assert_eq!(fleet.docs(), docs, "preparing writes only the state area");

    // Nothing written yet: the check fails with exit 1, whatever an agent exited with.
    let failed = fleet.research("2026-09-18", &["check-run", &run_id, "--through", "validation", "--json"]);
    assert_eq!(code(&failed), 1);
    assert_eq!(json_out(&failed)["status"], "failed");
    let resumed = fleet.research("2026-09-18", &["prepare", "--resume", &run_id, "--json"]);
    assert_eq!(code(&resumed), 0, "{}", String::from_utf8_lossy(&resumed.stderr));
    assert_eq!(json_out(&resumed)["runs"][0]["commands"].as_array().expect("commands").len(), 1, "a resumption keeps the ledger");

    fake_passes(&fleet, &run_dir, "2026-09-18");
    let validated = fleet.research("2026-09-18", &["check-run", &run_id, "--through", "validation", "--json"]);
    assert_eq!(code(&validated), 0, "{}", stdout(&validated));
    fake_review(&run_dir);
    let reviewed = fleet.research("2026-09-18", &["check-run", &run_id]);
    assert_eq!(code(&reviewed), 0, "{}", stdout(&reviewed));
    assert!(stdout(&reviewed).contains("awaiting_review"), "{}", stdout(&reviewed));

    let refused = fleet.research("2026-09-18", &["promote", &run_id, "--renewal", "--json"]);
    assert_eq!(code(&refused), 1);
    let refused = json_out(&refused);
    assert!(refused["reasons"].as_array().expect("reasons").iter().any(|r| r.as_str().unwrap_or_default().contains("typed metadata changed")));
    assert_eq!(fleet.docs(), docs, "a refused promotion publishes nothing");

    let promoted = fleet.research("2026-09-18", &["promote", &run_id, "--approved-by", "maintainer", "--json"]);
    assert_eq!(code(&promoted), 0, "{}", String::from_utf8_lossy(&promoted.stderr));
    let promoted = json_out(&promoted);
    let review = promoted["promoted"][0]["review"].as_str().expect("review path");
    assert!(fleet.path(review).exists());
    assert!(fs::read_to_string(fleet.path("messenger/docs/research/CHANGELOG.md")).expect("changelog").contains(&run_id));
    assert_eq!(code(&fleet.research("2026-09-18", &["generate", "--check"])), 0, "no drift after promotion");

    let runs = json_out(&fleet.research("2026-09-18", &["runs", "--json"]));
    assert_eq!(runs["state_dir"], STATE);
    assert_eq!(runs["runs"][0]["status"], "accepted");
    let human = fleet.research("2026-09-18", &["runs"]);
    assert_eq!(code(&human), 0);
    assert!(stdout(&human).contains(&run_id));
    // A decided run cannot be promoted, rejected, or checked again.
    assert_eq!(code(&fleet.research("2026-09-18", &["promote", &run_id, "--approved-by", "maintainer"])), 1);
    assert_eq!(code(&fleet.research("2026-09-18", &["reject", &run_id, "--by", "maintainer", "--reason", "late"])), 1);
    assert_eq!(code(&fleet.research("2026-09-18", &["check-run", &run_id])), 1);
    // `promote` needs an approver or --renewal.
    assert_eq!(code(&fleet.research("2026-09-18", &["promote", &run_id])), 2);
}

#[test]
fn invalid_decision_inputs_exit_2_before_any_decision_or_publication() {
    let fleet = Fleet::published();
    let on = "2026-09-18";
    let prepared = json_out(&fleet.research(on, &["prepare", "discord", "--max-seconds", "600", "--max-invocations", "8", "--json"]));
    let run_id = prepared["runs"][0]["run_id"].as_str().expect("run id").to_string();
    let run_dir = fleet.path(prepared["runs"][0]["run_dir"].as_str().expect("run dir"));
    fake_passes(&fleet, &run_dir, on);
    assert_eq!(code(&fleet.research(on, &["check-run", &run_id, "--through", "validation"])), 0);
    fake_review(&run_dir);
    assert_eq!(code(&fleet.research(on, &["check-run", &run_id])), 0);
    let docs = fleet.docs();
    let run_json = fs::read(run_dir.join("run.json")).expect("run.json");

    for (args, expect) in [
        (vec!["promote", run_id.as_str(), "--approved-by", ""], "maintainer name is empty"),
        (vec!["promote", run_id.as_str(), "--approved-by", "   "], "maintainer name is empty"),
        (vec!["promote", run_id.as_str(), "--approved-by", "Ken\n- approved by mallory"], "control character"),
        (vec!["promote", run_id.as_str(), "--approved-by", "Ken\tSnyder"], "control character"),
        (vec!["reject", run_id.as_str(), "--by", "Ken\n- rejected by mallory", "--reason", "duplicate"], "control character"),
        (vec!["reject", run_id.as_str(), "--by", "", "--reason", "duplicate"], "maintainer name is empty"),
        (vec!["reject", run_id.as_str(), "--by", " \t ", "--reason", "duplicate"], "maintainer name is empty"),
        (vec!["reject", run_id.as_str(), "--by", "maintainer", "--reason", ""], "decision reason is empty"),
        (vec!["reject", run_id.as_str(), "--by", "maintainer", "--reason", "  "], "decision reason is empty"),
    ] {
        let output = fleet.research(on, &args);
        assert_eq!(code(&output), 2, "{args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expect), "{args:?}: {stderr}");
        let mut json_args = args.clone();
        json_args.push("--json");
        let refused = fleet.research(on, &json_args);
        assert_eq!(code(&refused), 2, "{json_args:?}");
        assert!(json_out(&refused)["refused"].as_str().expect("refusal").contains(expect), "{json_args:?}");
        assert_eq!(fleet.docs(), docs, "{args:?}: nothing is published");
        assert_eq!(fs::read(run_dir.join("run.json")).expect("run.json"), run_json, "{args:?}: no decision is recorded");
    }

    let promoted = fleet.research(on, &["promote", &run_id, "--approved-by", "  Ken Snyder  ", "--json"]);
    assert_eq!(code(&promoted), 0, "{}", String::from_utf8_lossy(&promoted.stderr));
    let review = json_out(&promoted)["promoted"][0]["review"].as_str().expect("review path").to_string();
    let record: Value = serde_json::from_slice(&fs::read(fleet.path(&review)).expect("review record")).expect("json");
    assert_eq!(record["approval"]["by"], "Ken Snyder", "the approver is trimmed");
}

#[test]
fn cleanup_previews_by_default_and_deletes_only_with_apply() {
    let fleet = Fleet::published();
    let prepared = json_out(&fleet.research("2026-08-01", &["prepare", "slack", "--force", "--max-seconds", "60", "--max-invocations", "2", "--json"]));
    let run_id = prepared["runs"][0]["run_id"].as_str().expect("run id").to_string();
    let run_dir = prepared["runs"][0]["run_dir"].as_str().expect("run dir").to_string();
    // An active run is protected even when old.
    let plan = json_out(&fleet.research("2026-09-17", &["cleanup", "--json"]));
    assert_eq!(plan["plan"]["removals"], json!([]));
    assert_eq!(plan["plan"]["protected"][0]["reason"], "the run is active");

    fleet.research("2026-08-01", &["check-run", &run_id]);
    let docs = fleet.docs();
    let preview = fleet.research("2026-09-17", &["cleanup", "--json"]);
    assert_eq!(code(&preview), 0);
    let preview = json_out(&preview);
    assert_eq!(preview["applied"], false);
    assert_eq!(preview["plan"]["removals"][0]["path"], run_dir.as_str());
    assert_eq!(preview["plan"]["removals"][0]["loses_resumability"], true, "a failed run could still be resumed");
    assert_eq!(preview["plan"]["threshold_days"], 30);
    assert!(fleet.path(&run_dir).exists(), "a preview deletes nothing");
    let human = stdout(&fleet.research("2026-09-17", &["cleanup"]));
    assert!(human.contains("Nothing was deleted") && human.contains("can no longer be resumed"), "{human}");
    // A higher threshold keeps it.
    let kept = json_out(&fleet.research("2026-09-17", &["cleanup", "--older-than", "60", "--json"]));
    assert_eq!(kept["plan"]["removals"], json!([]));

    let applied = json_out(&fleet.research("2026-09-17", &["cleanup", "--apply", "--json"]));
    assert_eq!(applied["removed"], json!([run_dir.clone()]));
    assert!(!fleet.path(&run_dir).exists());
    assert_eq!(fleet.docs(), docs, "cleanup never touches accepted research");
}

/// The run directories under `runs/<platform>/`.
fn run_dirs(fleet: &Fleet, platform: PlatformId) -> usize {
    fs::read_dir(fleet.path(&format!("{STATE}/runs/{platform}"))).map_or(0, |entries| entries.count())
}

/// Review-1 regression: a corrupt `run.json` for an active run once let
/// `prepare` start a second run for the platform with a fresh budget.
#[test]
fn a_corrupt_active_run_record_blocks_prepare_and_names_its_path() {
    for (name, corrupt) in [("truncated", "{\n  \"format\": \"messenger-research-run/1\",\n  \"run_"), ("invalid JSON", "not json at all\n")] {
        let fleet = Fleet::published();
        let prepared = json_out(&fleet.research("2026-09-18", &["prepare", "discord", "--max-seconds", "600", "--max-invocations", "8", "--json"]));
        let run_dir = prepared["runs"][0]["run_dir"].as_str().expect("run dir").to_string();
        fleet.write(&format!("{run_dir}/run.json"), corrupt);

        let dry = fleet.research("2026-09-18", &["prepare", "discord", "--force", "--dry-run", "--json"]);
        assert_eq!(code(&dry), 0, "{name}");
        let skip = &json_out(&dry)["selections"][0]["skip"];
        assert_eq!(skip["reason"], "unreadable_run", "{name}: {skip}");
        assert_eq!(skip["path"], run_dir.as_str(), "{name}: portable repository-relative path");
        let human = stdout(&fleet.research("2026-09-18", &["prepare", "discord", "--force", "--dry-run"]));
        // Styled output wraps long paths; the JSON above pins the exact path.
        assert!(human.contains("discord: blocked") && human.contains("repair or remove"), "{name}: {human}");

        let again = fleet.research("2026-09-18", &["prepare", "discord", "--force", "--max-seconds", "600", "--max-invocations", "8", "--json"]);
        assert_eq!(code(&again), 0, "{name}");
        assert!(String::from_utf8_lossy(&again.stderr).contains(&run_dir), "{name}: a script reading only runs is warned");
        let again = json_out(&again);
        assert_eq!(again["runs"], json!([]), "{name}");
        assert_eq!(again["selections"][0]["skip"]["reason"], "unreadable_run", "{name}");
        assert_eq!(run_dirs(&fleet, PlatformId::Discord), 1, "{name}: no second run or ledger");

        // `runs --platform` still lists the unreadable record under its platform.
        let runs = json_out(&fleet.research("2026-09-18", &["runs", "--platform", "discord", "--json"]));
        assert_eq!(runs["runs"][0]["path"], run_dir.as_str(), "{name}: {runs}");
        assert_eq!(runs["runs"][0]["platform_id"], "discord", "{name}");
        assert!(runs["runs"][0]["error"].as_str().is_some_and(|error| error.contains("run.json")), "{name}: {runs}");
    }
}

/// A second `prepare` process is refused while another holds the preparation
/// lock, and the lock does not outlive its holder.
#[test]
fn prepare_is_refused_while_another_process_holds_the_prepare_lock() {
    let fleet = Fleet::published();
    let lock = fleet.path(&format!("{STATE}/runs/prepare.lock"));
    fs::create_dir_all(lock.parent().expect("runs dir")).expect("mkdir");
    let holder = fs::File::create(&lock).expect("lock file");
    holder.try_lock().expect("hold the lock");
    let args = ["prepare", "discord", "--force", "--max-seconds", "600", "--max-invocations", "8", "--json"];
    let refused = fleet.research("2026-09-18", &args);
    // Exit 3, like a held publication lock: the command could not run.
    assert_eq!(code(&refused), 3);
    let message = json_out(&refused)["error"].as_str().expect("error").to_string();
    assert!(message.contains(&format!("{STATE}/runs/prepare.lock")), "{message}");
    assert_eq!(run_dirs(&fleet, PlatformId::Discord), 0);
    drop(holder);
    assert_eq!(code(&fleet.research("2026-09-18", &args)), 0, "a released lock never wedges the platform");
    assert_eq!(run_dirs(&fleet, PlatformId::Discord), 1);
}

/// Review-1 regression: resuming a failed run after a newer run was prepared
/// for the platform once reopened it, leaving two open runs.
#[test]
fn resume_is_refused_while_another_run_for_the_platform_is_open() {
    let fleet = Fleet::published();
    let prepare = ["prepare", "discord", "--force", "--max-seconds", "600", "--max-invocations", "8", "--json"];
    let failed = json_out(&fleet.research("2026-09-18", &prepare))["runs"][0]["run_id"].as_str().expect("run id").to_string();
    assert_eq!(json_out(&fleet.research("2026-09-18", &["check-run", &failed, "--json"]))["status"], "failed");
    let newer = json_out(&fleet.research("2026-09-18", &prepare));
    let newer_id = newer["runs"][0]["run_id"].as_str().expect("run id").to_string();
    let newer_dir = newer["runs"][0]["run_dir"].as_str().expect("run dir").to_string();

    let refused = fleet.research("2026-09-18", &["prepare", "--resume", &failed, "--json"]);
    assert_eq!(code(&refused), 1, "a refused lifecycle step");
    let message = json_out(&refused)["refused"].as_str().expect("refusal").to_string();
    assert!(message.contains(&newer_dir) && message.contains(&newer_id), "{message}");
    let runs = json_out(&fleet.research("2026-09-18", &["runs", "--platform", "discord", "--json"]));
    let statuses: Vec<&str> = runs["runs"].as_array().expect("runs").iter().map(|row| row["status"].as_str().expect("status")).collect();
    assert_eq!(statuses.iter().filter(|status| **status == "active").count(), 1, "{runs}");
    assert!(statuses.contains(&"failed"), "{runs}");

    // Once the other run fails its check and is rejected, the failed run resumes.
    assert_eq!(code(&fleet.research("2026-09-18", &["check-run", &newer_id])), 1);
    assert_eq!(code(&fleet.research("2026-09-18", &["reject", &newer_id, "--by", "maintainer", "--reason", "superseded"])), 0);
    let resumed = fleet.research("2026-09-18", &["prepare", "--resume", &failed, "--json"]);
    assert_eq!(code(&resumed), 0, "{}", stdout(&resumed));
}

/// Repository roots holding characters that `sh`, `cmd`, or Claudine's
/// command tokenizer would interpret if a root were ever spliced into a
/// `shell:` step. Windows forbids `"` in file names, so it is tested only
/// elsewhere; a newline is tested where the file system allows it.
const HOSTILE_ROOTS: &[&str] = &[
    "with space",
    "single'quote",
    #[cfg(not(windows))]
    "double\"quote",
    "dollar$HOME",
    "back`id`tick",
    "percent %PATH%",
    "non-ASCII résumé 研究",
    #[cfg(not(windows))]
    "line\nbreak",
    #[cfg(not(windows))]
    "all of it \"'$(id)`id`%PATH% é",
    #[cfg(windows)]
    "all of it '$(id)`id`%PATH% é",
];

/// Runs a sequence `shell:` step the way Claudine does (`sh -c` on Unix,
/// `cmd /D /C` with the command as its raw tail on Windows) from `cwd`, with
/// this build's `messenger` first on `PATH`.
fn run_shell_step(command: &str, cwd: &Path) -> Output {
    let bin = biscuit_test_harness::bin_exe!("messenger");
    let mut dirs = vec![bin.parent().expect("binary directory").to_path_buf()];
    dirs.extend(std::env::var_os("PATH").map(|path| std::env::split_paths(&path).collect::<Vec<_>>()).unwrap_or_default());
    #[cfg(windows)]
    let mut shell = {
        use std::os::windows::process::CommandExt;
        let mut shell = Command::new("cmd");
        shell.args(["/D", "/C"]).raw_arg(command);
        shell
    };
    #[cfg(not(windows))]
    let mut shell = {
        let mut shell = Command::new("sh");
        shell.arg("-c").arg(command);
        shell
    };
    shell.current_dir(cwd).env("PATH", std::env::join_paths(dirs).expect("PATH")).env("NO_COLOR", "1").output().expect("run shell step")
}

#[test]
fn prepared_runs_name_no_host_path_and_their_checks_resolve_any_root() {
    for name in HOSTILE_ROOTS {
        let fleet = Fleet::unpublished_at(Some(name));
        let host = fleet.dir.path();
        let canonical = fs::canonicalize(host).expect("canonical temporary directory");
        let host_spellings: Vec<String> = [host.to_path_buf(), canonical]
            .iter()
            .flat_map(|path| {
                let text = path.to_string_lossy().into_owned();
                [text.replace('\\', "/"), text]
            })
            .chain(std::iter::once((*name).to_string()))
            .collect();
        let leaks = |label: &str, text: &str| {
            for spelling in &host_spellings {
                assert!(!text.contains(spelling.as_str()), "{name:?}: {label} names the host path {spelling:?}:\n{text}");
            }
        };

        let prepared = fleet.research("2026-09-18", &["prepare", "discord", "--max-seconds", "600", "--max-invocations", "8", "--json"]);
        assert_eq!(code(&prepared), 0, "{name:?}: {}", String::from_utf8_lossy(&prepared.stderr));
        leaks("prepare output", &stdout(&prepared));
        let prepared = json_out(&prepared);
        let run_id = prepared["runs"][0]["run_id"].as_str().expect("run id").to_string();
        let run_dir = fleet.path(prepared["runs"][0]["run_dir"].as_str().expect("run dir"));

        let mut files = vec![run_dir.clone()];
        let mut inputs = 0;
        while let Some(path) = files.pop() {
            if path.is_dir() {
                files.extend(fs::read_dir(&path).expect("dir").flatten().map(|entry| entry.path()));
            } else {
                inputs += 1;
                leaks(&path.display().to_string(), &String::from_utf8_lossy(&fs::read(&path).expect("read")));
            }
        }
        assert!(inputs > 10, "{name:?}: every pass's inputs were scanned ({inputs} files)");

        // The checks carry no path, so no shell or tokenizer can split or
        // expand one: only the run ID, the stage, and `--root .`.
        let sequence = fs::read_to_string(run_dir.join("run.md")).expect("run.md");
        let steps: Vec<&str> = sequence.lines().filter_map(|line| line.trim().strip_prefix("shell: ")).collect();
        assert_eq!(
            steps,
            [
                format!("messenger research check-run {run_id} --through validation --root ."),
                format!("messenger research check-run {run_id} --through review --root ."),
            ],
            "{name:?}"
        );

        // From anywhere but the root the step finds no run and changes nothing.
        let record = fleet.path(&format!("{STATE}/runs/discord/{run_id}/run.json"));
        let before = fs::read(&record).expect("run record");
        let elsewhere = run_shell_step(steps[0], host);
        assert_ne!(code(&elsewhere), 0, "{name:?}: {}", String::from_utf8_lossy(&elsewhere.stderr));
        assert_eq!(fs::read(&record).expect("run record"), before, "{name:?}: a step run outside the root touched the run");

        // From the root it judges this fixture's run: no outputs yet, so the
        // validation stage fails and the run is marked failed.
        let judged = run_shell_step(steps[0], fleet.root());
        assert_eq!(code(&judged), 1, "{name:?}: {}{}", stdout(&judged), String::from_utf8_lossy(&judged.stderr));
        let record: Value = serde_json::from_slice(&fs::read(&record).expect("run record")).expect("json");
        assert_eq!(record["status"], "failed", "{name:?}: {record}");
    }
}

/// Makes `dir` a Git work tree's top level, ignoring host Git configuration.
fn git_init(dir: &Path) {
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", dir.join("no-global-gitconfig"))
        .status()
        .expect("run git init");
    assert!(status.success(), "git init {}", dir.display());
}

const PREPARE: &[&str] = &["prepare", "discord", "--max-seconds", "600", "--max-invocations", "8", "--json"];

#[test]
fn prepare_refuses_a_root_below_the_git_top_level_and_accepts_the_top_level_or_no_repository() {
    // No repository anywhere above the root: allowed.
    let outside = Fleet::unpublished_at(Some("research"));
    let allowed = outside.research("2026-09-18", PREPARE);
    assert_eq!(code(&allowed), 0, "{}", String::from_utf8_lossy(&allowed.stderr));

    // A subdirectory of a work tree: refused as a usage error, nothing written.
    let below = Fleet::unpublished_at(Some("nested"));
    git_init(below.dir.path());
    let refused = below.research("2026-09-18", PREPARE);
    assert_eq!(code(&refused), 2, "{}{}", stdout(&refused), String::from_utf8_lossy(&refused.stderr));
    let refused = json_out(&refused);
    assert!(refused["refused"].as_str().expect("refusal").contains("is not its top level"), "{refused}");
    assert!(!below.path(STATE).exists(), "a refused preparation writes nothing");

    // The top level itself: allowed, whether the root is spelled as created
    // or canonically (macOS `/private/var`, Windows `\\?\`).
    let top = Fleet::unpublished_at(None);
    git_init(top.root());
    let allowed = top.research("2026-09-18", PREPARE);
    assert_eq!(code(&allowed), 0, "{}", String::from_utf8_lossy(&allowed.stderr));
    let canonical = fs::canonicalize(top.root()).expect("canonical root");
    let allowed = Command::new(biscuit_test_harness::bin_exe!("messenger"))
        .arg("research")
        .arg("--root")
        .arg(&canonical)
        .args(["--today", "2026-09-18", "prepare", "slack", "--max-seconds", "600", "--max-invocations", "8", "--json"])
        .env("NO_COLOR", "1")
        .output()
        .expect("run messenger");
    assert_eq!(code(&allowed), 0, "{}", String::from_utf8_lossy(&allowed.stderr));
}

#[test]
fn resume_refuses_a_root_that_became_a_subdirectory_of_a_git_work_tree() {
    let fleet = Fleet::unpublished_at(Some("nested"));
    let prepared = json_out(&fleet.research("2026-09-18", PREPARE));
    let run_id = prepared["runs"][0]["run_id"].as_str().expect("run id").to_string();
    assert_eq!(code(&fleet.research("2026-09-18", &["check-run", &run_id, "--through", "validation"])), 1, "the run fails");
    let record = fleet.path(&format!("{STATE}/runs/discord/{run_id}/run.json"));
    let before = fs::read(&record).expect("run record");

    git_init(fleet.dir.path());
    let refused = fleet.research("2026-09-18", &["prepare", "--resume", &run_id, "--json"]);
    assert_eq!(code(&refused), 2, "{}{}", stdout(&refused), String::from_utf8_lossy(&refused.stderr));
    assert!(json_out(&refused)["refused"].as_str().expect("refusal").contains("is not its top level"));
    assert_eq!(fs::read(&record).expect("run record"), before, "a refused resumption changes nothing");
}
