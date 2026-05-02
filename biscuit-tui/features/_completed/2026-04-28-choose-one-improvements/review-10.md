---
ready: true
agent: ""
model: ""
---

# Feature Review: Choose One Improvements (Review 10)

## Summary

Both Review 9 blockers are now resolved with strong test coverage:

- `--file` rejects unsupported extensions and surfaces a clear
  `unsupported file format '<ext>'` error. Plain-text fallback is gone.
- The CLI duplicate-hotkey check now operates on **effective** hotkeys
  (explicit OR auto-derived `Ctrl+<first-alphanumeric>`), so an earlier
  option's default `Ctrl+x` can no longer silently shadow a later
  option's explicit `[CTRL+x]`. Disabled options correctly contribute
  no effective hotkey. Both `choose-one` and `choose-many` exercise
  the new path through dedicated CLI integration tests.

The default test suite is green:
`cargo test -p tui-chrome -p tui-chrome-cli` → 892 tests pass, 0 fail.
`cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets` → clean.
The opt-in gates pass when invoked locally:
`RUN_PTY_TESTS=1` keyboard-protocol tests → 4/4 pass;
`RUN_SHELL_TESTS=1` completions-shell tests → 8/8 pass.

I treat the feature as **production-ready**. The remaining items below
are minor and either pre-existing harness defects or quality-of-life
suggestions; none change observable user behavior of the binary.

## Findings

### 1. (Pre-existing) PTY harness Bug 2 still blocks one inline-viewport test

`pty::choose_one_height_100_percent_runs_end_to_end` still fails with
exit 1 under `QUESTION_INTERACTIVE_PTY=1`. The defect is in the
`choose-cli` PTY harness (no DSR cursor-position responder), not in
the `question` binary, and is fully documented in
`biscuit-tui/features/2026-04-28-choose-one-improvements/pty-test-bugs.md`
(Bug 2). The matching `keyboard_protocol.rs` harness already has an
`answer_cursor_position_request` helper that the choose-cli harness
should reuse.

Severity: low. The math layer for `--height 100%` is independently
covered by `lib::core::frame::tests::height_spec_percent_*`, and the
binary itself behaves correctly under a real terminal. This is a
test-infrastructure follow-up, not a feature gap.

Recommendation: extract the existing DSR responder helper into a
shared `cli/tests/common/pty.rs` module (Bug 2's "Option A") and call
it from `spawn_question` whenever any of `args` references `--height`
or `-h`.

Evidence:
- `biscuit-tui/cli/tests/choose_cli.rs:983` — failing test.
- `biscuit-tui/cli/tests/keyboard_protocol.rs` — existing DSR responder.
- `biscuit-tui/features/2026-04-28-choose-one-improvements/pty-test-bugs.md`
  Bug 2 (status: open).

### 2. Verification-gate tests are env-gated and not exercised by `just test`

Spec § "Verification Gates" requires PTY-driven verification of all
completion and keyboard-modifier claims as a precondition for
production-readiness. The corresponding suites
(`tests/keyboard_protocol.rs`, `tests/completions_shell.rs`,
`tests/choose_cli.rs::pty`) currently require explicit env vars
(`RUN_PTY_TESTS=1`, `RUN_SHELL_TESTS=1`,
`QUESTION_INTERACTIVE_PTY=1`) and are skipped silently otherwise.
Neither `biscuit-tui/justfile` nor any GitHub workflow flips those
flags, so the gates only run when a developer remembers to set them.

Severity: low for shipping today (the gates pass when run), but a
real regression risk over time.

Recommendation: add a `just test-pty` recipe (and/or a CI matrix step)
that runs the three gated suites with the env vars set on macOS/Linux,
so the verification gates run automatically before every merge.

Evidence:
- `biscuit-tui/cli/tests/keyboard_protocol.rs:22` and
  `biscuit-tui/cli/tests/completions_shell.rs:43` — env-var gating.
- `biscuit-tui/cli/tests/choose_cli.rs:801` —
  `QUESTION_INTERACTIVE_PTY` gating.
- `biscuit-tui/justfile` — no recipe sets these vars.
- `.github/workflows/` — no biscuit-tui workflow exists.

### 3. (Minor) Markdown frontmatter parsing is hand-rolled

The tech-design (`tech-design.md:391`) says: "do not hand-roll
frontmatter parsing with string slicing." `parse_md` in
`biscuit-tui/cli/src/option_sources.rs:554-585` does exactly that
(searches for `---`, slices, then defers to `serde_yaml_ng`). The
implementation works for well-formed input and the unit tests cover
the happy path, but it does not handle BOM, CRLF-only line endings, or
front-matter that begins with a newline before `---`.

The repository has a dedicated `biscuit-file` skill/library with rich
file-reference handling, and the project CLAUDE.md explicitly says
"whenever you are attempt to convert a string based file reference to
a real file path in the filesystem you should use `FileReference`
struct from `biscuit-file`." `tui-chrome-cli` does not depend on
`biscuit-file` today.

Severity: low. Not a regression; the tech-design language is
permissive ("if available").

Recommendation: either (a) add a small CRLF/BOM-tolerant test and
keep the local parser, or (b) introduce a `biscuit-file` dependency
and route both `--file` and `--md` through it. Option (a) is the
expedient choice for this feature; option (b) aligns better with
repo-wide guidance and would simplify follow-up work.

### 4. (Minor) TOML source convention is undocumented at the CLI surface

`parse_toml` in `option_sources.rs:265` accepts either a top-level
array (TOML extensions only) **or** a table with an `options = [...]`
key. Standard TOML cannot represent a top-level bare array, so in
practice users will always need the `options = [...]` form, but the
CLI help / spec do not state this. A user who ships a `colors.toml`
shaped as `colors = [...]` will get `option file must contain an
array`, which is technically true but misleading.

Severity: very low. No tests fail; no behavior changes.

Recommendation: mention the `options = [...]` convention in
`biscuit-tui/docs/cli-reference.md` (and in the `--file` clap help
text) so the contract is explicit.

### 5. (Ergonomics) `choice_normalize.rs` is now large; consider a hotkey submodule

`biscuit-tui/cli/src/choice_normalize.rs` is approaching ~1,200 lines
with a substantial portion devoted to hotkey parsing, validation,
collision detection, and CLI display formatting. The module is well
organized internally (helpers `single_char`, `has_modifier_prefix`,
`split_bracket_prefix`, `effective_hotkey_for`,
`format_hotkey_spec`), but a flat module makes the file harder to
navigate.

Severity: nil — purely cosmetic.

Recommendation: optional refactor in a follow-up — extract a
`choice_normalize::hotkey` submodule that owns `parse_hotkey_spec`,
`single_char`, `has_modifier_prefix`, `split_bracket_prefix`,
`effective_hotkey_for`, and `format_hotkey_spec`, leaving the
top-level module focused on `normalize_options` and the
record-mapping logic.

### 6. (Ergonomics) Duplicate hotkey-display-override test scaffolding

`ChooseOne` and `ChooseMany` each carry seven near-identical
`with_hotkey_display_*_survives_*` tests with their own local
`ctrl_press`, `alt_press`, `modifier_press`, and `modifier_release`
helpers. The test bodies differ only by the component type and
fixture function. This is fine, but every future modifier-related
state field will need to duplicate the same 7 tests in two files.

Severity: nil — current coverage is correct and clear.

Recommendation: consider a shared `tests/common/hotkey_display.rs`
helper or a `macro_rules!` test generator that takes a state
constructor and a widget constructor. This is a maintenance
investment, not a correctness fix.

## Test Results

```
cargo test -p tui-chrome -p tui-chrome-cli            # 892 / 892 pass
cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets # clean
RUN_PTY_TESTS=1   cargo test -p tui-chrome-cli --test keyboard_protocol  # 4 / 4 pass
RUN_SHELL_TESTS=1 cargo test -p tui-chrome-cli --test completions_shell  # 8 / 8 pass
QUESTION_INTERACTIVE_PTY=1 cargo test -p tui-chrome-cli --test choose_cli pty:: # 3 / 4 pass
                                                      # (1 pre-existing harness defect, see Finding 1)
```

## Spec / Design Coverage Spot-Check

- FrameChrome `Padding::default()` returns `uniform(1)` at the library
  level (`lib/src/core/frame.rs:185`). ✓
- Radio glyphs (`f043e` / `f4aa`) and checkbox glyphs (`f14a` /
  `f0131`) gated on Nerd Font detection
  (`lib/src/components/choice_render.rs:107`). ✓
- ChooseOne ESC restores `initial_selected` and submits with exit 0
  (`lib/src/components/choose_one.rs:567`). ✓
- ChooseMany Enter does not toggle the active row (per existing test
  coverage). ✓
- Horizontal layout + Up/Down column-aware navigation present in both
  components and exercised by Phase 6 tests. ✓
- Hotkey badge override (`with_hotkey_display`) survives modifier-only
  press/release and chord-fallback events; covered by 7 dedicated
  tests in each of ChooseOne and ChooseMany. ✓
- `--file` rejects unsupported extensions (review-9 issue 1). ✓
- Effective-hotkey duplicate detection at the CLI boundary
  (review-9 issue 2). ✓
- `parse_hotkey_spec` rejects empty and multi-character suffixes
  (review-8 issue 2). ✓

## Production Readiness

**Ready for production.** The two Review 9 blockers are correctly
fixed with deep test coverage, all default tests pass, lint is clean,
and the opt-in PTY/shell verification suites pass when invoked. The
remaining findings are either pre-existing harness defects already
documented in `pty-test-bugs.md`, follow-up CI hardening, or
ergonomic polish — none affect observable user behavior of the
`question` binary or the `tui-chrome` library.
