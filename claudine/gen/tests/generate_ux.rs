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

/// Copies every generator input AND committed output for all providers
/// into a tempdir area (full scope — catalog.json spans every provider).
fn full_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let real = real_area();
    let copy_dir = |rel: &str| {
        let from = real.join(rel);
        let to = dir.path().join(rel);
        fs::create_dir_all(&to).unwrap();
        for entry in fs::read_dir(&from).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
            }
        }
    };
    let copy_file = |rel: &str| {
        let to = dir.path().join(rel);
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
        "agent-logging",
        "agent-models",
        "non-interactive-sessions",
        "resume",
        "skills",
    ] {
        copy_dir(&format!("docs/research/{topic}"));
    }
    for slug in claudine_gen::PROVIDER_SLUGS {
        copy_file(&format!("lib/src/provider/{slug}/data.rs"));
    }
    dir
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
