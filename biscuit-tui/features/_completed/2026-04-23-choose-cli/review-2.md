---
ready: true
---

# Review 2

## Verdict

**Ready for production.** The spec items from both `spec.md` and `tech-design.md` are implemented, test suite is comprehensive and green (342 lib unit tests + 134 CLI unit tests + 5 CLI integration test binaries + 14 doctests, with `cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets -- -D warnings` clean), and review-1's two findings were resolved:

- CLI defaults `ChoiceInput::filter_enabled = true` with a `--no-filter` opt-out (`cli/src/commands/choose_one.rs:154`, `cli/src/commands/choose_many.rs:191`).
- Deterministic green-path coverage now lives in the in-module `run_with_writer` tests for both subcommands, which exercise arg → state → output end-to-end without needing a TTY (e.g. `run_writes_delimited_positional_value`, `run_selected_default_matches_delimited_value`, `run_select_all_outputs_all_values`, `run_deselect_all_outputs_no_values`, `run_returns_1_without_output_on_esc`, `run_returns_130_without_output_on_ctrl_c`).

Below are remaining suggestions ordered by severity. None are blocking.

## Findings

### 1. `choose-many --selected` unconditionally splits on commas — values containing commas cannot be pre-selected

Severity: **Medium**

`flatten_selected` (`cli/src/commands/choose_many.rs:157`) always splits each `--selected` value on `,`:

```rust
raw.split(',').map(str::trim).filter(|s| !s.is_empty())
```

This is fine for the common `--selected "a,b,c"` ergonomics, but it silently drops any value that contains a literal comma. The risk is real when combined with `--delimiter`: e.g. `question choose-many --delimiter "|" "A|one,two" "B|three" --selected "one,two"` cannot target `one,two` — it expands to `[one, two]`, neither of which matches.

The spec does not mandate comma-splitting; it only says `--selected` is a "value based parameter". The tech-design calls out "repeatable and also accepts a comma-separated form for symmetry with the legacy CSV `--initial`." The repeatable form gives a comma-free path already (`--selected one,two --selected three` still gets mangled).

Suggested fix (pick one):

- **Minimal**: only split when the raw value has no standalone repetition semantics — preserve the legacy CSV split for a single `--selected` arg, but skip the split when clap has collected multiple values (i.e. `args.selected.len() > 1`). Document the limitation in README.
- **Preferred**: drop the comma-splitting on `--selected` entirely. Keep it on the deprecated `--initial` only (which advertises CSV). Document that `--selected` is repeatable. This removes the foot-gun while the legacy path keeps working.

Add a test that pre-selects a value containing a comma.

---

### 2. `cli/tests/choose_cli.rs` module docstring still describes pre-review-1 framing

Severity: Low

The file header (`cli/tests/choose_cli.rs:1-14`) still says the tests exist as "reached the event loop" smoke proxies, which was review-1's complaint. The real green-path coverage has moved to `run_with_writer` unit tests in `choose_one.rs` / `choose_many.rs`. The docstring reads like the old plan; new readers may think the feature is still under-covered.

Suggested fix: update the module header to note that the in-module `run_with_writer` tests are the authoritative green-path coverage and that this file remains for clap-level parsing regressions plus opt-in PTY flows.

---

### 3. README `--height` documentation is incomplete

Severity: Low

`cli/README.md:19` documents `--height <N>` only as "render inline in N rows below the cursor". The flag now accepts a percentage suffix (`50%`) with a floor-of-3 clamp, and the CHANGELOG lists this under "Added". Readers looking at README alone will not discover the percentage form.

Suggested fix: expand the global-flags entry to `--height <N|PCT%>` with the floor-of-3 note, matching the CHANGELOG wording. Same README should also mention `--sort` and the fuzzy-filter-on-by-default + `--no-filter` behaviour under `choose-one` and `choose-many` (both are already there, but worth cross-checking the height section).

---

### 4. `run_propagates_percent_height_to_prompt` in choose-many does not assert output

Severity: Low

`cli/src/commands/choose_many.rs:395-416` asserts only `status == 0` and the height value the closure saw. Its sibling in `choose_one.rs` asserts `output == b"B\n"`. The choose-many variant should likewise pin a concrete output so a regression in `write_list` under a non-None height is caught.

Suggested fix: add `assert_eq!(output, b"A\n")` (or similar, matching the fixture) after the status check.

---

### 5. Fuzzy highlighting has no test for multi-byte UTF-8 labels

Severity: Low

`FuzzyFilter::highlight_indices` returns `Vec<u32>` of `Utf32Str` (char) offsets (`lib/src/core/fuzzy.rs:181`), and `build_highlighted_spans` in both choose components iterates with `label.chars().enumerate()` and compares against char offsets (`lib/src/components/choose_one.rs:716`, `lib/src/components/choose_many.rs:773`). The math is correct, but no test covers a non-ASCII label. `smart_case_matches_case_insensitive_when_pattern_is_lower` only exercises the match path, not the highlight path, and uses ASCII letters that happen to be in uppercase form for the Ä/Ö vocabulary.

Suggested fix: add a test like `highlight_indices_handles_multibyte_labels` that builds a label such as `"Café"` or `"Grünes Tee"`, requests a pattern (`"ca"` or `"gr"`), and asserts the resulting `build_highlighted_spans` output contains a styled span at the right char position rather than a byte-offset mismatch.

---

### 6. No direct unit coverage of `resolve_option_strings` stdin path

Severity: Low

`resolve_option_strings` (`cli/src/commands/common_choose.rs:215`) reads `io::stdin()` directly when neither legacy sources nor positionals are present. That branch is covered only indirectly via `assert_cmd`'s `write_stdin` in `cli/tests/choose_cli.rs`. The unit tests in `common_choose.rs` cover the legacy-source and positional branches but skip the stdin branch because there is no seam for injecting a reader.

Suggested fix (optional): extract the read from `io::Read` into a helper, e.g.

```rust
pub fn read_option_strings_from(reader: impl Read) -> io::Result<Vec<String>>;
```

and add unit tests for CR stripping, empty-line filtering, and the "lines.is_empty() → InvalidInput" branch using a `Cursor<Vec<u8>>`. The public `resolve_option_strings` then delegates to it. This also reduces the assert_cmd surface.

---

### 7. `adjust_scroll` uses `state.filter.visible()` but is called with a `visible_indices` local — minor duplication

Severity: Low (ergonomics)

Both `choose_one::draw_list` and `choose_many::draw_list` clone `state.filter.visible().to_vec()` into a `visible_indices` local, then call `adjust_scroll(state, visible, visible_indices.len())`. Inside, `adjust_scroll` re-reads `state.filter.visible()` (`lib/src/components/choose_one.rs:758`, `choose_many.rs:813`) to locate `state.hover`. That is correct but is a second traversal of the same slice plus re-borrow, and it reads subtly because the caller already has the data.

Suggested fix: pass `&visible_indices` (or `&[usize]` by borrow) into `adjust_scroll` and remove the re-read. Small readability / maintenance win; no functional change.

---

### 8. `FrameChrome` + filter-active interaction has no rendering integration test

Severity: Low

Unit tests cover `FrameChrome` rendering (`lib/src/core/frame.rs` tests) and the search-prompt rendering (`render_draws_search_prompt_row_when_filter_visible`) separately, but nothing asserts that a bordered + margined + filter-active choose widget composes correctly. Ratatui's `render_stateful_widget` should make this trivial — the risk is in `draw_list`'s width checks (`area.width >= HIGHLIGHT_MIN_WIDTH`) after the border has shaved two columns off narrow frames.

Suggested fix: add one integration test in `lib/src/core/standalone.rs`'s `drive_event_loop_with_chrome` module that renders a ChooseOne widget inside a `FrameChromeConfig` with a filter pattern pre-seeded, then asserts the buffer still shows the search prompt row inside the border and at least one matching label.

---

### 9. `--height 100%` has no end-to-end coverage for the "close-to-fullscreen" path

Severity: Low

Tech-design §14 flagged `--height 100%` as a design question (should it opt into `Viewport::Fullscreen`?). The implementation currently resolves to `Viewport::Inline(term_rows)`. The library unit test `height_spec_percent_100_resolves_to_term_rows` covers the math, but there is no CLI smoke test proving that `question choose-one a b c --height 100%` runs end-to-end. Given that the path diverges from the fullscreen default, a single PTY-gated integration test would de-risk a regression in edge geometry.

Suggested fix: add a PTY case in the existing `mod pty` block that spawns with `--height 100%` and asserts exit code 0 after pressing Enter, so the inline-100% path keeps passing.

---

## Summary Table

| # | Severity | Area | Fix effort |
|---|----------|------|------------|
| 1 | Medium | `--selected` comma splitting for choose-many | ~30 min |
| 2 | Low | Integration-test docstring | <5 min |
| 3 | Low | README `--height` doc | <5 min |
| 4 | Low | Missing output assertion in one choose-many test | <5 min |
| 5 | Low | UTF-8 highlight test | ~10 min |
| 6 | Low | Stdin read seam + unit test | ~20 min |
| 7 | Low | `adjust_scroll` duplication | ~10 min |
| 8 | Low | FrameChrome + filter render integration | ~15 min |
| 9 | Low | `--height 100%` PTY smoke | ~10 min |

## Notes

I ran the following against the worktree:

- `cargo test -p biscuit-tui -p biscuit-tui-cli` — all suites pass (342 lib + 134 CLI + 6 + 39 doctests + per-subcommand integration + exit-codes + help-contract all green).
- `cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets -- -D warnings` — clean.

The PTY-backed tests (`cli/tests/choose_cli.rs::pty`) remain gated behind `QUESTION_INTERACTIVE_PTY=1` and cover Esc, Ctrl+C, and Ctrl+A end-to-end; they were not run in this non-interactive review but the gate-skip branches are green.
