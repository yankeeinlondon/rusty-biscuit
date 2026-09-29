//! Level 1 PTY tests for provided-partial `file`/`file[]` resolution.
//!
//! Phase 3 of `fixes/2026-06-30-completion-failures`. When a `compose`
//! invocation supplies a value for a `file`/`file[]` schema property that
//! does not resolve to a literal path, Claudine treats the value as a
//! **partial**: it walks the property's `match(...)` glob from the launch
//! area, filters candidates by the provided substring (case-insensitive),
//! and — finding exactly one — shows a confirmation dialog. On `y`,
//! composition proceeds with the resolved path.
//!
//! These tests drive that flow through a pseudo-terminal:
//!
//! - A single glob+substring match reaches the `Use this file? (Y/n)`
//!   confirmation dialog and, on `y`, launches the provider stub.
//! - Zero glob+substring matches preserve the original
//!   `no existing file matched reference` error and never launch the
//!   provider.
//! - Scalar string values for `file[]` properties are normalized to a
//!   single-element array before resolution.
//! - Root-union schemas (`2026-09-27-union-partial-file-completion`) reach the
//!   same dialog or chooser, including the D1 shape whose arms each declare
//!   the property over a different tree. Every failed completion (zero
//!   candidates, declined, `Ctrl-C`) is reported before the provider picker
//!   and before `initialize`, and the file dialog always precedes the picker.
//!
//! ## Tier
//!
//! **Level 1**, and gating mirrors `level1_schema_prompt_pty.rs`:
//! `#![cfg(unix)]` is the only exclusion, because `expectrl`'s `OsSession` is
//! Unix-only. On a selected platform
//! `expect_level!(Level::L1, pty_available(), ...)` **fails** when the PTY is
//! missing rather than skipping: Level 1 is the mandatory suite, where a skip
//! is indistinguishable from a pass. `expectrl` opens `/dev/ptmx` and the test
//! manufactures every byte the child reads; no terminal emulator participates.
//!
//! Run via the canonical recipe:
//!
//! ```text
//! just test
//! ```

use expectrl::Session;
use expectrl::session::OsSession;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use test_toolkit::{Level, expect_level};

use crate::common;
use common::pty::*;
use common::{CliProcessFixture, pty_available, write_executable};

/// Seed a workspace whose only `**/*spec*.md` files are two specs, exactly
/// one of which carries `everywhere` in its path.
fn seed_specs(root: &Path) {
    let target = root.join("features/2026-06-30-style-everywhere/spec.md");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, "---\ntitle: Everywhere\n---\nSpec body.\n").unwrap();
    let other = root.join("features/2026-06-01-other/spec.md");
    fs::create_dir_all(other.parent().unwrap()).unwrap();
    fs::write(&other, "---\ntitle: Other\n---\nSpec body.\n").unwrap();
}

/// Build a `claudine compose --goose <plan> <property>=<partial>` command
/// anchored at the fixture's launch area (the directory the glob walks) with
/// `HOME` inside the fixture so `prompt_for_missing` reads its default (`true`).
fn compose_command(
    fixture: &CliProcessFixture,
    md_file: &Path,
    property: &str,
    partial: &str,
) -> Command {
    stage_default_config(fixture.home());
    // `expectrl` needs a live `std::process::Command`; the builder's raw
    // surface hands one over carrying the same policy.
    let mut cmd = fixture.command_std();
    cmd.args([
        "compose",
        "--goose",
        md_file.to_str().unwrap(),
        &format!("{property}={partial}"),
    ]);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    cmd
}

fn plan_with_file_schema(root: &Path) -> PathBuf {
    let md_file = root.join("plan.md");
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "$schema:\n",
            "  spec: 'file(required;match(**/*spec*.md);eager)'\n",
            "---\n",
            "Plan body.\n",
        ),
    )
    .unwrap();
    md_file
}

fn plan_with_file_array_schema(root: &Path) -> PathBuf {
    let md_file = root.join("plan.md");
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "$schema:\n",
            "  attachments: 'file(required;match(**/*spec*.md);eager)[]'\n",
            "---\n",
            "Plan body.\n",
        ),
    )
    .unwrap();
    md_file
}

/// A fixture with the two specs seeded, a `goose` stub staged, and the marker
/// path the stub writes when it is launched.
fn staged_fixture() -> (CliProcessFixture, PathBuf) {
    let fixture = CliProcessFixture::named("level1-provided-partial-pty");
    let marker = fixture.cwd().join("launched.flag");
    stage_goose_stub(fixture.bin_dir(), &marker);
    seed_specs(fixture.cwd());
    (fixture, marker)
}

/// Answer the `Use this file? (Y/n)` dialog with `y` and drain until the stub
/// records its launch, returning the accumulated transcript.
///
/// `confirm_one_file` enables raw mode via crossterm directly (not the
/// `run_standalone` path), so it emits no raw-mode marker byte — nothing at all
/// between flushing the dialog and blocking on the key read. The line
/// discipline is the observable condition instead; see
/// [`wait_for_raw_mode_termios`].
fn confirm_and_drain(session: &mut OsSession, marker: &Path, seed: String) -> String {
    wait_for_raw_mode_termios(session, Duration::from_secs(10));
    session.write_all(b"y").expect("confirm file selection");
    session.flush().ok();

    let stop = Instant::now() + Duration::from_secs(15);
    let mut transcript = seed;
    while Instant::now() < stop {
        if marker.exists() {
            break;
        }
        transcript.push_str(&read_for(session, Duration::from_millis(200)));
    }
    transcript
}

#[test]
fn level1_pty_provided_partial_single_match_confirms_and_launches() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = plan_with_file_schema(fixture.cwd());

    let cmd = compose_command(&fixture, &md_file, "spec", "everywhere");
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    // A single glob+substring match drives the confirmation dialog, whose
    // trailer is `Use this file? (Y/n)`.
    let pre = wait_for_marker(&mut session, "Use this file", Duration::from_secs(10));

    // The provider must NOT have launched before the dialog is answered.
    assert!(
        !marker.exists(),
        "provider launched before the confirmation dialog was answered; \
         transcript so far:\n{}",
        common::strip_ansi(&pre)
    );

    let transcript = confirm_and_drain(&mut session, &marker, pre);

    assert!(
        marker.exists(),
        "provider stub should have launched after the confirmation dialog \
         resolved `everywhere` to the one matching spec.\ntranscript:\n{}",
        common::strip_ansi(&transcript)
    );
}

#[test]
fn level1_pty_provided_partial_zero_match_preserves_error() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = plan_with_file_schema(fixture.cwd());

    // No spec path contains `no-such-partial`, so the glob+substring filter
    // yields zero candidates and the original error is preserved unchanged.
    let cmd = compose_command(&fixture, &md_file, "spec", "no-such-partial");
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let transcript = read_for(&mut session, Duration::from_secs(8));
    let plain = common::strip_ansi(&transcript);

    assert!(
        !marker.exists(),
        "provider must NOT launch when the partial matches no candidate; \
         transcript:\n{plain}"
    );
    assert!(
        plain.contains("no existing file matched reference"),
        "expected the original file-reference failure text to be preserved; \
         transcript:\n{plain}"
    );
}

#[test]
fn level1_pty_provided_partial_file_array_scalar_confirms_and_launches() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = plan_with_file_array_schema(fixture.cwd());

    // A scalar string provided for a `file[]` property is normalized to a
    // single-element array and treated as a partial.
    let cmd = compose_command(&fixture, &md_file, "attachments", "everywhere");
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "Use this file", Duration::from_secs(10));

    assert!(
        !marker.exists(),
        "provider launched before the confirmation dialog was answered; \
         transcript so far:\n{}",
        common::strip_ansi(&pre)
    );

    let transcript = confirm_and_drain(&mut session, &marker, pre);

    assert!(
        marker.exists(),
        "provider stub should have launched after the file[] confirmation dialog \
         resolved `everywhere`.\ntranscript:\n{}",
        common::strip_ansi(&transcript)
    );
}

#[test]
fn level1_pty_provided_partial_file_array_array_confirms_and_launches() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = plan_with_file_array_schema(fixture.cwd());

    // An explicit JSON array value is also accepted as a `file[]` partial.
    let cmd = compose_command(&fixture, &md_file, "attachments", "[\"everywhere\"]");
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "Use this file", Duration::from_secs(10));

    assert!(
        !marker.exists(),
        "provider launched before the confirmation dialog was answered; \
         transcript so far:\n{}",
        common::strip_ansi(&pre)
    );

    let transcript = confirm_and_drain(&mut session, &marker, pre);

    assert!(
        marker.exists(),
        "provider stub should have launched after the file[] confirmation dialog \
         resolved `[\"everywhere\"]`.\ntranscript:\n{}",
        common::strip_ansi(&transcript)
    );
}

/// A root-union plan in the shape of `prompts/clarify.md`: both arms declare
/// `doc: file`, the frontmatter fills `doc` from a template over the arms'
/// discriminants, and the document authors a shell-free `initialize` stack.
///
/// The template makes `doc` a `file`-typed value that is only decidable after
/// composition; arm selection must not rule an arm out on it
/// (`2026-09-27-union-partial-file-completion`, C1).
fn plan_with_union_templated_sibling(root: &Path) -> PathBuf {
    let md_file = root.join("plan.md");
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "$schema:\n",
            "  - spec: 'file(required;match(**/*spec*.md);eager)'\n",
            "    doc: file\n",
            "  - design: 'file(required;match(**/*design*.md))'\n",
            "    doc: file\n",
            "doc: \"{{spec || design}}\"\n",
            "initialize:\n",
            "  stack:\n",
            "    - action: {append_line: [\"events.log\", \"initialize\"]}\n",
            "---\n",
            "Spec document: {{spec}}\n",
        ),
    )
    .unwrap();
    md_file
}

/// Stage a `goose` stub that records its argv, and its stdin when stdin is not
/// the terminal, so the test can see which path the composed prompt carried.
fn stage_recording_goose_stub(bin_dir: &Path, marker_file: &Path) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > {marker}\n[ -t 0 ] || cat >> {marker}\nexit 0\n",
            marker = marker_file.display()
        ),
    );
}

#[test]
fn union_partial_with_templated_file_sibling_reaches_chooser() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let fixture = CliProcessFixture::named("level1-provided-partial-union-pty");
    let marker = fixture.cwd().join("launched.flag");
    stage_recording_goose_stub(fixture.bin_dir(), &marker);
    seed_specs(fixture.cwd());
    let md_file = plan_with_union_templated_sibling(fixture.cwd());

    let cmd = compose_command(&fixture, &md_file, "spec", "everywhere");
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    // Exactly one `**/*spec*.md` path contains `everywhere`, so the union's
    // `spec` arm must drive the single-match confirmation dialog.
    let pre = wait_for_marker(&mut session, "Use this file", Duration::from_secs(10));
    assert!(
        !marker.exists(),
        "provider launched before the confirmation dialog was answered; \
         transcript so far:\n{}",
        common::strip_ansi(&pre)
    );

    let transcript = confirm_and_drain(&mut session, &marker, pre);
    let plain = common::strip_ansi(&transcript);
    assert!(
        marker.exists(),
        "provider stub should have launched after the union's `spec` arm \
         resolved `everywhere`.\ntranscript:\n{plain}"
    );
    assert!(
        !plain.contains("no existing file matched reference"),
        "the resolved partial must not fail validation; transcript:\n{plain}"
    );
    let launched_with = fs::read_to_string(&marker).expect("read stub record");
    assert!(
        launched_with.contains("features/2026-06-30-style-everywhere/spec.md"),
        "the composed prompt should carry the chosen spec path; stub \
         saw:\n{launched_with}\ntranscript:\n{plain}"
    );
}

/// `claudine compose <plan> <args…>` with no provider flag, so a run that gets
/// past its inputs opens the provider picker.
fn compose_without_provider(fixture: &CliProcessFixture, md_file: &Path, args: &[&str]) -> Command {
    stage_default_config(fixture.home());
    // `expectrl` needs a live `std::process::Command`; the builder's raw
    // surface hands one over carrying the same policy.
    let mut cmd = fixture.command_std();
    cmd.arg("compose").arg(md_file).args(args);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    cmd
}

/// The R7.1 union fixture with `goose` (recording) and `claude` stubs on
/// `PATH`, so the provider picker has two options. Returns the goose and
/// claude markers.
fn union_fixture_with_two_providers(label: &str) -> (CliProcessFixture, PathBuf, PathBuf) {
    let fixture = CliProcessFixture::named(label);
    let goose = fixture.cwd().join("goose.flag");
    let claude = fixture.cwd().join("claude.flag");
    stage_recording_goose_stub(fixture.bin_dir(), &goose);
    write_executable(
        &fixture.bin_dir().join("claude"),
        &format!("#!/bin/sh\necho 'launched' > {}\nexit 0\n", claude.display()),
    );
    seed_specs(fixture.cwd());
    (fixture, goose, claude)
}

/// Drain the session until `done` holds or `deadline` passes.
fn drain_until(session: &mut OsSession, mut transcript: String, deadline: Duration, done: impl Fn() -> bool) -> String {
    let stop = Instant::now() + deadline;
    while Instant::now() < stop && !done() {
        transcript.push_str(&read_for(session, Duration::from_millis(200)));
    }
    transcript
}

/// The provider picker's observable traces: its raw-mode entry, and the
/// option label no other prompt in these runs prints.
fn assert_picker_never_rendered(transcript: &str, label: &str) {
    let plain = common::strip_ansi(transcript);
    assert!(
        !transcript.contains(KBD_ENHANCEMENT_PUSH),
        "{label}: the provider picker must not open; transcript:\n{plain}"
    );
    assert!(
        !plain.contains("Goose"),
        "{label}: the provider picker's options must not render; transcript:\n{plain}"
    );
}

/// R7.2: two `**/*spec*.md` paths contain the partial, so the chooser opens;
/// the path the user picks is the one the provider receives.
#[test]
fn union_partial_with_two_matches_opens_the_chooser_and_launches_the_pick() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let fixture = CliProcessFixture::named("level1-provided-partial-union-chooser");
    // A repository makes the chooser's labels repository-relative, short
    // enough to read in the PTY's default width.
    fixture.initialize_repository();
    let marker = fixture.cwd().join("launched.flag");
    stage_recording_goose_stub(fixture.bin_dir(), &marker);
    seed_specs(fixture.cwd());
    let second = fixture.cwd().join("features/everywhere-else/spec.md");
    fs::create_dir_all(second.parent().unwrap()).unwrap();
    fs::write(&second, "---\ntitle: Everywhere else\n---\nSpec body.\n").unwrap();
    let md_file = plan_with_union_templated_sibling(fixture.cwd());

    let cmd = compose_command(&fixture, &md_file, "spec", "everywhere");
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "Enter=Submit", Duration::from_secs(10));
    let pre = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));
    let plain = common::strip_ansi(&pre);
    // The list pane truncates the longer seeded label, so match its prefix.
    assert!(
        plain.contains("features/2026-06-30-style-every")
            && plain.contains("features/everywhere-else/spec.md"),
        "the chooser should list both matches; transcript:\n{plain}"
    );
    assert!(!plain.contains("Use this file"), "two matches need the chooser; transcript:\n{plain}");
    assert!(!marker.exists(), "provider launched before the chooser was answered:\n{plain}");

    // The chooser lists the matches in path order; `j` moves to the second.
    session.write_all(b"j\r").expect("pick the second match");
    session.flush().ok();
    let transcript = drain_until(&mut session, pre, Duration::from_secs(15), || marker.exists());
    let plain = common::strip_ansi(&transcript);

    let launched_with = fs::read_to_string(&marker)
        .unwrap_or_else(|_| panic!("the stub should launch; transcript:\n{plain}"));
    assert!(
        launched_with.contains("features/everywhere-else/spec.md")
            && !launched_with.contains("2026-06-30-style-everywhere"),
        "the provider should receive the picked path; stub saw:\n{launched_with}\ntranscript:\n{plain}"
    );
    assert!(fixture.cwd().join("events.log").exists(), "initialize should run after the pick");
}

/// R2/R3 (R7.3): with zero candidates, the failure prints before the provider
/// picker, which never opens; `initialize` never runs.
#[test]
fn union_partial_with_zero_matches_fails_before_the_provider_picker() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, goose, claude) = union_fixture_with_two_providers("level1-union-zero-before-picker");
    let md_file = plan_with_union_templated_sibling(fixture.cwd());

    let cmd = compose_without_provider(&fixture, &md_file, &["spec=no-such-partial"]);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");
    // `read_for` returns at end of output, when the failed run exits.
    let transcript = read_for(&mut session, Duration::from_secs(10));
    let plain = common::strip_ansi(&transcript);

    assert!(
        plain.contains("no existing file matched reference `no-such-partial`"),
        "the unresolved reference should be reported; transcript:\n{plain}"
    );
    assert_picker_never_rendered(&transcript, "zero candidates");
    assert!(!goose.exists() && !claude.exists(), "no provider may launch:\n{plain}");
    assert!(!fixture.cwd().join("events.log").exists(), "initialize must not run:\n{plain}");
}

/// R3 (R7.3): with one candidate, the file dialog comes before the provider
/// picker, and the confirmed path reaches the provider the user then picks.
#[test]
fn union_partial_file_dialog_renders_before_the_provider_picker() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, goose, claude) = union_fixture_with_two_providers("level1-union-dialog-before-picker");
    let md_file = plan_with_union_templated_sibling(fixture.cwd());

    let cmd = compose_without_provider(&fixture, &md_file, &["spec=everywhere"]);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "Use this file", Duration::from_secs(10));
    assert_picker_never_rendered(&pre, "before the file dialog is answered");
    wait_for_raw_mode_termios(&mut session, Duration::from_secs(10));
    session.write_all(b"y").expect("confirm the file");
    session.flush().ok();

    // Only now does the picker open.
    let mut transcript = pre.clone();
    transcript.push_str(&wait_for_marker(&mut session, "Goose", Duration::from_secs(10)));
    let transcript = wait_for_raw_mode(&mut session, transcript, Duration::from_secs(10));
    let dialog_at = transcript.find("Use this file").expect("dialog rendered");
    let picker_at = transcript.find(KBD_ENHANCEMENT_PUSH).expect("picker entered raw mode");
    assert!(dialog_at < picker_at, "the file dialog must precede the picker");

    // Claude is the picker's default; `j` moves to Goose.
    session.write_all(b"j\r").expect("select Goose");
    session.flush().ok();
    let transcript = drain_until(&mut session, transcript, Duration::from_secs(15), || goose.exists());
    let plain = common::strip_ansi(&transcript);

    let launched_with = fs::read_to_string(&goose)
        .unwrap_or_else(|_| panic!("goose should launch; transcript:\n{plain}"));
    assert!(
        launched_with.contains("features/2026-06-30-style-everywhere/spec.md"),
        "the confirmed path should reach the provider; stub saw:\n{launched_with}"
    );
    assert!(!claude.exists(), "the unselected provider must not launch:\n{plain}");
}

/// R2 (R7.3): declining the single-file dialog, or cancelling it with
/// `Ctrl-C`, fails before the provider picker opens and before `initialize`.
#[test]
fn union_partial_declined_or_cancelled_fails_before_the_provider_picker() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    // `n` declines. `Ctrl-C` arrives as a key in raw mode, not as a signal,
    // so the dialog must treat it as a cancellation rather than wait on.
    for (label, key) in [("declined", b"n".as_slice()), ("cancelled", b"\x03".as_slice())] {
        let (fixture, goose, claude) =
            union_fixture_with_two_providers(&format!("level1-union-{label}-before-picker"));
        let md_file = plan_with_union_templated_sibling(fixture.cwd());

        let cmd = compose_without_provider(&fixture, &md_file, &["spec=everywhere"]);
        let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");
        let pre = wait_for_marker(&mut session, "Use this file", Duration::from_secs(10));
        wait_for_raw_mode_termios(&mut session, Duration::from_secs(10));
        session.write_all(key).expect("answer the file dialog");
        session.flush().ok();

        // `read_for` returns at end of output, when the failed run exits.
        let transcript = pre + &read_for(&mut session, Duration::from_secs(10));
        let plain = common::strip_ansi(&transcript);
        assert!(
            plain.contains("no existing file matched reference `everywhere`"),
            "{label}: the unresolved reference should be reported; transcript:\n{plain}"
        );
        assert_picker_never_rendered(&transcript, label);
        assert!(!goose.exists() && !claude.exists(), "{label}: no provider may launch:\n{plain}");
        assert!(
            !fixture.cwd().join("events.log").exists(),
            "{label}: initialize must not run:\n{plain}"
        );
    }
}

/// The D1 prompt body. `count` is authored as a number; the `features` arm
/// types it `string` and the `fixes` arm `number`, so the composed text shows
/// which arm coerced it.
const FIX_ARM_BODY: &str = "count: 5\n---\nSpec document: {{spec}}\nCount is a number: {{ is_number(count) }}\n";

/// R7.4 (ruling D1): arms discriminated by an optional `kind`, each declaring
/// `spec` as an eager `file(match)` over its own tree. The chooser searches
/// both trees, and picking the `fixes` spec composes under the `fixes` arm.
///
/// With `initialize` the early supplied-file pass offers the chooser (the D1
/// fallback); without it the pre-validator's union classification does. Both
/// surfaces must agree.
///
/// Both arms accept the instance, so only the picked path's glob can select
/// the `fixes` arm; its `number` typing of `count` is visible in the prompt
/// the provider receives.
#[test]
fn d1_union_chooser_lists_both_trees_and_the_fixes_pick_composes() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let fixture = CliProcessFixture::named("level1-provided-partial-d1");
    // A repository makes the chooser's labels repository-relative, short
    // enough to read in the PTY's default width.
    fixture.initialize_repository();
    let marker = fixture.cwd().join("launched.flag");
    stage_recording_goose_stub(fixture.bin_dir(), &marker);
    for spec in ["features/cli-colors/spec.md", "fixes/cli-switches/spec.md"] {
        let path = fixture.cwd().join(spec);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "---\ntitle: Spec\n---\nSpec body.\n").unwrap();
    }
    let schema = concat!(
        "$schema:\n",
        "  - kind: 'literal(feature)'\n",
        "    spec: 'file(required;eager;match(**/features/**/spec.md))'\n",
        "    count: 'string'\n",
        "  - kind: 'literal(fix)'\n",
        "    spec: 'file(required;eager;match(**/fixes/**/spec.md))'\n",
        "    count: 'number'\n",
    );
    let initialize = "initialize:\n  stack:\n    - action: {append_line: [\"events.log\", \"initialize\"]}\n";

    for (label, extra) in [("with initialize", initialize), ("without initialize", "")] {
        let md_file = fixture.cwd().join("plan.md");
        fs::write(&md_file, format!("---\n{schema}{extra}{FIX_ARM_BODY}")).unwrap();
        let _ = fs::remove_file(&marker);

        let cmd = compose_command(&fixture, &md_file, "spec", "cli");
        let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

        let pre = wait_for_marker(&mut session, "Enter=Submit", Duration::from_secs(10));
        let pre = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));
        let plain = common::strip_ansi(&pre);
        assert!(
            plain.contains("features/cli-colors/spec.md") && plain.contains("fixes/cli-switches/spec.md"),
            "{label}: the chooser should list a match from each tree; transcript:\n{plain}"
        );
        assert!(!marker.exists(), "{label}: provider launched before the pick:\n{plain}");

        // Path order puts `features/…` first; `j` moves to `fixes/…`.
        session.write_all(b"j\r").expect("pick the fixes spec");
        session.flush().ok();
        let transcript = drain_until(&mut session, pre, Duration::from_secs(15), || marker.exists());
        let plain = common::strip_ansi(&transcript);

        let launched_with = fs::read_to_string(&marker).unwrap_or_else(|_| {
            panic!("{label}: the fixes arm should validate and launch; transcript:\n{plain}")
        });
        assert!(
            launched_with.contains("fixes/cli-switches/spec.md")
                && launched_with.contains("Count is a number: true"),
            "{label}: the provider should receive the fixes spec under the fixes arm; stub saw:\n{launched_with}"
        );
        assert!(!plain.contains("schema validation"), "{label}: composition should validate:\n{plain}");
    }
}

/// The D1 union again, but with the prompt in `prompts/`, no `initialize`,
/// inside a Git repository: the chooser's pick resolves from the launch
/// directory rather than the prompt's, its glob selects the `fixes` arm
/// (visible in how `count` is coerced), and the run composes and launches.
#[test]
fn d1_union_chooser_pick_composes_for_a_prompt_outside_the_launch_directory() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let fixture = CliProcessFixture::named("level1-provided-partial-d1-prompts-dir");
    fixture.initialize_repository();
    let marker = fixture.cwd().join("launched.flag");
    stage_recording_goose_stub(fixture.bin_dir(), &marker);
    for spec in ["features/cli-colors/spec.md", "fixes/cli-switches/spec.md"] {
        let path = fixture.cwd().join(spec);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "---\ntitle: Spec\n---\nSpec body.\n").unwrap();
    }
    let md_file = fixture.cwd().join("prompts/plan.md");
    fs::create_dir_all(md_file.parent().unwrap()).unwrap();
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "$schema:\n",
            "  - kind: 'literal(feature)'\n",
            "    spec: 'file(required;eager;match(**/features/**/spec.md))'\n",
            "    count: 'string'\n",
            "  - kind: 'literal(fix)'\n",
            "    spec: 'file(required;eager;match(**/fixes/**/spec.md))'\n",
            "    count: 'number'\n",
        ).to_string()
            + FIX_ARM_BODY,
    )
    .unwrap();

    let cmd = compose_command(&fixture, &md_file, "spec", "cli");
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "Enter=Submit", Duration::from_secs(10));
    let pre = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));
    let plain = common::strip_ansi(&pre);
    assert!(
        plain.contains("features/cli-colors/spec.md") && plain.contains("fixes/cli-switches/spec.md"),
        "the chooser should list a match from each tree; transcript:\n{plain}"
    );

    // Path order puts `features/…` first; `j` moves to `fixes/…`.
    session.write_all(b"j\r").expect("pick the fixes spec");
    session.flush().ok();
    let transcript = drain_until(&mut session, pre, Duration::from_secs(15), || marker.exists());
    let plain = common::strip_ansi(&transcript);

    let launched_with = fs::read_to_string(&marker).unwrap_or_else(|_| {
        panic!("the picked spec should compose and launch; transcript:\n{plain}")
    });
    assert!(
        launched_with.contains("fixes/cli-switches/spec.md")
            && launched_with.contains("Count is a number: true"),
        "the provider should receive the fixes spec under the fixes arm; stub saw:\n{launched_with}"
    );
    assert!(!plain.contains("schema validation"), "composition should validate:\n{plain}");
}
