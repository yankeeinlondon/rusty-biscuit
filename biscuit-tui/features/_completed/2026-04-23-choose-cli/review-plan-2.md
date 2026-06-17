# Review 2 Implementation Plan

## Context

Review 2 pronounces the feature "ready for production" and lists nine non-blocking
follow-ups, one Medium (`--selected` comma-splitting foot-gun on `choose-many`)
and eight Low (docs, missing assertions, extra coverage, and minor ergonomics).
This plan addresses **all nine findings**.

Repo layout reminder:

- Library: `biscuit-tui/lib/` (package `biscuit-tui`).
- CLI: `biscuit-tui/cli/` (package `biscuit-tui-cli`, binary `question`).
- Area verification: `just -f biscuit-tui/justfile test`, `just -f biscuit-tui/justfile lint`
  (run from repo root, or `cd biscuit-tui/ && just test && just lint`).
- The root `justfile` does **not** cover `biscuit-tui`; use the area `justfile`.

Scope sizing: three phases of 3 tasks each. Phases are independently
completable (tests green + lint clean at phase end) and run in order 1 → 2 → 3
because phase 2 builds a seam introduced in phase 1 and phase 3 adds test
coverage that depends on both.

---

## Phase 1: `--selected` Semantics, Stdin Seam, And `adjust_scroll` Tidy-Up

Goal: fix the Medium finding (#1) and the two refactors that touch runtime
code (#6 seam, #7 signature tweak). These changes all modify production
behaviour or internal signatures, so they go first while the codebase is
quiet.

### Task 1.1 — Stop splitting `--selected` on commas for `choose-many` (finding #1, Medium)

**File:** `biscuit-tui/cli/src/commands/choose_many.rs`

Change the "preferred" path from review 2: stop CSV-splitting `--selected`
entirely; the flag is already `Vec<String>` because clap accepts it
repeatedly. Keep the legacy CSV split **only** for the deprecated `--initial`
path.

- Delete (or inline away) `flatten_selected` at
  `biscuit-tui/cli/src/commands/choose_many.rs:157`.
- Update `effective_selected` (around line 146) so the `--selected` branch
  returns `args.selected.clone()` directly (after filtering empties for safety).
- Keep `parse_initial_ids` for the `--initial` branch — that flag is
  advertised as CSV and the CHANGELOG already notes the deprecation.
- Update the rustdoc on `ChooseManyArgs::selected` (around line 55) to drop the
  "each repetition is split on `,`" sentence. Replace with:
  "Repeatable; pass `--selected foo --selected bar` to pre-select multiple
  values. Comma-splitting is **not** applied — if you need CSV semantics, use
  the deprecated `--initial` flag."

**New tests** (append to the existing `mod tests` in `choose_many.rs`):

1. `selected_value_containing_comma_is_preserved` — builds `ChooseManyArgs`
   with `selected: vec!["one,two".into()]`, options containing a literal
   `one,two` value (e.g. positionals `["A|one,two", "B|three"]` with
   `--delimiter "|"`), runs `run_with_writer` and asserts the state pre-selects
   exactly `one,two` and not `one` + `two`.
2. `selected_repeated_flag_collects_all_values` — passes
   `selected: vec!["one".into(), "two".into(), "three".into()]` against
   options `["one", "two", "three"]`, asserts `state.selected_ids()` contains
   all three.
3. `initial_still_splits_on_commas_for_backward_compat` — passes
   `initial: Some("one,two".into())`, options `["one", "two"]`, asserts both
   pre-selected and that the deprecation warning still fires (`stderr` capture
   not required — the test only cares about the state shape).

**Risk / sequencing:** This is a user-visible semantic change. The previous
behaviour was already a bug (silently dropped comma-bearing values), but any
downstream script that relied on `--selected "a,b,c"` expansion must now use
`--selected a --selected b --selected c`. Call this out in the phase 3
CHANGELOG update.

### Task 1.2 — Extract a `Read`-seam helper for stdin option sourcing (finding #6, Low)

**File:** `biscuit-tui/cli/src/commands/common_choose.rs`

Today `resolve_option_strings` (line 215) reads `io::stdin()` directly, making
the "no options on stdin" / "CR stripping" / "empty-line filtering" branches
only reachable through `assert_cmd`. Refactor:

- Add a pub-crate helper `read_option_strings_from(reader: impl Read) -> io::Result<Vec<String>>`
  that encapsulates the `read_to_string` → split lines → CR-trim →
  filter-empty → empty-check pipeline.
- Return `Err(io::ErrorKind::InvalidInput, "no options provided: …")` when
  the resulting vec is empty, matching the current message.
- Change `resolve_option_strings` to delegate: if stdin is not a TTY, call
  `read_option_strings_from(io::stdin().lock())` and wrap the result in
  `Some`.

**New tests** (in the `mod tests` block of `common_choose.rs`, alongside the
existing `resolve_option_strings_*` tests):

1. `read_option_strings_strips_trailing_carriage_return` — cursor over
   `b"alpha\r\nbeta\r\n"`, asserts `vec!["alpha", "beta"]`.
2. `read_option_strings_filters_empty_lines` — cursor over
   `b"\nalpha\n\n\nbeta\n"`, asserts `vec!["alpha", "beta"]`.
3. `read_option_strings_empty_input_is_invalid_input` — cursor over `b""`,
   asserts `Err` with `ErrorKind::InvalidInput` and message contains
   "no options provided".
4. `read_option_strings_whitespace_only_lines_are_kept_as_is` — cursor over
   `b"  alpha  \nbeta\n"`, asserts `vec!["  alpha  ", "beta"]` (we preserve
   label whitespace; trimming is the caller's responsibility via
   `parse_label_value`).

**Risk / sequencing:** Internal seam change only; no public API changes. Must
land before Phase 3 finding #6 coverage is complete.

### Task 1.3 — Pass `visible_indices` slice into `adjust_scroll` (finding #7, Low)

**Files:**

- `biscuit-tui/lib/src/components/choose_one.rs` (line 606 call site, line 747 definition)
- `biscuit-tui/lib/src/components/choose_many.rs` (line 665 call site, line 804 definition)

Both files clone `state.filter.visible().to_vec()` into a local `visible_indices`
then call `adjust_scroll(state, visible, visible_indices.len())`, which then
re-reads `state.filter.visible()` to locate `state.hover`. Remove the redundant
re-read:

- Change both `adjust_scroll` signatures to take `visible_indices: &[usize]`
  instead of `visible_len: usize`.
- In the body, use the passed-in slice for both the `.iter().position(…)`
  lookup and the length check.
- Update both call sites to pass `&visible_indices` instead of
  `visible_indices.len()`.

Keep `&mut state` for the `scroll_offset` / `hover` writes.

**New tests:** No new tests — this is a refactor with no behaviour change.
Existing `scroll_*` / `draw_list` tests continue to exercise the path. Confirm
by running `cargo test -p biscuit-tui components::choose_one` and
`cargo test -p biscuit-tui components::choose_many`.

**Risk / sequencing:** `adjust_scroll` is a private `fn`, so this is a pure
internal refactor. No callers outside the same file.

### Phase 1 Verification

```bash
cd biscuit-tui
cargo test -p biscuit-tui -p biscuit-tui-cli
cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets -- -D warnings
just test
just lint
```

All existing tests must pass, plus the seven new tests added above
(3 in `choose_many.rs`, 4 in `common_choose.rs`). Lint must be clean.

---

## Phase 2: Missing Test Assertions And UTF-8 Coverage

Goal: tighten three tests that are weaker than their siblings (#4), add
multi-byte UTF-8 coverage for the fuzzy highlighter (#5), and add the
`FrameChrome` + filter composition integration test (#8). These are
test-only additions with no runtime changes.

### Task 2.1 — Assert concrete output in `run_propagates_percent_height_to_prompt` (choose-many) (finding #4, Low)

**File:** `biscuit-tui/cli/src/commands/choose_many.rs` (test at line 395)

The sibling test in `choose_one.rs` (line 342) asserts `output == b"B\n"`.
The choose-many variant only asserts `status == 0`. Strengthen it:

- Inside the `run_with_writer` closure, replace the bare
  `Ok(state.selected_values().into_iter().cloned().collect())` with code that
  returns the expected pre-selected value (`"A"`).
- After `assert_eq!(status, 0)`, add `assert_eq!(output, b"A\n")` to pin the
  output format.

No new test is added; the existing test is strengthened. Also review the
sibling `run_propagates_cells_height_to_prompt` (around line 378) — it does
assert output, so no change needed there.

### Task 2.2 — Multi-byte UTF-8 highlight test (finding #5, Low)

**File:** `biscuit-tui/lib/src/core/fuzzy.rs` (append to the `mod tests` block)

Add one test that proves char-offset highlighting survives multi-byte UTF-8
labels — this regression-tests the comment on line 175-180 that callers must
convert char→byte offsets correctly.

**New test:**

1. `highlight_indices_handles_multibyte_labels` — builds a `FuzzyFilter`,
   calls `set_pattern("ca", &["Café".into(), "Grünes Tee".into()])`, then
   calls `highlight_indices("Café")` and asserts the returned vec contains
   char indices `0` and `1` (matching `C` and `a`), NOT byte indices (the
   `é` at byte offset 3 would otherwise shift).

Additionally, in both `biscuit-tui/lib/src/components/choose_one.rs` and
`biscuit-tui/lib/src/components/choose_many.rs` (append to their `mod tests`
blocks), add a test that exercises `build_highlighted_spans` against a
multi-byte label so the char-index iteration on line 716 / 773 is covered
end-to-end:

2. `build_highlighted_spans_styles_matched_chars_in_multibyte_label`
   (one in each file, adjusted for each module's visibility) — calls
   `build_highlighted_spans("Café", &[0, 1], base_style, match_style)` and
   asserts the first span's text is `"Ca"` styled with `match_style` and the
   remainder `"fé"` styled with `base_style`. This depends on the function
   being either `pub(super)` or test-accessible; if it is currently private
   inside the module, the test can live in the same module since both files
   already use `#[cfg(test)] mod tests { use super::*; }`.

### Task 2.3 — FrameChrome + filter-active rendering integration test (finding #8, Low)

**File:** `biscuit-tui/lib/src/core/standalone.rs` (append inside the
`drive_event_loop_with_chrome` test module, near line 710)

Add a test that composes a `FrameChromeConfig` with a real `ChooseOne`
widget whose state has a pre-seeded filter pattern, and asserts the
rendered buffer contains (a) border glyphs in the expected corners and
(b) the search prompt row **inside** the border, and (c) at least one
matching label glyph.

**New test:**

1. `drive_event_loop_with_chrome_preserves_search_prompt_inside_border` —
   - Build a `TestBackend::new(30, 8)` terminal.
   - Construct a `ChooseOneState` from a `ChoiceInput` with options
     `["Red", "Green", "Blue"]`, `filter_enabled = true`.
   - Seed the filter with pattern `"re"` (either via a public accessor if
     one exists, or via a typed `KeyEvent` in the event iterator before
     the final `Enter`).
   - Build `FrameChromeConfig { border: BorderStyle::Rounded,
     border_label: Some("Pick".into()), .. Default::default() }`.
   - Drive the event loop with a single `Enter` event and inspect the
     backend buffer.
   - Assert top-left / bottom-right corner cells are not spaces (border is
     drawn).
   - Assert that a row interior to the border contains the theme's default
     search indicator glyph (`/ ` per tech-design §6.4) followed by the
     seeded pattern.
   - Assert at least one visible row inside the border contains a non-space
     label character from `"Red"` or `"Green"` (either is fine because the
     test only proves the filter pattern routed through).

If pre-seeding via events is cleaner than via public accessors, route the
keystrokes through the same event iterator pattern used by
`drive_event_loop_with_chrome_draws_border_around_inner` (line 740).

### Phase 2 Verification

```bash
cd biscuit-tui
cargo test -p biscuit-tui core::fuzzy
cargo test -p biscuit-tui components::choose_one
cargo test -p biscuit-tui components::choose_many
cargo test -p biscuit-tui core::standalone
cargo test -p biscuit-tui-cli commands::choose_many::tests::run_propagates_percent_height_to_prompt
just test
just lint
```

All four new tests pass; the one strengthened test still passes with the
added `assert_eq!` on output. Lint clean.

---

## Phase 3: Docs, Integration-Test Docstring, And PTY Smoke

Goal: close the remaining docs findings (#2, #3) and add the PTY-gated
`--height 100%` end-to-end coverage (#9). Fully serial after Phase 1's
`--selected` behaviour change lands so the CHANGELOG entry can describe the
final shape.

### Task 3.1 — Update `cli/tests/choose_cli.rs` module docstring (finding #2, Low)

**File:** `biscuit-tui/cli/tests/choose_cli.rs` (lines 1-14)

Replace the "reached the event loop" framing with the post-review-1 reality:

- State that the **authoritative green-path coverage** lives in the
  `run_with_writer` unit tests inside
  `biscuit-tui/cli/src/commands/choose_one.rs` and
  `biscuit-tui/cli/src/commands/choose_many.rs`.
- This file now covers: (a) clap-level parsing regressions
  (help contents, flag conflicts, value-parser rejects), (b) source-resolution
  smoke tests that exercise the process-level stdin / positional argv entry
  points, and (c) the opt-in PTY flows under `mod pty`.

Keep the PTY-gate note (referencing `QUESTION_INTERACTIVE_PTY=1`) — that is
still accurate.

### Task 3.2 — README `--height`, `--sort`, filter section (finding #3, Low)

**File:** `biscuit-tui/cli/README.md`

- Global flags section (line 19): change `--height <N>` to `--height <N|PCT%>`.
  Add the explanation: "accepts either an absolute cell count or a percentage
  (e.g. `50%`); percentages are resolved against the current terminal rows at
  render time and clamped to a floor of 3."
- Cross-check the `choose-one` subcommand section (lines 72-109) and the
  `choose-many` subcommand section (lines 110-135):
  - `--sort` (line 92) already appears — leave as-is unless wording drift is
    found.
  - Fuzzy-filter-by-default + `--no-filter` (line 91) already appears — leave
    as-is.
  - Add one sentence near the bottom of each subcommand section
    cross-referencing the global `--height` update ("`--height` accepts a
    percentage suffix — see **Global Flags**.") so scanners looking only at
    the subcommand section find the feature.
- No CHANGELOG addition for the doc-drift fix itself, but if Phase 1 task 1.1
  landed a breaking semantic change for `--selected`, append a matching
  "Changed" entry to the existing `[Unreleased]` block in
  `biscuit-tui/cli/CHANGELOG.md`: "`choose-many --selected` no longer splits
  comma-separated values; use the repeatable form (`--selected a --selected b`)
  or the deprecated `--initial` flag for CSV input."

### Task 3.3 — PTY smoke for `--height 100%` (finding #9, Low)

**File:** `biscuit-tui/cli/tests/choose_cli.rs` (inside `mod pty`, after
`choose_many_ctrl_a_then_submit_writes_all_values`)

Add one PTY-gated test that spawns the CLI with `--height 100%`, sends
`Enter` to auto-submit the first (active) option, and asserts exit code 0.

**New test:**

1. `choose_one_height_100_percent_runs_end_to_end` — spawns
   `question choose-one alpha beta gamma --height 100%`, waits 200ms, writes
   `b"\r"` (Enter, which triggers fallback-submit-on-active), asserts
   `wait_exit_code == 0`. Gate on `interactive_enabled()` exactly like the
   existing tests in the module.

This covers the inline-100% geometry path that `height_spec_percent_100_resolves_to_term_rows`
only exercises at the math layer.

### Phase 3 Verification

```bash
cd biscuit-tui
cargo test -p biscuit-tui-cli --test choose_cli
# Optional PTY run (local only):
QUESTION_INTERACTIVE_PTY=1 cargo test -p biscuit-tui-cli --test choose_cli -- pty::
just test
just lint
```

Docstring update compiles with the existing tests; the new PTY test is
skipped in default CI runs and exercised locally only. Lint clean.

---

## Final Full Verification

Run from the repo root after all three phases are complete:

```bash
cd biscuit-tui
just test
just lint
```

Equivalent direct commands if `just` is unavailable:

```bash
cargo test -p biscuit-tui -p biscuit-tui-cli
cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets -- -D warnings
```

Both must pass with zero failures and zero warnings. Review 2 pre-stated the
baseline — 342 lib unit tests + 134 CLI unit tests + 14 doctests green, clippy
clean — this plan should raise both unit-test counts by the additions above
(≈ +12 tests across the three phases) while keeping the baseline green.

Optional local smoke:

```bash
QUESTION_INTERACTIVE_PTY=1 cargo test -p biscuit-tui-cli --test choose_cli -- pty::
```

Confirms the PTY-backed flows including the new `--height 100%` case.

---

## Summary Table

| Phase | Finding(s) addressed | Files touched (principal) |
|-------|----------------------|---------------------------|
| 1 | #1 (Medium), #6, #7 | `cli/src/commands/choose_many.rs`, `cli/src/commands/common_choose.rs`, `lib/src/components/choose_one.rs`, `lib/src/components/choose_many.rs` |
| 2 | #4, #5, #8 | `cli/src/commands/choose_many.rs`, `lib/src/core/fuzzy.rs`, `lib/src/components/choose_one.rs`, `lib/src/components/choose_many.rs`, `lib/src/core/standalone.rs` |
| 3 | #2, #3, #9 | `cli/tests/choose_cli.rs`, `cli/README.md`, `cli/CHANGELOG.md` |

All nine findings from `review-2.md` are addressed.
