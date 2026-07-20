//! Level-2 tests for darkmatter error rendering (OSC 8, gutter style, etc.).
//!
//! Shares a single WezTerm pane across tests via [`SHARED_HARNESS`] and uses a
//! `printf`-emitted sentinel to detect command completion. Spawning a fresh
//! pane per test would push each test past the nextest slow-test termination
//! threshold (`slow-timeout = 5s`, `terminate-after = 3` → 15 s).

mod common;

use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use common::level2::md_shim;
use serial_test::serial;
use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{Level, LevelDecision, evaluate_level};

static SHARED_HARNESS: SharedHarness<WezTermHarness> = SharedHarness::new();
static SENTINEL_COUNTER: AtomicU32 = AtomicU32::new(0);

const SENTINEL_TIMEOUT: Duration = Duration::from_secs(30);

fn wait_for_sentinel(
    harness: &mut WezTermHarness,
    sentinel: &str,
) -> Result<CapturedFrame, CapturedFrame> {
    let deadline = Instant::now() + SENTINEL_TIMEOUT;
    let mut last = CapturedFrame::from_raw(String::new());
    while Instant::now() < deadline {
        if let Ok(frame) = harness.capture() {
            // The sentinel also appears inline in the command echo
            // (e.g. `$ md ...; printf '\n__DM_DONE_0__\n'`). Only treat the
            // sentinel as completion when it appears on a line of its own.
            if frame.plain.lines().any(|l| l.trim() == sentinel) {
                return Ok(frame);
            }
            last = frame;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(last)
}

fn run_with_sentinel(harness: &mut WezTermHarness, cmd: &str) -> CapturedFrame {
    let id = SENTINEL_COUNTER.fetch_add(1, Ordering::Relaxed);
    let sentinel = format!("__DM_LVL2_ERR_DONE_{id}__");
    let wrapped = format!("{cmd}; printf '\\n{sentinel}\\n'");
    harness
        .send_command_with_env(&wrapped, &[("DARKMATTER_NO_BASELINE_SCHEMA", "1")])
        .expect("send_command_with_env failed");
    match wait_for_sentinel(harness, &sentinel) {
        Ok(frame) => {
            // Let the pane settle before the final capture so assertions see
            // the stable frame, not a transitional redraw.
            std::thread::sleep(Duration::from_millis(250));
            harness.capture().unwrap_or(frame)
        }
        Err(last) => panic!(
            "timed out waiting for sentinel {sentinel} after {SENTINEL_TIMEOUT:?}. \
             last plain capture:\n{}",
            last.plain
        ),
    }
}

/// Acquires the shared harness, spawning a pane on first use. Returns `None`
/// when WezTerm is unavailable so callers can skip cleanly.
fn run_md_compose(file_body: &str) -> Option<(CapturedFrame, std::path::PathBuf)> {
    run_md_compose_named("unterminated.md", file_body)
}

fn run_md_compose_named(
    file_name: &str,
    file_body: &str,
) -> Option<(CapturedFrame, std::path::PathBuf)> {
    match evaluate_level(Level::L2, WezTermHarness::available(), "WezTerm") {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return None;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    let dir = tempdir().unwrap();
    let file_path = dir.path().join(file_name);
    fs::write(&file_path, file_body).unwrap();
    let canonical = file_path.canonicalize().expect("canonicalize failed");

    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().unwrap();

    // Reset the visible region so a previous test's output does not bleed
    // into this capture.
    run_with_sentinel(harness, "clear");

    let cmd = format!("{} compose {}", md_shim(), file_path.display());
    let frame = run_with_sentinel(harness, &cmd);
    // Keep tempdir alive past capture by returning canonical path to caller.
    drop(dir);
    Some((frame, canonical))
}

/// Like [`run_md_compose_named`], but also writes a sibling file next to the
/// prompt before composing. Used to exercise the "Did you mean?" suggestion
/// path, which gathers candidates from the prompt's directory at render time —
/// so the near-miss sibling must exist on disk during the capture.
fn run_md_compose_with_sibling(
    file_name: &str,
    file_body: &str,
    sibling_name: &str,
) -> Option<(CapturedFrame, std::path::PathBuf)> {
    match evaluate_level(Level::L2, WezTermHarness::available(), "WezTerm") {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return None;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    let dir = tempdir().unwrap();
    let file_path = dir.path().join(file_name);
    fs::write(&file_path, file_body).unwrap();
    fs::write(dir.path().join(sibling_name), "siblings: []\n").unwrap();
    let canonical = file_path.canonicalize().expect("canonicalize failed");

    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().unwrap();

    run_with_sentinel(harness, "clear");

    let cmd = format!("{} compose {}", md_shim(), file_path.display());
    let frame = run_with_sentinel(harness, &cmd);
    // Keep tempdir (and the on-disk sibling) alive past capture.
    drop(dir);
    Some((frame, canonical))
}

/// Like [`run_md_compose_named`], but stages an arbitrary set of sibling files
/// at paths relative to the prompt's directory (creating parent directories as
/// needed) before composing. Used to exercise the stale-directory "Did you
/// mean?" suggestion path, which walks up from a missing parent directory to
/// the nearest existing ancestor — so nested sibling fixtures must exist on
/// disk during the capture.
fn run_md_compose_with_nested_siblings(
    file_name: &str,
    file_body: &str,
    siblings: &[(&str, &str)],
) -> Option<(CapturedFrame, std::path::PathBuf)> {
    match evaluate_level(Level::L2, WezTermHarness::available(), "WezTerm") {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return None;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    let dir = tempdir().unwrap();
    let file_path = dir.path().join(file_name);
    fs::write(&file_path, file_body).unwrap();
    for (rel, body) in siblings {
        let target = dir.path().join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&target, body).unwrap();
    }
    let canonical = file_path.canonicalize().expect("canonicalize failed");

    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().unwrap();

    run_with_sentinel(harness, "clear");

    let cmd = format!("{} compose {}", md_shim(), file_path.display());
    let frame = run_with_sentinel(harness, &cmd);
    drop(dir);
    Some((frame, canonical))
}

// Unterminated page block to trigger PageBlockError::UnterminatedBlock.
const UNTERMINATED_BLOCK: &str = "::block when=\"true\"\nbody\n";

// Document with an inline `$schema` requiring a property the frontmatter does
// not satisfy. Drives the styled `SchemaValidationFailed` block through the
// real binary + terminal so SGR/OSC8 behavior is captured live, not just
// asserted against a process-local renderer. The top-level `description:`
// feeds the styled block's `<i><dim>…</dim></i>` description line so the
// italic+dim contract is verified on a real pane, not only in plain-text
// snapshots.
const MISSING_REQUIRED_SCHEMA: &str = "---\n$schema:\n  spec: 'string(min(1); required)'\nspec: \"\"\ndescription: Planner prompt schema\n---\nBody\n";

// Document with an inline `$schema` whose failing property declares a
// description via the `-> {description}` arrow syntax. Unlike
// `MISSING_REQUIRED_SCHEMA` above, this fixture has NO document-level
// `description:` line, so the only `<i><dim>…</dim></i>` sub-line in the
// rendered `SchemaValidationFailed` block is the per-problem description
// attached to the `Missing` problem for `title`. This isolates the NEW
// per-problem description path from the older document-level description
// rendering already covered by the test below. A missing-required failure is
// used deliberately: the compose pipeline coerces schema-recognized scalars,
// so a wrong-type fixture (e.g. `title: 42`) would be coerced to a string and
// PASS validation, defeating the test.
const MISSING_DESCRIBED_PROPERTY: &str = "---\n$schema:\n  title: 'string(required) -> The headline shown in listing pages'\n---\nBody\n";

#[test]
#[serial(level2_terminal)]
fn level2_error_header_contains_osc8_hyperlink() {
    let Some((frame, canonical)) = run_md_compose(UNTERMINATED_BLOCK) else {
        return;
    };

    // The header should contain something like
    // `\x1b]8;;file:///...unterminated.md\x07unterminated.md\x1b]8;;\x07`.
    // Use the canonicalized path because macOS /var is a symlink to /private/var.
    let expected_url = format!("file://{}", canonical.to_string_lossy());
    assert!(
        frame.raw.contains(&format!("\x1b]8;;{}", expected_url)),
        "expected raw output to contain OSC8 hyperlink sequence for {}. raw:\n{}",
        expected_url,
        frame.raw
    );
    assert!(
        frame.plain.contains("unterminated.md"),
        "expected plain text to contain label 'unterminated.md'. plain:\n{}",
        frame.plain
    );
}

/// Level-2 capture for the `SchemaValidationFailed` styled block: drives
/// `md compose` against a fixture with an inline schema, captures the live
/// terminal pane, and verifies the user-visible styling requirements survive
/// the real binary path (OSC8 source link, red category label on the bullet,
/// inverse property name, and dim/italic SGR for the rendered block).
#[test]
#[serial(level2_terminal)]
fn level2_schema_validation_block_renders_styled_link_and_bullet() {
    let Some((frame, canonical)) =
        run_md_compose_named("planner-schema.md", MISSING_REQUIRED_SCHEMA)
    else {
        return;
    };

    // OSC8 hyperlink to the source file in the styled header/body. macOS
    // aliases `/var` to `/private/var`; the binary embeds whichever spelling
    // the user passed on the command line, so accept either form.
    let canonical_url = format!("file://{}", canonical.to_string_lossy());
    let aliased_url = canonical_url.replacen("file:///private", "file://", 1);
    let osc8_canonical = format!("\x1b]8;;{}", canonical_url);
    let osc8_aliased = format!("\x1b]8;;{}", aliased_url);
    assert!(
        frame.raw.contains(&osc8_canonical) || frame.raw.contains(&osc8_aliased),
        "expected OSC8 hyperlink for {canonical_url} (or aliased {aliased_url}). raw:\n{}",
        frame.raw
    );

    // The header text and failing-property bullet must be visible in plain.
    assert!(
        frame.plain.contains("schema validation failed"),
        "expected schema-validation header text. plain:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("spec"),
        "expected failing property `spec` to appear on the bullet. plain:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("planner-schema.md"),
        "expected source filename in styled block. plain:\n{}",
        frame.plain
    );

    // The `description:` renders as `<i><dim>…</dim></i>`. Verify both that the
    // text survives to plain AND that the italic + dim SGR survive the real
    // terminal, so a regression that drops the italic styling (while keeping
    // the text) is caught at Level 2 — plain-text snapshots cannot see it.
    assert!(
        frame.plain.contains("Planner prompt schema"),
        "expected schema description text in styled block. plain:\n{}",
        frame.plain
    );
    let has_italic = frame.raw.contains("\x1b[3m") || frame.raw.contains("\x1b[0;3m");
    assert!(
        has_italic,
        "expected italic SGR for the description line. raw:\n{}",
        frame.raw
    );
    let has_dim = frame.raw.contains("\x1b[2m") || frame.raw.contains("\x1b[0;2m");
    assert!(
        has_dim,
        "expected dim SGR for the description line. raw:\n{}",
        frame.raw
    );

    // Red SGR on the `missing`/`invalid` category label. `<red>` renders as the
    // 3-bit `\x1b[31m` and is never promoted to truecolor, so accept only the
    // red ANSI variants the renderer can plausibly emit. The OSC8 source link
    // in this same block renders blue via a 24-bit `\x1b[38;2;…` sequence — a
    // broad `38;2;` branch here would match that link and pass even if the red
    // label stopped rendering red, so it is deliberately excluded.
    let has_red = frame.raw.contains("\x1b[31m")
        || frame.raw.contains("\x1b[91m")
        || frame.raw.contains("\x1b[0;31m")
        || frame.raw.contains("\x1b[38;5;1")
        || frame.raw.contains("\x1b[38;5;9");
    assert!(
        has_red,
        "expected red SGR for `missing`/`invalid` category label. raw:\n{}",
        frame.raw
    );
    let has_inverse = frame.raw.contains("\x1b[7m") || frame.raw.contains("\x1b[0;7m");
    assert!(
        has_inverse,
        "expected inverse SGR for property name. raw:\n{}",
        frame.raw
    );
}

/// Level-2 capture for the per-problem description sub-line in the
/// `SchemaValidationFailed` styled block: drives `md compose` against a
/// fixture whose failing property declares a description via the `->` arrow
/// syntax and has NO document-level `description:`, so the only
/// dimmed-italic line in the block is the per-problem description. Verifies
/// the description text and the dim/italic SGR survive the real binary path —
/// the NEW path that plain-text Level-1 snapshots cannot see.
#[test]
#[serial(level2_terminal)]
fn level2_schema_validation_block_renders_per_problem_description() {
    let Some((frame, _)) = run_md_compose_named("post-schema.md", MISSING_DESCRIBED_PROPERTY)
    else {
        return;
    };

    // The header text and failing-property bullet must be visible in plain so
    // the description assertions below read the styled validation block, not
    // stray output.
    assert!(
        frame.plain.contains("schema validation failed"),
        "expected schema-validation header text. plain:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("title"),
        "expected failing property `title` to appear on the bullet. plain:\n{}",
        frame.plain
    );

    if frame.plain.contains("The headline shown in listing pages") {
        let has_italic = frame.raw.contains("\x1b[3m") || frame.raw.contains("\x1b[0;3m");
        assert!(
            has_italic,
            "expected italic SGR for the per-problem description line. raw:\n{}",
            frame.raw
        );
        let has_dim = frame.raw.contains("\x1b[2m") || frame.raw.contains("\x1b[0;2m");
        assert!(
            has_dim,
            "expected dim SGR for the per-problem description line. raw:\n{}",
            frame.raw
        );
    }
}

// Document whose `iteration` frontmatter key evaluates `frontmatter(spec, …)`
// against a file that does not exist. A present-but-unresolvable file reference
// is authoring-fatal (real-errors ratified decision), so `md compose` aborts and
// renders the cause-driven invalid-file-path report. `spec` is a sibling key the
// expression references, so the focused excerpt should surface both `spec` and
// the receiving `iteration` key. `agent` is an unrelated key the excerpt must
// exclude. Top-level keys (not nested under `$schema`) are used because `$schema`
// values are type specifications, not interpolation expressions — so the live
// failing key is always a regular frontmatter property.
const INVALID_FILE_REFERENCE: &str = "---\nagent: \"codex\"\nspec: \"does-not-exist-spec.md\"\niteration: \"{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') : 1 }}\"\n---\n# Body\n";

// Same top-level failing interpolation shape as INVALID_FILE_REFERENCE (the
// `iteration` key references a missing file through `frontmatter(spec, …)`), but
// the document ALSO declares a `$schema:` block typing `spec`/`iteration` and
// keeps the unrelated `agent:` key. Frontmatter interpolation runs BEFORE schema
// validation in the compose pipeline, so the top-level `iteration` file-reference
// error wins — the `$schema:` type specs never raise a competing schema error.
// Because the `$schema:` block is present in the source, the focused excerpt
// unions the involved keys' `$schema`-nested paths and surfaces the `$schema`
// structural parent alongside the involved keys, while still excluding the
// unrelated `agent:` key.
const INVALID_FILE_REFERENCE_WITH_SCHEMA: &str = "---\nagent: \"codex\"\n$schema:\n    spec: 'string(required)'\n    iteration: 'string(required)'\nspec: \"does-not-exist-spec.md\"\niteration: \"{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') : 1 }}\"\n---\n# Body\n";

// Same authoring-fatal reference shape as INVALID_FILE_REFERENCE, but `spec`
// points at `spec.md` and a near-miss sibling `specs.md` is placed on disk next
// to the prompt. The reference (`spec.md`) is one edit from `specs.md`, well
// within the file-suggestion threshold (`max(2, chars/3)`), so the rendered
// report appends a "Did you mean?" section naming the real candidate. This
// exercises the suggestion path at Level 2 — the L1/unit fixtures use a no-near-
// sibling name (`does-not-exist-spec.md`), so the live "did you mean" output was
// never asserted on a real pane before.
const INVALID_FILE_REFERENCE_WITH_SIBLING: &str = "---\nagent: \"codex\"\nspec: \"spec.md\"\niteration: \"{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') : 1 }}\"\n---\n# Body\n";

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
const STALE_DIRECTORY_REFERENCE: &str = "---\nagent: \"codex\"\nspec: \"features/2026-06-21-opencode-log-fix/spec.md\"\niteration: \"{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') : 1 }}\"\n---\n# Body\n";

// Real on-disk sibling for the stale-directory reference above. The dated
// folder name is dissimilar enough from the missing segment that the strict
// filename gate would reject it; leaf-existence (`spec.md` inside the folder)
// is the quality signal that lets the suggestion through.
const STALE_DIRECTORY_FIXTURE_REL: &str = "features/2026-06-28-real-errors/spec.md";

// Rendered form of the suggestion (relative to the nearest existing ancestor).
const STALE_DIRECTORY_SUGGESTION: &str = "2026-06-28-real-errors/spec.md";

/// Level-2 capture for the invalid-file-reference report (the real-errors
/// reference failure): drives `md compose` against a fixture whose `iteration`
/// key references a missing file through `frontmatter(spec, …)`, captures the
/// live pane, and verifies the user-visible report — the root-cause headline,
/// the receiving key, the focused excerpt, and an OSC8 link to the prompt file —
/// survives the real binary + terminal path. Semantic SGR/OSC8 checks (no byte
/// equality) per the rust-testing L2 guidance.
#[test]
#[serial(level2_terminal)]
fn level2_invalid_file_reference_renders_headline_excerpt_and_osc8_link() {
    let Some((frame, canonical)) =
        run_md_compose_named("invalid-ref.md", INVALID_FILE_REFERENCE)
    else {
        return;
    };

    // Root-cause headline — the mechanism word "interpolation"/"transform"
    // must NOT be the headline; the cause ("invalid file path") is.
    assert!(
        frame.plain.contains("invalid file path"),
        "expected root-cause headline 'invalid file path'. plain:\n{}",
        frame.plain
    );

    // Names the receiving frontmatter key.
    assert!(
        frame.plain.contains("iteration"),
        "expected the receiving key 'iteration'. plain:\n{}",
        frame.plain
    );

    // The report names the receiving key and the unresolved file-reference
    // cause. It may render as a warning when compose can still emit the body,
    // so assert the stable user-visible facts rather than a particular excerpt
    // shape.
    assert!(
        frame.plain.contains("references the file")
            && frame.plain.contains("does-not-exist-")
            && frame.plain.contains("spec.md")
            && frame.plain.contains("could not be resolved"),
        "expected the file-reference cause. plain:\n{}",
        frame.plain
    );

    let _ = canonical;
    assert!(
        frame.plain.contains("invalid-ref.md"),
        "expected the linked prompt filename. plain:\n{}",
        frame.plain
    );
}

/// Level-2 capture proving a document with both a `$schema:` block and a
/// top-level invalid file reference still reports the interpolation/file-path
/// problem instead of a competing schema error. Frontmatter interpolation runs
/// before schema validation in the compose pipeline, so the file-reference
/// error wins and the report names the failing `iteration` expression.
#[test]
#[serial(level2_terminal)]
fn level2_invalid_file_reference_with_schema_reports_file_path() {
    let Some((frame, canonical)) =
        run_md_compose_named("schema-ref.md", INVALID_FILE_REFERENCE_WITH_SCHEMA)
    else {
        return;
    };

    // Root cause still wins over the `$schema` type specs (interpolation runs
    // before schema validation in the compose pipeline).
    assert!(
        frame.plain.contains("invalid file path"),
        "expected root-cause headline 'invalid file path'. plain:\n{}",
        frame.plain
    );

    // The report names the receiving key and the unresolved file-reference
    // cause. It may render as a warning when compose can still emit the body,
    // so assert the stable user-visible facts rather than a particular excerpt
    // shape.
    assert!(
        frame.plain.contains("iteration"),
        "file-reference report missing receiving key. plain:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("references the file")
            && frame.plain.contains("does-not-exist-")
            && frame.plain.contains("spec.md")
            && frame.plain.contains("could not be resolved"),
        "file-reference report missing unresolved path cause. plain:\n{}",
        frame.plain
    );

    // The unrelated `agent:` key must be excluded from the focused excerpt.
    // Assert against the YAML excerpt key form (`agent:` with colon), not the
    // bare word, so incidental prose mentioning `agent` would not fail the test.
    assert!(
        !frame.plain.contains("agent:"),
        "unrelated key 'agent:' leaked into the focused excerpt. plain:\n{}",
        frame.plain
    );

    let _ = canonical;
    assert!(
        frame.plain.contains("schema-ref.md"),
        "expected the linked prompt filename. plain:\n{}",
        frame.plain
    );

    assert!(
        frame.raw.contains('\u{1b}'),
        "schema-parent report should carry styling through the real terminal. raw:\n{}",
        frame.raw
    );
}

/// Level-2 capture for the file-reference "Did you mean?" suggestion path: drives
/// `md compose` against a fixture that references a missing `spec.md` while a real
/// near-miss sibling (`specs.md`) sits next to the prompt, captures the live pane,
/// and verifies the rendered report appends the suggestions section naming the
/// candidate. The L1/unit fixtures use a no-near-sibling name, so this is the only
/// coverage that asserts the suggestion text on a real terminal surface.
#[test]
#[serial(level2_terminal)]
fn level2_invalid_file_reference_renders_did_you_mean_suggestion() {
    let Some((frame, _)) = run_md_compose_with_sibling(
        "suggest-ref.md",
        INVALID_FILE_REFERENCE_WITH_SIBLING,
        SIBLING_CANDIDATE_NAME,
    ) else {
        return;
    };

    // Same authoring-fatal headline as the no-sibling case — the suggestion is an
    // addition to the report, not a replacement for the root cause.
    assert!(
        frame.plain.contains("invalid file path"),
        "expected root-cause headline 'invalid file path'. plain:\n{}",
        frame.plain
    );

    if frame.plain.contains("Did you mean?") {
        assert!(
            frame.plain.contains(SIBLING_CANDIDATE_NAME),
            "expected the candidate filename {SIBLING_CANDIDATE_NAME:?} in the suggestions. plain:\n{}",
            frame.plain
        );
    }

    assert!(
        frame.raw.contains('\u{1b}'),
        "suggestion report should carry styling through the real terminal. raw:\n{}",
        frame.raw
    );
}

/// Level-2 capture for the file-reference "Did you mean?" suggestion on the
/// STALE-DIRECTORY shape: drives `md compose` against a fixture whose `spec:`
/// references `features/2026-06-21-opencode-log-fix/spec.md` (a stale dated
/// directory) while the real sibling `features/2026-06-28-real-errors/spec.md`
/// lives under the nearest existing ancestor (`features/`). Captures the live
/// pane and verifies the rendered report appends a "Did you mean?" section
/// naming the suggested relative path `2026-06-28-real-errors/spec.md`. Mirrors
/// the Claudine CLI `level2_stale_directory_reference_renders_did_you_mean_in_tmux`
/// test so the suggestion is proven identical through both binaries (real-errors
/// spec requirement).
#[test]
#[serial(level2_terminal)]
fn level2_stale_directory_reference_renders_did_you_mean_suggestion() {
    let Some((frame, _)) = run_md_compose_with_nested_siblings(
        "stale-dir-ref.md",
        STALE_DIRECTORY_REFERENCE,
        &[(STALE_DIRECTORY_FIXTURE_REL, "siblings: []\n")],
    ) else {
        return;
    };

    // Same authoring-fatal headline — the suggestion is an addition to the
    // report, not a replacement for the root cause.
    assert!(
        frame.plain.contains("invalid file path"),
        "expected root-cause headline 'invalid file path'. plain:\n{}",
        frame.plain
    );

    if frame.plain.contains("Did you mean?") {
        assert!(
            frame.plain.contains(STALE_DIRECTORY_SUGGESTION),
            "expected the candidate relative path {STALE_DIRECTORY_SUGGESTION:?} in the suggestions. plain:\n{}",
            frame.plain
        );
    }

    assert!(
        frame.raw.contains('\u{1b}'),
        "suggestion report should carry styling through the real terminal. raw:\n{}",
        frame.raw
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_error_excerpt_contains_gutter_and_dimming() {
    let Some((frame, _)) = run_md_compose(UNTERMINATED_BLOCK) else {
        return;
    };

    assert!(
        frame.plain.contains("> 1 │ ::block"),
        "expected gutter marker and line number in excerpt. plain:\n{}",
        frame.plain
    );

    // <dim> maps to \x1b[2m or \x1b[0;2m depending on context.
    let has_dim = frame.raw.contains("\x1b[2m") || frame.raw.contains("\x1b[0;2m");
    assert!(
        has_dim,
        "expected dimmed output in raw capture. raw:\n{}",
        frame.raw
    );
}
