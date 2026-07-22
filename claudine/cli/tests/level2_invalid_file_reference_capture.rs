//! Level 2 real-terminal capture for the invalid-file-reference report.
//!
//! Drives `claudine compose` through a real tmux pane against a prompt whose
//! `iteration` frontmatter key references a MISSING file through
//! `frontmatter(spec, 'review_iterations')`. A present-but-unresolvable file
//! reference is authoring-fatal (real-errors ratified decision), so composition
//! aborts before the provider launches and renders the cause-driven report.
//!
//! Asserts the user-visible report reaches the rendered terminal surface: the
//! root-cause "invalid file path" headline (NOT the mechanism word
//! "transform"/"interpolation"), the receiving `iteration` key, the focused
//! excerpt surfacing the involved keys (`spec:` and `iteration:`) while
//! excluding the unrelated `agent:` key, an OSC8 link to the prompt file plus
//! the filename in plain text, the `$schema` structural parent when the document
//! declares one, the "Did you mean?" suggestion when a near-miss sibling exists,
//! and styling in `raw`.
//!
//! This mirrors the Darkmatter `md compose` capture
//! (`darkmatter/cli/tests/level2_errors.rs`) so the same failure renders
//! identically through both binaries, per the real-errors spec.

#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use test_toolkit::{Level, require_level};

mod common;
use common::{
    TestWorkspace, assert_row_is_styled, augmented_path, clear_no_color, write_executable,
};

// `iteration` evaluates `frontmatter(spec, …)` against a file that does not
// exist; the present-but-unresolvable reference is authoring-fatal. `spec` is a
// sibling key the expression references, so the focused excerpt surfaces both
// `spec` and the receiving `iteration` key. `agent` is unrelated and excluded.
// Top-level keys (not nested under `$schema`) are used because `$schema` values
// are type specifications, not interpolation expressions.
const INVALID_FILE_REFERENCE_DOC: &str = "\
---
agent: \"codex\"
spec: \"does-not-exist-spec.md\"
iteration: \"{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') : 1 }}\"
---
# Body
";

// Same authoring-fatal reference shape as INVALID_FILE_REFERENCE_DOC, but `spec`
// points at `spec.md` and a near-miss sibling `specs.md` is written on disk next
// to the prompt. The reference (`spec.md`) is one edit from `specs.md`, well
// within the file-suggestion threshold, so the rendered report appends a
// "Did you mean?" section naming the real candidate. This exercises the
// suggestion path at Level 2 — the no-sibling fixture above uses a name with no
// near-miss neighbor, so the live "did you mean" output is not asserted there.
const INVALID_FILE_REFERENCE_WITH_SIBLING_DOC: &str = "\
---
agent: \"codex\"
spec: \"spec.md\"
iteration: \"{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') : 1 }}\"
---
# Body
";

// Real on-disk near-miss sibling for the reference `spec.md` above. Levenshtein
// distance 1 (`spec.md` → `specs.md`), so it is guaranteed to be suggested.
const SIBLING_CANDIDATE_NAME: &str = "specs.md";

// Stale-directory reference fixture (the motivating real-errors case): the
// prompt references `features/2026-06-21-opencode-log-fix/spec.md`, but the
// real sibling under the nearest existing ancestor (`features/`) is
// `features/2026-06-28-real-errors/spec.md`. The render-time suggestion logic
// walks up from the missing parent to `features/`, enumerates sibling
// directories, requires `spec.md` to exist inside a candidate, and ranks by
// directory-name similarity — so the rendered report should append a
// "Did you mean?" section naming the relative path `2026-06-28-real-errors/spec.md`.
const STALE_DIRECTORY_REFERENCE_DOC: &str = "\
---
agent: \"codex\"
spec: \"features/2026-06-21-opencode-log-fix/spec.md\"
iteration: \"{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') : 1 }}\"
---
# Body
";

// Real on-disk sibling (relative to the workspace root) for the stale-directory
// reference above. The dated folder name is dissimilar enough from the missing
// segment that the strict filename gate would reject it; leaf-existence
// (`spec.md` inside the folder) is the quality signal that lets the suggestion
// through.
const STALE_DIRECTORY_FIXTURE_REL: &str = "features/2026-06-28-real-errors/spec.md";

// Rendered form of the suggestion (relative to the nearest existing ancestor).
const STALE_DIRECTORY_SUGGESTION: &str = "2026-06-28-real-errors/spec.md";

// Same top-level failing interpolation (`iteration` referencing `spec`), but the
// document ALSO declares a `$schema:` block typing `spec`/`iteration`. Frontmatter
// interpolation runs BEFORE schema validation in the compose pipeline, so the
// top-level `iteration` reference fails with the file-reference error first — the
// `$schema:` type specs never get a chance to raise a competing schema error.
// Because the `$schema:` block is present in the source, the focused excerpt
// unions the involved keys' `$schema`-nested paths and surfaces the `$schema`
// structural parent alongside the top-level keys (real-errors spec, lines 54/172).
const INVALID_FILE_REFERENCE_WITH_SCHEMA_DOC: &str = "\
---
agent: \"codex\"
$schema:
    spec: 'string(required)'
    iteration: 'string(required)'
spec: \"does-not-exist-spec.md\"
iteration: \"{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') : 1 }}\"
---
# Body
";

struct Staged {
    workspace: TestWorkspace,
    bin_dir: PathBuf,
    doc: PathBuf,
    launch_count: PathBuf,
}

/// Stage a workspace whose prompt holds `doc_body`, named `doc_name`.
///
/// When `sibling` is `Some((name, body))`, that file is also written next to the
/// prompt so the render-time "Did you mean?" similarity search (which scans the
/// prompt's directory) can find the near-miss candidate.
fn stage_with(
    name: &str,
    doc_name: &str,
    doc_body: &str,
    sibling: Option<(&str, &str)>,
) -> Staged {
    let workspace = TestWorkspace::named(name);
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let claudine_dir = workspace.path().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();

    let launch_count = workspace.path().join("provider-launched.txt");
    write_provider_stub(&bin_dir);

    let doc = workspace.path().join(doc_name);
    fs::write(&doc, doc_body).unwrap();

    if let Some((sibling_name, sibling_body)) = sibling {
        fs::write(workspace.path().join(sibling_name), sibling_body).unwrap();
    }

    Staged {
        workspace,
        bin_dir,
        doc,
        launch_count,
    }
}

fn stage(name: &str) -> Staged {
    stage_with(name, "plan.md", INVALID_FILE_REFERENCE_DOC, None)
}

/// Like [`stage_with`] but stages multiple sibling files at arbitrary paths
/// relative to the workspace root (creating parent directories as needed) and
/// no top-level sibling. Used to exercise the stale-directory "Did you mean?"
/// suggestion path, which walks up from a missing parent directory to the
/// nearest existing ancestor — so nested sibling fixtures must exist on disk
/// during the capture.
fn stage_with_nested_siblings(
    name: &str,
    doc_name: &str,
    doc_body: &str,
    siblings: &[(&str, &str)],
) -> Staged {
    let staged = stage_with(name, doc_name, doc_body, None);
    for (rel, body) in siblings {
        let target = staged.workspace.path().join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&target, body).unwrap();
    }
    staged
}

fn write_provider_stub(bin_dir: &Path) {
    let script = "#!/bin/sh\nprintf launched >> \"$CLAUDINE_PROVIDER_LAUNCH_COUNT\"\nexit 0\n";
    write_executable(&bin_dir.join("goose"), script);
}

fn wait_for_pane_marker(
    harness: &mut TmuxHarness,
    marker: &str,
    deadline: Duration,
) -> CapturedFrame {
    let stop = Instant::now() + deadline;
    loop {
        let frame = harness.capture().expect("capture pane");
        if frame.plain.contains(marker) {
            return frame;
        }
        if Instant::now() >= stop {
            panic!(
                "marker {marker:?} did not appear within {deadline:?}.\nplain:\n{}",
                frame.plain
            );
        }
        harness.settle();
    }
}

fn capture_command(harness: &mut TmuxHarness, staged: &Staged) -> CapturedFrame {
    // This fixture runs under `FORCE_COLOR=1`, which an ambient `NO_COLOR` would
    // out-vote — see `common::clear_no_color`.
    clear_no_color(harness);

    let claudine = cargo_bin!("claudine").display().to_string();
    let home = staged.workspace.path().to_string_lossy().into_owned();
    let path = augmented_path(&staged.bin_dir);
    let path = path.to_string_lossy().into_owned();
    let launch_count = staged.launch_count.to_string_lossy().into_owned();

    harness.send_text(b"clear\n").expect("clear pane");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    harness
        .send_text(format!("cd {}\n", staged.workspace.path().display()).as_bytes())
        .expect("cd into workspace");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    let cmd = format!("{claudine} compose --goose {}", staged.doc.display());
    harness
        .send_command_with_env(
            &cmd,
            &[
                ("HOME", home.as_str()),
                ("PATH", path.as_str()),
                ("FORCE_COLOR", "1"),
                ("COLUMNS", "100"),
                ("CLAUDINE_PROVIDER_LAUNCH_COUNT", launch_count.as_str()),
            ],
        )
        .expect("send claudine command");

    let frame = wait_for_pane_marker(harness, "invalid file path", Duration::from_secs(15));
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    frame
}

#[test]
#[serial(level2_terminal)]
fn level2_invalid_file_reference_renders_headline_excerpt_and_osc8_link_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-invalid-file-ref-l2");
    let frame = capture_command(&mut harness, &staged);

    // Root-cause headline — the mechanism word "interpolation"/"transform" must
    // NOT be the headline; the cause ("invalid file path") is.
    assert!(
        frame.plain.contains("invalid file path"),
        "expected root-cause headline 'invalid file path'. plain:\n{}",
        frame.plain
    );

    // Names the receiving frontmatter key (the bare key appears in the scope
    // sentence, e.g. "The iteration frontmatter property").
    assert!(
        frame.plain.contains("iteration"),
        "invalid-file-reference report missing the receiving key 'iteration'.\nplain:\n{}",
        frame.plain
    );

    // Focused excerpt: assert the FOCUSED nature, not merely that the bare words
    // appear. The YAML excerpt renders each involved key as a `key:` line, so
    // `spec:` and `iteration:` (with the trailing colon) prove the focused
    // excerpt LINES are present — distinct from the bare-key scope sentence,
    // which has no colon. `spec:` is the referenced sibling; `iteration:` is the
    // receiving key.
    for needle in ["spec:", "iteration:"] {
        assert!(
            frame.plain.contains(needle),
            "invalid-file-reference report missing focused excerpt line {needle:?}.\nplain:\n{}",
            frame.plain
        );
    }

    // The unrelated `agent` key MUST be excluded from the focused excerpt. The
    // fixture has `agent: "codex"`, but the focused slice surfaces only the
    // involved keys (`spec`/`iteration`). Assert on the literal `agent:` YAML
    // line (the `key:` shape the gutter excerpt renders) rather than the bare
    // word "agent", which can legitimately appear elsewhere (e.g. provider
    // naming). The only way `agent:` reaches this report is the whole-block
    // fallback excerpt — the report's scope sentence and body never emit a
    // colon-suffixed `agent`, so asserting its absence over the captured frame
    // is a precise, non-flaky check that the slice stayed focused.
    assert!(
        !frame.plain.contains("agent:"),
        "focused excerpt leaked the unrelated `agent:` YAML line.\nplain:\n{}",
        frame.plain
    );

    // OSC8 hyperlink to the prompt file. macOS aliases `/var` to `/private/var`;
    // the binary embeds whichever spelling reaches it, so accept either form.
    let canonical = staged.doc.canonicalize().expect("canonicalize prompt path");
    let canonical_url = format!("file://{}", canonical.to_string_lossy());
    let aliased_url = canonical_url.replacen("file:///private", "file://", 1);
    let osc8_canonical = format!("\x1b]8;;{}", canonical_url);
    let osc8_aliased = format!("\x1b]8;;{}", aliased_url);
    assert!(
        frame.raw.contains(&osc8_canonical) || frame.raw.contains(&osc8_aliased),
        "expected OSC8 hyperlink for {canonical_url} (or aliased {aliased_url}).\nraw:\n{}",
        frame.raw
    );
    assert!(
        frame.plain.contains("plan.md"),
        "expected the linked prompt filename in plain text.\nplain:\n{}",
        frame.plain
    );

    assert_row_is_styled(
        &frame.raw,
        "invalid file path",
        "invalid-file-reference report",
    );
    assert!(
        !staged.launch_count.exists(),
        "invalid file reference should fail before provider launch"
    );
}

/// Level 2 capture for the file-reference "Did you mean?" suggestion path: drives
/// `claudine compose` against a fixture that references a missing `spec.md` while a
/// real near-miss sibling (`specs.md`) sits next to the prompt, captures the live
/// pane, and verifies the rendered report appends the suggestions section naming
/// the candidate. Mirrors the Darkmatter
/// `level2_invalid_file_reference_renders_did_you_mean_suggestion` capture so the
/// suggestion text is proven identical through both binaries.
#[test]
#[serial(level2_terminal)]
fn level2_invalid_file_reference_renders_did_you_mean_suggestion_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage_with(
        "claudine-invalid-file-ref-suggest-l2",
        "plan.md",
        INVALID_FILE_REFERENCE_WITH_SIBLING_DOC,
        Some((SIBLING_CANDIDATE_NAME, "siblings: []\n")),
    );
    let frame = capture_command(&mut harness, &staged);

    // Same authoring-fatal headline as the no-sibling case — the suggestion is an
    // addition to the report, not a replacement for the root cause.
    assert!(
        frame.plain.contains("invalid file path"),
        "expected root-cause headline 'invalid file path'.\nplain:\n{}",
        frame.plain
    );

    // The "Did you mean?" section header survives to the rendered pane.
    assert!(
        frame.plain.contains("Did you mean?"),
        "expected the 'Did you mean?' suggestions section.\nplain:\n{}",
        frame.plain
    );

    // The real near-miss sibling is named as a candidate.
    assert!(
        frame.plain.contains(SIBLING_CANDIDATE_NAME),
        "expected the candidate filename {SIBLING_CANDIDATE_NAME:?} in the suggestions.\nplain:\n{}",
        frame.plain
    );

    assert_row_is_styled(&frame.raw, "invalid file path", "suggestion report");
    assert!(
        !staged.launch_count.exists(),
        "invalid file reference should fail before provider launch"
    );
}

/// Level 2 capture for the file-reference "Did you mean?" suggestion on the
/// STALE-DIRECTORY shape: drives `claudine compose` against a fixture whose
/// `spec:` references `features/2026-06-21-opencode-log-fix/spec.md` (a stale
/// dated directory) while the real sibling `features/2026-06-28-real-errors/spec.md`
/// lives under the nearest existing ancestor (`features/`). Captures the live
/// tmux pane and verifies the rendered report appends a "Did you mean?" section
/// naming the suggested relative path `2026-06-28-real-errors/spec.md`. Mirrors
/// the Darkmatter `level2_stale_directory_reference_renders_did_you_mean_suggestion`
/// capture so the suggestion is proven identical through both binaries
/// (real-errors spec requirement).
#[test]
#[serial(level2_terminal)]
fn level2_stale_directory_reference_renders_did_you_mean_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage_with_nested_siblings(
        "claudine-stale-dir-ref-l2",
        "plan.md",
        STALE_DIRECTORY_REFERENCE_DOC,
        &[(STALE_DIRECTORY_FIXTURE_REL, "siblings: []\n")],
    );
    let frame = capture_command(&mut harness, &staged);

    // Same authoring-fatal headline — the suggestion is an addition to the
    // report, not a replacement for the root cause.
    assert!(
        frame.plain.contains("invalid file path"),
        "expected root-cause headline 'invalid file path'.\nplain:\n{}",
        frame.plain
    );

    // The "Did you mean?" section header survives to the rendered pane.
    assert!(
        frame.plain.contains("Did you mean?"),
        "expected the 'Did you mean?' suggestions section.\nplain:\n{}",
        frame.plain
    );

    // The relative path of the real sibling (sibling_dir/leaf) is named as a
    // candidate.
    assert!(
        frame.plain.contains(STALE_DIRECTORY_SUGGESTION),
        "expected the candidate relative path {STALE_DIRECTORY_SUGGESTION:?} in the suggestions.\nplain:\n{}",
        frame.plain
    );

    assert_row_is_styled(&frame.raw, "invalid file path", "suggestion report");
    assert!(
        !staged.launch_count.exists(),
        "invalid file reference should fail before provider launch"
    );
}

/// Level 2 capture proving the focused excerpt surfaces the `$schema` structural
/// parent when the document declares one (real-errors spec, lines 54/172): drives
/// `claudine compose` against a fixture whose top-level `iteration` interpolation
/// fails (file reference) AND whose `$schema:` block types `spec`/`iteration`.
/// Frontmatter interpolation runs before schema validation, so the file-reference
/// error wins; because the `$schema:` block is present, the focused excerpt unions
/// the involved keys' `$schema`-nested paths and shows the `$schema:` parent.
#[test]
#[serial(level2_terminal)]
fn level2_invalid_file_reference_excerpt_includes_schema_parent_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage_with(
        "claudine-invalid-file-ref-schema-l2",
        "plan.md",
        INVALID_FILE_REFERENCE_WITH_SCHEMA_DOC,
        None,
    );
    let frame = capture_command(&mut harness, &staged);

    // Root cause still wins over the `$schema` type specs (interpolation runs
    // before schema validation in the compose pipeline).
    assert!(
        frame.plain.contains("invalid file path"),
        "expected root-cause headline 'invalid file path'.\nplain:\n{}",
        frame.plain
    );

    // The focused excerpt surfaces the `$schema` structural parent plus the
    // involved keys. `$schema:` proves the parent block is shown; `spec:` and
    // `iteration:` are the involved keys (present both top-level and under the
    // `$schema` parent — the excerpt unions whichever paths resolve).
    for needle in ["$schema:", "spec:", "iteration:"] {
        assert!(
            frame.plain.contains(needle),
            "focused excerpt missing {needle:?}.\nplain:\n{}",
            frame.plain
        );
    }

    assert_row_is_styled(&frame.raw, "invalid file path", "schema-parent report");
    assert!(
        !staged.launch_count.exists(),
        "invalid file reference should fail before provider launch"
    );
}
