---
ready: false
implemented: true
agent: "codex/default"
created: "2026-06-26T02:12:03"
---

# Review 1

Not ready for production. The implementation covers a substantial amount of the TAB path and basic ENTER-path plumbing, but the runtime chooser behavior is not verified at the level required by the review rubric, and a couple of implementation details can produce incorrect user-visible behavior.

## Findings

### High: Keyboard-driven chooser behavior has no Level 3 verification

Requirement: the runtime chooser is user-keyboard driven: users move through choices, submit with Enter, cancel with Esc, confirm with `Y`/Enter or decline with `n`/Esc, and `file[]` uses multi-select toggling. The spec also requires type-driven chooser behavior and SplitPane layout to be verified through the terminal harness ([spec.md](spec.md:159), [spec.md](spec.md:193), [spec.md](spec.md:207)).

Implementation/test coverage: the only new terminal tests are Level 2 tmux/WezTerm tests that inject terminal bytes or terminal-CLI key names (`send_key("Enter")`, `send_key("Space")`, `send_text(b"\r")`, `send_text(b" ")`) in [level2_auto_complete_chooser.rs](../../cli/tests/level2_auto_complete_chooser.rs:43). There is no `level3_*` test for autocomplete, no `RUN_LEVEL3=1` gate, and no OS keyboard injection path comparable to the existing Ctrl+C Level 3 suite.

Impact: under this review's required taxonomy, terminal-CLI byte injection does not verify what the terminal actually emits for the user's keypresses. Any regression in terminal input encoding, focus, or key handling for Enter/Esc/Space/arrow navigation can ship while the current tests still pass.

Fix direction: add a Level 3 test for the runtime chooser in a real terminal window using OS keyboard injection. At minimum cover Enter submit, Esc cancel, arrow navigation, and Space toggling for `file[]`. Keep the current tmux/WezTerm capture tests as Level 2 rendering/layout coverage.

### High: The current Level 2 tests do not prove `file` uses `ChooseOne` and `file[]` uses `ChooseMany`

Requirement: a `file` property/argument must drive `ChooseOne`, and a `file[]` property must drive `ChooseMany` ([spec.md](spec.md:193), [spec.md](spec.md:207)).

Implementation/test coverage: the production code does call separate wrappers, `choose_one_file` and `choose_many_files`, in [schema_interactive.rs](../../cli/src/commands/schema_interactive.rs:531). However, the L2 assertions only check for generic candidate/detail markers in [level2_auto_complete_chooser.rs](../../cli/tests/level2_auto_complete_chooser.rs:240). The `file[]` test sends Space and then Enter, but it never asserts a multi-select marker, selected count, array value, or that multiple selected files reached the provider. A single-select chooser would still render candidates/details and could still submit successfully, so the test would not catch the type regression it claims to cover.

Impact: the most visible new `file[]` behavior could silently degrade to single-select while the advertised L2 coverage remains green.

Fix direction: assert UI markers unique to each widget (`ChooseOne` active marker vs `ChooseMany` checkbox/checked marker), and verify the merged frontmatter value shape after submission: string for `file`, array for `file[]`, ideally with two selected files.

### Medium: Operation-file over-cap is counted before applying the mode contract

Requirement: operation-file autocomplete should error only when the count of query-matching candidate files exceeds `MAX_CANDIDATES`, with the operation's file contract applied to candidates ([spec.md](spec.md:159), [spec.md](spec.md:171), [spec.md](spec.md:206)).

Implementation: `gather_candidates` counts every path returned by the scope walker and returns `AutocompleteOverCap` before filtering through `frontmatter::valid_for_mode` ([operation_file.rs](../../cli/src/completion/operation_file.rs:72), [operation_file.rs](../../cli/src/completion/operation_file.rs:96)). For example, `claudine compose plan` can report "narrow your query" because 501 `plan*.md` files match the substring, even if most are inline-compose docs with `prompt:` and only one is a valid compose candidate.

Impact: users can get an over-cap error even when the actual chooser would have had a small, valid candidate set. This is a functionality gap, not just a test gap.

Fix direction: push both the substring predicate and the mode predicate into the counted path, or count only after `valid_for_mode` while still early-aborting when valid candidates exceed the cap. Add a regression with more than 500 query matches where fewer than 500 satisfy the active mode.

### Medium: The detail block does not meet the specified rendering/content contract

Requirement: the confirmation/detail block must show name and path according to the frontmatter-name rules, use a BlockQuote for description, use an UnorderedList/Prose-rendered dim italic `no schema defined`, and avoid raw pseudo-markup/ad hoc strings ([spec.md](spec.md:141), [spec.md](spec.md:148), [spec.md](spec.md:154), [spec.md](spec.md:198), [spec.md](spec.md:212)).

Implementation: `detail_body` builds one raw Markdown/prose string manually: it writes `> ` for the description, `- no schema defined` for schema, hand-builds badge markup strings, and always renders the path on a later `Path:` line rather than the specified no-name/name-with-parenthesized-path line ([autocomplete_ui.rs](../../cli/src/completion/autocomplete_ui.rs:49), [autocomplete_ui.rs](../../cli/src/completion/autocomplete_ui.rs:57), [autocomplete_ui.rs](../../cli/src/completion/autocomplete_ui.rs:66), [autocomplete_ui.rs](../../cli/src/completion/autocomplete_ui.rs:79)). The L2 tests only check broad plain-text markers, not OSC8 link emission, badge styling, BlockQuote/list rendering, or the name/path layout.

Impact: the UI can look and behave differently from the spec while still passing tests. It also misses the repo convention to use `TerminalRenderable` components for terminal output.

Fix direction: build the detail block from the actual `BlockQuote` and `UnorderedList` components or another structured `TerminalRenderable` composition, render the path inline with the name as specified, and add Level 2 capture assertions for OSC8/styling-sensitive output.

## Verification Level Summary

- Shell `__complete`, `completions <shell>`, bare `file` fallback, comma continuation, and YAML sequence surface: Level 1 subprocess/unit coverage is present.
- Operation-file and missing-property chooser rendering/layout: Level 2 tmux/WezTerm capture exists for broad rendering and layout markers.
- Keybinding behavior for chooser submit/cancel/navigation/toggle: strongest coverage is Level 2 terminal-CLI byte injection; required Level 3 coverage is missing.
- Detail block styling/OSC8/TerminalRenderable contract: unit/plain-text checks plus broad Level 2 markers; no strong Level 2 assertions for the specified rendering details.

## Notes

I did not run the full test suite for this review; the findings above come from source and test inspection.
