---
ready: true
agent: rust-developer
status: planned
related:
  - claudine/lib/src/harness/report.rs
  - claudine/lib/src/harness/parse.rs
  - claudine/lib/src/harness/model.rs
  - claudine/features/_completed/2026-05-02-validation-reporter/spec.md
  - claudine/features/_completed/2026-05-02-validation-reporter/review-1.md
---

# Review-Plan 1 — Address review-1 findings on validation-reporter

This plan addresses every finding from `review-1.md` on the
`2026-05-02-validation-reporter` feature. Phases are executed one at a time
by a `rust-developer` subagent. Each phase is self-contained: it lists files
to touch, the specific changes, and the tests to add or update, plus a phase
acceptance gate. **Phase 7 is the final verification gate** and must pass
before this plan is considered closed.

Working directory for all `cargo`/`just` invocations:
`/Users/ken/.claudine/worktrees/rusty-biscuit/claudine`.

> **Workspace note.** Never `cargo build` at the repo root. Always pass
> `-p claudine`. The package lives in workspace member `claudine/lib` and
> `claudine/cli`.

---

## Phase 1 — Extract a pure render-to-string writer for failure blocks

### Goal

Make terminal output of `report_check_outcomes` and `render_failure_block`
testable as plain strings. This unblocks every Level 1 asserted text test in
later phases and is the foundation the review's High finding requires.

Today both functions write directly to `stderr` via `eprintln!`. The
formatting logic (header text, `in <path>:<lo>-<hi>` markup, YAML indent,
muted Reason line) is fused with I/O.

### Files to touch

- `claudine/lib/src/harness/report.rs`

### Changes

1. Introduce a private `Writer` abstraction that the renderers write to. It
   must work for both real stderr emission and in-memory capture in tests.
   Use the simplest shape the codebase tolerates:

   ```rust
   use std::fmt::Write as _;

   trait LineSink {
       fn write_line(&mut self, line: &str);
   }

   struct StderrSink;
   impl LineSink for StderrSink {
       fn write_line(&mut self, line: &str) { eprintln!("{line}"); }
   }

   #[derive(Default)]
   struct StringSink { buf: String }
   impl LineSink for StringSink {
       fn write_line(&mut self, line: &str) {
           self.buf.push_str(line);
           self.buf.push('\n');
       }
   }
   ```

2. Refactor private helpers so each emits via a `&mut dyn LineSink` instead
   of `eprintln!` directly. Concretely:

   - `emit_status` becomes `emit_status_to(sink, markup, state, term)`. Keep
     the existing `emit_status` as a one-line wrapper that calls
     `emit_status_to(&mut StderrSink, ...)` so all existing call sites
     (`report_source_file`, `report_phase_discovery`, etc.) keep working
     unchanged.
   - `render_failure_block` becomes
     `render_failure_block_to(sink, outcome, phase, term)`. Add a thin
     wrapper `render_failure_block(...)` that still writes to stderr, used
     by `report_check_outcomes`.
   - `report_check_outcomes` gains a new sibling
     `report_check_outcomes_to_string(report, term) -> String` that runs
     the full pass/fail dispatch through a `StringSink` and returns the
     captured output. Keep `report_check_outcomes` as the stderr-emitting
     entry point so external callers do not change.

3. The legacy fallback path (failing outcome with `source: None`) must also
   be routed through the sink so it is testable. The existing 2-line shape
   (status row + `  Reason: <dim>...</dim>`) must be preserved byte-for-byte
   in stderr; only the I/O seam changes.

4. No public API changes. Everything new is `pub(crate)` or test-only.
   Existing public functions (`report_check_outcomes`, `report_source_file`,
   …) keep their signatures.

### Tests to add (in `report.rs::tests`)

- `report_check_outcomes_to_string_pass_renders_single_compact_row`:
  builds a single-passing report, calls
  `report_check_outcomes_to_string`, asserts the output is exactly one
  non-empty line and contains the rendered markup substring (e.g.
  `the file /a exists`). Confirms compact pass-state output.

- `render_failure_block_to_string_emits_four_sections_for_full_source`:
  outcome with `passed=false`, `source: Some(...)` with `line_range:
  Some(7..=9)` and a multi-line `yaml_snippet`. Capture, then assert:
  - line 0 contains the failure header
    (`Pre-validation failed` / `Post-validation failed` per phase),
  - some line contains `in ` and the `:7-9` suffix,
  - some line begins with the four-space indent and contains the
    `file_exists` rule name from the snippet,
  - some line contains `Reason:` followed by the `failure_message`.

- `legacy_failure_path_without_source_emits_status_then_reason`:
  outcome with `source: None` and `failure_message: Some(...)`. Capture and
  assert exactly two non-empty lines (status row + Reason row), and that
  the second line begins with two leading spaces and contains the reason.

### Acceptance

- `cargo test -p claudine --lib harness::report::tests` passes.
- `cargo clippy -p claudine --lib --tests --no-deps -- -D warnings` is
  clean.
- Manual diff review: every existing call site of `emit_status` and
  `render_failure_block` still compiles and behaves identically when run
  against stderr.

---

## Phase 2 — Implement a conservative line-range span finder

### Goal

Address review finding #3. Populate `RuleSource.line_range` for the common
list and map forms of `pre_checks` / `post_checks` so failure blocks render
`path:42-44` whenever the location is unambiguous. Omit the range when
ambiguity is detected.

### Files to touch

- `claudine/lib/src/harness/parse.rs`
- (new) `claudine/lib/src/harness/parse/span.rs` *or* a private module
  inline at the bottom of `parse.rs`. Choice is up to the implementer;
  keep span helpers private.

### Changes

1. Read the source markdown file from disk inside `parse_harness_plan` (it
   already receives `source_path: &Path`). On `Err` from `fs::read_to_string`,
   skip span recovery — `line_range` stays `None`. Never propagate the IO
   error.

2. Extract the YAML frontmatter slice (`---\n...\n---`) using the existing
   pattern used elsewhere in claudine. Look for the first two `---` line
   delimiters; if either is missing, skip span recovery.

3. Implement a conservative span finder for `pre_checks` and `post_checks`:

   - **List form** (`pre_checks:` followed by `-` items at deeper indent):
     - Locate the `pre_checks:` / `post_checks:` line.
     - Walk subsequent lines while their indent is strictly greater than
       the parent's, treating each top-level `-` at the inner indent as a
       new rule entry.
     - For each rule entry, the span is the line of the `-` through the
       last consecutive line at the inner-rule indent level (so multi-line
       map values like `file_exists:` + `message:` are captured together).
     - Match list-entry index to the parsed rule index in declaration order.

   - **Map form** (`pre_checks:` directly followed by indented `name: value`
     keys):
     - Locate the `pre_checks:` line.
     - Walk subsequent lines while the indent is greater than the parent's.
     - Each rule's span starts at the line containing `name:` and ends at
       the last consecutive line at deeper indent (multi-line block values
       included).

4. If the finder cannot match the rule's parsed name to a unique span
   (e.g. duplicate keys, comment-mangled indentation, irregular whitespace),
   it returns `None` for that rule. Do not guess.

5. Thread the optional span result through `parse_single_validation` →
   `build_rule_source`. Add a new param `line_range: Option<RangeInclusive<usize>>`
   to `build_rule_source` and assign it. Keep the existing fallback when
   no span was recovered.

6. The finder is a pure function over `&str` text; expose it as
   `pub(crate) fn find_rule_spans(frontmatter_text: &str) -> SpanIndex`
   where `SpanIndex` exposes `pre_check(i)` and `post_check(i)` lookups
   keyed by rule declaration order.

### Tests to add (in `parse.rs::tests`)

- `find_rule_spans_list_form_returns_per_rule_ranges`: a frontmatter
  string with `pre_checks:` list of three rules, asserts the returned
  `SpanIndex` yields three contiguous, non-overlapping ranges in
  declaration order. Use 1-indexed lines.

- `find_rule_spans_map_form_returns_per_rule_ranges`: same as above for
  the shorthand map form.

- `find_rule_spans_multiline_map_value_extends_range`:
  `file_exists:` followed by an indented `message: "..."` key — assert
  the rule's range covers both lines.

- `find_rule_spans_returns_none_when_ambiguous`: duplicate top-level
  rule names → finder returns `None` for both. Asserts the conservative
  fallback.

- `parse_rules_carry_line_range_when_recoverable`:
  drive a real `parse_harness_plan` with a frontmatter file written to a
  `tempfile::NamedTempFile`, then assert
  `plan.pre_checks[0].source.unwrap().line_range == Some(2..=2)` (or the
  appropriate range). This replaces the obsolete `is_none()` assertion at
  `parse.rs:1630`.

- `parse_rules_no_line_range_when_source_file_unreadable`: pass a
  bogus path that does not exist on disk. The harness already accepts the
  path for resolution; assert that parsing still succeeds and `line_range`
  is `None`. Confirms the IO-failure fallback.

### Acceptance

- All new span tests pass.
- The pre-existing `parse_rules_carry_source_with_yaml_snippet` test is
  updated to no longer assert `line_range.is_none()` if the input frontmatter
  is feedable to the finder, or kept asserting `is_none()` if the test still
  uses the in-memory `serde_json::Value` path that has no source text. The
  test must remain green either way.
- `cargo test -p claudine --lib harness::parse::tests` passes.
- `cargo clippy -p claudine --lib --tests --no-deps -- -D warnings` clean.

### Risks / open questions

- Frontmatter delimiter detection assumes the `---\n…\n---` shape. Files
  with BOMs, CRLF line endings, or non-standard fences will fall through
  to the `None` fallback. Acceptable for v1; document inline.

---

## Phase 3 — Carry raw frontmatter text and prefer original YAML slice

### Goal

Address review finding #2. The displayed snippet should match what the
author actually wrote (preserving comments, quoting, anchors, and ordering).
Reconstructed YAML stays only as a labeled fallback.

### Files to touch

- `claudine/lib/src/harness/parse.rs`
- `claudine/lib/src/harness/model.rs` (only if a new field is added; see
  below)

### Changes

1. Decide on the storage shape for the raw snippet. Two options — pick
   option A unless it complicates downstream consumers:

   - **Option A (recommended):** Keep `RuleSource.yaml_snippet: String` as
     the single user-facing snippet field. When span recovery succeeds,
     populate it with the exact source slice (joined source lines for the
     rule's range, trailing newline normalized). When span recovery fails,
     fall back to the existing reconstructed YAML.

   - **Option B:** Add a sibling enum
     `pub enum YamlSnippetSource { Original, Reconstructed }` on
     `RuleSource` so the reporter and tests can label the fallback. Use
     this only if option A makes test labeling impossible.

2. Implement option A: inside `parse_single_validation` (or its caller),
   when a span is available, slice the corresponding lines out of the
   already-loaded frontmatter text and store them as `yaml_snippet`. The
   slice must:
   - Use 1-indexed inclusive line numbers.
   - Trim a single trailing newline only.
   - Preserve all interior whitespace, comments, quoting, and indentation
     exactly as authored.

3. When span recovery fails, fall back to the existing `serde_yaml_ng`
   reconstruction in `build_rule_source`.

4. Add an internal hint so tests can distinguish original vs reconstructed
   without exposing it publicly. Use a private boolean on the test path
   only (e.g. a `pub(crate)` helper `is_original_snippet(&RuleSource,
   &str) -> bool` that compares the snippet to the source slice). Do not
   leak this to the public API.

### Tests to add

- `parse_rules_yaml_snippet_preserves_authored_quoting`: write a
  frontmatter file with a single-quoted value
  (`file_exists: 'Cargo.toml'`). Assert the resulting `yaml_snippet`
  contains `'Cargo.toml'` exactly, not `Cargo.toml` (which is what
  reconstruction would emit by default).

- `parse_rules_yaml_snippet_preserves_inline_comments`: input includes a
  `# author comment` on the rule line. Assert the snippet retains the
  comment.

- `parse_rules_yaml_snippet_falls_back_when_span_unrecoverable`:
  pass a frontmatter shape that the span finder rejects (e.g. duplicate
  keys). Assert the snippet still parses through `serde_yaml_ng::from_str`
  (i.e. reconstruction kicked in).

- Update the existing `parse_rules_yaml_snippet_round_trips` test to
  remain valid for both the original-slice and reconstruction branches.

### Acceptance

- All new and existing parse tests pass.
- `cargo clippy -p claudine --lib --tests --no-deps -- -D warnings` clean.
- Manual review: `RuleSource.yaml_snippet` for a normal map-form rule
  loaded from a real file is byte-identical to its source slice.

---

## Phase 4 — Render YAML snippet without fence chrome

### Goal

Address review finding #4. The failure block currently delegates to
`darkmatter::markdown::YamlBlock`, which emits a `yaml` label and dark
code-block chrome unsuitable for an inline failure context. The spec asks
for a syntax-highlighted YAML snippet, indented two spaces, with no
surrounding box.

### Files to touch

- `claudine/lib/src/harness/report.rs`
- (possibly) `darkmatter/lib/src/markdown/highlighting/mod.rs` and
  `darkmatter/lib/src/markdown/yaml_block.rs` to expose a chrome-free
  highlighter helper if one does not already exist.

### Investigation step (do first)

Inspect `darkmatter/lib/src/markdown/highlighting/` for an existing public
helper that highlights raw lines without rendering the markdown code-block
header (`format_header_row`) or background fill. Likely candidates:

- `darkmatter::markdown::highlighting::CodeHighlighter` (if `pub`)
- A direct `syntect::easy::HighlightLines` usage already exposed through
  darkmatter

If a suitable helper exists, use it. If not, **add** a thin public helper
in darkmatter — for example:

```rust
// darkmatter/lib/src/markdown/highlighting/mod.rs (new pub fn)
pub fn highlight_yaml_lines(yaml: &str, term: &Terminal) -> Vec<String>;
```

that returns one ANSI-styled string per input line, with no header, no
background fill, and no fence borders. Use the existing theme-detection
path (`detect_code_theme` / `ColorMode`) so colors match the rest of
darkmatter's output. Add a `#[cfg(test)]` unit test in darkmatter that
asserts the output line count equals the input line count and that
`yaml` is not present as a label.

### Changes in claudine

1. In `render_failure_block_to`, replace the `YamlBlock::new(...)` +
   `for line in rendered.lines() { eprintln!("    {line}"); }` block with
   a call to the chrome-free highlighter.

2. Indent each highlighted line by exactly four spaces (current behavior),
   then route through the `LineSink` from Phase 1.

3. If highlighting fails for any reason, fall back to plain (un-highlighted)
   `yaml_snippet` lines with the same four-space indent. The user must
   never see a missing snippet.

### Tests to add

- `render_failure_block_yaml_section_has_no_yaml_label_or_box`: capture
  with the Phase 1 `StringSink`. Strip ANSI and assert:
  - the substring `\nyaml\n` does not appear,
  - no line is a "block of background-fill spaces" (heuristic: no line
    consisting solely of spaces and ANSI resets after stripping color, in
    the YAML section).

- `render_failure_block_yaml_section_indents_each_line_by_four_spaces`:
  multi-line snippet (`file_exists: x\nmessage: "..."`). Capture, strip
  ANSI, locate the YAML region (lines between source line and `Reason:`),
  assert each non-empty line starts with exactly `"    "`.

### Acceptance

- New report tests pass.
- `cargo test -p claudine --lib` and `cargo test -p darkmatter --lib`
  both pass (the latter only if darkmatter was modified).
- `cargo clippy -p claudine -p darkmatter --lib --tests --no-deps -- -D warnings`
  clean.

### Risks / open questions

- If darkmatter does not expose a chrome-free helper today and adding one
  is contentious, a smaller in-claudine alternative is to hand-roll a
  `syntect::easy::HighlightLines` call gated to YAML. Prefer the darkmatter
  route so theming stays centralized; fall back to in-claudine if the
  darkmatter PR scope grows.

---

## Phase 5 — Asserted Level 1 text tests for the full failure block

### Goal

Round out the High finding. With Phase 1 (string sink), Phase 2 (line
ranges), Phase 3 (raw snippet), and Phase 4 (chrome-free YAML) in place,
write the Level 1 asserted tests that the review explicitly calls out.

### Files to touch

- `claudine/lib/src/harness/report.rs` (extend `mod tests`)

### Changes

Use a single helper in tests:

```rust
fn capture_check_outcomes(report: &ValidationPhaseReport) -> String {
    let term = test_terminal();
    let raw = report_check_outcomes_to_string(report, &term);
    strip_ansi_escapes::strip_str(&raw)
}
```

> If `strip-ansi-escapes` is not already a dev-dependency of `claudine`,
> add it to `claudine/lib/Cargo.toml` `[dev-dependencies]` (it is already
> used widely in the workspace, e.g. biscuit-tui PTY tests).

### Tests to add

1. `pass_state_renders_exactly_one_compact_row`:
   build a 1-outcome report, assert the captured (ANSI-stripped) string is
   exactly one trimmed non-empty line, and contains the rendered markup
   substring.

2. `failure_state_full_block_contains_all_four_sections`:
   outcome with full source (line range present, original YAML snippet,
   real reason). Capture, assert in order:
   - first non-empty line contains `Pre-validation failed`,
   - the next non-empty line starts with `in ` and ends with `.md:42-44`
     (or whatever the chosen test range is),
   - subsequent lines include the YAML snippet content with four-space
     indent,
   - a later line contains `Reason: file does not exist:` followed by the
     test path.

3. `failure_state_pre_phase_uses_pre_validation_failed`:
   `phase: PreCheck` → header is `Pre-validation failed`.

4. `failure_state_post_phase_uses_post_validation_failed`:
   `phase: PostCheck` → header is `Post-validation failed`.

5. `failure_block_omits_line_range_suffix_when_unknown`:
   outcome with `line_range: None`. Assert the source-location line ends
   at the file path with no `:N-M` suffix.

6. `legacy_failure_path_renders_two_lines_only`:
   asserts the no-source fallback output is exactly two non-empty lines
   (status row + Reason row), no YAML section.

7. `osc8_hyperlink_target_present_in_raw_output`:
   capture the **un-stripped** output (skip ANSI strip). Assert the raw
   string contains `\x1b]8;;` (OSC-8 introducer) followed by the absolute
   path of the rule's source file. This is the asserted complement to the
   PTY check in Phase 6.

### Acceptance

- All seven new tests pass.
- `cargo clippy -p claudine --lib --tests --no-deps -- -D warnings` clean.
- The previously-smoke-only tests
  (`report_check_outcomes_failure_with_source_emits_block`,
  `report_check_outcomes_pass_path_unchanged`,
  `render_failure_block_handles_all_phases`) are kept as fast smoke
  coverage but are no longer the only signal for the user-visible shape.

---

## Phase 6 — Level 2 PTY/real-terminal capture test for OSC-8 + styled output

### Goal

Address the second half of the High finding: terminal-emulator behaviors
(OSC-8 hyperlinks, SGR styling, code-block chrome absence) need a real-PTY
test. We do not need to spin up tmux/WezTerm/Kitty to satisfy this — a
direct PTY harness is sufficient and matches existing patterns in this
repo (see `claudine/cli/tests/pty_tests.rs`,
`biscuit-tui/cli/tests/common/pty.rs`). If a tmux-based path is already
required by other claudine tests, prefer that.

### Files to touch

- `claudine/cli/tests/validation_reporter_pty.rs` (new)
- `claudine/cli/tests/fixtures/validation_reporter/missing_file.md` (new
  fixture markdown with a deliberately failing `pre_checks` rule)
- (possibly) `claudine/cli/Cargo.toml` `[dev-dependencies]` — confirm
  `expectrl` and `assert_cmd` are already present (they are used by
  `pty_tests.rs`).

### Approach

1. Create a fixture markdown file with frontmatter such as:

   ```yaml
   ---
   prompt: "noop"
   pre_checks:
     - file_exists: "definitely-not-a-real-path-xyz.toml"
   ---
   body
   ```

2. Spawn `claudine compose <fixture> --<provider-stub>` (use the same
   stub-provider pattern as `pty_tests.rs`: write a fake binary onto
   `PATH` so the wrapper does not actually launch a real model). Set
   `TERM=xterm-256color` and unset `NO_COLOR`. Do **not** set
   `TERM_WIDTH=80` for this test — we want real ANSI output.

3. Capture the full PTY transcript with `expectrl`. Assert against the
   raw bytes:

   - **OSC-8 link present:** the transcript contains
     `\x1b]8;;` (the link introducer), followed by the absolute path of
     the fixture, followed by `\x1b\\` (ST terminator).
   - **OSC-8 link closes:** there is a corresponding empty `\x1b]8;;\x1b\\`
     close sequence.
   - **SGR styling for the failure header:** the transcript contains a
     SGR sequence (e.g. `\x1b[3` for foreground color) on the line that
     also contains `Pre-validation failed`.
   - **No `yaml` fence label:** the transcript does not contain the
     literal substring `\nyaml\n` (covers the chrome-removal fix).
   - **Reason line present:** the ANSI-stripped transcript contains
     `Reason: file does not exist:`.

4. Mark the test `#[cfg(unix)]` and gate it with `#[ignore]` if the
   existing claudine PTY tests are gated, so CI behavior matches
   `pty_tests.rs`. Document the gate inline; running it locally with
   `cargo test -p claudine --test validation_reporter_pty -- --ignored`
   must still pass.

### Tests to add

- `pty_pre_check_failure_emits_osc8_link_and_styled_header` — see above.

  Optional second test if implementation effort is small:
  `pty_pre_check_failure_yaml_snippet_has_no_fence_label` — separate
  assertion for the chrome-free YAML rendering. May be combined into a
  single test to avoid double PTY spawn cost.

### Acceptance

- `cargo test -p claudine --test validation_reporter_pty -- --ignored`
  passes locally (gated identically to the existing PTY tests).
- `cargo clippy -p claudine --tests --no-deps -- -D warnings` clean.

### Risks / open questions

- If the existing claudine harness cannot be coerced into emitting the
  failure block from a stub binary (e.g. because the harness short-circuits
  before reporting when no provider is installed), fall back to a tiny
  test-only binary that calls
  `claudine::harness::report::report_check_outcomes` directly through a
  PTY-attached process. Add the binary as `claudine/cli/examples/...` or
  a `[[bin]]` test-only target. Decide at implementation time; document
  whichever path is taken.

- tmux/WezTerm/Kitty pane-text capture is **not** required to clear this
  review finding. If during implementation the team decides to add a
  tmux variant, mirror the patterns in
  `biscuit-tui/cli/tests/common/real_terminal/tmux.rs`.

---

## Phase 7 — Final verification and clippy gate

### Goal

Single, repeatable verification step that proves all four review findings
are addressed and no regressions introduced.

### Files to touch

None directly — this is a verification-only phase.

### Commands (run in order, all must succeed)

1. **Workspace-targeted unit + integration tests**

   ```sh
   cargo test -p claudine
   ```

2. **Darkmatter unit tests** (only required if Phase 4 modified darkmatter)

   ```sh
   cargo test -p darkmatter --lib
   ```

3. **PTY tests** (gated, run explicitly)

   ```sh
   cargo test -p claudine --test validation_reporter_pty -- --ignored
   ```

4. **Clippy gate**

   ```sh
   cargo clippy -p claudine --lib --tests --no-deps -- -D warnings
   cargo clippy -p claudine --bin claudine --no-deps -- -D warnings
   ```

   If darkmatter was touched in Phase 4:

   ```sh
   cargo clippy -p darkmatter --lib --tests --no-deps -- -D warnings
   ```

5. **Doctests**

   ```sh
   cargo test -p claudine --doc
   ```

### Acceptance

- All five command groups exit 0 with no warnings.
- The three review findings (High + 3 Medium) each have at least one
  asserted test referenced by file and name in the closing summary
  (drop these into the eventual feature SUMMARY.md):

  | Finding | Asserted by |
  |---|---|
  | High: terminal output only smoke-tested | Phase 5 tests + Phase 6 PTY test |
  | Medium: YAML reconstructed not raw | `parse_rules_yaml_snippet_preserves_authored_quoting`, `parse_rules_yaml_snippet_preserves_inline_comments` |
  | Medium: line ranges never attempted | `parse_rules_carry_line_range_when_recoverable`, `find_rule_spans_*` family |
  | Medium: YAML rendering has fence chrome | `render_failure_block_yaml_section_has_no_yaml_label_or_box`, `pty_pre_check_failure_emits_osc8_link_and_styled_header` |

- The smoke-only assertions noted in the review
  (`report.rs:392`, `report.rs:415`) are **not deleted**; they remain as
  fast non-PTY smoke coverage but are no longer the strongest signal.

---

## Cross-phase notes

- **Test naming.** Use the `#[test] fn snake_case_describing_behavior`
  convention already established in `report.rs::tests` and
  `parse.rs::tests`.

- **No ad-hoc `eprintln!` left behind.** After Phase 1, any new code
  written in Phases 2-4 that emits user-visible terminal output must go
  through the `LineSink` / `report_check_outcomes_to_string` plumbing.

- **No public API breakage.** All four findings can be addressed with
  internal-only changes plus optional darkmatter helper exposure. If a
  change appears to require breaking `claudine::harness` public types, stop
  and revisit — the existing `RuleSource` shape is sufficient.

- **Rust doc convention.** Per repo CLAUDE.md: no `# H1` inside `///`
  blocks; use `## Examples`, `## Returns`, `## Errors`, `## Panics`.
  Apply when adding doc comments to the new helpers.

- **Subagent commit policy.** The rust-developer subagent MUST NOT commit.
  Each phase ends with `git status` clean of untracked binaries and the
  user reviews/commits the phase.
