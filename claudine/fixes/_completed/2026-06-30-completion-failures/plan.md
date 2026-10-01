---
total_phases: 4
phase: 4
source_files_during_phase_1:
  - claudine/cli/src/completion/operation_file.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/cli/src/completion/autocomplete_ui.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/schema_validation.rs
  - claudine/cli/src/commands/schema_interactive.rs
  - claudine/cli/src/completion/operation_file.rs
  - claudine/cli/src/completion/scopes.rs
  - claudine/cli/tests/level2_provided_partial_file_pty.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
  - claudine/docs/topics/composition.md
  - claudine/docs/topics/completions/auto-complete.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/claudine/composition.md
source_code:
  - claudine/cli/src/completion/operation_file.rs
  - claudine/cli/src/completion/autocomplete_ui.rs
  - claudine/cli/src/completion/scopes.rs
  - claudine/cli/src/commands/schema_interactive.rs
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/schema_validation.rs
  - claudine/cli/tests/level2_provided_partial_file_pty.rs
documentation:
  - claudine/docs/topics/composition.md
  - claudine/docs/topics/completions/auto-complete.md
packages:
  - claudine
  - claudine-cli
---

# Plan — Completion / Compose Resolution Failures (`compose plan spec=everywhere`)

Fixes the four distinct defects observed when running
`claudine compose plan spec=everywhere` from a worktree. Each defect is
independently root-caused below with file:line citations and a verification
goal. Phases are ordered most-isolated → most-involved and can land as
separate commits.

> **Reproduction context (ground truth captured during analysis)**
>
> - Launch cwd: `…/worktrees/rusty-biscuit/renderable/renderable` (the
>   `renderable` package area).
> - Repo prompt: `…/renderable/prompts/plan.md` →
>   `$schema: { spec: file(required;match(**/*spec*.md);eager), design: file(match(**/*design*.md)), plan: file }`.
> - Stale global prompt: `~/.claudine/prompts/plan.md` →
>   `$schema: { spec: string(required) }` (old shape — only `spec`, typed `string`).
> - Intended target: `…/renderable/renderable/features/2026-06-30-style-everywhere/spec.md`
>   — it **is** under the launch cwd, and matches `**/*spec*.md` with the
>   substring `everywhere`.

---

## Root-cause summary

| # | Symptom | Layer | Root cause (verified) | Primary site |
|---|---------|-------|-----------------------|--------------|
| 1 | TUI description shows raw `_feature_` markup, not styled Prose | CLI render | `description_prose` wraps the value in `Prose::escape_text`, which backslash-escapes `_ * < >` so Prose renders them literally | `cli/src/completion/autocomplete_ui.rs:79-86` |
| 2 | Two indistinguishable `plan` entries; stale global copy shown next to repo copy | CLI candidate gather | `gather_candidates` dedups only by **canonical path**, never by prompt name; most-local-wins is never applied | `cli/src/completion/operation_file.rs:99-103` |
| 3 | "Schema:" block drops `spec`, mangles `match(`, renders as a bullet list | CLI render | `schema_list` renders each `$schema` line as a `Prose::escape_text` bullet in an `UnorderedList`; narrow non-wrapping detail pane then corrupts/clips it. **Data layer is faithful** (proven). | `cli/src/completion/autocomplete_ui.rs:88-98`, `:237-244` |
| 4 | `spec=everywhere` fails `no existing file matched reference` instead of glob-matching the one spec | lib validation | A **provided** `file(match(GLOB))` value is only ever resolved as a literal path; the `match()` glob is completion-only metadata and is never consulted to resolve a partial value | `lib/src/composition/schema_validation.rs` (translate path) → darkmatter `format.rs:180` |

**Non-bug confirmed:** the doubled `…/renderable/renderable` in the error's
"resolving from" clause is the **correct** launch-area anchor
(`scopes::property_value_root` = cwd; `cli/src/completion/scopes.rs:86-105`).
No change required there. It is documented as a non-bug so reviewers do not
"fix" it.

### Why Bug 3 is presentational, not data loss

`extract_markdown_detail` → `schema_lines_from_map` → `yaml_lines_from_json`
round-trips `$schema` through `serde_json::Value` → `serde_yaml_ng`
(`lib/src/composition/file_detail.rs:147-189`). The workspace enables
`serde_json`'s `preserve_order` feature (lockfile shows `serde_json → indexmap`),
so a standalone round-trip of the exact `$schema` block yields, faithfully and
in order:

```
[0] "spec: file(required;match(**/*spec*.md);eager)"
[1] "design: file(match(**/*design*.md))"
[2] "plan: file"
```

`Prose::escape_text` (`biscuit-terminal/lib/src/components/prose/prose.rs:139-196`)
escapes `( ) * _ < >`, and `Prose` round-trips the escaped form — so
the `spec` drop and `match(` mangle arise only once the escaped bullets pass
through `UnorderedList` + `render_optimistic(width)` wrapping + the
`Paragraph` (no `.wrap()`, vertical-clipping) detail pane
(`autocomplete_ui.rs:237-244`). Rendering the block as a faithful YAML
**code block** removes every escaping/wrapping hazard at once.

---

## Phase 1 — Dedup prompt-file candidates (most-local-wins) · Bug 2

**Goal:** running `claudine compose plan …` with `prompts/plan.md` (repo) and
`~/.claudine/prompts/plan.md` (global) both present shows **one** `plan`
entry — the most-local (repo) one. The stale global copy is suppressed.

**Change** — `cli/src/completion/operation_file.rs`, `gather_candidates`
(currently `:99-105`):

- Scopes are already iterated in priority order
  (`repo → package_area → package → repo_claudine → user_claudine → extras`;
  `scopes.rs:135-143`) and appended in that order, so a first-seen dedup
  preserves most-local-wins.
- Replace the canonical-path-only `seen` set with a **two-key** dedup that
  runs **before** the alphabetical `sort_by_key`:
  1. drop exact canonical-path duplicates (existing behavior — keep it), and
  2. drop later candidates whose **file stem** (lowercased) was already seen
     from an earlier (more-local) scope.
- Keep the existing `AutocompleteNoMatches` empty check.

**Edge note (document, do not over-engineer):** two genuinely-different
prompts sharing a stem across scopes collapse to the most-local one — this is
exactly the spec's intent ("only the most local one should be retained").
Same-stem files **within one scope/dir** are still both kept only if they
canonicalize differently; the stem rule will collapse them too, which is
acceptable for prompt files (a directory will not hold two `plan.md`).

**Verify:**

- New unit test in `operation_file.rs` tests: seed repo `prompts/plan.md` and a
  fake user-scope `…/.claudine/prompts/plan.md` (via a `ScopeContext` whose
  `home` points at a temp dir); assert `gather_candidates("plan", Compose, ctx)`
  returns exactly one path, and it is the repo one.
- Existing `gather_candidates_*` tests still pass (no behavior change for the
  single-scope cases).
- `just test` in `claudine/`.

---

## Phase 2 — Faithful detail-pane rendering · Bugs 1 & 3

**Goal:** in the chooser/confirmation detail pane, (a) the description renders
as styled Prose (`_feature_` → italic), and (b) the `$schema` block renders as
a syntax-highlighted YAML **code block** containing **all** properties in
authored order, unmangled (`spec`, `design`, `plan` all visible;
`match(**/*spec*.md)` intact).

**Change A — description** (`autocomplete_ui.rs:79-86`, `description_prose`):

- Stop calling `Prose::escape_text` on the description. Render the frontmatter
  description through Prose's markdown subset so inline emphasis renders
  (`Prose::new(text)`), keeping the empty/`"no description"` fallback.
- If a verification shows Prose's subset does not cover the authored markdown,
  route the description through darkmatter's inline markdown → Prose path
  instead (darkmatter is already a dependency of this crate via `FileDetail`).
  Decide by the verification below; do not escape.

**Change B — schema as a YAML code block**
(`autocomplete_ui.rs:88-98`, `schema_list`):

- Replace the `UnorderedList`-of-escaped-`Prose` rendering with a YAML code
  block built from `detail.schema_lines.join("\n")` using
  `darkmatter::markdown::CodeBlock` (re-exported at
  `darkmatter/lib/src/markdown/mod.rs:57`) with language `yaml`.
- Preserve the existing "no schema defined" placeholder when
  `schema_lines.is_empty()`.
- Update `render_file_detail_prose` (`:41-50`) to splice the code block in
  place of the old `schema_list` call; keep the `"\n\nSchema:\n\n"` header.
- `file_detail.rs` needs **no change** — `schema_lines` is already faithful.

**Change C — detail pane must not vertically corrupt/clip the block**
(`autocomplete_ui.rs:237-244`, `render_detail_pane`; `:156-163`,
`chooser_height`):

- A code block must not soft-wrap mid-token. Render it at its natural width and
  ensure the pane reserves enough rows. Raise the `DETAIL_ROWS` floor (and/or
  derive a content-aware height from name + wrapped description + schema line
  count) so the full schema is visible for typical prompt schemas.
- Confirm the confirmation-dialog path (`confirm_one_file` → `eprintln!` of the
  full Prose, `:105-116`) already renders the whole block (no pane height
  constraint there) — it should now show the faithful code block too.

**Verify (goal-driven):**

- Extend `autocomplete_ui.rs` tests with a fixture whose `$schema` is the exact
  repo `plan.md` block. Render `render_file_detail_prose` at a realistic narrow
  pane width and assert the stripped output **contains** `spec`, `design`,
  `plan`, and the literal substring `match(**/*spec*.md)` (un-mangled), in that
  order.
- Add a description test: a `_feature_` description renders with italic SGR
  (`\x1b[3m`) and **not** literal `_feature_`.
- Update the existing `detail_prose_no_schema_renders_dim_italic_unordered_list`
  test (it asserts the old bullet-list `- no schema defined` shape) to the new
  placeholder rendering.
- `just test` in `claudine/`.

---

## Phase 3 — Resolve provided partial `file(match)` values via glob + substring · Bug 4

**Goal:** `claudine compose plan spec=everywhere` (interactive TTY) treats
`everywhere` as a partial: it walks the `match(**/*spec*.md)` glob from the
launch area, filters candidates whose path contains `everywhere`
(case-insensitive), and — finding exactly one — shows a confirmation dialog
("Use this file? (Y/n)"). On confirm, composition proceeds with the resolved
path. Multiple matches → chooser. Zero matches → the existing
`no existing file matched reference` error is preserved.

### Why this is low-risk

Every primitive already exists; this phase **wires** them, symmetric to the
existing missing-required-property flow:

- Match patterns are reachable lib-side via `interactive_shape_for_atom`
  (`schema_validation.rs:814`, `Constraint::Match` at `:883`,
  `InteractiveShape::File { patterns }` at `:887`).
- Candidate walk already exists: `file_candidate_paths(patterns, ctx)`
  (`cli/src/completion/schema_completion.rs:537`), anchored at
  `scopes::property_value_root` (cwd / launch area) — the same anchor the
  runtime resolver uses, so offered == accepted (the established invariant in
  `scopes.rs:86-105`).
- Choosers/confirm already exist: `confirm_one_file` / `choose_one_file`
  (`autocomplete_ui.rs`), used today by the missing-property collector
  `collect_file` (`cli/src/commands/schema_interactive.rs:498-565`).

### Design (mirror the `MissingProperties` round-trip)

The missing-property flow is: lib `prepare` raises a typed error → CLI catches
it, drives the TUI, rewrites overrides, retries `prepare`. Reuse that shape.

1. **lib — detect & classify** (`schema_validation.rs`, in/near
   `translate_schema_failure:258`): when a schema problem is an
   **unresolved file reference** for a property that (a) is `file`/`file[]`
   typed with non-empty `match` patterns, and (b) had a **provided** value that
   is not an existing path, surface a new typed variant instead of the generic
   `SchemaValidation`:
   `CompositionError::UnresolvedFileReference { property, provided, patterns }`
   (carry the same data `MissingProperties` carries for files). Required and
   `eager`-optional file failures both reach this point today
   (`is_eager_file_problem`, `:664-677`); optional-non-eager values are dropped
   and are **out of scope** (note as follow-up).
   - Distinguish "provided partial" from "missing": the value is present in the
     effective frontmatter / `set_overrides` but failed existence resolution.

2. **CLI — resolve interactively** (new helper alongside `collect_file` in
   `schema_interactive.rs`, reusing its candidate→option plumbing): on
   `UnresolvedFileReference`:
   - `candidates = file_candidate_paths(&patterns, &ctx)`.
   - Filter by `provided` as a case-insensitive path substring — reuse the
     exact predicate from `operation_file::path_matches_query` (factor it into a
     shared helper rather than duplicating).
   - `1` → `confirm_one_file` (the spec explicitly wants a confirmation dialog).
     `>1` → `choose_one_file`. `0` → return the original unresolved error.
   - Non-interactive (not both stdin+stderr TTY) → return the original error
     (consistent with `AutocompleteNotInteractive`).
   - On selection, rewrite the `spec` override to the chosen path (repo/`~`/abs
     form via the existing `format_relative_insert` contract) and **retry**
     `prepare` once — exactly as the missing-property loop does.

3. **Literal-first ordering:** only attempt the glob+substring fallback after
   literal resolution fails, so valid explicit paths keep their current
   behavior untouched.

### Crate-boundary note

Glob compile (`MatchGlobs`) + the `ignore`/`globset` walk stay in **claudine-cli**
(`schema_completion.rs`); the lib only classifies and carries `{property,
provided, patterns}`. No `globset`/`ignore` dependency moves into the lib. This
preserves the lib/cli split and matches how `MissingProperties` already hands
file work to the CLI.

**Verify (goal-driven):**

- lib unit test: a source with `$schema: { spec: file(required;match(**/*spec*.md);eager) }`
  and `set_overrides = { spec: "everywhere" }` where no literal `everywhere`
  file exists produces `CompositionError::UnresolvedFileReference { property:
  "spec", provided: "everywhere", patterns: ["**/*spec*.md"] }` (not the generic
  `SchemaValidation`).
- CLI unit test for the substring filter + selection→override-rewrite helper
  (drive with a seeded temp tree; assert single-match resolves to the one
  `…/style-everywhere/spec.md`-shaped path).
- L2 (real-TTY) test under `cli/tests` mirroring the existing autocomplete L2
  coverage: from a seeded launch area with exactly one glob+substring match,
  `compose plan spec=<partial>` reaches the confirmation dialog and, on `y`,
  proceeds. Keep the zero-match case asserting the unchanged error text.
- `just test` and `just test-l2` in `claudine/`.

---

## Phase 4 — Integration verification & docs

- [x] **End-to-end verification** — the interactive manual check cannot run in
  this non-interactive session (compose drives a TTY and cannot accept keypress
  input). Substituted by the automated real-TTY L2 coverage added in Phase 3:
  `level2_pty_provided_partial_single_match_confirms_and_launches` (reaches the
  confirmation dialog and, on `y`, proceeds) and
  `level2_pty_provided_partial_zero_match_preserves_error` (unchanged error) —
  both **pass**. Full unit suite (1864 tests) and lint are green.
  - Note: three **pre-existing** L2 detail-pane layout tests
    (`level2_tmux_chooser_detail_right_in_wide_terminal`,
    `level2_tmux_operation_file_chooser_detail_above_in_tall_terminal`,
    `level2_tmux_sequence_yaml_chooser_detail_above_in_tall_terminal`) fail
    **identically on clean HEAD** on this host — a host-specific tmux-capture
    layout mismatch, **not** a regression from this fix (verified via `git
    stash`).
- [x] **Drift maintenance** (per repo `CLAUDE.md`): documented the
  provided-partial `file(match)` resolution contract. Added a "Provided Partial
  File References" subsection to the schema-validation section of
  `claudine/docs/topics/composition.md` (symlinked into the claudine skill) and
  a "Completing Setter Values" section to
  `claudine/docs/topics/completions/auto-complete.md`. Both record that
  `file(match)` value resolution now consults the glob for provided partials,
  anchored at the launch area (offered == accepted). Regenerated the
  composition.md `hash:` frontmatter.
- [x] Added the `MEMORY.md` pointer summarizing: "doubled launch-area path is
  correct; Bug-3 schema drop was presentational (faithful data + serde
  preserve_order)" (`project_claudine_completion_failures_fix.md`).

---

## Risks & mitigations

- **Prose markdown coverage (Bug 1):** if Prose's markdown subset does not
  render the authored description faithfully, fall back to darkmatter's inline
  renderer (decided by the Phase 2 description test). Mitigated by verifying
  rendered SGR, not by assumption.
- **Code-block width in the split pane (Bug 3):** a wide schema line in a narrow
  pane can still overflow horizontally. The fix targets faithful content +
  adequate height; horizontal overflow of an over-long single line is a
  pre-existing TUI limitation, not a regression — note it, do not expand scope.
- **Provided-partial false positives (Bug 4):** a substring like a common word
  could match several specs; the confirmation dialog (single) and chooser
  (multiple) keep the user in control — never auto-substitute silently.
- **Invariant preservation (Bug 4):** value resolution stays anchored at
  `property_value_root` (cwd), so completion suggestions and runtime resolution
  remain byte-identical. Do **not** re-anchor to repo root.

## Out of scope

- Optional **non-eager** `file` properties whose provided value is silently
  dropped (`drop_invalid_optionals`) — they never surface today; a follow-up if
  desired.
- Re-anchoring `**` globs to the repo root (would break offered == accepted).
- The `$schema` raw-text-vs-serialized question — serialized YAML is proven
  faithful; no need to thread raw frontmatter source.

## Success criteria (Rule 4)

1. Two same-named prompts across scopes → one entry, most-local kept.
2. Detail/confirm view: description styled; `$schema` shown as a YAML code block
   with all properties, in order, unmangled.
3. `compose plan spec=<partial>` with one glob+substring match → confirmation
   dialog → composition proceeds with the resolved path; zero matches → existing
   error unchanged; non-interactive → existing error unchanged.
4. `just test`, `just test-l2`, and `just lint` green in `claudine/`.
