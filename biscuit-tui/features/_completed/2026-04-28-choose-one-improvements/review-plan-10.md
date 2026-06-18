# Review 10 Implementation Plan

## Goal

Address every actionable finding in `review-10.md` for the ChooseOne
improvements feature and leave the `biscuit-tui` package area with passing
focused tests, passing full package tests (including the env-gated PTY/shell
verification suites), and zero clippy warnings/errors.

Review 10 confirms the feature is **production-ready**; every finding is
LOW severity. This plan implements findings 1–4 (correctness/coverage/docs)
and explicitly defers findings 5–6 (purely cosmetic refactors). Each phase
is independently verifiable and concludes with the same standard
verification pair:

- `cargo test -p biscuit-tui -p biscuit-tui-cli`
- `cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets -- -D warnings`

plus, where the phase touches gated suites, the env-gated PTY/shell tests.

> **Non-interactive context.** The orchestrator and any developer subagents
> are running non-interactively. Tasks must be deterministic and
> self-contained. Do not introduce prompts, manual inspection steps, or
> "ask the maintainer" branches. All decisions in this plan are pre-resolved
> below; in particular, finding 3 picks **option (a)** (keep the local
> parser, harden tests) and finding 5/6 are **deferred** (see "Deferred
> Items").

## Phase Overview

| # | Title | Findings | Verification |
|---|-------|----------|--------------|
| 1 | Shared PTY DSR responder helper | 1 | gated `choose_cli::pty` |
| 2 | `just test-pty` recipe runs all three gated suites | 2 | gated suites via the recipe |
| 3 | CRLF/BOM-tolerant `parse_md` test | 3 | unit tests in `option_sources.rs` |
| 4 | TOML `options = [...]` documentation | 4 | doc inspection + `--help` text |
| 5 | Final full-package verification + lint | (cross-cutting) | full suite + clippy + gated suites |

Finding 5 (extract `choice_normalize::hotkey` submodule) and finding 6
(shared hotkey-display test scaffolding) are NOT included as phases. See
"Deferred Items" at the bottom for the rationale.

## Phase 1 — Shared PTY DSR Responder Helper (Finding 1)

### Scope

Unblock `pty::choose_one_height_100_percent_runs_end_to_end` (currently
fails with exit 1 under `QUESTION_INTERACTIVE_PTY=1`) by giving the
choose-cli PTY harness the DSR cursor-position responder that
`keyboard_protocol.rs` already has. Per `pty-test-bugs.md` Bug 2, this is
"Option A" (the long-term fix).

### Implementation Steps

1. **Create the shared helper module.** Add a new file at
   `biscuit-tui/cli/tests/common/pty.rs`. The integration tests already
   share scaffolding via `cli/tests/common/mod.rs`, so the new module
   slots into the same place.

   The module MUST expose, at minimum:

   - `pub fn answer_cursor_position_request(session: &mut expectrl::session::OsSession)`
     — drains the master FD until it sees `\x1b[6n`, then writes
     `\x1b[1;1R` and returns. This is the same behavior as
     `keyboard_protocol.rs:55-82`. Match that signature exactly so the
     existing call site can be updated to call into the shared helper
     without re-reading raw bytes.

   The helper MUST:
   - Use the same drain loop / interrupted/wouldblock/timedout match
     arms as the existing keyboard-protocol implementation.
   - Use a 2s deadline for receiving the DSR query, then a 300ms
     post-response settle sleep (matching the existing implementation).
   - Reset the session expect timeout to 10s on exit (matching the
     existing implementation), so callers downstream still see
     reasonable defaults.
   - Be `#[cfg(unix)]`-gated, like the harnesses that consume it
     (the choose-cli `mod pty` and `mod keyboard_protocol` are both
     `#[cfg(unix)]`).
   - Carry a `//!` module doc comment explaining: "Shared PTY helpers
     for `biscuit-tui-cli` integration tests. Any test that spawns
     `question` with `--height` (Inline-viewport mode) MUST call
     `answer_cursor_position_request` before sending input; otherwise
     the binary blocks during initialisation while crossterm waits
     synchronously for the DSR (`\x1b[6n`) reply." Cite
     `pty-test-bugs.md` Bug 2.

2. **Wire `cli/tests/common/mod.rs` to expose the new submodule.**
   Add `pub mod pty;` (gated `#[cfg(unix)]` if the existing module is
   not already `cfg(unix)`-gated). Confirm with a quick read of the
   existing `mod.rs` (currently a flat helper file with
   `run_question_in_pty`/`clean_terminal_text`); adding a sibling
   module is the lowest-friction change.

3. **Update `cli/tests/keyboard_protocol.rs` to use the shared helper.**
   Remove the inline `fn answer_cursor_position_request(...)` at
   lines 55-82 and replace its call sites with
   `common::pty::answer_cursor_position_request(&mut p)` (or the
   appropriate `use` path; `keyboard_protocol.rs` already includes
   `mod common;` per the existing test layout — confirm and add it if
   missing). This deduplication is mandatory: the helper must have
   exactly one source of truth so future fixes do not drift between
   harnesses. If `keyboard_protocol.rs` currently does not declare
   `mod common;`, add `#[path = "common/mod.rs"] mod common;` at the
   top of the file (mirroring what `choose_cli.rs` does).

4. **Update `cli/tests/choose_cli.rs::pty::spawn_question` to call the
   helper when `--height` or `-h` is in `args`.**
   Concretely, modify the existing `spawn_question` at line ~804 so
   that, after `Session::spawn(command)` and the existing
   `set_expect_timeout`, it inspects `args` and conditionally invokes
   the responder:

   ```rust
   fn args_use_inline_viewport(args: &[&str]) -> bool {
       let mut iter = args.iter().peekable();
       while let Some(arg) = iter.next() {
           // Long form: `--height <value>` or `--height=<value>`.
           if *arg == "--height" || arg.starts_with("--height=") {
               return true;
           }
           // Short form: `-h <value>`. Note that `--help` and
           // `--height` both contain "-h" so we explicitly match the
           // bare short flag.
           if *arg == "-h" {
               return true;
           }
       }
       false
   }

   fn spawn_question(args: &[&str]) -> OsSession {
       let binary = assert_cmd::cargo::cargo_bin("question");
       let mut command = Command::new(binary);
       command.args(args);
       let mut p = Session::spawn(command).expect("spawn question under PTY");
       p.set_expect_timeout(Some(Duration::from_secs(5)));
       if args_use_inline_viewport(args) {
           super::common::pty::answer_cursor_position_request(&mut p);
       }
       p
   }
   ```

   - Add `#[path = "common/mod.rs"] mod common;` at the top of
     `choose_cli.rs` if it is not already present, so the
     `super::common::pty` path resolves. (The existing
     `cli/tests/choose_cli.rs` already references `mod common;` for
     the assert_cmd-based helpers; verify and reuse the same import
     path.)

   - The `args_use_inline_viewport` check MUST live next to
     `spawn_question` in `choose_cli.rs` (not in the shared module),
     because the heuristic — "this binary uses Inline viewport when
     `--height`/`-h` is on the command line" — is a `question`-binary
     specific contract, not a general PTY contract.

5. **Confirm the failing test now passes.** Re-read
   `pty::choose_one_height_100_percent_runs_end_to_end` at
   `cli/tests/choose_cli.rs:983`. No test-body changes should be
   required; the responder runs implicitly via `spawn_question`. If the
   test still fails, check the deadline (Bug 1's documented
   wait-loop drain pattern at `wait_exit_code_within` is already in
   place; do not regress it).

### Tests to Add or Adjust

- **No new test files.** The unblock is for an existing test.
- **Adjust** the call sites in `keyboard_protocol.rs` to use the shared
  helper (Step 3 above). The 4 keyboard-protocol tests must keep
  passing.
- **Verify**
  `pty::choose_one_height_100_percent_runs_end_to_end`,
  `pty::esc_restores_initial_and_exits_with_code_0`,
  `pty::ctrl_c_exits_with_code_130`, and
  `pty::choose_many_ctrl_a_then_submit_writes_all_values` all pass
  under `QUESTION_INTERACTIVE_PTY=1`.
- **Update `pty-test-bugs.md`** to mark Bug 2 as resolved
  (`status: open` → `status: resolved`) and tick the matching
  checkbox in the "Status" section. Cite the new helper path and the
  `spawn_question` change.

### Focused Verification

```bash
# Verifies the previously-failing inline-viewport test now passes.
QUESTION_INTERACTIVE_PTY=1 \
  cargo test -p biscuit-tui-cli --test choose_cli \
  pty::choose_one_height_100_percent_runs_end_to_end -- --nocapture

# Ensures the dedup'd keyboard-protocol harness still passes.
RUN_PTY_TESTS=1 \
  cargo test -p biscuit-tui-cli --test keyboard_protocol -- --nocapture

# Full choose_cli pty suite (4 tests).
QUESTION_INTERACTIVE_PTY=1 \
  cargo test -p biscuit-tui-cli --test choose_cli pty:: -- --nocapture
```

### Verification Checklist

- [ ] `cli/tests/common/pty.rs` exists, exports
      `answer_cursor_position_request`, and is `#[cfg(unix)]`.
- [ ] `cli/tests/common/mod.rs` re-exports the submodule (`pub mod pty;`).
- [ ] `cli/tests/keyboard_protocol.rs` no longer defines its own
      `answer_cursor_position_request`; it calls the shared helper.
- [ ] `cli/tests/choose_cli.rs::pty::spawn_question` calls the
      responder iff `args_use_inline_viewport(args)` returns true.
- [ ] `QUESTION_INTERACTIVE_PTY=1 cargo test -p biscuit-tui-cli --test
      choose_cli pty::choose_one_height_100_percent_runs_end_to_end`
      passes.
- [ ] All 4 `choose_cli::pty` tests pass under
      `QUESTION_INTERACTIVE_PTY=1`.
- [ ] All 4 `keyboard_protocol` tests pass under `RUN_PTY_TESTS=1`.
- [ ] `pty-test-bugs.md` Bug 2 marked resolved with the new helper
      path cited.
- [ ] `cargo test -p biscuit-tui -p biscuit-tui-cli` passes.
- [ ] `cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets --
      -D warnings` is clean.

## Phase 2 — `just test-pty` Recipe (Finding 2)

### Scope

Spec § "Verification Gates" requires PTY-driven verification of all
completion and keyboard-modifier claims. Today those gates only run if a
developer remembers to set `RUN_PTY_TESTS=1`, `RUN_SHELL_TESTS=1`, and
`QUESTION_INTERACTIVE_PTY=1` by hand. Add a `just test-pty` recipe to
`biscuit-tui/justfile` that flips all three flags and runs the three
gated suites in one shot.

### Implementation Steps

1. **Add the recipe to `biscuit-tui/justfile`.** Insert after the
   existing `test` recipe (line 42-44), preserving the file's
   formatting/casing conventions:

   ```just
   # Run the env-gated PTY/shell verification suites
   # (keyboard protocol, completions shell, choose-cli PTY).
   # Requires bash + zsh on the host. macOS/Linux only.
   test-pty:
       @RUN_PTY_TESTS=1 cargo test -p {{CLI}} --test keyboard_protocol -- --nocapture
       @RUN_SHELL_TESTS=1 cargo test -p {{CLI}} --test completions_shell -- --nocapture
       @QUESTION_INTERACTIVE_PTY=1 cargo test -p {{CLI}} --test choose_cli pty:: -- --nocapture
   ```

   Notes on shape:
   - The recipe uses `{{CLI}}` (already defined as `biscuit-tui-cli` at
     line 21 of the justfile) for consistency with the existing
     `test` / `lint` / `build` recipes.
   - Each invocation gets its own env-var prefix so the three suites
     are independent — no cross-contamination if a future suite
     decides to consult only one of the env vars.
   - `--nocapture` is included so PTY-related stderr/stdout is
     visible during local debugging; without it, intermittent macOS
     PTY back-pressure issues are hard to diagnose. (This mirrors the
     existing review-9 / review-10 invocation patterns documented in
     `review-10.md:171`.)
   - The third invocation filters to `pty::` so only the four
     PTY-gated tests in `choose_cli.rs` run; the rest of `choose_cli`
     already runs under default `cargo test` and would just duplicate
     work here.
   - The leading `@` on each line suppresses just's command echoing
     (matching the existing recipes).
   - Do NOT chain the three commands with `&&` — each must run even
     if a prior one fails so the developer sees the complete failure
     surface. (Just runs each line as a separate process by default;
     a failure aborts the recipe, which is the desired
     verification-gate semantics.)

2. **Confirm bash + zsh availability.** The `completions_shell` suite
   spawns real bash and zsh shells. On macOS both are available by
   default. No new install step is required for the recipe; document
   the dependency in the recipe comment (above).

3. **Do NOT add the recipe to the default `test` recipe.** Spec
   § "Verification Gates" says these suites must run before merge,
   not on every local `just test`. They are slow and PTY-flaky on
   some macOS versions. Treat `just test-pty` as the documented
   pre-merge gate; CI will pick it up in a later sweep.

4. **Document the recipe.** Add a one-line entry to
   `biscuit-tui/cli/README.md` (or `biscuit-tui/lib/README.md`,
   whichever already lists test recipes; if neither does, add it
   under a new "Verification Gates" subsection in
   `biscuit-tui/cli/README.md`):

   > Run `just test-pty` from `biscuit-tui/` to execute the
   > env-gated PTY/shell verification suites required by the
   > Verification Gates contract.

### Tests to Add or Adjust

- **No new Rust tests.** The existing gated suites are the test
  surface; this phase is purely about making them runnable via a
  documented entry point.
- The recipe itself is the test: invoking `just test-pty` from the
  `biscuit-tui/` directory MUST exit 0 with all three suites passing.

### Focused Verification

```bash
cd biscuit-tui
just test-pty
```

Expected output:

```
running 4 tests
... keyboard_protocol passes (4/4)
running 8 tests
... completions_shell passes (8/8)
running 4 tests
... choose_cli pty passes (4/4)  ← Phase 1 must be complete first
```

If `keyboard_protocol` or `completions_shell` fail with anything other
than the count above, file the regression separately — they were green
at the time of review-10 (`review-10.md:171-172`).

### Verification Checklist

- [ ] `biscuit-tui/justfile` has a `test-pty` recipe that runs the
      three suites with `RUN_PTY_TESTS=1`, `RUN_SHELL_TESTS=1`, and
      `QUESTION_INTERACTIVE_PTY=1` respectively.
- [ ] Recipe uses `{{CLI}}` for consistency with the existing recipes.
- [ ] `just test-pty` invoked from `biscuit-tui/` exits 0.
- [ ] `keyboard_protocol` reports 4/4 passing.
- [ ] `completions_shell` reports 8/8 passing.
- [ ] `choose_cli pty::` reports 4/4 passing (depends on Phase 1).
- [ ] README change is in place pointing at the recipe.
- [ ] `cargo test -p biscuit-tui -p biscuit-tui-cli` (default suite)
      still passes — adding the recipe must not perturb default test
      runs.
- [ ] `cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets --
      -D warnings` is clean.

## Phase 3 — CRLF/BOM-Tolerant `parse_md` Test (Finding 3)

### Scope

`parse_md` (`cli/src/option_sources.rs:554-585`) is hand-rolled and
slices on `"---"` and `"\n---"`. Review 10 notes it does not handle
BOM, CRLF-only line endings, or front-matter that begins with a
newline before `---`. Tech-design language is permissive ("if
available"), so the **expedient fix** chosen here is **option (a)**:
keep the local parser and add a CRLF/BOM-tolerant unit test that
documents the supported envelope. Option (b) (introduce a
`biscuit-file` dependency) is explicitly out of scope for this
review — it broadens the workspace dependency graph and is not
required by the spec.

The local parser MUST handle:

- UTF-8 BOM (`\u{feff}` / `[0xEF, 0xBB, 0xBF]`) at the start of file.
- CRLF line endings (`\r\n`) throughout the file.
- Leading whitespace/blank lines before the opening `---`.

If the existing parser does not handle one of these cases, this phase
adds the **minimum tolerance fix** to `parse_md` to make the new test
pass, **without** broadening the parser's contract beyond
"BOM/CRLF-tolerant YAML frontmatter." Anything more elaborate
(non-YAML frontmatter, TOML frontmatter, etc.) remains out of scope.

### Implementation Steps

1. **Audit `parse_md`'s current behavior against the new test.**
   Read `cli/src/option_sources.rs:554-585`. The current parser:
   - Trims leading whitespace via `body.trim_start()` (line 557) — so
     leading newlines are already tolerated.
   - Strips a single literal `---` prefix (line 558) — this is where
     a UTF-8 BOM fails today, because `trim_start()` does not remove
     `\u{feff}` (BOM is not classified as whitespace by Rust's
     default `char::is_whitespace`).
   - Searches for `\n---` (line 561) to find the close — this is
     where CRLF fails today, because the line ending before the
     closing fence is `\r\n`, leaving the search needle as `\r\n---`
     (which contains the substring `\n---` only by coincidence; the
     trailing `\r` then leaks into `frontmatter`, which serde_yaml_ng
     handles, but anything more pathological breaks).

   Concretely, the BOM case is the only one definitely broken; the
   CRLF case is the one most worth proving correct.

2. **Harden `parse_md` minimally.** Apply this two-line change before
   the existing `trim_start()`:

   ```rust
   fn parse_md(path: &Path, prop: &str) -> Result<Vec<RawOption>, SourceError> {
       let body = fs::read_to_string(path)?;
       // Strip an optional UTF-8 BOM, then normalize CRLF to LF so
       // the literal `\n---` close-fence search is robust.
       let body = body.strip_prefix('\u{feff}').unwrap_or(&body);
       let body = body.replace("\r\n", "\n");
       // Existing logic operates on `body` from here on.
       let trimmed = body.trim_start();
       let after_first = trimmed.strip_prefix("---").ok_or_else(|| {
           SourceError::Parse("markdown file must have frontmatter delimited by ---".into())
       })?;
       // …unchanged below…
   }
   ```

   - The BOM strip uses `strip_prefix('\u{feff}')` so a single-byte
     UTF-8 BOM is removed. (BOM as a `char` is one Unicode scalar
     value; `strip_prefix` accepts a `char` argument.)
   - The CRLF normalisation is a single `String::replace` on the
     full body. This is O(n) and runs only at parse time, so the
     extra allocation is negligible.
   - Do NOT touch the rest of the function. The downstream YAML
     parser already handles whatever the normalisation leaves behind.

3. **Add the new unit test in
   `cli/src/option_sources.rs::tests`.** Place it next to the
   existing `parse_md_*` tests (search the file for `parse_md_` —
   if no parse_md tests exist yet, place the new test in the same
   `mod tests` block immediately after the `parse_file_*` tests
   added in review-9). The test must cover both BOM and CRLF in a
   single fixture so a regression in either reproduces the failure:

   ```rust
   #[test]
   fn parse_md_tolerates_bom_and_crlf_line_endings() {
       use std::io::Write;
       let dir = std::env::temp_dir();
       let path = dir.join("question_review10_md_bom_crlf.md");
       // Build a frontmatter document with:
       //   * UTF-8 BOM at the very start
       //   * CRLF line endings throughout
       //   * Frontmatter property `colors` as a YAML array
       //   * A leading blank line before the opening `---`
       let body = b"\xef\xbb\xbf\r\n---\r\ncolors:\r\n  - Red\r\n  - Green\r\n  - Blue\r\n---\r\n# Body content\r\n";
       std::fs::File::create(&path)
           .unwrap()
           .write_all(body)
           .unwrap();

       let result = parse_md(&path, "colors").expect("parse_md should tolerate BOM + CRLF");
       assert_eq!(labels(&result), vec!["Red", "Green", "Blue"]);

       std::fs::remove_file(&path).unwrap();
   }
   ```

   - Use a deterministic, review-tagged tempfile name
     (`question_review10_md_bom_crlf.md`) so parallel test runs do
     not collide and so the test fixture is recognisable in
     `/tmp` if cleanup is skipped.
   - The fixture writes raw bytes (not `fs::write(&path, body)`
     which expects a `&str`) so the BOM survives unaltered; we go
     through `File::create` + `write_all` to keep the bytes
     literal.
   - Reuse the existing `labels(&items)` helper at line ~591 in the
     same `mod tests` block (it is already in scope inside the
     test module).

4. **Add a second, smaller unit test for the BOM-only and CRLF-only
   cases** so the parser's tolerance is documented per dimension and
   future regressions are easier to triage:

   ```rust
   #[test]
   fn parse_md_tolerates_utf8_bom_with_lf_line_endings() {
       use std::io::Write;
       let dir = std::env::temp_dir();
       let path = dir.join("question_review10_md_bom_only.md");
       let body = b"\xef\xbb\xbf---\nopts:\n  - a\n  - b\n---\n";
       std::fs::File::create(&path).unwrap().write_all(body).unwrap();
       let result = parse_md(&path, "opts").expect("parse_md should tolerate BOM");
       assert_eq!(labels(&result), vec!["a", "b"]);
       std::fs::remove_file(&path).unwrap();
   }

   #[test]
   fn parse_md_tolerates_crlf_line_endings_without_bom() {
       use std::io::Write;
       let dir = std::env::temp_dir();
       let path = dir.join("question_review10_md_crlf_only.md");
       let body = b"---\r\nopts:\r\n  - a\r\n  - b\r\n---\r\n";
       std::fs::File::create(&path).unwrap().write_all(body).unwrap();
       let result = parse_md(&path, "opts").expect("parse_md should tolerate CRLF");
       assert_eq!(labels(&result), vec!["a", "b"]);
       std::fs::remove_file(&path).unwrap();
   }
   ```

5. **Do not change the surrounding error vocabulary.** The existing
   `SourceError::Parse` and `SourceError::MdPropNotArray` variants
   stay; the new tests do not exercise them.

### Tests to Add or Adjust

- **Add** in `cli/src/option_sources.rs::tests`:
  - `parse_md_tolerates_bom_and_crlf_line_endings`
  - `parse_md_tolerates_utf8_bom_with_lf_line_endings`
  - `parse_md_tolerates_crlf_line_endings_without_bom`
- **No** existing tests should change. The pre-existing happy-path
  `parse_md` tests must still pass against the hardened parser.

### Focused Verification

```bash
cargo test -p biscuit-tui-cli option_sources::tests::parse_md_
cargo test -p biscuit-tui-cli option_sources::tests
```

Confirm all `parse_md_*` tests pass and no other `option_sources`
test regresses.

### Verification Checklist

- [ ] `parse_md` strips a UTF-8 BOM and normalises CRLF to LF before
      its existing slicing logic runs.
- [ ] `parse_md_tolerates_bom_and_crlf_line_endings` passes.
- [ ] `parse_md_tolerates_utf8_bom_with_lf_line_endings` passes.
- [ ] `parse_md_tolerates_crlf_line_endings_without_bom` passes.
- [ ] All other `option_sources::tests` continue to pass.
- [ ] `cargo test -p biscuit-tui -p biscuit-tui-cli` passes.
- [ ] `cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets --
      -D warnings` is clean.

## Phase 4 — Document the TOML `options = [...]` Convention (Finding 4)

### Scope

`parse_toml` accepts either a top-level array (TOML extensions only,
not standard TOML) **or** a table with an `options = [...]` key. In
practice, standard TOML cannot represent a top-level bare array, so
users will always use the `options = [...]` form. This phase documents
that contract in two places:

1. The user-facing CLI reference (`docs/cli-reference.md`).
2. The clap `--file` long help text on `choose-one` and `choose-many`.

No code behavior changes — this is documentation only.

### Implementation Steps

1. **Update `biscuit-tui/docs/cli-reference.md`.** Add a new subsection
   under the existing `--file` documentation (search the file for
   `--file`; if no `--file` subsection exists, add one under
   "Subcommands" → "choose-one" → "Source Flags"). The subsection
   MUST contain:

   - A bullet listing the supported extensions:
     `json`, `jsonl`, `ndjson`, `yaml`/`yml`, `toml`, `csv`.
   - An explicit "TOML convention" callout:

     > **TOML convention.** Standard TOML cannot represent a top-level
     > bare array, so a TOML options file MUST use the
     > `options = [...]` table form. Files structured with any other
     > top-level key (e.g. `colors = [...]`) will fail with
     > `option file must contain an array`.
     >
     > Example:
     >
     > ```toml
     > options = ["Red", "Green", "Blue"]
     > ```
     >
     > Or, with explicit labels and values:
     >
     > ```toml
     > [[options]]
     > label = "Red Delicious"
     > value = "apple"
     >
     > [[options]]
     > label = "Cavendish"
     > value = "banana"
     > ```

   - A pointer to `--md <file> <prop>` for Markdown frontmatter
     (one-line cross-reference; the new section is about `--file`).

2. **Update the `--file` clap long help on `choose-one`.**
   `cli/src/commands/choose_one.rs:42-44` currently reads:

   ```rust
   /// Path to a file containing options (JSON, JSONL, YAML, TOML, or CSV).
   #[arg(long, value_name = "PATH")]
   pub file: Option<PathBuf>,
   ```

   Replace the doc comment with:

   ```rust
   /// Path to a file containing options. Supported formats are
   /// `json`, `jsonl`, `ndjson`, `yaml` (or `yml`), `toml`, and
   /// `csv`. The file's top level must be an array of strings or
   /// an array of objects with `label` / `value` / `hotkey` keys.
   ///
   /// **TOML note:** standard TOML cannot represent a top-level
   /// bare array, so a TOML options file must use the
   /// `options = [...]` table form (e.g. `options = ["Red",
   /// "Green"]`). Other top-level keys (e.g. `colors = [...]`)
   /// will be rejected with `option file must contain an array`.
   #[arg(long, value_name = "PATH")]
   pub file: Option<PathBuf>,
   ```

   - Use a triple-slash doc comment so clap surfaces the long help
     under `question choose-one --help`. The current comment is
     already a doc comment, so this is a content swap, not a syntax
     change.
   - Avoid an explicit `# Heading` H1 inside `///` (per the project
     rustdoc convention in CLAUDE.md). Use `**TOML note:**` as a
     bold inline emphasis instead.

3. **Repeat step 2 verbatim for `choose-many`.**
   `cli/src/commands/choose_many.rs:42-44` has the identical comment
   shape. Apply the same edit. The two help texts MUST stay in sync —
   any future edit should touch both files together. (A shared
   constant is overkill for two short doc strings.)

4. **Do not change `parse_toml`'s body or error message.** The error
   surfaced today (`option file must contain an array`) is still
   technically correct; this phase only documents the convention so
   users understand what shape "an array" actually means in TOML.

### Tests to Add or Adjust

This phase is documentation-only. The following deterministic
verification checks are sufficient:

- **Read-back check** (mandatory): after editing
  `docs/cli-reference.md`, re-read the file and confirm the new
  "TOML convention" subsection contains the literal string
  `options = [...]` and the example.
- **Help-text check** (mandatory): run

  ```bash
  cargo run -p biscuit-tui-cli --bin question -- choose-one --help \
      | grep -A 6 -- "--file"
  ```

  and confirm the output mentions `options = [...]`. Repeat for
  `choose-many`. (Both subcommands must reflect the updated long
  help.)

- **Existing `parse_toml` tests must still pass.** Confirm by
  running:

  ```bash
  cargo test -p biscuit-tui-cli parse_toml
  cargo test -p biscuit-tui-cli parse_file_toml
  ```

  No assertions change — the help-text update must not affect parser
  behavior.

### Focused Verification

```bash
cargo build -p biscuit-tui-cli
cargo run -p biscuit-tui-cli --bin question -- choose-one --help \
    | grep -F "options = [...]"
cargo run -p biscuit-tui-cli --bin question -- choose-many --help \
    | grep -F "options = [...]"
cargo test -p biscuit-tui-cli parse_toml
```

Each `grep` must produce at least one matching line; the test command
must pass.

### Verification Checklist

- [ ] `biscuit-tui/docs/cli-reference.md` has a new "TOML
      convention" subsection mentioning `options = [...]` and a
      worked example.
- [ ] `cli/src/commands/choose_one.rs` `--file` long help mentions
      `options = [...]` and the standard-TOML caveat.
- [ ] `cli/src/commands/choose_many.rs` `--file` long help mentions
      `options = [...]` and the standard-TOML caveat (matching
      `choose_one.rs` text).
- [ ] `question choose-one --help` and
      `question choose-many --help` both surface the new TOML note.
- [ ] `cargo test -p biscuit-tui -p biscuit-tui-cli` passes.
- [ ] `cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets --
      -D warnings` is clean.

## Phase 5 — Final Verification + Lint Cleanup

### Scope

Prove the full `biscuit-tui` package area is clean after Phases 1–4
land. This phase exists because Phase 1 changes the test harness,
Phase 2 adds a build-system entry point, Phase 3 changes a parser, and
Phase 4 changes clap help text — each on its own is independently
verifiable, but a final cross-cutting sweep is required before the
review can be marked closed.

### Required Verification Commands

Run from the repository root:

```bash
# Default suite — must pass with zero failures.
cargo test -p biscuit-tui -p biscuit-tui-cli

# Lint with -D warnings — zero warnings, zero errors.
cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets -- -D warnings

# Re-run default suite after any clippy-induced edits.
cargo test -p biscuit-tui -p biscuit-tui-cli

# Env-gated PTY/shell verification suites — must all pass.
cd biscuit-tui && just test-pty
```

The last command depends on Phase 2 having shipped the `test-pty`
recipe and on Phase 1 having unblocked the previously-failing
`pty::choose_one_height_100_percent_runs_end_to_end` test.

### Lint Expectations

- Fix any clippy warnings/errors introduced by Phases 1–4.
- Do not suppress lints unless the suppression is narrowly scoped and
  locally justified (and document why in a `// SAFETY:` or
  `// allow:` comment).
- After lint fixes, rerun the full default test suite to confirm no
  behavioral regression.

### Documentation Sweep

- `biscuit-tui/cli/README.md` mentions `just test-pty` (added in
  Phase 2).
- `biscuit-tui/docs/cli-reference.md` documents the TOML convention
  (added in Phase 4).
- `biscuit-tui/features/2026-04-28-choose-one-improvements/pty-test-bugs.md`
  marks Bug 2 as resolved (updated in Phase 1).
- No other docs change is required by this review.

### Verification Checklist

- [ ] `cargo test -p biscuit-tui -p biscuit-tui-cli` passes (892+ tests
      after the new `parse_md` tests, all green).
- [ ] `cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets --
      -D warnings` is clean.
- [ ] `cd biscuit-tui && just test-pty` passes:
  - `keyboard_protocol`: 4/4 pass.
  - `completions_shell`: 8/8 pass.
  - `choose_cli pty::`: 4/4 pass.
- [ ] `pty-test-bugs.md` status reflects Bug 2 resolved.
- [ ] `docs/cli-reference.md` reflects the TOML convention.
- [ ] `cli/README.md` references `just test-pty`.

## Completion Criteria

The review is complete when:

- **Finding 1 resolved.** A shared
  `cli/tests/common/pty.rs::answer_cursor_position_request` exists,
  is consumed by both `keyboard_protocol.rs` and `choose_cli.rs::pty`,
  and `pty::choose_one_height_100_percent_runs_end_to_end` passes
  under `QUESTION_INTERACTIVE_PTY=1`. `pty-test-bugs.md` Bug 2 is
  marked resolved.
- **Finding 2 resolved.** `biscuit-tui/justfile` exposes a
  `test-pty` recipe that runs the three gated suites with the
  appropriate env vars, and invoking it from `biscuit-tui/` exits 0
  with all three suites green. The recipe is referenced from
  `cli/README.md`.
- **Finding 3 resolved.** `parse_md` tolerates UTF-8 BOM and CRLF
  line endings, with three new tests
  (`parse_md_tolerates_bom_and_crlf_line_endings`,
  `parse_md_tolerates_utf8_bom_with_lf_line_endings`,
  `parse_md_tolerates_crlf_line_endings_without_bom`) covering the
  BOM, CRLF, and combined cases.
- **Finding 4 resolved.** `docs/cli-reference.md` and the `--file`
  clap help on both `choose-one` and `choose-many` document the
  TOML `options = [...]` convention.
- `cargo test -p biscuit-tui -p biscuit-tui-cli` passes.
- `cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets --
  -D warnings` is clean.
- `just test-pty` passes from `biscuit-tui/`.

## Deferred Items

These review-10 findings are **explicitly not implemented** in this
plan. They are tracked here so the developer (and any future
reviewer) can pick them up as standalone follow-ups without
re-deriving the rationale.

### Finding 5 — Extract `choice_normalize::hotkey` submodule

**Status:** deferred (cosmetic).

**Reason:** `cli/src/choice_normalize.rs` is ~1,200 lines but is
internally well-organized. Splitting it into
`choice_normalize::hotkey` is purely a navigation-ergonomics win;
it changes no observable behavior, no public API surface, and no
test outcome. Doing it in this review pulls a large refactor under
the same commit set as small correctness/coverage/docs fixes,
which makes the review's blast radius bigger than necessary.

**Pickup hint for a follow-up:** create
`cli/src/choice_normalize/hotkey.rs`, move `parse_hotkey_spec`,
`single_char`, `has_modifier_prefix`, `split_bracket_prefix`,
`effective_hotkey_for`, and `format_hotkey_spec` into it, leaving
`normalize_options` and the record-mapping logic in
`choice_normalize/mod.rs` (renamed from `choice_normalize.rs`).
Public re-exports must stay byte-for-byte identical so no caller
sees a break.

### Finding 6 — Shared hotkey-display test scaffolding

**Status:** deferred (cosmetic).

**Reason:** the seven `with_hotkey_display_*_survives_*` tests in
each of `ChooseOne` and `ChooseMany` are duplicated but correct.
Folding them into a `macro_rules!` generator or a
`tests/common/hotkey_display.rs` helper is a maintenance investment
for *future* test additions; today's coverage is already complete.
This review is about the production-readiness gate; cleanup of
green tests is the wrong scope.

**Pickup hint for a follow-up:** the natural shape is a
`macro_rules! hotkey_display_override_tests` macro that takes a
state constructor and a widget constructor and emits the seven
test functions. The helper lives next to the existing
`tests/common/mod.rs` so both component test files can `use` it.

## Risks and Open Questions

- **Phase 1 `args_use_inline_viewport` heuristic.** The check looks
  for `--height` (long, with or without `=`) and bare `-h`. If a
  future flag adds another path to inline mode, the helper will not
  be invoked and the test will hang on the DSR query again. Mitigation:
  the helper is cheap to call unconditionally; if a future audit
  shows more inline entrypoints, drop the gate and always call it.
  We keep the gate today only to avoid adding 2s of latency to the
  three non-inline `choose_cli pty::` tests.
- **Phase 2 `test-pty` on hosts without zsh.** The
  `completions_shell` suite spawns real zsh. macOS has zsh by
  default; some Linux distros do not. The recipe documents this in
  its comment. CI integration is out of scope for this review;
  finding 2 is satisfied as long as the recipe exists, runs locally,
  and is documented.
- **Phase 3 `parse_md` BOM-strip semantics.** The BOM strip removes
  exactly one leading `\u{feff}`. If a file has multiple BOMs (rare
  but possible from misbehaving editors), only the first is stripped
  and the second leaks into `trim_start()` (which does not classify
  BOM as whitespace). This matches the option (a) "minimum tolerance
  fix" scope; option (b) (route through `biscuit-file`) would handle
  this, but is out of scope.
- **Phase 4 help-text drift.** The two `--file` help strings on
  `choose_one.rs` and `choose_many.rs` are now non-trivially
  identical. Future edits must touch both. A shared constant is
  unnecessary for two short doc strings, but if either subcommand
  acquires a third dimension of variation, revisit the duplication.
