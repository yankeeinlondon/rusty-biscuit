//! Level 2 real-terminal capture for typed error propagation.
//!
//! Feature: `claudine/features/2026-07-13-error-propogation/` (Phase 7).
//!
//! Drives the real `claudine` binary through a real tmux pane for every route
//! the migration touched, and asserts the property the feature exists to
//! deliver: **a typed failure reaches the user as a rendered `StatusBlock`, not
//! the generic `Error:` line**.
//!
//! ## Why these need a terminal
//!
//! `effective_diagnostic_render.rs` already proves the block renders headlessly.
//! What it cannot prove is the part whose contract *is* the terminal: that the
//! block survives a real TTY with its red SGR and OSC 8 link intact, and that
//! the same information still arrives when `NO_COLOR` suppresses both. Those are
//! different code paths (`ColorDepth` resolution and the walker's
//! `strip_escape_codes` pass), and only a real pane exercises them.
//!
//! ## What each route pins
//!
//! Alongside the rendering, every route re-asserts the three properties
//! `characterization_error_routes.rs` froze in Phase 1 — **exit code**,
//! **lifecycle event order**, and **exactly-once emission** — so an L2 rendering
//! change cannot quietly perturb them. The exit code is read back through the
//! pane (`; echo RC[$?]`) rather than from a child handle, because the whole
//! point is to observe what the interactive surface actually did.
//!
//! ## Assertion style
//!
//! Snapshots assert **actionable content** — the authored reference, the
//! document, the contract that was violated — never a `to_string()` substring.
//! Colour is asserted on the block's specific red triple (`251;44;54`) in both
//! semicolon and ITU colon forms; a broad `38;2;` match is deliberately avoided
//! because it also matches the blue OSC 8 link and would pass on a block that
//! rendered no red at all.

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
use common::{TestWorkspace, augmented_path, clear_no_color, write, write_executable};

// --- Fixtures -------------------------------------------------------------

/// Route 1 — the motivating incident. `initialize` proxies to a target that does
/// not exist; `composition::resolve_proxy_target` probes existence and returns a
/// typed `HarnessError::PathResolutionFailed`, which the composition wrapper
/// carries as `#[source]`.
const INITIALIZE_PROXY_DOC: &str = "\
---
title: initialize proxy
initialize:
  stack:
    - action: {append_line: [\"events.log\", \"initialize\"]}
    - action: {proxy: \"no/such/target.md\"}
start:
  stack:
    - action: {append_line: [\"events.log\", \"start\"]}
failure:
  stack:
    - action: {append_line: [\"events.log\", \"failure\"]}
finalize:
  stack:
    - action: {append_line: [\"events.log\", \"finalize\"]}
---
Body
";

/// Route 2 — proxy hand-off from a terminal (`failure`) event. Since the
/// file-resolution feature converged the proxy routes, terminal-control dispatch
/// resolves the target through the same existence-checking `resolve_proxy_target`
/// as the initialize route, so a missing target fails *at* resolution. The
/// terminal lifecycle still runs to `finalize` (the abort path fires `finalize`
/// once before propagating), so this route keeps its full `start`/`failure`/
/// `finalize` marker sequence while now failing with the shared
/// `InvalidFileReference` block rather than a later adopted-document read.
const TERMINAL_PROXY_DOC: &str = "\
---
title: failure proxy
start:
  stack:
    - action: {append_line: [\"events.log\", \"start\"]}
failure:
  stack:
    - action: {append_line: [\"events.log\", \"failure\"]}
    - action: {proxy: \"no/such/target.md\"}
finalize:
  stack:
    - action: {append_line: [\"events.log\", \"finalize\"]}
---
Body
";

/// Route 4 — a schema-validation failure. `reviewers` is declared `number` and
/// supplied as a string.
///
/// The value is deliberately **present**: a required-*missing* value opens the
/// biscuit-tui prompt loop on a real TTY (that is the whole point of the loop),
/// which would hang this capture rather than render a block. A type violation on
/// a supplied value fails validation outright, so it is the schema shape that
/// has a non-interactive rendering contract to assert.
const SCHEMA_FAILURE_DOC: &str = "\
---
title: schema failure
$schema:
    reviewers: \"number(required)\"
reviewers: \"not-a-number\"
---
Body
";

/// Route 5 — a Darkmatter transclusion failure. `::file` names a chapter that
/// does not exist; Darkmatter's own `TransclusionError` block must survive the
/// Claudine `Semantic` wrapper that carries it (`decisions.md` D-3).
const TRANSCLUSION_DOC: &str = "\
---
title: transclusion
---
::file ./missing-chapter.md
";

/// Route 6 — harness pre-flight failure. A blacklisted `::shell` directive is
/// rejected before `start`, carrying a typed `HarnessError`.
const PREFLIGHT_SHELL_DOC: &str = "\
---
title: preflight shell denial
start:
  stack:
    - action: {append_line: [\"events.log\", \"start\"]}
---

::shell rm -rf /
";

/// Route 7's document is irrelevant — the failure is an argument-shape rejection
/// that never reaches composition.
const TRIVIAL_DOC: &str = "\
---
title: trivial
---
Body
";

/// The block's error glyph (`⤫`) and its body gutter (`┃`). A `StatusBlock` is
/// detected structurally from the pair; the glyph alone is not sufficient,
/// because the wrapper also prints ordinary status lines with it (`⤫ agent
/// exited with error code 3`).
const ERROR_GLYPH: char = '\u{292B}';
const BODY_GUTTER: char = '\u{2503}';

/// The block header's red, as truecolor SGR. Asserted in every form a terminal
/// may normalize it into rather than by a broad `38;2;` prefix — see the module
/// doc.
const RED_SGR_FORMS: [&str; 3] = ["38;2;251;44;54", "38:2::251:44:54", "38:2:251:44:54"];

// --- Staging --------------------------------------------------------------

struct Staged {
    workspace: TestWorkspace,
    bin_dir: PathBuf,
    doc: PathBuf,
}

impl Staged {
    /// Ordered lifecycle markers, or empty when the route failed before any
    /// stack ran.
    fn events(&self) -> Vec<String> {
        let Some(path) = find_events_log(self.workspace.path()) else {
            return Vec::new();
        };
        fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect()
    }
}

/// Search for the marker file rather than assuming a fixed location: the wrapper
/// may switch CWD to a discovered repo root, which moves where a relative
/// `append_line` target lands.
fn find_events_log(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
        } else if path.file_name().is_some_and(|n| n == "events.log") {
            return Some(path);
        }
    }
    subdirs.into_iter().find_map(|d| find_events_log(&d))
}

/// Stage an isolated workspace holding `doc_body`, with a `claude` shim that
/// exits `provider_exit`.
fn stage(name: &str, doc_body: &str, provider_exit: i32) -> Staged {
    let workspace = TestWorkspace::named(name);
    let root = workspace.path().to_path_buf();
    let bin_dir = root.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    write_executable(
        &bin_dir.join("claude"),
        &format!("#!/bin/sh\nexit {provider_exit}\n"),
    );

    let claudine_dir = root.join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    write(&claudine_dir.join("config.json"), "{}");

    let doc = root.join("route.md");
    write(&doc, doc_body);

    Staged {
        workspace,
        bin_dir,
        doc,
    }
}

// --- Capture --------------------------------------------------------------

struct Capture {
    frame: CapturedFrame,
    exit_code: i32,
}

impl Capture {
    /// Whether a `StatusBlock` rendered, detected structurally (glyph line
    /// immediately followed by a gutter line).
    fn has_status_block(&self) -> bool {
        let lines: Vec<&str> = self.frame.plain.lines().collect();
        lines.windows(2).any(|pair| {
            pair[0].trim_start().starts_with(ERROR_GLYPH)
                && pair[1].trim_start().starts_with(BODY_GUTTER)
        })
    }

    fn has_generic_error_line(&self) -> bool {
        self.frame
            .plain
            .lines()
            .any(|line| line.trim_start().starts_with("Error:"))
    }

    /// How many times the failure was surfaced.
    ///
    /// Sums both surfaces the CLI can use — the generic `Error:` line and a
    /// rendered block — because a route may legitimately render one or the
    /// other while still needing to emit exactly once.
    fn emission_count(&self) -> usize {
        let lines: Vec<&str> = self.frame.plain.lines().collect();
        let generic = lines
            .iter()
            .filter(|line| line.trim_start().starts_with("Error:"))
            .count();
        let blocks = lines
            .windows(2)
            .filter(|pair| {
                pair[0].trim_start().starts_with(ERROR_GLYPH)
                    && pair[1].trim_start().starts_with(BODY_GUTTER)
            })
            .count();
        generic + blocks
    }

    fn has_red_sgr(&self) -> bool {
        RED_SGR_FORMS
            .iter()
            .any(|form| self.frame.raw.contains(form))
    }

    /// Whether an OSC 8 hyperlink introducer reached the pane.
    fn has_osc8_link(&self) -> bool {
        self.frame.raw.contains("\x1b]8;;")
    }

    fn assert_contains(&self, needle: &str, why: &str) {
        assert!(
            self.frame.plain.contains(needle),
            "{why}: expected {needle:?} in the rendered pane.\nplain:\n{}",
            self.frame.plain
        );
    }
}

/// The exit-code marker echoed after the command.
///
/// Deliberately bracket- and glob-free: a `RC[$?]` shape is expanded by zsh's
/// filename globbing into `no matches found` before the exit code is ever
/// printed, so a marker's syntax has to survive whichever login shell the pane
/// happens to run.
const EXIT_MARKER: &str = "claudine_rc:";

/// Wait until a `claudine_rc:<digits>` line appears, then return the frame.
///
/// The command line echoed into the pane contains the literal
/// `claudine_rc:$?`, whose suffix is not digits and therefore cannot match — so
/// this distinguishes the shell's *output* from the keystrokes that produced it
/// without any settle-time guesswork.
fn wait_for_exit_marker(harness: &mut TmuxHarness, deadline: Duration) -> Capture {
    let stop = Instant::now() + deadline;
    loop {
        let frame = harness.capture().expect("capture pane");
        if let Some(exit_code) = parse_exit_marker(&frame.plain) {
            return Capture { frame, exit_code };
        }
        if Instant::now() >= stop {
            panic!(
                "the `{EXIT_MARKER}<code>` exit marker did not appear within \
                 {deadline:?}.\nplain:\n{}",
                frame.plain
            );
        }
        harness.settle();
    }
}

fn parse_exit_marker(plain: &str) -> Option<i32> {
    plain.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix(EXIT_MARKER)?;
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    })
}

/// Run `claudine compose --claude <doc>` in a real pane and capture the result.
///
/// `env` is applied on top of the isolation defaults (`HOME`, `PATH`), so a case
/// can select its colour contract (`FORCE_COLOR` vs `NO_COLOR`).
fn run_in_pane(
    harness: &mut TmuxHarness,
    staged: &Staged,
    env: &[(&str, &str)],
    extra_args: &[&str],
) -> Capture {
    // A tall pane: `capture()` reads the *visible* region only (tmux keeps no
    // scrollback for us), and route 2 renders a full lifecycle — system prompt,
    // agent prompt, hand-off — ahead of its block. On a default 40-row pane the
    // earlier emission would scroll away and `emission_count` would silently
    // undercount, turning a real duplicate-emission regression into a pass.
    let _ = harness.resize(120, 200);

    // A fixture that does not itself select `NO_COLOR` is asserting the *colored*
    // contract, so an ambient `NO_COLOR` on the host must not reach it. See
    // `common::clear_no_color` for why the env list cannot express this.
    if !env.iter().any(|(key, _)| *key == "NO_COLOR") {
        clear_no_color(harness);
    }

    let claudine = cargo_bin!("claudine").display().to_string();
    let home = staged.workspace.path().to_string_lossy().into_owned();
    let path = augmented_path(&staged.bin_dir);
    let path = path.to_string_lossy().into_owned();

    harness.send_text(b"clear\n").expect("clear pane");
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    harness
        .send_text(format!("cd {}\n", staged.workspace.path().display()).as_bytes())
        .expect("cd into workspace");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    let args = extra_args.join(" ");
    let cmd = format!(
        "{claudine} compose --claude {} {args}; echo {EXIT_MARKER}$?",
        staged.doc.display()
    );

    let mut full_env: Vec<(&str, &str)> = vec![("HOME", home.as_str()), ("PATH", path.as_str())];
    full_env.extend_from_slice(env);

    harness
        .send_command_with_env(&cmd, &full_env)
        .expect("send claudine command");

    let capture = wait_for_exit_marker(harness, Duration::from_secs(30));
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    capture
}

/// The styled contract: a real TTY with colour enabled.
const TTY_COLOR: [(&str, &str); 2] = [("FORCE_COLOR", "1"), ("COLUMNS", "100")];

/// The plain contract: a real TTY with colour suppressed.
const TTY_NO_COLOR: [(&str, &str); 2] = [("NO_COLOR", "1"), ("COLUMNS", "100")];

/// The auto-detect contract: a real TTY with `NO_COLOR`, `FORCE_COLOR`, and
/// `CLICOLOR_FORCE` absent, so the wrapper's `Terminal::new()` probe — not
/// `Terminal::new_optimistic()` — decides depth. `FORCE_COLOR=1` short-circuits
/// `cli/src/log.rs::compute_terminal` into its forced-styling arm and never
/// reaches that probe; only this const exercises the third arm, which is what
/// an unforced `claudine` process actually runs in a real pane.
const TTY_AUTO: [(&str, &str); 1] = [("COLUMNS", "100")];

const COLOR_OVERRIDES_ABSENT_MARKER: &str = "claudine_color_overrides_absent";

/// Remove the color overrides installed by `TmuxHarness::spawn_shell`, then
/// verify the interactive shell will not export any of them to Claudine.
fn unset_and_prove_color_overrides_absent(harness: &mut TmuxHarness) {
    harness
        .send_text(b"unset NO_COLOR FORCE_COLOR CLICOLOR_FORCE\n")
        .expect("unset color overrides");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    let probe = format!(
        "if [ -z \"${{NO_COLOR+x}}\" ] && [ -z \"${{FORCE_COLOR+x}}\" ] && \
         [ -z \"${{CLICOLOR_FORCE+x}}\" ]; then echo \
         {COLOR_OVERRIDES_ABSENT_MARKER}; fi\n"
    );
    harness
        .send_text(probe.as_bytes())
        .expect("probe color overrides");

    let stop = Instant::now() + Duration::from_secs(5);
    loop {
        let frame = harness.capture().expect("capture color override probe");
        if frame
            .plain
            .lines()
            .any(|line| line.trim() == COLOR_OVERRIDES_ABSENT_MARKER)
        {
            let _ = biscuit_test_harness::wait_for_prompt(harness);
            return;
        }
        if Instant::now() >= stop {
            panic!(
                "NO_COLOR, FORCE_COLOR, or CLICOLOR_FORCE remained set in the \
                 automatic-TTY pane.\nplain:\n{}",
                frame.plain
            );
        }
        harness.settle();
    }
}

// --- Route 1: `initialize` proxy resolution -------------------------------

/// The motivating incident, rendered in a real terminal.
///
/// Before this feature the same run produced a single generic line — the typed
/// `HarnessError` was in the chain the whole time, but the walker's own downcast
/// list could not see it.
#[test]
#[serial(level2_terminal)]
fn level2_initialize_proxy_renders_status_block_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-l2-init-proxy", INITIALIZE_PROXY_DOC, 0);
    let capture = run_in_pane(&mut harness, &staged, &TTY_COLOR, &[]);

    assert!(
        capture.has_status_block(),
        "the motivating failure must render a StatusBlock in a real terminal.\nplain:\n{}",
        capture.frame.plain
    );
    assert!(
        !capture.has_generic_error_line(),
        "the motivating failure must not fall back to the generic `Error:` line.\nplain:\n{}",
        capture.frame.plain
    );

    // Actionable content: the authored reference, the surface that authored it,
    // the document it lives in, and the contract it violated.
    capture.assert_contains("no/such/target.md", "the authored reference must be named");
    capture.assert_contains("initialize", "the authoring event must be named");
    capture.assert_contains("route.md", "the authoring document must be named");
    capture.assert_contains("does not exist", "the resolution failure must be stated");

    // The pre-migration flattened report text must be gone.
    assert!(
        !capture.frame.plain.contains("lifecycle initialize proxy:"),
        "the pre-migration flattened report text survived.\nplain:\n{}",
        capture.frame.plain
    );

    // Pinned by the Phase 1 characterization baseline.
    assert_eq!(
        capture.exit_code, 1,
        "initialize-proxy resolution failure must exit 1.\nplain:\n{}",
        capture.frame.plain
    );
    assert_eq!(
        staged.events(),
        vec!["initialize"],
        "the resolution failure aborts before `start`, so no terminal event may fire"
    );
    assert_eq!(
        capture.emission_count(),
        1,
        "the failure must be surfaced exactly once.\nplain:\n{}",
        capture.frame.plain
    );
}

/// The styled surface: red header SGR and an OSC 8 link to the document both
/// survive a real TTY.
#[test]
#[serial(level2_terminal)]
fn level2_initialize_proxy_block_carries_red_sgr_and_osc8_link_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-l2-init-proxy-styled", INITIALIZE_PROXY_DOC, 0);
    let capture = run_in_pane(&mut harness, &staged, &TTY_COLOR, &[]);

    assert!(
        capture.has_red_sgr(),
        "the block header must render its red SGR ({RED_SGR_FORMS:?}) through tmux.\nraw:\n{}",
        capture.frame.raw
    );
    assert!(
        capture.has_osc8_link(),
        "the block must link the authoring document with an OSC 8 hyperlink.\nraw:\n{}",
        capture.frame.raw
    );
}

/// The plain surface: `NO_COLOR` in a real TTY carries the **same information**
/// with none of the bytes.
///
/// This is the pairing that makes the assertion meaningful — the styled test
/// above proves the red and the link are reachable, so their absence here is a
/// suppression, not an artifact of a block that never rendered.
#[test]
#[serial(level2_terminal)]
fn level2_initialize_proxy_block_is_plain_under_no_color_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-l2-init-proxy-plain", INITIALIZE_PROXY_DOC, 0);
    let capture = run_in_pane(&mut harness, &staged, &TTY_NO_COLOR, &[]);

    assert!(
        capture.has_status_block(),
        "the block must still render under NO_COLOR.\nplain:\n{}",
        capture.frame.plain
    );
    assert!(
        !capture.has_red_sgr(),
        "NO_COLOR must suppress the block's red SGR.\nraw:\n{}",
        capture.frame.raw
    );
    assert!(
        !capture.has_osc8_link(),
        "NO_COLOR must suppress the block's OSC 8 hyperlink.\nraw:\n{}",
        capture.frame.raw
    );

    // Same information, none of the bytes.
    capture.assert_contains("no/such/target.md", "plain output lost the reference");
    capture.assert_contains("route.md", "plain output lost the document");
    capture.assert_contains("does not exist", "plain output lost the failure");
    assert_eq!(capture.exit_code, 1, "NO_COLOR must not change the exit code");
}

/// Automatic TTY colour detection — the branch `FORCE_COLOR=1` bypasses.
///
/// `level2_initialize_proxy_block_carries_red_sgr_and_osc8_link_in_tmux` pins
/// the styled block through `FORCE_COLOR=1`, which selects
/// `Terminal::new_optimistic` in `cli/src/log.rs::compute_terminal` and never
/// probes the pane. The spec's Testing Strategy requires L2 to cover "TTY,
/// `NO_COLOR`, `FORCE_COLOR`, and piped stderr variants ... where their output
/// contracts differ"; the auto-detect arm (`Terminal::new()`) is the TTY case
/// proper, and it is the code path an unforced `claudine` process actually
/// runs in a real pane. Unsetting all three overrides — rather than reusing
/// `TTY_COLOR` — is what makes this distinct from the forced-color test: a
/// regression that breaks `is_tty`/depth detection while leaving
/// `FORCE_COLOR=1` working would pass the existing suite and fail this case.
#[test]
#[serial(level2_terminal)]
fn level2_initialize_proxy_block_auto_detects_tty_color_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    unset_and_prove_color_overrides_absent(&mut harness);
    let staged = stage("claudine-l2-init-proxy-auto", INITIALIZE_PROXY_DOC, 0);
    let capture = run_in_pane(&mut harness, &staged, &TTY_AUTO, &[]);

    assert!(
        capture.has_status_block(),
        "the block must render under automatic TTY detection.\nplain:\n{}",
        capture.frame.plain
    );
    assert!(
        capture.has_red_sgr(),
        "the wrapper must recognise its real stderr TTY and enable the red \
         SGR ({RED_SGR_FORMS:?}) without FORCE_COLOR.\nraw:\n{}",
        capture.frame.raw
    );
    assert!(
        capture.has_osc8_link(),
        "the wrapper must recognise its real stderr TTY and emit the OSC 8 \
         document link without FORCE_COLOR.\nraw:\n{}",
        capture.frame.raw
    );

    // Same actionable content the forced-colour and plain cases pin: the
    // authored reference and the resolution-failure reason.
    capture.assert_contains("no/such/target.md", "the authored reference must be named");
    capture.assert_contains("does not exist", "the resolution failure must be stated");
}

// --- Route 2: terminal/recovery proxy resolution --------------------------

/// Proxy hand-off from a terminal (`failure`) event.
///
/// This route is why Phase 7 is not a formality. It reached the pane as a
/// generic `Error: failed to load Markdown: …` line until the `Box`
/// un-downcastability recorded in `decisions.md` D-7 was fixed at its live site:
/// `Report::from(Box<CompositionError>)` publishes `Box<CompositionError>` to
/// the cause chain, so `as_diagnostic`'s downcast allowlist — keyed on the
/// concrete type — could not see it. The typed error was present and the
/// registry listed it; it was simply unreachable.
#[test]
#[serial(level2_terminal)]
fn level2_terminal_proxy_renders_status_block_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-l2-term-proxy", TERMINAL_PROXY_DOC, 3);
    let capture = run_in_pane(&mut harness, &staged, &TTY_COLOR, &[]);

    assert!(
        capture.has_status_block(),
        "the terminal-proxy failure must render a StatusBlock.\nplain:\n{}",
        capture.frame.plain
    );
    assert!(
        !capture.has_generic_error_line(),
        "the terminal-proxy failure must not fall back to the generic `Error:` line.\nplain:\n{}",
        capture.frame.plain
    );

    capture.assert_contains("target.md", "the unresolvable target must be named");

    // Pinned by the Phase 1 characterization baseline: the `failure` stack's
    // proxy fails at resolution, but the abort path still fires `finalize` once
    // before propagating, so the full terminal marker sequence
    // (`start`/`failure`/`finalize`) is preserved. The exactly-once emission and
    // exit code stay pinned so the resolver convergence cannot perturb them.
    assert_eq!(
        capture.exit_code, 1,
        "terminal-proxy failure must exit 1.\nplain:\n{}",
        capture.frame.plain
    );
    assert_eq!(
        staged.events(),
        vec!["start", "failure", "finalize"],
        "the terminal proxy route must run its full terminal lifecycle"
    );
    assert_eq!(
        capture.emission_count(),
        1,
        "the failure must be surfaced exactly once.\nplain:\n{}",
        capture.frame.plain
    );
}

/// Cross-route parity — AC5, now satisfied.
///
/// Error-propagation's Acceptance Criterion 5 asks the two proxy routes to agree
/// on code, headline, hint, and typed resolution detail. Its `decisions.md` D-2
/// found the routes did not yet share a resolver: `initialize` failed *at*
/// resolution with `composition.invalid_file_reference`, while the terminal
/// route resolved a missing target successfully (via the old
/// `resolve_harness_path` private grammar) and only failed later reading the
/// adopted document (`failed to load Markdown`). That divergence was pinned by
/// the earlier form of this test, which asserted the two different failure
/// stages so the convergence could not land silently.
///
/// The file-resolution feature converged them: the terminal-control dispatch now
/// routes through the same existence-checking `resolve_proxy_target` as the
/// initialize route (Phase 6), so **both** fail at resolution with the identical
/// `CompositionError::InvalidFileReference` — same code, same
/// "Unresolvable file reference" headline, and the same `PROXY_TARGET_HINT`
/// (identical by construction: both routes build the block from one constant).
///
/// Identity is asserted across both routes, while event and property context
/// remain separate: the event-label assertion must not pass on the event name
/// embedded in the property path.
#[test]
#[serial(level2_terminal)]
fn level2_proxy_routes_share_identity_across_routes_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");

    let init_staged = stage("claudine-l2-parity-init", INITIALIZE_PROXY_DOC, 0);
    let init = run_in_pane(&mut harness, &init_staged, &TTY_COLOR, &[]);
    let term_staged = stage("claudine-l2-parity-term", TERMINAL_PROXY_DOC, 3);
    let term = run_in_pane(&mut harness, &term_staged, &TTY_COLOR, &[]);

    // Full AC5 identity: both routes render the SAME typed failure. The
    // per-route `property` is the one intentional structured difference (see the
    // dotted-path assertion below).
    for (name, capture, property) in [
        ("initialize", &init, "initialize.stack[*].proxy"),
        ("terminal", &term, "failure.stack[*].proxy"),
    ] {
        assert!(
            capture.has_status_block(),
            "proxy route `{name}` must render a StatusBlock.\nplain:\n{}",
            capture.frame.plain
        );
        assert!(
            !capture.has_generic_error_line(),
            "proxy route `{name}` must not reach the generic `Error:` line.\nplain:\n{}",
            capture.frame.plain
        );
        assert_eq!(
            capture.exit_code, 1,
            "proxy route `{name}` must exit 1.\nplain:\n{}",
            capture.frame.plain
        );
        assert_eq!(
            capture.emission_count(),
            1,
            "proxy route `{name}` must emit exactly once.\nplain:\n{}",
            capture.frame.plain
        );
        assert!(
            capture.frame.plain.contains("target.md"),
            "proxy route `{name}` must name the unresolvable target.\nplain:\n{}",
            capture.frame.plain
        );
        // Identical code + headline: both routes fail AT resolution with
        // `CompositionError::InvalidFileReference`. The terminal route no longer
        // reaches the adopted-document read — its old `failed to load Markdown`
        // stage is gone.
        assert!(
            capture.frame.plain.contains("Unresolvable file reference"),
            "proxy route `{name}` must fail AT resolution with the shared \
             `Unresolvable file reference` headline.\nplain:\n{}",
            capture.frame.plain
        );
        assert!(
            !capture.frame.plain.contains("failed to load Markdown"),
            "proxy route `{name}` must not fall through to the adopted-document \
             read — the routes converged on the resolution-time failure.\nplain:\n{}",
            capture.frame.plain
        );
        // The structured `property` path reaches the block as its full dotted
        // form — not the bare word `proxy`, which also appears in the fixture,
        // hint, and surrounding prose. Anchoring on the whole path ties the
        // assertion to the rendered structured field.
        assert!(
            capture.frame.plain.contains(property),
            "proxy route `{name}` must name its `{property}` property.\nplain:\n{}",
            capture.frame.plain
        );
        assert!(
            capture.frame.plain.contains("does not exist"),
            "proxy route `{name}` must state the resolution failure.\nplain:\n{}",
            capture.frame.plain
        );
        // Identical hint: the shared `PROXY_TARGET_HINT` (a distinctive early
        // fragment that fits on one wrapped line at COLUMNS=100).
        assert!(
            capture.frame.plain.contains("must name an existing"),
            "proxy route `{name}` must carry the shared proxy-target hint.\nplain:\n{}",
            capture.frame.plain
        );
    }

    // Match the event label rather than the event name embedded in the
    // independently asserted property path.
    assert!(
        init.frame.plain.contains("`initialize` event of"),
        "the initialize route must render its structured event label separately \
         from the property path.\nplain:\n{}",
        init.frame.plain
    );
    assert!(
        term.frame.plain.contains("`failure` event of"),
        "the terminal route must render its structured event label separately \
         from the property path.\nplain:\n{}",
        term.frame.plain
    );
}

// --- Route 3: composition source lookup -----------------------------------

/// The composition source document itself cannot be found.
///
/// This route's contract is genuinely terminal-dependent, which is why it earns
/// an L2 case rather than a headless one: piped, the lookup reports that
/// autocomplete is *unavailable*; on a real TTY autocomplete runs, finds
/// nothing, and reports **that** instead. Both are typed blocks, but only a real
/// pane exercises the second path at all.
#[test]
#[serial(level2_terminal)]
fn level2_composition_source_lookup_renders_status_block_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let mut staged = stage("claudine-l2-source-lookup", TRIVIAL_DOC, 0);
    // Point the run at a document that was never written.
    staged.doc = staged.workspace.path().join("no-such-document.md");

    let capture = run_in_pane(&mut harness, &staged, &TTY_COLOR, &[]);

    assert!(
        capture.has_status_block(),
        "a missing composition source must render a StatusBlock.\nplain:\n{}",
        capture.frame.plain
    );
    assert!(
        !capture.has_generic_error_line(),
        "a missing composition source must not reach the generic `Error:` line.\nplain:\n{}",
        capture.frame.plain
    );
    capture.assert_contains(
        "No files matched",
        "the block must state that the autocomplete query found nothing",
    );
    capture.assert_contains(
        "no-such-document.md",
        "the block must name the document that could not be found",
    );
    assert_eq!(
        capture.exit_code, 1,
        "a missing composition source must exit 1.\nplain:\n{}",
        capture.frame.plain
    );
    assert!(
        staged.events().is_empty(),
        "source lookup precedes every lifecycle event; got {:?}",
        staged.events()
    );
    assert_eq!(
        capture.emission_count(),
        1,
        "the failure must be surfaced exactly once.\nplain:\n{}",
        capture.frame.plain
    );
}

// --- Route 4: schema failure ----------------------------------------------

/// A `$schema`-declared required property is never supplied.
///
/// The file-reference half of this route has its own dedicated L2 suite
/// (`level2_invalid_file_reference_capture.rs`, which asserts the headline,
/// focused excerpt, OSC 8 link, and "Did you mean?" suggestions); this covers
/// the schema half so the pair is complete without duplicating it.
#[test]
#[serial(level2_terminal)]
fn level2_schema_failure_renders_status_block_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-l2-schema", SCHEMA_FAILURE_DOC, 0);
    let capture = run_in_pane(&mut harness, &staged, &TTY_COLOR, &[]);

    assert!(
        capture.has_status_block(),
        "a schema failure must render a StatusBlock.\nplain:\n{}",
        capture.frame.plain
    );
    assert!(
        !capture.has_generic_error_line(),
        "a schema failure must not reach the generic `Error:` line.\nplain:\n{}",
        capture.frame.plain
    );
    capture.assert_contains("reviewers", "the offending property must be named");
    capture.assert_contains(
        "is not of type",
        "the violated type constraint must be stated",
    );
    assert_eq!(
        capture.exit_code, 1,
        "a schema failure must exit 1.\nplain:\n{}",
        capture.frame.plain
    );
    assert_eq!(
        capture.emission_count(),
        1,
        "the failure must be surfaced exactly once.\nplain:\n{}",
        capture.frame.plain
    );
}

// --- Route 5: Darkmatter transclusion -------------------------------------

/// A Darkmatter transclusion failure keeps Darkmatter's own rich block.
///
/// This is the detector `decisions.md` D-3 named. `CompositionError::ComposeFailed`
/// is `Semantic` — it owns the code, because a Darkmatter cause supplies no
/// facets under Option A — so once selection started preferring it, its
/// `status_block` had to keep delegating to the inner cause. If it ever falls
/// through to the flat catch-all arm, the pane loses Darkmatter's path/reason
/// detail and this test fails on the resolved-path assertion.
#[test]
#[serial(level2_terminal)]
fn level2_transclusion_failure_renders_darkmatter_block_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-l2-transclusion", TRANSCLUSION_DOC, 0);
    let capture = run_in_pane(&mut harness, &staged, &TTY_COLOR, &[]);

    assert!(
        capture.has_status_block(),
        "a transclusion failure must render a StatusBlock.\nplain:\n{}",
        capture.frame.plain
    );
    assert!(
        !capture.has_generic_error_line(),
        "a transclusion failure must not reach the generic `Error:` line.\nplain:\n{}",
        capture.frame.plain
    );

    // Darkmatter's own block, not a flattened `Display` line: it names the
    // authored reference AND the path it resolved to, which the catch-all arm
    // cannot produce.
    capture.assert_contains("missing-chapter.md", "the authored reference must be named");
    capture.assert_contains(
        "Check file existence and permissions",
        "Darkmatter's own hint must survive the Claudine wrapper",
    );
    assert_eq!(
        capture.exit_code, 1,
        "a transclusion failure must exit 1.\nplain:\n{}",
        capture.frame.plain
    );
    assert_eq!(
        capture.emission_count(),
        1,
        "the failure must be surfaced exactly once.\nplain:\n{}",
        capture.frame.plain
    );
}

// --- Route 6: harness pre-flight ------------------------------------------

/// A blacklisted `::shell` directive is rejected at pre-flight.
#[test]
#[serial(level2_terminal)]
fn level2_preflight_shell_denial_renders_status_block_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-l2-preflight", PREFLIGHT_SHELL_DOC, 0);
    let capture = run_in_pane(&mut harness, &staged, &TTY_COLOR, &[]);

    assert!(
        capture.has_status_block(),
        "a pre-flight shell denial must render a StatusBlock.\nplain:\n{}",
        capture.frame.plain
    );
    assert!(
        !capture.has_generic_error_line(),
        "a pre-flight shell denial must not reach the generic `Error:` line.\nplain:\n{}",
        capture.frame.plain
    );

    // Actionable: which command, and why it was refused.
    capture.assert_contains("rm -rf /", "the refused command must be named");
    capture.assert_contains("dangerous command", "the reason for refusal must be stated");

    assert_eq!(
        capture.exit_code, 1,
        "a pre-flight shell denial must exit 1.\nplain:\n{}",
        capture.frame.plain
    );
    assert!(
        staged.events().is_empty(),
        "pre-flight precedes `start`, so no lifecycle marker may fire; got {:?}",
        staged.events()
    );
    assert_eq!(
        capture.emission_count(),
        1,
        "the failure must be surfaced exactly once.\nplain:\n{}",
        capture.frame.plain
    );
}

// --- Route 7: the unstructured control ------------------------------------

/// The control: a genuinely unstructured error must **still** reach the generic
/// fallback in a real terminal.
///
/// That path stays valid (spec §D5) — if it starts rendering a block, selection
/// has become too eager. Argument-shape rejection is the honest unstructured
/// case: authored prose with no typed error behind it. Per `decisions.md` D-10,
/// `--timeout not-a-duration` is deliberately **not** used here — it looks
/// unstructured but carries a typed `HarnessError::InvalidTimeout`, and
/// correctly renders a block.
#[test]
#[serial(level2_terminal)]
fn level2_unstructured_failure_still_uses_the_generic_fallback_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-l2-unstructured", TRIVIAL_DOC, 0);
    let capture = run_in_pane(&mut harness, &staged, &TTY_COLOR, &["also-a-file.md"]);

    assert!(
        capture.has_generic_error_line(),
        "an unstructured error must still reach the generic `Error:` line.\nplain:\n{}",
        capture.frame.plain
    );
    assert!(
        !capture.has_status_block(),
        "an unstructured error must not render a StatusBlock.\nplain:\n{}",
        capture.frame.plain
    );
    assert_eq!(
        capture.exit_code, 1,
        "an unstructured failure must exit 1.\nplain:\n{}",
        capture.frame.plain
    );
    assert_eq!(
        capture.emission_count(),
        1,
        "the failure must be surfaced exactly once.\nplain:\n{}",
        capture.frame.plain
    );
}
