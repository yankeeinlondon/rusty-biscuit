---
$schema: feature-review.yaml
ready: true
human_review: true
human_review_items:
  - |-
      Before merging, review the pull request description and confirm that it clearly says this change reverses the earlier decision called “D4” in the single-sourcing-schema work: arrays embedded in text now render as compact JSON instead of one item per line. The description should also tell authors to use `as_line_separated(...)` when they want the previous output.
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-16T09:39:15-07:00
spec: 2026-09-13-unify-array-rendering/spec.md
implemented: false
description: A **fix** review of `2026-09-13-unify-array-rendering/spec.md`
fix: 2026-09-13-unify-array-rendering/review-1.md
---

# Review 1: Unify Array Rendering

## Verdict

The fix is **ready for production**. All three interpolation text-output sites
now use `scalar_string`, the competing renderer is deleted, and typed
whole-value boundaries remain typed. The new `as_json` and `as_json5`
functions follow the existing list-family argument contract, use the required
serializers, retain empty containers, and are wired through runtime dispatch,
the catalog, generated documentation, DMLS completion, and DMLS hover.

The changed chooser prompt is verified at Level 2 in a headless tmux session.
The remaining requirements are representation, parsing, composition, catalog,
or documentation contracts for which Level 1 is the appropriate boundary. No
requirement depends on a terminal emulator's input encoder, so Level 3 is not
applicable.

## Findings

### Low — The `as_json` integration test named for bare interpolation uses the `+` path

`as_json_is_byte_identical_to_bare_interpolation` constructs its comparison
value with `{{ '' + items }}` rather than bare interpolation. The separate
`plus_path_and_interpolation_path_render_arrays_identically` test bridges the
two paths for a representative mixed array, and the unit matrix compares
`as_json` with `scalar_string` for empty, scalar, mixed, nested, escaped, and
Unicode shapes. The production equivalence is therefore well supported, and
this does not block readiness.

Rename the test to describe the `+` comparison, or change it to compare two
mixed-string interpolation boundaries such as `x{{ items }}` and
`x{{ as_json(items) }}`. Extending that boundary-level comparison over the
existing shape matrix would make the test independently match its stated
oracle and the specification's exact wording.

### Low — The specification's Level-2 evidence statement is stale

The Evidence section says there are no Level-2 behavior changes and describes
the chooser edit as comment-only. In fact,
`run_file_array_chooser_test` changed its captured-output contract from two
newline-separated paths to one compact JSON array and its assertions were
rewritten accordingly. The implementation record correctly identifies this
drift, and the changed tmux test passes, so this is not a product or coverage
blocker.

Correct that Evidence paragraph in a follow-up specification-maintenance edit
so the durable design record agrees with the implemented and tested behavior.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Bare arrays render as compact JSON and agree with `+` | Level 1 library and spawned-CLI tests | Appropriate; no terminal-specific rendering behavior is involved |
| Objects remain unchanged and exact whole-value frontmatter remains typed | Level 1 compose integration tests | Appropriate |
| `as_json` and `as_json5` serialization, empty arrays, escaping, nesting, null, type errors, arity, aliases, and no trailing newline | Level 1 unit, catalog, and executable-example tests | Appropriate |
| `as_line_separated` remains newline-joined | Level 1 library and spawned-CLI tests | Appropriate |
| Dynamic sequence, loop, and lifecycle whole-value arrays remain typed while mixed strings use JSON | Level 1 sequence, loop, lifecycle, and shared conformance tests | Appropriate |
| Catalog, generated documentation, DMLS completion, and DMLS hover expose both functions | Level 1 catalog and provider tests | Appropriate |
| File-array chooser prompt contains a JSON array with both selected paths | Level 2 tmux pane capture | Appropriate and passing |
| Prompt/example migration and active documentation use intentional explicit formatters | Level 1 shipped-artifact tests plus source audit | Appropriate |
| OS keyboard, mouse, paste, IME, or terminal input encoding | None required | Level 3 is not applicable |

## Implementation Assessment

- `interpolation_output_string` is absent from production code. Its former
  three call sites invoke `scalar_string`, while the object-specific lookup
  hook remains intact.
- `as_json` delegates to the same `scalar_string` authority used by bare text
  interpolation. `as_json5` delegates to
  `biscuit_file::json5::to_json5_compact`, and Darkmatter explicitly enables
  the `biscuit-file/json5` feature.
- Runtime dispatch includes `as_json`/`asjson` and
  `as_json5`/`asjson5`. Catalog orders 97 and 98 preserve the six existing
  formatters' numeric order as specified.
- The conditional nested-span suppression work is correctly absent because
  the related fix has not landed and there is no suppression implementation
  to remove.
- The repository prompt/example audit preserves typed whole-value consumers,
  uses explicit formatters at text boundaries, and leaves the author-owned
  `prompts/commit.md` edit untouched.
- No material performance or ergonomics problem was found. The shared
  serializer helper avoids duplicating validation, and both serialization
  paths are linear in the rendered value size.

## Checks Run

- GitNexus was bound to the `feat-dark-fixes` worktree. Its index matched
  `HEAD` but reported working-tree staleness from the review/spec edits.
  `scalar_string` has CRITICAL upstream blast radius: 205 symbols, 12 direct
  callers, and affected Darkmatter shell-expansion plus Claudine lifecycle,
  loop, sequence, and dispatch paths. The final change analysis reported 99
  changed symbols across 30 files, zero affected indexed processes, and low
  aggregate change risk; it was not partial or truncated.
- `darkmatter/ just test`: **7,820 passed, 0 failed, 7 skipped**.
- `claudine/ just test`: reached **4,391 passed, 1 failed, 9 skipped** before
  fail-fast canceled the remainder. The failure is the pre-existing shipped
  `prompts/implement.md` undefined-`review` failure documented in
  `results.md`; it does not touch array rendering.
- Focused Claudine Level 1 interpolation conformance: **4 passed**.
- Focused Claudine Level 2
  `level2_tmux_file_array_property_uses_choose_many`: **1 passed** in tmux
  parallel self-spawn mode, with no GUI terminal backend selected.
- `git diff --check HEAD`: clean.

The first attempts to use the shared Cargo target directory failed before
compilation because cached `.rmeta` artifacts were read-only. All reported
test evidence above was produced in a separate writable target directory; the
shared artifacts were not modified or deleted.

## Human Review

Human review is limited to the specification's author-facing history
obligation: confirm that the pull request description plainly records the D4
reversal and the migration path. No manual terminal interaction or visual
judgment is required for the implementation itself.
