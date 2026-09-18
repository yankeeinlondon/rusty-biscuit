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
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
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
}

impl Fleet {
    fn published() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        for path in SHIPPED {
            let target = dir.path().join(path);
            fs::create_dir_all(target.parent().expect("parent")).expect("mkdir");
            fs::copy(repo_root().join(path), target).expect("copy");
        }
        let fleet = Self { dir };
        for platform in PlatformId::ALL {
            fleet.write(&format!("messenger/docs/research/platforms/{platform}.md"), &fixture(*platform, None));
        }
        let loader = Loader::new(Workspace::new(fleet.root()).expect("absolute"));
        generate(&loader, &BTreeMap::new(), &Date::parse("2026-09-17").expect("date"), Options::default()).expect("publish");
        fleet
    }

    fn root(&self) -> &Path {
        self.dir.path()
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
