//! End-to-end `claudine-gen generate` UX over a doctored copy of the real
//! area: non-TTY report-only behavior, the unreconciled-drift exit code,
//! `--dry-run`, `--yes` writes, and re-convergence under `check`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use biscuit_test_harness::bin_exe;
use claudine_gen::CheckOutcome;
use darkmatter::markdown::compose::RequestSnapshot;

/// The real claudine package-area root.
fn real_area() -> &'static Path {
    static AREA: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    AREA.get_or_init(|| {
        biscuit_test_harness::manifest_dir!()
            .parent()
            .expect("gen crate lives under the claudine package area")
            .to_path_buf()
    })
}

/// A workspace-shaped fixture: the copied area lives at `<tmp>/claudine`
/// so the workspace-relative unchained-ai artifact resolves next to it.
struct Fixture {
    _dir: tempfile::TempDir,
    area: PathBuf,
}

impl Fixture {
    fn path(&self) -> &Path {
        &self.area
    }
}

/// Every topic whose research documents the per-provider generator reads.
const RESEARCH_TOPICS: [&str; 10] = [
    "acp",
    "agent-cli",
    "agent-errors",
    "agent-logging",
    "agent-models",
    "model-config",
    "non-interactive-sessions",
    "resume",
    "skills",
    "steering",
];

/// Copies the files of `rel` that `keep` accepts (by file name).
fn copy_dir_where(area: &Path, rel: &str, keep: impl Fn(&str) -> bool) {
    let from = real_area().join(rel);
    let to = area.join(rel);
    fs::create_dir_all(&to).unwrap();
    for entry in fs::read_dir(&from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if entry.file_type().unwrap().is_file() && keep(&name.to_string_lossy()) {
            fs::copy(entry.path(), to.join(name)).unwrap();
        }
    }
}

fn copy_area_file(area: &Path, rel: &str) {
    let to = area.join(rel);
    fs::create_dir_all(to.parent().unwrap()).unwrap();
    fs::copy(real_area().join(rel), to).unwrap();
}

/// Copies a workspace-relative file (a sibling of the area) next to the copy.
fn copy_workspace_file(workspace: &Path, rel: &str) {
    let to = workspace.join(rel);
    fs::create_dir_all(to.parent().unwrap()).unwrap();
    fs::copy(
        real_area()
            .parent()
            .expect("area lives under the workspace root")
            .join(rel),
        to,
    )
    .unwrap();
}

/// Copies every generator input AND committed output for all providers
/// into a tempdir area (full scope — catalog.json spans every provider).
fn full_fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let area = dir.path().join("claudine");
    let copy_dir = |rel: &str| copy_dir_where(&area, rel, |_| true);
    copy_area_file(&area, "docs/providers.yaml");
    copy_area_file(&area, "docs/providers/catalog.json");
    // Types every research contract shares (agent-cli imports them).
    copy_area_file(&area, "docs/research/_types.yaml");
    copy_dir("docs/providers/facts");
    copy_dir("docs/providers/overrides");
    for topic in RESEARCH_TOPICS {
        copy_dir(&format!("docs/research/{topic}"));
    }
    // Signals corpus + committed tables (docs only — fixture existence is
    // not checked at generate time, so fixtures/ stays uncopied).
    copy_dir("docs/research/signals");
    copy_area_file(&area, "lib/src/signals/generated.rs");
    copy_area_file(&area, "lib/src/model_catalog/families_generated.rs");
    copy_area_file(&area, "lib/src/stream/providers/vocabulary.rs");
    copy_area_file(&area, "docs/providers/steering-activation.yaml");
    copy_area_file(&area, "lib/src/steering/generated.rs");
    for slug in claudine_gen::provider_slugs() {
        copy_area_file(&area, &format!("lib/src/provider/{slug}/data.rs"));
    }
    copy_workspace_file(
        dir.path(),
        "darkmatter/lib/src/markdown/compose/expression/functions/agentic_cli_generated.rs",
    );
    copy_workspace_file(dir.path(), "unchained-ai/artifacts/models-catalog.json");
    Fixture { _dir: dir, area }
}

/// The inputs `validate <slug>` and a single-provider `check_area` read: the
/// roster, shared types, facts, overrides, and only `slug`'s research
/// documents plus its committed `data.rs`. One provider generates in about a
/// tenth of the time a `full_fixture` run takes.
fn single_provider_fixture(slug: &str) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let area = dir.path().join("claudine");
    let others = claudine_gen::provider_slugs();
    let keep = |name: &str| {
        let stem = name.split('.').next().unwrap_or(name);
        stem == slug || !others.contains(&stem)
    };
    copy_area_file(&area, "docs/providers.yaml");
    copy_area_file(&area, "docs/research/_types.yaml");
    copy_dir_where(&area, "docs/providers/facts", keep);
    copy_dir_where(&area, "docs/providers/overrides", keep);
    for topic in RESEARCH_TOPICS {
        copy_dir_where(&area, &format!("docs/research/{topic}"), keep);
    }
    copy_area_file(&area, &format!("lib/src/provider/{slug}/data.rs"));
    copy_workspace_file(dir.path(), "unchained-ai/artifacts/models-catalog.json");
    Fixture { _dir: dir, area }
}

/// Runs the built binary against `area` with a closed (non-TTY) stdin.
fn run_gen(area: &Path, args: &[&str]) -> Output {
    run_gen_env(area, args, &[])
}

/// Runs the built binary with extra environment variables (for color-mode
/// coverage). `stdin` is closed and stdout is captured (non-TTY).
fn run_gen_env(area: &Path, args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(bin_exe!("claudine-gen"));
    // Launched inside the real monorepo, the process snapshot would walk
    // every package of it on each run; the fixture is all the run reads.
    cmd.current_dir(area).arg("--area").arg(area).args(args);
    // Neutralize any ambient color-forcing so the base case is deterministic.
    cmd.env_remove("FORCE_COLOR")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("NO_COLOR");
    for (key, value) in envs {
        cmd.env(key, value);
    }
    cmd.stdin(std::process::Stdio::null())
        .output()
        .expect("claudine-gen binary runs")
}

fn data_path(area: &Path, slug: &str) -> PathBuf {
    area.join(format!("lib/src/provider/{slug}/data.rs"))
}

/// Doctors the claude override so both claude's data.rs and catalog.json
/// drift from their committed copies.
fn introduce_drift(area: &Path) {
    let path = area.join("docs/providers/overrides/claude.yaml");
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("CLAUDE_MODEL"), "fixture expectation drifted");
    fs::write(&path, text.replace("CLAUDE_MODEL", "DOCTORED_MODEL")).unwrap();
}

/// Compact snapshot of the human-facing clean report. Detail rows remain
/// covered by the focused report tests; the top-level lines pin provider and
/// full-scope artifact ordering without embedding a volatile wall of research
/// diagnostics.
#[test]
fn clean_check_report_summary_matches_phase_1_snapshot() {
    let fixture = full_fixture();
    let output = run_gen(fixture.path(), &["check"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("generator output must be UTF-8");
    // The models-catalog staleness WARNING is wall-clock dependent (the
    // fixture copies the real committed artifact, whose `generated_at` ages),
    // so it cannot appear in a fixed snapshot. Its trigger and wording are
    // pinned by the clock-parameterized unit tests in `artifact.rs`.
    let summary = stdout
        .lines()
        .filter(|line| !line.starts_with(' '))
        .filter(|line| !line.starts_with("WARNING: models-catalog artifact is stale"))
        .collect::<Vec<_>>()
        .join("\n");

    assert_eq!(
        summary,
        "claude: clean (inputs match the committed data.rs)\n\
codex: clean (inputs match the committed data.rs)\n\
gemini: clean (inputs match the committed data.rs)\n\
goose: clean (inputs match the committed data.rs)\n\
kimi: clean (inputs match the committed data.rs)\n\
opencode: clean (inputs match the committed data.rs)\n\
qwen: clean (inputs match the committed data.rs)\n\
kilo: clean (inputs match the committed data.rs)\n\
pi: clean (inputs match the committed data.rs)\n\
antigravity: clean (inputs match the committed data.rs)\n\
catalog.json: clean (inputs match the committed catalog)\n\
signals generated.rs: clean (inputs match the committed tables)\n\
stream vocabulary.rs: clean (inputs match the committed tables)\n\
steering generated.rs: clean (research and activation policy match the committed tables)\n\
darkmatter agentic_cli_generated.rs: clean (roster matches the committed has_agentic_cli names)\n\
families generated.rs: clean (26 family keys compiled)\n\
roster: every active entry has a wired Provider variant"
    );
}

/// A fresh fixture whose roster alias is renamed, drifting the Darkmatter
/// `has_agentic_cli` table.
fn fixture_with_renamed_roster_alias() -> Fixture {
    let fixture = full_fixture();
    let roster = fixture.path().join("docs/providers.yaml");
    let text = fs::read_to_string(&roster).unwrap();
    assert!(text.contains("\"kimi_code\""), "fixture expectation drifted");
    fs::write(&roster, text.replace("\"kimi_code\"", "\"kimi_cli\"")).unwrap();
    fixture
}

/// AC19 through the normal invocation path: renaming a roster alias drifts
/// the Darkmatter `has_agentic_cli` table, and `check` exits non-zero and
/// names it.
#[test]
fn roster_alias_rename_drifts_the_darkmatter_name_table() {
    let fixture = fixture_with_renamed_roster_alias();
    let output = run_gen(fixture.path(), &["check"]);
    assert!(!output.status.success(), "a stale name table must fail check");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("darkmatter agentic_cli_generated.rs: DRIFT"), "{stdout}");
}

/// `generate --yes` writes the renamed alias back and `check` then converges.
#[test]
fn roster_alias_rename_is_regenerated_until_check_converges() {
    let fixture = fixture_with_renamed_roster_alias();
    let output = run_gen(fixture.path(), &["generate", "--yes"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let table = fs::read_to_string(claudine_gen::agentic_clis_path(fixture.path())).unwrap();
    assert!(table.contains("(\"kimi_cli\", \"KimiCli\")") && !table.contains("\"kimi_code\""));
    assert!(run_gen(fixture.path(), &["check"]).status.success());
}

const PI_ADAPTER: &str = "adapters:\n  - { id: pi-rpc, revision: 1, provider: pi, mechanism_ids: [rpc-steer] }\n";

fn pi_grant(verification_id: &str) -> String {
    format!(
        "{PI_ADAPTER}grants:\n  - provider: pi\n    mechanism_id: rpc-steer\n    operation: steer_active_turn\n    \
         adapter: {{ id: pi-rpc, revision: 1 }}\n    profile_id: retained-rpc\n    os: macos\n    \
         provider_version: \"0.84.4\"\n    launch_mode: non_interactive\n    origin: native\n    \
         session_state: working\n    verification_ids: [{verification_id}]\nblocks: []\n"
    )
}

/// End to end over the shipped steering research and the normal binary: a
/// grant resting on an expected-loss record fails both `check` and
/// `generate` without writing.
#[test]
fn steering_activation_policy_refuses_an_expected_loss_grant() {
    let fixture = full_fixture();
    let policy = fixture.path().join("docs/providers/steering-activation.yaml");
    let generated = claudine_gen::steering_catalog_path(fixture.path());
    let committed = fs::read_to_string(&generated).unwrap();

    fs::write(&policy, pi_grant("pi-rpc-steer-eof-0844")).unwrap();
    for args in [&["check"][..], &["generate", "--yes"][..]] {
        let output = run_gen(fixture.path(), args);
        let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        assert!(!output.status.success(), "{args:?} must fail: {text}");
        assert!(text.contains("documents an expected loss and cannot activate delivery"), "{args:?}: {text}");
        assert_eq!(fs::read_to_string(&generated).unwrap(), committed, "{args:?} must not write");
    }
}

/// A grant over the passing fixture record drifts the committed table, is
/// written by `generate`, and then checks clean (read → write → read). The
/// drift and convergence reads call the library `check_steering_catalog`
/// (the same function `check` reports `steering generated.rs` from), so the
/// test pays for one ten-provider binary run instead of three.
#[test]
fn steering_activation_policy_grant_is_written_and_checks_clean() {
    let fixture = full_fixture();
    let policy = fixture.path().join("docs/providers/steering-activation.yaml");
    let generated = claudine_gen::steering_catalog_path(fixture.path());
    let snapshot = RequestSnapshot::new(fixture.path());

    fs::write(&policy, pi_grant("pi-rpc-steer-active-0844")).unwrap();
    let before = claudine_gen::check_steering_catalog(fixture.path(), &snapshot).unwrap();
    assert!(matches!(before, CheckOutcome::Drift { .. }), "{before:?}");

    let output = run_gen(fixture.path(), &["generate", "--yes"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let written = fs::read_to_string(&generated).unwrap();
    assert!(written.contains("AdapterRef { id: \"pi-rpc\", revision: 1 }"), "{written}");
    assert!(written.contains("verification_ids: &[\"pi-rpc-steer-active-0844\"]"), "{written}");
    let after = claudine_gen::check_steering_catalog(fixture.path(), &snapshot).unwrap();
    assert!(matches!(after, CheckOutcome::Clean), "{after:?}");
}

/// The `mapping` mode is machine-facing: pure JSON on stdout, never routed
/// through the terminal renderer, so it carries no ANSI or prose framing and
/// round-trips through a JSON parser.
#[test]
fn mapping_output_is_raw_parseable_json() {
    let fixture = full_fixture();
    let output = run_gen(fixture.path(), &["mapping"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("mapping output must be UTF-8");
    assert!(!stdout.contains('\u{1b}'), "machine JSON must carry no ANSI:\n{stdout}");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("mapping stdout must be valid JSON");
    assert_eq!(parsed["provider_scope"], "all");
    assert!(parsed["fields"].is_array());
}

// The three color-mode tests exercise `report::output_terminal` and the
// `<green>` markup every report line shares, through `validate codex`: it
// prints one such line after generating a single provider, where `check` would
// generate all ten. The `check` report text is pinned by the summary snapshot.

/// Piped (non-TTY) stdout degrades to plain text even though `COLORTERM`
/// would otherwise report truecolor: the report gates color on the output
/// stream, so a captured run carries no SGR.
#[test]
fn non_tty_report_output_has_no_ansi() {
    let fixture = single_provider_fixture("codex");
    let output = run_gen(fixture.path(), &["validate", "codex"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("report output must be UTF-8");
    assert!(!stdout.contains('\u{1b}'), "non-TTY output must be plain:\n{stdout}");
    assert!(stdout.contains("codex: the generator accepts every input"));
}

/// `NO_COLOR` forces plain output regardless of the output stream.
#[test]
fn no_color_report_output_has_no_ansi() {
    let fixture = single_provider_fixture("codex");
    let output = run_gen_env(fixture.path(), &["validate", "codex"], &[("NO_COLOR", "1")]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("report output must be UTF-8");
    assert!(!stdout.contains('\u{1b}'), "NO_COLOR must strip all SGR:\n{stdout}");
}

/// `FORCE_COLOR=1` emits SGR into a captured (non-TTY) stream, matching
/// `bt`'s escape hatch, while the visible status words are unchanged.
#[test]
fn force_color_report_output_carries_ansi() {
    let fixture = single_provider_fixture("codex");
    let output = run_gen_env(fixture.path(), &["validate", "codex"], &[("FORCE_COLOR", "1")]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("report output must be UTF-8");
    assert!(stdout.contains('\u{1b}'), "FORCE_COLOR must emit SGR");
    // The accepted-input keyword is styled; the surrounding prose is intact.
    assert!(stdout.contains("the generator accepts every input"));
    assert!(stdout.contains("codex: "));
}

#[test]
fn clean_area_generates_nothing_and_exits_zero() {
    let fixture = full_fixture();
    let output = run_gen(fixture.path(), &["generate"]);
    assert!(output.status.success(), "clean generate must exit zero");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("wrote "), "nothing to write:\n{stdout}");
}

/// Non-TTY stdin with drift: report-only, nothing written, exit non-zero
/// (the unreconciled-drift contract), and the decline → override snippet
/// pins the committed value.
#[test]
fn non_tty_drift_is_report_only_and_exits_nonzero() {
    let fixture = full_fixture();
    introduce_drift(fixture.path());
    let committed = fs::read_to_string(data_path(fixture.path(), "claude")).unwrap();

    let output = run_gen(fixture.path(), &["generate"]);
    assert!(!output.status.success(), "unreconciled drift must fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("report-only"), "{stdout}");
    assert!(stdout.contains("declined:"), "{stdout}");
    // The scaffolded override pins the committed catalog value.
    assert!(stdout.contains("overrides/claude.yaml"), "{stdout}");
    assert!(stdout.contains("CLAUDE_MODEL"), "{stdout}");
    assert!(stdout.contains("reason: TODO"), "{stdout}");
    // Report-only wrote nothing.
    assert_eq!(
        fs::read_to_string(data_path(fixture.path(), "claude")).unwrap(),
        committed
    );
}

#[test]
fn dry_run_reports_and_writes_nothing() {
    let fixture = full_fixture();
    introduce_drift(fixture.path());
    let committed = fs::read_to_string(data_path(fixture.path(), "claude")).unwrap();

    let output = run_gen(fixture.path(), &["generate", "--dry-run"]);
    assert!(!output.status.success());
    assert_eq!(
        fs::read_to_string(data_path(fixture.path(), "claude")).unwrap(),
        committed
    );
}

/// `--yes` writes every drifted file (data.rs + catalog.json) and the area
/// re-converges: a follow-up `check` is clean and exits zero.
#[test]
fn yes_writes_all_drift_and_check_reconverges() {
    let fixture = full_fixture();
    introduce_drift(fixture.path());

    let output = run_gen(fixture.path(), &["generate", "--yes"]);
    assert!(output.status.success(), "--yes writes must exit zero");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wrote "), "{stdout}");
    assert!(
        fs::read_to_string(data_path(fixture.path(), "claude"))
            .unwrap()
            .contains("DOCTORED_MODEL")
    );

    let check = run_gen(fixture.path(), &["check"]);
    let check_stdout = String::from_utf8_lossy(&check.stdout);
    assert!(check.status.success(), "{check_stdout}");
    assert!(check_stdout.contains("catalog.json: clean"), "{check_stdout}");
}

/// `--scaffold` on a slug with no wired `Provider` variant (kilo, whose
/// variant is intentionally unwired) fails at variant resolution — before
/// any file is written — with the "wire the enum variant first" message.
#[test]
fn scaffold_unwired_slug_errors_before_writing() {
    let fixture = full_fixture();
    // `nonesuch` is a permanently-fictional slug (kilo graduated to a wired
    // Provider; pi wires at M-Pi), so it exercises the variant gate stably.
    let output = run_gen(fixture.path(), &["generate", "nonesuch", "--scaffold"]);
    assert!(!output.status.success(), "unwired scaffold must fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no Provider variant is wired for slug `nonesuch`"), "{stderr}");
    // The failure precedes any write: the lib module stays absent.
    assert!(!fixture.path().join("lib/src/provider/nonesuch").exists());
}

/// `validate` answers one question for a research fleet: does the generator
/// accept this provider's inputs? Drift from the committed data.rs is the
/// expected state while research is being rewritten, so it is not a failure;
/// a refused input is, and its reason is on stdout.
#[test]
fn validate_accepts_drifted_inputs_and_reports_a_refusal_on_stdout() {
    const REVISION_TWO: &str = include_str!("../fixtures/agent-cli-r2/codex.md");
    let fixture = single_provider_fixture("codex");
    let doc = fixture.path().join("docs/research/agent-cli/codex.md");
    fs::write(&doc, REVISION_TWO).unwrap();

    // The single-provider library check is what `check codex` runs for the
    // provider line, without generating the other nine.
    let (_, drifted) =
        claudine_gen::check_area(fixture.path(), "codex", &RequestSnapshot::new(fixture.path())).unwrap();
    assert!(matches!(drifted, CheckOutcome::Drift { .. }), "a revision-2 document drifts codex's data.rs: {drifted:?}");

    let accepted = run_gen(fixture.path(), &["validate", "codex"]);
    let stdout = String::from_utf8(accepted.stdout).unwrap();
    assert!(accepted.status.success(), "drift is not a refusal: {stdout}");
    assert_eq!(stdout.trim(), "codex: the generator accepts every input");

    // One spelling claimed by two records where both apply: a relation the
    // shape check cannot see and the generator refuses.
    let conflicting = REVISION_TWO.replacen("  - flag: --oss\n", "  - flag: --oss\n    aliases: [-c]\n", 1);
    assert_ne!(conflicting, REVISION_TWO, "fixture expectation drifted");
    fs::write(&doc, conflicting).unwrap();
    let refused = run_gen(fixture.path(), &["validate", "codex"]);
    let stdout = String::from_utf8(refused.stdout).unwrap();
    assert!(!refused.status.success());
    assert!(
        stdout.starts_with("codex: the generator refuses an input:") && stdout.contains("`-c`"),
        "{stdout}"
    );
    assert_eq!(
        fs::read(data_path(fixture.path(), "codex")).unwrap(),
        fs::read(data_path(real_area(), "codex")).unwrap(),
        "validate writes nothing"
    );
}
