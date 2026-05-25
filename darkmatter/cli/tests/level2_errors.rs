//! Level-2 tests for darkmatter error rendering (OSC 8, gutter style, etc.).
//!
//! Shares a single WezTerm pane across tests via [`SHARED_HARNESS`] and uses a
//! `printf`-emitted sentinel to detect command completion. Spawning a fresh
//! pane per test would push each test past the nextest slow-test termination
//! threshold (`slow-timeout = 5s`, `terminate-after = 3` → 15 s).

use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
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
        .send_command_with_env(&wrapped, &[])
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

    let mut guard = SHARED_HARNESS.get_or_init(|| {
        let mut harness = WezTermHarness::new();
        harness.spawn_shell().expect("spawn_shell failed");
        harness
    });
    let harness = guard.as_mut().unwrap();

    // Reset the visible region so a previous test's output does not bleed
    // into this capture.
    run_with_sentinel(harness, "clear");

    let cmd = format!("md compose {}", file_path.display());
    let frame = run_with_sentinel(harness, &cmd);
    // Keep tempdir alive past capture by returning canonical path to caller.
    drop(dir);
    Some((frame, canonical))
}

// Unterminated page block to trigger PageBlockError::UnterminatedBlock.
const UNTERMINATED_BLOCK: &str = "::block when=\"true\"\nbody\n";

// Document with an inline `$schema` requiring a property the frontmatter does
// not satisfy. Drives the styled `SchemaValidationFailed` block through the
// real binary + terminal so SGR/OSC8 behavior is captured live, not just
// asserted against a process-local renderer.
const MISSING_REQUIRED_SCHEMA: &str =
    "---\n$schema:\n  spec: 'string(min(1); required)'\nspec: \"\"\n---\nBody\n";

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

    // Red SGR (31 / 91 / 38;5;1 / 38;2;...) somewhere in the rendered block,
    // and inverse SGR (7) on the property name. Be tolerant of theme palette
    // differences: any one of the red ANSI variants is enough.
    let has_red = frame.raw.contains("\x1b[31m")
        || frame.raw.contains("\x1b[91m")
        || frame.raw.contains("\x1b[0;31m")
        || frame.raw.contains("\x1b[38;5;1")
        || frame.raw.contains("\x1b[38;2;");
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
