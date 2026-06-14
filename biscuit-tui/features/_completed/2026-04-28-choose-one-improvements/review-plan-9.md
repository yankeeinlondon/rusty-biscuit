# Review 9 Implementation Plan

## Goal

Address every finding in `review-9.md` for the ChooseOne improvements feature
and leave the `biscuit-tui` package area with passing focused tests, passing
full package tests, and zero clippy warnings/errors.

This plan covers two scoped correctness gaps:

1. **`--file` accepts unsupported/plain-list files.** The current
   `parse_file` (`biscuit-tui/cli/src/option_sources.rs:201`) sniffs the body
   for unknown extensions and falls through to `parse_list` for ordinary text,
   which violates the spec's contract that `--file` accepts only JSON, JSONL,
   NDJSON, YAML, CSV, or TOML and must be array-shaped
   (`spec.md:185-187`).

2. **Default `Ctrl+<first-alphanumeric>` hotkeys can shadow explicit
   hotkeys without a CLI duplicate error.** The CLI duplicate check
   (`biscuit-tui/cli/src/choice_normalize.rs:319-333`) only inspects parsed
   explicit hotkeys, while `ChoiceOption::effective_hotkey`
   (`biscuit-tui/lib/src/components/choose.rs:139-149`) and the first-wins
   `build_effective_hotkeys` map
   (`biscuit-tui/lib/src/components/choose_one.rs:847-865`, also reused by
   `ChooseMany` at `biscuit-tui/lib/src/components/choose_many.rs:98`) cause an
   earlier option's auto-derived `Ctrl+<x>` to swallow a later option's
   explicit `[CTRL+x]`.

The two findings are independent in code surface (CLI source parser vs.
CLI/lib hotkey precedence) but are both about the CLI's user-facing input
contract, so each gets its own implementation phase followed by a single
shared verification phase.

## Phase 1 — Restrict `--file` to Spec-Sanctioned Formats

### Scope

Make `--file <path>` reject any file whose extension is not one of the
spec-sanctioned formats and reject sniffed-fallback plain-text content. The
allowed extensions are exactly:

- `.json`
- `.jsonl`
- `.ndjson`
- `.yaml` / `.yml`
- `.toml`
- `.csv`

All other extensions (including `.txt`, `.md`, no extension, or unknown
extensions) MUST surface a clear unsupported-format error before any sniffing
or `parse_list` fallback runs.

### Implementation Steps

1. Update `biscuit-tui/cli/src/option_sources.rs::SourceError`.

   - Add a new variant:

     ```rust
     #[error("unsupported file format '{ext}': supported extensions are json, jsonl, ndjson, yaml, yml, toml, csv")]
     UnsupportedFormat { ext: String },
     ```

   - Keep the existing `NotAnArray` and `Parse` variants unchanged so
     downstream rendering is not perturbed.

2. Update `biscuit-tui/cli/src/option_sources.rs::parse_file` (line ~201) so
   the `match ext.as_str()` arm for unknown extensions becomes an explicit
   `Err(SourceError::UnsupportedFormat { .. })`:

   ```rust
   match ext.as_str() {
       "json" => parse_json(&body),
       "jsonl" | "ndjson" => parse_jsonl(&body),
       "yaml" | "yml" => parse_yaml(&body),
       "toml" => parse_toml(&body),
       "csv" => parse_csv_file(&body),
       other => Err(SourceError::UnsupportedFormat {
           ext: if other.is_empty() {
               "(none)".to_string()
           } else {
               other.to_string()
           },
       }),
   }
   ```

   - This deletes the existing `_ => { ... starts_with('[') ... parse_list(&body) ... }`
     fallback entirely. Do NOT keep partial sniffing for `[` / `{` /
     `---` — the contract is extension-driven, not body-driven. If a
     user has a JSON file with an unusual extension they can rename it
     or pipe it via stdin / `--list`.

3. Confirm the empty-extension case is reported coherently. Files with
   no extension (`extension()` returns `None`) currently produce
   `ext = ""`. The error message above handles that with `(none)` so
   the user sees `unsupported file format '(none)'`.

4. Touch nothing else in this phase — `parse_list`, `parse_json`,
   `parse_jsonl`, `parse_yaml`, `parse_toml`, and `parse_csv_file` all
   keep their existing behavior. The change is strictly the dispatch
   default in `parse_file`.

5. Update or remove any in-tree references to the old plain-text
   fallback. Search the cli crate for tests or docs that rely on `--file
   foo.txt` succeeding:

   - `biscuit-tui/cli/src/option_sources.rs` (existing `mod tests`).
   - `biscuit-tui/cli/tests/choose_cli.rs`,
     `biscuit-tui/cli/tests/choose_one_output.rs`,
     `biscuit-tui/cli/tests/choose_many_output.rs`.
   - Any README/doc that advertises `--file foo.txt` (none expected,
     but check `biscuit-tui/cli/README.md` and
     `biscuit-tui/docs/cli-reference.md`).

   If a test currently uses `--file foo.txt` to feed plain text, migrate
   it to `--list` or rename the fixture to a supported extension.

### Tests to Add or Adjust

In `biscuit-tui/cli/src/option_sources.rs` (existing `#[cfg(test)] mod
tests`, near the existing `parse_file_*` tests around line ~647):

- `parse_file_rejects_txt_plain_list`
  - Write a temp file `options.txt` with body `"Red\nGreen\nBlue\n"`.
  - Assert `parse_file(&path)` returns
    `Err(SourceError::UnsupportedFormat { ext })` with `ext == "txt"`.
- `parse_file_rejects_unknown_extension`
  - Write a temp file `options.dat` containing valid JSON
    (`["Red","Green"]`).
  - Assert `parse_file(&path)` still returns
    `Err(SourceError::UnsupportedFormat { ext: "dat" })`. Body content
    must not be sniffed; the extension is authoritative.
- `parse_file_rejects_no_extension`
  - Write a temp file `options` (no extension) with any body.
  - Assert `parse_file(&path)` returns
    `Err(SourceError::UnsupportedFormat { ext })` with `ext == "(none)"`.
- `parse_file_rejects_md_extension`
  - Write a temp file `options.md` with body `"- Red\n- Green\n"`.
  - Assert `Err(SourceError::UnsupportedFormat { ext: "md" })`. (`.md`
    is intentionally routed through `--md <file> <prop>`, not `--file`.)
- Regression coverage:
  - Keep `parse_file_json_array`, `parse_file_yaml_array`,
    `parse_file_toml_options_*` tests passing unchanged.
  - Add `parse_file_csv_extension_still_works` covering `options.csv`
    with two columns to prove `.csv` is accepted.
  - Add `parse_file_jsonl_extension_still_works` and
    `parse_file_ndjson_extension_still_works` if not already covered.

CLI-level coverage in `biscuit-tui/cli/tests/choose_cli.rs` (or the
nearest existing CLI integration test using `assert_cmd`):

- `choose_one_file_txt_extension_errors`
  - Invoke `question choose-one --file <tmp>/options.txt` with body
    `"Red\nGreen\nBlue\n"`.
  - Assert non-zero exit and that stderr contains the substring
    `unsupported file format 'txt'`.
- `choose_one_file_unknown_extension_errors`
  - Same shape with `options.dat` containing valid JSON; assert the
    process still exits non-zero with the unsupported-format message.

If a parallel test already exists for `--md` rejection or `NotAnArray`,
follow its style for tempfile setup so the new tests stay consistent.

### Focused Verification

Run:

```bash
cargo test -p biscuit-tui-cli option_sources
cargo test -p biscuit-tui-cli parse_file
cargo test -p biscuit-tui-cli choose_cli
```

If exact module filters differ, fall back to `cargo test -p biscuit-tui-cli`
and read output. Confirm the new `UnsupportedFormat` variant fires and the
old plain-list fallback no longer accepts `.txt`.

## Phase 2 — Reject Implicit-vs-Explicit Hotkey Collisions

### Scope

Close the precedence/duplicate-check gap so an earlier option's auto-
derived `Ctrl+<first-alphanumeric>` can never silently shadow a later
option's explicit `[CTRL+<same-char>]` hotkey for either `choose-one` or
`choose-many`.

The fix lives at the CLI normalization layer
(`biscuit-tui/cli/src/choice_normalize.rs`) so the error fires before the
component is constructed. Library precedence is left as first-wins for
embedded callers (per `tech-design.md:340-342`), but the CLI now diagnoses
the collision and refuses to construct a component with an effective-hotkey
collision.

### Implementation Steps

1. Update `biscuit-tui/cli/src/choice_normalize.rs::normalize_options`
   (line ~303) so the duplicate-hotkey check operates on **effective**
   hotkeys rather than only on `ParsedOption.hotkey`.

   - Replace the existing `// Check for duplicate hotkeys.` block at
     lines ~319-333 with a two-pass check that compares effective
     hotkeys post-construction. Concretely, compute the effective
     hotkey for each parsed option using the same rule as
     `ChoiceOption::effective_hotkey`:

     ```rust
     fn effective_hotkey_for(parsed: &ParsedOption) -> Option<HotkeySpec> {
         if parsed.disabled {
             return None;
         }
         parsed.hotkey.or_else(|| {
             parsed
                 .label
                 .chars()
                 .find(|c| c.is_ascii_alphanumeric())
                 .map(|c| HotkeySpec::Ctrl(c.to_ascii_lowercase()))
         })
     }
     ```

     Place this helper next to the existing parser helpers in
     `choice_normalize.rs` so it stays colocated with normalization.

   - Walk `parsed` in order and collect a map keyed by the effective
     hotkey. When an option's effective hotkey collides with an
     earlier option's effective hotkey, emit
     `NormalizeError::DuplicateHotkey` carrying:

     - `hotkey`: a stable display form of the chord (`Ctrl+r`, `Alt+b`)
       — keep the format consistent with the existing error message
       at line 18, but prefer a human-readable form rather than
       `format!("{:?}", hotkey)`. A small `format_hotkey_spec` helper
       returning `"Ctrl+r"` / `"Alt+b"` keeps error rendering clean.
     - `first`: the earlier option's `raw` field.
     - `second`: the later option's `raw` field.

   - The check MUST cover all four collision shapes:
     1. explicit/explicit (already covered today).
     2. implicit/explicit (the review-9 finding — earlier label-derived
        Ctrl shadows later explicit Ctrl).
     3. explicit/implicit (earlier explicit Ctrl shadows later
        label-derived Ctrl).
     4. implicit/implicit (two labels both starting with the same
        ascii-alphanumeric char, which would otherwise quietly
        first-wins the second one out of any keyboard activation).

   - Disabled options contribute no effective hotkey (matches the
     library helper) and so participate in no collision.

   - Numeric-hotkey assignment runs before this check (it already does
     today at lines 315-317), so numeric assignments are seen as
     explicit hotkeys and participate in the same collision logic.

2. Confirm the upgraded check emits the same
   `NormalizeError::DuplicateHotkey` variant that already exists at
   lines 17-22 and is rendered through the CLI error path. No new
   error variant is needed.

3. No library changes are required.
   `ChoiceOption::effective_hotkey`,
   `build_effective_hotkeys` (`choose_one.rs:847`), and the
   `ChooseMany` reuse at `choose_many.rs:98` keep first-wins behavior
   for embedded callers. The CLI now guarantees that no collision
   reaches that layer in practice.

4. Update or relax tests that exercise the old "explicit-only"
   duplicate-check semantics. In particular, verify whether any
   existing CLI test passes options like `["Red", "[CTRL+R] Rose"]`
   and expects success; if so, those tests describe the bug and must
   be updated to expect `DuplicateHotkey`.

### Tests to Add or Adjust

In the existing `#[cfg(test)] mod tests` in
`biscuit-tui/cli/src/choice_normalize.rs` (where the existing
duplicate-hotkey coverage lives, alongside other `NormalizeError`
expectations):

- `normalize_rejects_implicit_default_vs_explicit_ctrl_collision`
  - Inputs (positional shape):
    `["Red", "[CTRL+R] Rose"]`.
  - Expect:
    `Err(NormalizeError::DuplicateHotkey { hotkey, first, second })`
    where `hotkey == "Ctrl+r"`, `first == "Red"`, `second == "[CTRL+R] Rose"`.
- `normalize_rejects_explicit_ctrl_vs_implicit_default_collision`
  - Inputs reversed order: `["[CTRL+R] Rose", "Red"]`.
  - Expect `DuplicateHotkey` (covers the explicit-then-implicit
    direction so first-wins of the explicit doesn't mask the
    implicit collision).
- `normalize_rejects_implicit_default_vs_implicit_default_collision`
  - Inputs: `["Red", "Rose"]` (both start with `R`, both default to
    `Ctrl+r`).
  - Expect `DuplicateHotkey { hotkey: "Ctrl+r", .. }`.
- `normalize_disabled_implicit_does_not_collide`
  - Construct two `RawOption` records where the first carries
    `disabled: Some(true)` and the second's label starts with the
    same alphanumeric char. (Use the existing object-source
    construction helper if one already exists; otherwise build
    `RawOption` literals directly.)
  - Expect `Ok(_)` because disabled options have no effective hotkey.
- `normalize_numeric_hotkey_collision_with_explicit_ctrl_one_is_rejected`
  - Inputs of length >= 1 with `--numeric-hot-keys` enabled and the
    first option carrying an explicit `[CTRL+1]` prefix.
  - Expect `DuplicateHotkey` because the numeric assignment for index
    0 would also be `Ctrl+1`. (Confirms the new check sees numeric
    hotkeys as explicit when `assign_numeric_hotkeys` runs first.)
- Regression: `normalize_rejects_two_explicit_ctrl_r_options`
  (existing) keeps passing unchanged.

CLI integration coverage in `biscuit-tui/cli/tests/choose_cli.rs` (or
the nearest analog), one test per subcommand:

- `choose_one_implicit_explicit_hotkey_collision_errors`
  - Invoke `question choose-one Red "[CTRL+R] Rose"`.
  - Assert non-zero exit and stderr contains `duplicate hotkey 'Ctrl+r'`
    with the two raw labels mentioned.
- `choose_many_implicit_explicit_hotkey_collision_errors`
  - Invoke `question choose-many Red "[CTRL+R] Rose"`.
  - Same assertion. This proves the fix protects both subcommands,
    which matters because `ChooseMany` reuses the same effective-hotkey
    map (`choose_many.rs:98`).

If `assert_cmd` style tests already exist for the existing explicit-
explicit duplicate path, mirror their fixture/setup so the new tests
stay consistent.

### Focused Verification

Run:

```bash
cargo test -p biscuit-tui-cli choice_normalize
cargo test -p biscuit-tui-cli hotkey
cargo test -p biscuit-tui-cli choose_cli
```

Confirm the new collision tests fire `NormalizeError::DuplicateHotkey`
both for `choose-one` and `choose-many` invocations.

## Phase 3 — Full Package Verification and Lint Cleanup

### Scope

Prove the whole `biscuit-tui` package area is clean after Phase 1 and
Phase 2.

### Required Verification Commands

Run from the repository root:

```bash
cargo test -p biscuit-tui -p biscuit-tui-cli
cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets -- -D warnings
cargo test -p biscuit-tui -p biscuit-tui-cli
```

The first command must pass with zero failing tests. The second must
pass with zero warnings (since `-D warnings` upgrades warnings to
errors). The third reruns the full non-gated package tests after any
clippy-induced edits to prove lint cleanup did not regress behavior.

### Optional Gated Verification

Review 9 did not flag the gated PTY suites, but rerun them as a sanity
check that Phase 1 and Phase 2 did not perturb PTY behavior:

```bash
RUN_PTY_TESTS=1 cargo test -p biscuit-tui-cli --test keyboard_protocol -- --nocapture
RUN_SHELL_TESTS=1 cargo test -p biscuit-tui-cli --test completions_shell -- --nocapture
```

Failures here are out of scope for this review unless they were caused
by changes in this plan.

### Lint Expectations

- Fix all clippy warnings/errors in the `biscuit-tui` package area,
  even if they predate this change.
- Do not suppress lints unless the suppression is narrowly scoped and
  locally justified.
- After lint fixes, rerun the full non-gated package tests.

### Documentation Expectations

Light documentation touch-ups limited to the public surface that this
plan changes:

- `biscuit-tui/cli/README.md` and/or
  `biscuit-tui/docs/cli-reference.md`: if either advertises `--file`
  formats, ensure the supported list is exactly
  `json | jsonl | ndjson | yaml | yml | toml | csv` and that
  unsupported extensions are documented as an error.
- No documentation changes are required for the hotkey precedence fix
  beyond the existing
  `tech-design.md:339-342` text, which already specifies that duplicate
  hotkeys must be rejected at CLI parsing time. Phase 2 makes the code
  match that contract.

## Completion Criteria

The review is complete when:

- `review-9.md` finding 1 is resolved: `--file` only accepts files with
  extensions in `{json, jsonl, ndjson, yaml, yml, toml, csv}` and surfaces
  `SourceError::UnsupportedFormat` for everything else, including `.txt`
  and missing extensions, with new unit and CLI integration tests
  covering the rejection paths.
- `review-9.md` finding 2 is resolved: the CLI duplicate-hotkey check
  in `normalize_options` operates on **effective** hotkeys (explicit or
  default `Ctrl+<first-alphanumeric>`) so implicit/explicit and
  implicit/implicit collisions emit `NormalizeError::DuplicateHotkey`
  for both `choose-one` and `choose-many`, with new tests covering each
  collision shape and a CLI-level test per subcommand.
- `cargo test -p biscuit-tui -p biscuit-tui-cli` passes.
- `cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets -- -D warnings`
  passes.
- The post-lint `cargo test -p biscuit-tui -p biscuit-tui-cli` rerun passes.

## Risks and Open Questions

- **Backward compatibility of `--file foo.txt`.** Removing the
  plain-list fallback is the spec-correct behavior, but any existing
  in-tree test or downstream script that passes `.txt` to `--file`
  will break. Migration: use `--list "$(cat foo.txt)"` or rename to a
  supported extension. The repo grep step in Phase 1 mitigates this.
- **Library first-wins vs. CLI strict-reject.** This plan keeps
  library `build_effective_hotkeys` as first-wins (per
  `tech-design.md:340-342`) and only tightens the CLI. If future
  callers want a strict-by-default library behavior, a follow-up could
  expose a `ChoiceInput::validate_unique_hotkeys()` helper that
  embedded apps can opt into; that is out of scope for review-9.
- **Hotkey error-message stylistic change.** Switching from
  `format!("{:?}", hotkey)` to a `Ctrl+r` / `Alt+b` form is a
  user-facing message change. If existing snapshot/string-equality
  tests assert the old `"Ctrl('r')"` rendering, they must be updated
  alongside Phase 2. Grep for `Ctrl(` in `cli/tests/` and
  `cli/src/choice_normalize.rs` test bodies before committing.
- **Disabled-option semantics in the duplicate check.** Phase 2
  treats disabled options as having no effective hotkey, matching
  `ChoiceOption::effective_hotkey`. If the spec ever requires
  disabled options to still reserve their hotkey (e.g., to keep
  it grayed out without conflict), the check would need to widen.
  Today, the spec only requires uniqueness on **active** hotkeys, so
  this is the right default.
