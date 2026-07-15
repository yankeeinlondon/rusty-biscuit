//! End-to-end `claudine-gen generate` UX over a doctored copy of the real
//! area: non-TTY report-only behavior, the unreconciled-drift exit code,
//! `--dry-run`, `--yes` writes, and re-convergence under `check`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The real claudine package-area root.
fn real_area() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("gen crate lives under the claudine package area")
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

/// Copies every generator input AND committed output for all providers
/// into a tempdir area (full scope — catalog.json spans every provider).
fn full_fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let area = dir.path().join("claudine");
    let real = real_area();
    let copy_dir = |rel: &str| {
        let from = real.join(rel);
        let to = area.join(rel);
        fs::create_dir_all(&to).unwrap();
        for entry in fs::read_dir(&from).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
            }
        }
    };
    let copy_file = |rel: &str| {
        let to = area.join(rel);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(real.join(rel), to).unwrap();
    };
    copy_file("docs/providers.yaml");
    copy_file("docs/providers/catalog.json");
    copy_dir("docs/providers/facts");
    copy_dir("docs/providers/overrides");
    for topic in [
        "acp",
        "agent-cli",
        "agent-errors",
        "agent-logging",
        "agent-models",
        "model-config",
        "non-interactive-sessions",
        "resume",
        "skills",
    ] {
        copy_dir(&format!("docs/research/{topic}"));
    }
    // Signals corpus + committed tables (docs only — fixture existence is
    // not checked at generate time, so fixtures/ stays uncopied).
    copy_dir("docs/research/signals");
    copy_file("lib/src/signals/generated.rs");
    copy_file("lib/src/model_catalog/families_generated.rs");
    copy_file("lib/src/stream/providers/vocabulary.rs");
    for slug in claudine_gen::provider_slugs() {
        copy_file(&format!("lib/src/provider/{slug}/data.rs"));
    }
    let artifact_rel = "unchained-ai/artifacts/models-catalog.json";
    let artifact_to = dir.path().join(artifact_rel);
    fs::create_dir_all(artifact_to.parent().unwrap()).unwrap();
    fs::copy(
        real.parent()
            .expect("area lives under the workspace root")
            .join(artifact_rel),
        artifact_to,
    )
    .unwrap();
    Fixture { _dir: dir, area }
}

/// Runs the built binary against `area` with a closed (non-TTY) stdin.
fn run_gen(area: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_claudine-gen"))
        .arg("--area")
        .arg(area)
        .args(args)
        .stdin(std::process::Stdio::null())
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
    let summary = stdout
        .lines()
        .filter(|line| !line.starts_with(' '))
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
families generated.rs: clean (26 family keys compiled)\n\
roster: every active entry has a wired Provider variant"
    );
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
