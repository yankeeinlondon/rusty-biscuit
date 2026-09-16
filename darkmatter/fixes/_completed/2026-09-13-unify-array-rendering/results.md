---
created: 2026-09-16
fix: 2026-09-13-unify-array-rendering
status: implementation complete, ready for review
---

# Verification Record: One Array Rendering, and It Is JSON

Base commit: `cb617ba1f`. Host: macOS (Darwin 27.0.0), worktree
`/Volumes/coding/wt/rusty-biscuit/feat-dark-fixes` on `feat/dark-fixes`.

## Ruling 1 — the D3 gate is CLOSED; Phase 5 was dropped

The nested-span fix `claudine/fixes/2026-09-13-better-static-analysis/` is
**active but not implemented**. Its directory contains only `spec.md` and
`spikes/` — no plan, no implementation. Repository searches for the
declared-array-or-object suggestion suppression and its type-lookup plumbing
returned **no implementation hits anywhere in the tree**; every match for
`nested_span` / `NestedSpan` / `nested-span` is in a `spec.md` or `plan.md`.

Per the plan's default ruling, this fix lands first and **Phase 5 (D3) is
dropped entire**. No removal was implemented against code that never existed.

## Phase 2 (D1) — red-first evidence

`cargo nextest run -p darkmatter --test array_rendering_json` on the pre-fix
tree. 3 of 6 failed; the other 3 are guards that must stay green.

| Test | Pre-fix result |
| --- | --- |
| `bare_array_in_body_renders_compact_json` | **RED** — got `a b c` |
| `mixed_frontmatter_string_renders_compact_json` | **RED** — got `String("values: a\nb")` |
| `plus_path_and_interpolation_path_render_arrays_identically` | **RED** — `a b "quoted" 3` != `["a","b \"quoted\"",3]` |
| `exact_whole_value_frontmatter_array_stays_typed` | GREEN guard (typed boundary must not widen) |
| `bare_object_rendering_is_unchanged` | GREEN guard (fix must reach arrays only) |
| `as_line_separated_still_joins_with_newlines` | GREEN guard (migration path must be real) |

All 7 pass after the redirect (`as_json_is_byte_identical_to_bare_interpolation`
was added once Phase 3 landed).

### Intentionally removed tests

Removed with `interpolation_output_string` itself, per the spec's
Breaking-Change Surface table. Their obligations moved to the evaluator-boundary
tests above, not dropped:

- `interpolation_output_string_renders_arrays_line_separated` — asserted the
  superseded default; its subject no longer exists.
- `interpolation_output_string_matches_scalar_string_for_non_arrays` — the
  non-array equivalence is now trivially true (one renderer), and unchanged
  scalar/object behavior is pinned at the evaluator boundary by
  `bare_object_rendering_is_unchanged`.

### Intentionally changed tests

- `evaluator.rs` `bare_array_renders_line_separated` -> `bare_array_renders_compact_json`
  (`"a\nb\nc"` -> `["a","b","c"]`).
- `evaluator.rs` `empty_array_renders_empty` -> `empty_array_renders_empty_json_array`
  (`""` -> `"[]"`).
- `provider_network::a_successful_empty_query_is_still_an_empty_list`
  (`"PRs: "` -> `"PRs: []"`). The test's own doc comment calls this "the one
  case that legitimately renders as an empty list"; `[]` states that where the
  empty string hid it.
- `level2_auto_complete_chooser.rs` `run_file_array_chooser_test` — see the
  L2 finding below.

`rg interpolation_output_string` now returns only this fix's own `spec.md` and
`plan.md` plus the nested-span `spec.md` — historical prose only, no code.

## Phase 3 (D2) — two serializers

`as_json` / `asjson` and `as_json5` / `asjson5` registered as `EvaluationMode::Pure`,
catalogued at orders 97 and 98, generated table regenerated, DMLS completion and
hover covered. `darkmatter/lib/Cargo.toml` now names `biscuit-file`'s `json5`
feature explicitly; `cargo tree -p darkmatter -e features -i biscuit-file`
confirms it.

**Three behaviors of `to_json5_compact` that the plan did not predict** and that
the tests and the catalog example were corrected to match (the formatter was
**not** changed — it is the repository authority):

1. Compact mode is **not** whitespace-free: `['a', 1]`, with `", "` between
   elements and `": "` after keys.
2. Keys are unquoted only when identifier-eligible: `{'dev deps': [...], package: 'x'}`.
3. Key order is **alphabetical**, not insertion order — Darkmatter's
   `serde_json` has no `preserve_order`.

Neither serializer emits a trailing newline (asserted).

`authored_catalog_matches_registration_baseline` (94->96 functions, 101->103
overloads) was updated; it failed `left: 96 right: 94` first, proving the
baseline actually guards the catalog. `every_example_evaluates_to_its_declared_result`
executes both new catalog examples, so they are machine-verified rather than
decorative.

**Table placement note:** orders 97/98 sort globally, so the two new rows render
at the bottom of the generated table rather than beside the other six List
Formatting rows. This follows the spec's instruction to preserve the six shipped
entries' numeric order, but the table now shows List Formatting in two separated
blocks. Flagged for the author's call.

## Phase 4 — audit inventory and dispositions

The repository was already ~95% migrated: nearly every array interpolation in
active prompts, example docs, and fixtures already used an explicit formatter.
**No `.snap` snapshot file was affected** — all 208 under `darkmatter/` and
`claudine/` were checked.

### Migrated (text-embedding)

| Site | Formatter | Why |
| --- | --- | --- |
| `prompts/merge-conflicts.md:18` | `as_unordered_list(conflicts)` | prose introduces an enumerated file list |
| `darkmatter/example-docs/safe-expressions/test.md:24` | `as_csv(ctx.dirty_files)` | the prose above it literally says "as a CSV list"; surrounding prose updated, and `as_json`/`as_json5` added to the contrast list since the family grew 6->8 |
| `claudine/docs/getting-started/index.md:379` | `as_unordered_list(ctx.dirty_files)` | illustrative document, prose introduces a file list |

### Retained (typed whole-value — must NOT be converted)

`prompts/merge-conflicts.md:3` (`conflicts: {{ ctx.merge_conflicts || null }}`,
also feeds a `when:` guard); `claudine/docs/topics/flow-control/sequences.md:216`;
`claudine/cli/tests/sequence_cli.rs:785`;
`claudine/cli/tests/level2_sequence_task_stream_capture.rs:159`;
`claudine/cli/tests/sequence_sources_cli.rs` (7 sites);
`claudine/cli/tests/sequence_errors_cli.rs:197`;
`claudine/lib/src/composition/sequence/tests.rs:1433`.

### Not output-affecting

`darkmatter/dmls/tests/lsp_session.rs:2330,2409` (hover/semantic-token fixtures,
never composed); `context/capture/groups.rs:240-241` and `context/runtime.rs:572`
(group-detection input only); `git_context_integration.rs:265-338`.

### Closed scope, confirmed untouched

`prompts/commit.md` — verified already migrated by the author (`as_csv` for the
singular arm, `as_unordered_list` for the multi arm; no bare array reaches `+`).
`git status --short -- prompts/commit.md` is empty.

### Typed-boundary regressions added

- Darkmatter: `exact_whole_value_frontmatter_array_stays_typed` and
  `mixed_frontmatter_string_renders_compact_json` (`array_rendering_json.rs`).
- Claudine: two rows added to the shared conformance matrix
  (`interpolation_conformance.rs`) — `mixed text + array becomes compact JSON`
  and `mixed text + empty array keeps the empty container`. Claudine's loop and
  lifecycle renderers already call `scalar_string` directly, so their whole-value
  behavior is unchanged by construction; these rows pin the mixed-string half.

## Finding: the spec's "no L2 behavior changes" claim does not hold

`claudine/cli/tests/level2_auto_complete_chooser.rs` `run_file_array_chooser_test`
stages `attachments: {{ doc.attachments }}` — a **mixed string**, not a
whole-value span — then asserted `lines.len() == 2` on the composed output. Post-fix
that value is one line of compact JSON, so the assertion breaks.

The spec's Evidence section says "No L2 or browser behavior changes; the
corrected chooser comment is covered by the existing L2 suite but does not
require rerunning that tier for a comment-only edit." That is **incorrect**: the
edit there is not comment-only. The assertions were re-cut to parse the value as
a JSON array and assert two elements. **The L2 tier was not rerun** (this host
runs L1 only per the recipe); the change compiles and lints clean, but a
reviewer should run `just test-l2` in `claudine/` before merge.

## Documentation

All six authored locations corrected, plus one the audit found:

1. `darkmatter/docs/topics/context-variables.md` — compact-JSON default, `[]` when empty, typed whole-value carve-out, migration note.
2. `darkmatter/docs/inline/fm-interpolation.md` — dropped "or rely on the default line-separated".
3. `darkmatter/docs/topics/darkmatter-expressions.md` — regenerated; `as_line_separated` row no longer claims to be the default; both new rows present.
4. `darkmatter/docs/schemas/expression-functions.yaml` — same description fix plus the two new entries.
5. `darkmatter/features/_completed/.../examples/as_line_separated.yaml` — equivalence claim removed, nothing else touched.
6. `.claude/skills/darkmatter/compose.md` — JSON default, `as_json(...)` spelling, migration path.
7. **Found by the audit, not in the spec's list:**
   `darkmatter/lib/src/markdown/compose/context/capture/datetime.rs:250` —
   `string_array`'s doc comment asserted the line-separated default.

Active comments corrected: `context/capture/repo.rs:193`,
`claudine/cli/tests/level2_auto_complete_chooser.rs:448`,
`functions/mod.rs` (`as_line_separated` doc comment),
`darkmatter/dmls/docs/autocomplete.md` (six -> eight formatters).

Skill hash refreshed with `md hash --save` (never hand-edited):
`ef46db3751d8e999-3dadcf4453a273ba`, and re-verified clean. Frontmatter hash
unchanged; only the body hash moved.

### Final search review — `bare array`, `bare-array`, `line-separated by default`

46 hits, every one reviewed, none silently excluded.

- **Fixed:** the YAML descriptor, the generated table row, and the
  `as_line_separated` doc comment in `functions/mod.rs`.
- **Historical records, left byte-identical:** the `2026-07-08-single-sourcing-schema`
  spec/plan/reviews (the D4 decision being reversed), and
  `darkmatter/fixes/_completed/2026-07-10-function-schemas/phase-1-baseline.md`.
- **This fix's own planning documents:** correct as written.
- **Unrelated:** four `biscuit-tui` docs and two `claudine` config sites use
  "bare array" to mean TOML/config shorthand, nothing to do with interpolation.

## Test results

| Area | Result |
| --- | --- |
| `darkmatter/` `just test` | **7820 passed, 0 failed, 7 skipped** |
| `darkmatter/` `just lint` | clean (exit 0), incl. `zed-dmls` wasm32-wasip2 |
| `claudine/` `just test` | 7015 passed, **4 failed (all pre-existing)**, 9 skipped |
| `claudine/` `just lint` | clean (exit 0) |

### The 4 Claudine failures are pre-existing and unrelated

No Claudine baseline was captured before the D1 edit (a gap in Phase 1), so
these were verified by **changeset exclusion plus file history** instead: every
one reads a file that is not in `git status --short`, and none reports an
array-rendering error.

1. `shipped_prompt_contract::shipped_prompts_have_parseable_schemas_and_expressions`
   — `prompts/_docs.md` lines 23/28: `file_exists(^system-prompt.md)` ->
   "Unexpected character: `^`". Last changed `5d50d4aa8` (2026-09-14).
2. `compose_caller_file_provenance::shipped_implement_router_keeps_the_callers_launch_origin_for_its_lazy_target`
   — `prompts/implement.md` `when:` references undefined variable `review`.
   Last changed `d21cbba06` (2026-09-15).
3. `shipped_prompt_route_drift::fixture_preserves_the_shipped_schema_and_loop_semantics`
   — fixture `$schema` still carries a `log:` property the shipped prompt dropped.
4. `shipped_prompt_route_drift::shipped_implement_prompts_have_not_drifted_from_their_fixture`
   — stale hash pins. `prompts/implement.md`'s **body** hash matches
   (`da1fe238a7b10784`); only its **frontmatter** hash differs, so this is
   frontmatter drift, not rendering drift.

Root cause of 3 and 4: the fixture at
`claudine/cli/tests/fixtures/shipped_implement_route/` last changed `37dbdb33a`
(2026-09-12), while the shipped prompts it mirrors changed 2026-09-15. It was
never re-derived. **Out of scope for this fix** — flagged for the author.

## Graph and scope check

`detect_changes(scope: "all")`: 58 changed symbols across 24 files,
`affected_count: 0`, risk **low**, not partial, not truncated. Every entry falls
inside the intended surface — the evaluator redirect, the deleted renderer, the
two serializers, the registrations and catalog, the audited sites, and the docs.
No unexpected entry.

## PR obligation (author-owned)

The PR description must state in plain terms that this **reverses D4 of
`darkmatter/features/_completed/2026-07-08-single-sourcing-schema/spec.md`**,
and the change should land as its own revertible commit carrying the reversal in
its message, so it can be reverted without taking the static analysis with it.

No commit was made and no lifecycle move was performed; both are author-owned.

## Open items for the reviewer

1. Run `just test-l2` in `claudine/` — the chooser test's assertions changed
   shape and this host ran L1 only.
2. Decide whether the generated table's split List Formatting block is
   acceptable, or whether the six shipped entries should be renumbered.
3. The 4 pre-existing Claudine failures and the stale
   `shipped_implement_route` fixture are unrelated to this fix but are red on
   `main`'s descendant and will need their own change.
