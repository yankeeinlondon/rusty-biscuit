//! Level 2 real-editor semantic-token tests against Neovim.
//!
//! The L1 provider and in-process JSON-RPC session tests prove the token data
//! `dmls` emits; they cannot prove what an editor *does* with it. These tests
//! drive Neovim's real LSP client against the real `dmls` binary and verify
//! the editor-side half of the semantic-tokens contract
//! (`darkmatter/features/2026-07-11-semantic-tokens/`, review-4 finding
//! "Intended presentation and live repaint remain unverified in real
//! editors"):
//!
//! - the client's token decode and byte-column mapping after
//!   position-encoding negotiation (Unicode and multiline spans land on the
//!   intended columns),
//! - family classification as the editor sees it (interpolation / literal
//!   `inert` / directive / closer / wiki frame vs. segments / precedence of
//!   interpolation inside a directive option),
//! - fenced-code exclusion,
//! - the `workspace/didChangeConfiguration` →
//!   `workspace/semanticTokens/refresh` repaint loop with no restart, and
//! - (tmux) that the documented Neovim styling recipe's highlight groups
//!   produce visible SGR color in a real terminal, and that a live config
//!   toggle visibly repaints the pane.
//!
//! Two driving modes:
//!
//! - **Headless** (`nvim --clean --headless -l probe.lua`): the probe script
//!   attaches the client, waits for tokens, and reports
//!   `vim.lsp.semantic_tokens.get_at_pos` results as JSON. Needs only `nvim`.
//! - **tmux** ([`TmuxHarness`]): a real TUI Neovim renders the fixture and the
//!   captured pane's SGR bytes are asserted. Needs `nvim` + `tmux`. The
//!   session is owned (spawned per test, killed on drop), so the test is
//!   parallel-safe and independent of the shared broker pane.
//!
//! What Neovim cannot exercise (still manual per the smoke checklist): range
//! requests at viewport boundaries (Neovim requests `full` only), and the
//! VS Code / Zed recipes.
//!
//! Skip-clean: missing `nvim` (or `tmux`) skips via `require_level!`;
//! `BISCUIT_TEST_LEVEL_REQUIRED=2` hard-fails instead. Run via `just test-l2`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serde::Deserialize;
use test_toolkit::{Level, require_level};

/// One deterministic token site per assertion target; distinct identifier
/// names (`title` / `east` / `cond` / `nope`) keep needle lookups unambiguous.
const FIXTURE_DOC: &str = r#"---
title: Fixture
---
# Demo

Intro {{ title }} and [[missing-note|alias]] here.

café {{ east }} 東京

{{{ raw
still raw }}}

::file ./other.md

::block when="{{ cond }}"
inside the block
::end-block

```rust
let fenced = "{{ nope }}";
```
"#;

const DMLS_TOML: &str = "[semantic_tokens]\nenable = true\n";

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/editor_neovim")
        .join(name)
}

fn nvim_available() -> bool {
    Command::new("nvim")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Workspace root for one test run: `tokens.md`, its transclusion target, and
/// a `.dmls.toml` root marker. Canonicalized so the paths Neovim reports match
/// the paths `dmls` resolves (macOS `/var` → `/private/var`).
fn stage_workspace() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    fs::write(dir.path().join("tokens.md"), FIXTURE_DOC).unwrap();
    fs::write(dir.path().join("other.md"), "# Other\n").unwrap();
    fs::write(dir.path().join(".dmls.toml"), DMLS_TOML).unwrap();
    let root = fs::canonicalize(dir.path()).unwrap_or_else(|_| dir.path().to_path_buf());
    (dir, root)
}

// ── Headless probe plumbing ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ProbeToken {
    line: u32,
    start_col: u32,
    end_col: u32,
    #[serde(rename = "type")]
    token_type: String,
    modifiers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeSite {
    row: u32,
    col: u32,
    tokens: Vec<ProbeToken>,
}

#[derive(Debug, Deserialize)]
struct ProbeReport {
    #[serde(default)]
    offset_encoding: Option<String>,
    #[serde(default)]
    advertised_encodings: Vec<String>,
    #[serde(default)]
    errors: Vec<String>,
    #[serde(default)]
    probes: HashMap<String, ProbeSite>,
    #[serde(default)]
    initial: Option<u32>,
    #[serde(default)]
    cleared: Option<bool>,
    #[serde(default)]
    repainted: Option<bool>,
    #[serde(default)]
    wiki_suppressed: Option<bool>,
}

fn run_probe(mode: &str, root: &Path) -> ProbeReport {
    let probe = fixture_path("probe.lua");
    let output = Command::new("nvim")
        .args(["--clean", "--headless", "-l"])
        .arg(&probe)
        .env("DMLS_L2_BIN", env!("CARGO_BIN_EXE_dmls"))
        .env("DMLS_L2_ROOT", root)
        .env("DMLS_L2_MODE", mode)
        .output()
        .expect("spawn headless nvim");
    // `nvim -l` routes `print()` through the message system, which lands on
    // stderr in headless mode — search both streams for the markers.
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let start_marker = "DMLS_L2_JSON<";
    let end_marker = ">DMLS_L2_JSON";
    let start = combined
        .find(start_marker)
        .unwrap_or_else(|| panic!("probe emitted no JSON\noutput: {combined}"))
        + start_marker.len();
    let end = start
        + combined[start..]
            .find(end_marker)
            .unwrap_or_else(|| panic!("unterminated probe JSON\noutput: {combined}"));
    let report: ProbeReport = serde_json::from_str(&combined[start..end])
        .unwrap_or_else(|e| panic!("malformed probe JSON: {e}\njson: {}", &combined[start..end]));
    assert!(
        report.errors.is_empty(),
        "probe reported errors: {:?}\noutput: {combined}",
        report.errors
    );
    report
}

/// The token starting exactly at the probe site (every needle aligns to a
/// token start). Neovim's `get_at_pos` is end-inclusive at boundaries, so a
/// probe at a token's first byte can also return the adjacent preceding token
/// (e.g. the wiki `[[` frame when probing the path segment) — filtering on
/// `start_col` keeps the assertion on the intended token while requiring it
/// to be unique.
fn single<'a>(report: &'a ProbeReport, name: &str) -> (&'a ProbeSite, &'a ProbeToken) {
    let site = report
        .probes
        .get(name)
        .unwrap_or_else(|| panic!("probe `{name}` missing from report"));
    let mut starting: Vec<&ProbeToken> = site
        .tokens
        .iter()
        .filter(|t| t.start_col == site.col)
        .collect();
    assert_eq!(
        starting.len(),
        1,
        "probe `{name}` expected one token starting at col {}, got {:?}",
        site.col,
        site.tokens
    );
    (site, starting.remove(0))
}

fn assert_classified(report: &ProbeReport, name: &str, token_type: &str, mods: &[&str]) {
    let (site, token) = single(report, name);
    assert_eq!(
        token.token_type, token_type,
        "probe `{name}` type mismatch: {token:?}"
    );
    for m in mods {
        assert!(
            token.modifiers.iter().any(|have| have == m),
            "probe `{name}` missing modifier `{m}`: {token:?}"
        );
    }
    assert_eq!(token.line, site.row, "probe `{name}` token on wrong line");
}

fn assert_empty(report: &ProbeReport, name: &str) {
    let site = report
        .probes
        .get(name)
        .unwrap_or_else(|| panic!("probe `{name}` missing from report"));
    assert!(
        site.tokens.is_empty(),
        "probe `{name}` expected no tokens, got {:?}",
        site.tokens
    );
}

// ── Headless tests ─────────────────────────────────────────────────────────

#[test]
fn level2_neovim_decodes_semantic_token_families_and_positions() {
    require_level!(Level::L2, nvim_available(), "nvim");
    let (_dir, root) = stage_workspace();
    let report = run_probe("tokens", &root);

    // docs/editors/neovim.md: `dmls` takes the client's first supported
    // offer. Which encoding that is depends on the Neovim release — 0.11+
    // advertises utf-8 ahead of utf-16, 0.10 advertises only utf-16 — so the
    // expectation is derived from what this client actually offered. Every
    // column assertion below is in *bytes* either way, which is the real
    // subject: the decode must land on source bytes under either encoding.
    assert!(
        !report.advertised_encodings.is_empty(),
        "probe could not read the client's advertised encodings; without them the \
         expectation below would silently default and stop testing negotiation"
    );
    let expected_encoding = if report.advertised_encodings.iter().any(|e| e == "utf-8") {
        "utf-8"
    } else {
        "utf-16"
    };
    assert_eq!(
        report.offset_encoding.as_deref(),
        Some(expected_encoding),
        "dmls must negotiate the client's first supported advertised encoding, offered {:?}",
        report.advertised_encodings
    );

    // F1 interpolation: whole `{{ … }}` span, not inert.
    assert_classified(&report, "interpolation", "macro", &["interpolation"]);
    let (site, token) = single(&report, "interpolation");
    assert_eq!(token.start_col, site.col, "interpolation must span the `{{{{`");
    assert_eq!(
        token.end_col - token.start_col,
        "{{ title }}".len() as u32,
        "interpolation span length"
    );
    assert!(
        !token.modifiers.iter().any(|m| m == "inert"),
        "plain interpolation must not be inert: {token:?}"
    );

    // Byte-column fidelity on a multibyte line ("café " is 6 bytes): the
    // editor-decoded columns must land exactly on the source bytes.
    let (site, token) = single(&report, "unicode_interpolation");
    assert_eq!(site.col, 6, "fixture drift: `{{{{ east }}}}` byte column");
    assert_eq!(token.start_col, site.col, "unicode token start column");
    assert_eq!(
        token.end_col - token.start_col,
        "{{ east }}".len() as u32,
        "unicode token span length"
    );
    assert_classified(&report, "unicode_interpolation", "macro", &["interpolation"]);

    // F1 literal: inert, and split per line (continuation starts at col 0).
    assert_classified(&report, "literal_open", "macro", &["interpolation", "inert"]);
    assert_classified(
        &report,
        "literal_continuation",
        "macro",
        &["interpolation", "inert"],
    );
    let (open_site, _) = single(&report, "literal_open");
    let (cont_site, cont_token) = single(&report, "literal_continuation");
    assert_eq!(cont_site.row, open_site.row + 1, "literal spans two lines");
    assert_eq!(cont_token.start_col, 0, "continuation line starts at col 0");

    // F2 directives: keyword, structured target, option key, closer.
    assert_classified(&report, "directive_keyword", "macro", &["directive"]);
    assert_classified(&report, "directive_target", "string", &["directive"]);
    assert_classified(&report, "option_key", "property", &["directive"]);
    assert_classified(&report, "closer_keyword", "macro", &["directive", "closer"]);

    // Precedence: `when="{{ cond }}"` is an interpolation inside a directive.
    assert_classified(&report, "option_interpolation", "macro", &["interpolation"]);

    // F4 wiki: frame vs. inner segments.
    assert_classified(&report, "wiki_frame", "macro", &["wiki"]);
    assert_classified(&report, "wiki_path", "string", &["wiki"]);
    assert_classified(&report, "wiki_alias", "string", &["wiki"]);

    // Exclusions: fenced-code content and ordinary prose carry no tokens.
    assert_empty(&report, "fenced_interpolation");
    assert_empty(&report, "prose");
}

#[test]
fn level2_neovim_semantic_tokens_config_toggle_repaints_live() {
    require_level!(Level::L2, nvim_available(), "nvim");
    let (_dir, root) = stage_workspace();
    let report = run_probe("refresh", &root);

    assert!(
        report.initial.unwrap_or(0) > 0,
        "tokens must paint before the toggle: {report:?}"
    );
    assert_eq!(
        report.cleared,
        Some(true),
        "disabling [semantic_tokens] must clear the editor's tokens without a restart: {report:?}"
    );
    assert_eq!(
        report.repainted,
        Some(true),
        "re-enabling [semantic_tokens] must repaint without a restart: {report:?}"
    );
    assert_eq!(
        report.wiki_suppressed,
        Some(true),
        "wiki.enable = false must suppress only the wiki family: {report:?}"
    );
}

// ── tmux rendering test ────────────────────────────────────────────────────

/// SGR foreground for a 256-color index, in both the semicolon and ITU colon
/// forms (capture re-encoding may use either).
fn has_indexed_fg(raw_line: &str, index: u16) -> bool {
    raw_line.contains(&format!("38;5;{index}")) || raw_line.contains(&format!("38:5:{index}"))
}

/// The raw (SGR-bearing) pane line whose plain text contains `needle`.
fn raw_line_containing<'a>(frame: &'a CapturedFrame, needle: &str) -> Option<&'a str> {
    let idx = frame
        .plain
        .lines()
        .position(|line| line.contains(needle))?;
    frame.raw.lines().nth(idx)
}

/// Polls the pane until `predicate` holds, returning the satisfying frame.
fn wait_for_frame(
    harness: &mut TmuxHarness,
    what: &str,
    predicate: impl Fn(&CapturedFrame) -> bool,
) -> CapturedFrame {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last = None;
    while Instant::now() < deadline {
        if let Ok(frame) = harness.capture() {
            if predicate(&frame) {
                return frame;
            }
            last = Some(frame);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!(
        "timed out waiting for {what}; last pane:\n{}",
        last.map(|f| f.plain).unwrap_or_default()
    );
}

#[test]
fn level2_neovim_tmux_renders_recipe_colors_and_repaints() {
    require_level!(
        Level::L2,
        nvim_available() && TmuxHarness::available(),
        "nvim+tmux"
    );
    let (_dir, root) = stage_workspace();

    let init = fs::read_to_string(fixture_path("init.lua"))
        .unwrap()
        .replace("__DMLS_BIN__", env!("CARGO_BIN_EXE_dmls"))
        .replace("__ROOT__", &root.to_string_lossy());
    let init_path = root.join("init.lua");
    fs::write(&init_path, init).unwrap();

    let init_arg = init_path.to_string_lossy().into_owned();
    let doc_arg = root.join("tokens.md").to_string_lossy().into_owned();
    let mut harness = TmuxHarness::new();
    harness
        .spawn_program(
            "nvim",
            &["-u", &init_arg, "-i", "NONE", "--noplugin", &doc_arg],
        )
        .expect("spawn nvim in tmux");

    // Poll for the interpolation color, then assert the whole recipe on that
    // same frame (no second capture — avoids the half-painted-frame race).
    let painted = wait_for_frame(&mut harness, "semantic-token SGR paint", |frame| {
        raw_line_containing(frame, "Intro")
            .map(|line| has_indexed_fg(line, 201))
            .unwrap_or(false)
    });

    let intro = raw_line_containing(&painted, "Intro").unwrap();
    assert!(
        has_indexed_fg(intro, 87),
        "wiki inner segments must render the link-like color: {intro}"
    );
    let directive = raw_line_containing(&painted, "::file").unwrap();
    assert!(
        has_indexed_fg(directive, 93),
        "directive keyword must render the muted color: {directive}"
    );
    let closer = raw_line_containing(&painted, "::end-block").unwrap();
    assert!(
        has_indexed_fg(closer, 93),
        "closer must render the muted color: {closer}"
    );
    let fenced = raw_line_containing(&painted, "let fenced").unwrap();
    for index in [201, 93, 87] {
        assert!(
            !has_indexed_fg(fenced, index),
            "fenced-code content must not carry Darkmatter styling: {fenced}"
        );
    }

    // Live repaint: disabling via .dmls.toml + didChangeConfiguration must
    // visibly clear the pane; re-enabling must visibly repaint it.
    harness.send_key("Escape").unwrap();
    harness.send_text(b":lua DmlsToggle(false)").unwrap();
    harness.send_key("Enter").unwrap();
    wait_for_frame(&mut harness, "token colors to clear", |frame| {
        raw_line_containing(frame, "Intro")
            .map(|line| !has_indexed_fg(line, 201))
            .unwrap_or(false)
    });

    harness.send_text(b":lua DmlsToggle(true)").unwrap();
    harness.send_key("Enter").unwrap();
    wait_for_frame(&mut harness, "token colors to repaint", |frame| {
        raw_line_containing(frame, "Intro")
            .map(|line| has_indexed_fg(line, 201))
            .unwrap_or(false)
    });
}
