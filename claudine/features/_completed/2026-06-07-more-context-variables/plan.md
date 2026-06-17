---
phases: 5
created: 2026-06-07
start_phase: 1
source_files_during_phase_1:
- darkmatter/lib/src/markdown/compose/context/catalog.rs
- darkmatter/lib/src/markdown/compose/context/mod.rs
- darkmatter/lib/src/markdown/compose/expression/catalog.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/effects/catalog.rs
- darkmatter/lib/src/effects/mod.rs
- darkmatter/lib/src/lib.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
- claudine/cli/src/commands/context_render.rs
- claudine/cli/src/commands/mod.rs
docs_updated_during_phase_2:
- claudine/features/2026-06-07-more-context-variables/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
- claudine/cli/src/commands/context.rs
- claudine/cli/tests/context_command.rs
- claudine/cli/tests/level2_context_pty.rs
docs_updated_during_phase_3:
- claudine/features/2026-06-07-more-context-variables/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
- claudine/cli/src/commands/context.rs
- claudine/cli/src/commands/context_render.rs
- claudine/cli/tests/context_command.rs
- claudine/cli/tests/level2_context_pty.rs
docs_updated_during_phase_4:
- claudine/features/2026-06-07-more-context-variables/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
- darkmatter/lib/src/lib.rs
docs_updated_during_phase_5:
- claudine/features/2026-06-07-more-context-variables/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
- .claude/skills/claudine/SKILL.md
- .claude/skills/claudine/cli-reference.md
- .opencode/skill/claudine/SKILL.md
- .opencode/skill/claudine/cli-reference.md
source_code:
- darkmatter/lib/src/markdown/compose/context/catalog.rs
- darkmatter/lib/src/markdown/compose/context/mod.rs
- darkmatter/lib/src/markdown/compose/expression/catalog.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/effects/catalog.rs
- darkmatter/lib/src/effects/mod.rs
- darkmatter/lib/src/lib.rs
- claudine/cli/src/commands/context_render.rs
- claudine/cli/src/commands/mod.rs
- claudine/cli/src/commands/context.rs
- claudine/cli/tests/context_command.rs
- claudine/cli/tests/level2_context_pty.rs
documentation:
- claudine/features/2026-06-07-more-context-variables/plan.md
packages:
- darkmatter
- claudine
hash: 560022cd39f7c2f2-82d2c6a01a21cb64
---

# Execution Plan — Claudine Context Catalogs

Converts the [functional specification](spec.md) into an ordered, dependency-aware
execution plan.

## Orientation

This is a **cross-crate** feature spanning two package areas:

- **Darkmatter** (`darkmatter/lib`) — the *source of truth*. It must expose
  **public typed descriptor catalogs** for the three runtime surfaces. None
  exist today; variable/function/effect names are implicit in dispatch and
  capture code:
  - context variables — implicit in `markdown/compose/context/capture.rs`
    (`ContextGroup`, `for_key()`), captured into `ComposeContext`.
  - expression functions — implicit in `markdown/compose/expression/`
    (`dispatch`, `dispatch_fs`, `mod.rs` core operators).
  - side-effect capabilities — concrete `EffectEngine` verb methods in
    `effects/verbs.rs` (incl. overloads: `ensure_file/1` + `ensure_file/2`).
- **Claudine** (`claudine/cli`) — the *consumer*. `claudine context` currently
  parses two embedded Markdown docs (`context.rs` `parse_context_variables` /
  `parse_expressions_doc` via `include_str!`) and renders side-effects as
  `"not implemented yet"`. It must be rewired onto Darkmatter descriptors, the
  Markdown parsers and `include_str!` removed, and all four reports brought
  under one rendering contract.

### Strict dependency

Claudine cannot consume descriptors that do not exist. **Phase 1 (Darkmatter
catalogs) gates Phase 3 (Claudine wiring).** Phase 2 (Claudine rendering infra)
is pure presentation and has **no data dependency on Phase 1**, so it runs in
parallel with Phase 1.

### Confirmed building blocks (verified during planning)

- `biscuit_terminal::components::list::UnorderedList` — supports `with_bullet`,
  `with_hanging_indent` / `without_hanging_indent`. Use for all lists.
- `Prose` supports `<inverse>` / `<reverse>` tags (`prose/tokens.rs:141`) →
  inline-code inverse styling target.
- `biscuit_terminal::utils::block_constraint::visible_width` — ANSI-aware cell
  width for intrinsic measurement and assertions.
- `EffectEngine` verbs are concrete public methods → descriptor catalog can be
  authored adjacent to them and parity-tested against them.

### Parallelization legend

`⇄ PARALLEL` marks tasks/tracks with no ordering dependency that may be worked
concurrently. Everything else is sequential within its phase.

---

## Phase 1 — Darkmatter typed descriptor catalogs (source of truth)

**Goal:** Three public, ordered, parity-guarded descriptor catalogs exported
from `darkmatter`, each mechanically proven to match its runtime surface. No
Claudine changes here.

The three tracks (A, B, C) are mutually independent — `⇄ PARALLEL`. The shared
export/benchmark task (D) depends on A–C.

### Track A — Context-variable descriptors `⇄ PARALLEL`

- [x] Define a public descriptor type in `darkmatter/lib/src/markdown/compose/context/`
      (e.g. `ContextVariableDescriptor { name, display_type, description, category, subsection, order }`)
      and a public `ContextValueType`/display-type representation.
- [x] Author the authoritative descriptor set covering **every** runtime
      variable produced by `ContextGroup` capture (DateTime, Repo, FileChanges,
      Languages, Documents, Os, Hardware, Gpu), including the variables added by
      the completed `more-context-variables` work: repo-scope (`area`,
      `area_description`, `area_root`, `current_packages`, `depends_on`,
      `used_by`), `time_utc`/`time_military_utc`, the document/OS/hardware
      variables, etc. Keep descriptors **adjacent to** the capture registration
      so parity is mechanically enforceable.
- [x] Provide a stable-order accessor (immutable slice or `iter()` in display
      order) plus category/subsection grouping metadata.
- [x] Add an authoritative runtime key enumerator (e.g. extend/expose
      `ContextGroup::for_key` coverage into an `all_keys()`), so a parity test
      has a runtime set to compare against.
- [x] **Parity test:** descriptor name set == runtime context-variable key set
      (exact; adding/removing/renaming a runtime key fails the test).
- [x] **Ordering test:** descriptor traversal order is deterministic.

### Track B — Expression-function descriptors `⇄ PARALLEL`

- [x] Define a public descriptor type in `darkmatter/lib/src/markdown/compose/expression/`
      (e.g. `ExpressionFunctionDescriptor { signature, description, category }`).
- [x] Author descriptors for every callable dispatched by `dispatch` +
      `dispatch_fs` + core operators in `expression/mod.rs` (type predicates,
      math, collection, string predicates/mutations, date formatting + strict/
      relative date validators, `and`/`or`/`has_key`/`contains`/`length`/
      `number`/`round`, and the filesystem set `absolute`/`relative`/
      `file_exists`/`frontmatter` (both signatures)/`markdown_body_empty`/
      `markdown_title`/`validate_schema` (both signatures)). Canonical
      **snake_case** signatures only.
- [x] Provide a stable-order accessor in display order with category grouping.
- [x] **Parity test:** every descriptor name dispatches via `dispatch`/
      `dispatch_fs`, and every dispatchable name has a descriptor (exact set
      equality; new dispatch arm without a descriptor fails the test).
- [x] **Ordering test:** deterministic.

### Track C — Side-effect capability descriptors `⇄ PARALLEL`

- [x] Define a public descriptor type in `darkmatter/lib/src/effects/`
      (e.g. `EffectDescriptor { signature, description, safety }`) with a
      `safety` representation able to express: filesystem-write→mutation-root,
      network→host-allowlist (deny-all default), Markdown-mutation→auto-rehash.
- [x] Author descriptors for **every** `EffectEngine` verb in `verbs.rs`,
      including **all overloaded signatures** (`ensure_file/1` and
      `ensure_file/2` as distinct entries; the 7 frontmatter mutators;
      `merge_frontmatter`; `ensure_dir`; `append_line`; `append_jsonl`;
      `http_post`). Adjacent to the verb impls.
- [x] Provide a stable-order accessor in display order with grouping
      (Frontmatter Mutations / File & Directory / Network).
- [x] **Parity test:** descriptor signature set == dispatchable verb/overload
      set (exact). Adding a verb or overload without a descriptor fails.
- [x] **Ordering test:** deterministic.

### Track D — Public export + performance guard (depends on A, B, C)

- [x] Export the three catalogs from `darkmatter`'s public surface
      (`lib.rs` / module re-exports) so Claudine can reach them without
      reaching into private modules. Prefer immutable static slices / typed
      catalog APIs (per spec Implementation Guidance).
- [x] Confirm catalogs are **static typed metadata**: constructing/reading a
      catalog must not capture runtime context, instantiate an `EffectEngine`,
      or probe the host. Add a test asserting catalog access performs no
      capture/probe where feasible.
- [x] Verify existing Darkmatter context benchmarks show no material
      regression; if a benchmarked capture path changed, update/extend
      regression coverage rather than relaxing the benchmark.

### ✅ Phase 1 checkpoint

- [x] `darkmatter` builds; new public catalogs compile and are exported.
- [x] All Phase 1 parity + ordering + no-probe tests pass
      (`just -f darkmatter/justfile test` or `cargo test -p darkmatter`).
- [x] Benchmarks free of material regression.

---

## Phase 2 — Claudine shared rendering layer  `⇄ PARALLEL with Phase 1`

**Goal:** Reusable, descriptor-agnostic rendering helpers that enforce the
spec's Shared Rendering Requirements. No descriptor data is needed, so this
phase may begin immediately, concurrently with Phase 1. Lives in
`claudine/cli/src/commands/context/` (new submodule) or a `context_render.rs`
helper module.

- [x] **Inline-code helper:** convert backtick-delimited spans in a string to
      `<inverse>…</inverse>` for `Prose` in styled output; preserve surrounding
      prose styling; handle **multiple** spans independently. In plain /
      `NO_COLOR` / `--plain` output, render with **visible backticks**.
      Unmatched backticks must **not panic** (render as literal text).
- [x] **Unordered-list helper:** wrap items through
      `UnorderedList` with the `- ` marker, each item as `Prose`, hanging
      indentation preserved, 1ch right margin, exactly one blank line before and
      after. Never manually prefix `-`/`*`/`•`.
- [x] **Shared table-layout helper:** enforce the 140ch-inclusive width contract
      — fills available width, 1ch left + 1ch right margin, total max **140
      visible cells** counting margins/borders/separators/content; below 140ch
      uses available width without intentional overflow. All width math via
      `visible_width` (visible cells, not bytes/styled length). A single helper
      so all tables share the contract.
- [x] **Context-column width helper:** compute intrinsic `Property` and `Type`
      widths across the **complete** visible context catalog once, independently
      (not forced equal), reused by every context section; final column takes
      remaining width and wraps.
- [x] **Function/capability first-column helper:** intrinsic width **capped at
      40ch**, remaining width to descriptive columns; long signatures wrap via
      `Table` behavior without breaking the width contract.
- [x] Unit tests for each helper at widths **below, at, and above 140ch**,
      including narrow-width wrap-without-panic and the inline-code styled/plain
      duality.

### ✅ Phase 2 checkpoint

- [x] `claudine` builds; helper unit tests pass.
- [x] Inline-code, list, and width helpers proven independently before wiring
      into reports.

---

## Phase 3 — Rewire Claudine context reports onto descriptors

**Depends on:** Phase 1 (catalogs exported) **and** Phase 2 (rendering helpers).

**Goal:** All four reports render from Darkmatter descriptors using the Phase 2
helpers; Markdown parsing fully removed.

- [x] **Remove Markdown sourcing:** delete `CONTEXT_VARIABLES_MD` /
      `EXPRESSIONS_MD` `include_str!`, the `parse_context_variables*` /
      `parse_expressions*` parsers, `ContextSection`/`ExprSection`/`ExprBlock`
      scaffolding, and their tests. No report may read or parse Markdown at
      runtime (Acceptance Criteria 1–2).
- [x] **clap mutual exclusivity:** make `--values` / `--expressions` /
      `--side-effects` mutually exclusive (ArgGroup or `conflicts_with`), failing
      validation before rendering.
- [x] **Default report** (`claudine context`): context-variable catalog grouped
      by Darkmatter category/subsection; columns `Property` (canonical name
      prefixed `ctx.`), `Type` (descriptor display type), `Description`. Shared
      computed `Property`/`Type` widths. Footer points to `--values`/
      `--expressions`/`--side-effects`, describing side effects as **Darkmatter
      capabilities**, not auto-available Claudine operations.
- [x] **`--values` report:** same sections/order/`Property`/`Type`/global widths
      as default; replace `Description` with `Value`. Capture context **once**
      per invocation via the Darkmatter runtime context API; look up by the
      **same descriptor names** used to render rows. **Every** catalog entry
      yields one row even when null/unavailable. Plain value representation:
      strings raw, bool/number textual, arrays comma-separated, objects compact
      serialized, null/unavailable as a **dimmed `null`**. No per-section/per-row
      duplicate capture.
- [x] **`--expressions` report:** expression-language overview (operators,
      precedence, truthiness, variable access, interpolation, condition behavior
      — Claudine-owned presentation prose, **not** parsed from Markdown) plus the
      typed function catalog. Function table columns `Function` (canonical
      signature, 40ch-capped first column) + `Description`. Correct the
      **"Interpolation vs. Condition Mode"** introduction to read exactly:
      `The parser supports two modes with different operator behavior:` followed
      by the mode table/list **without** an immediately adjacent second intro
      line ending in a colon.
- [x] **`--side-effects` report (new):** replace `"not implemented yet"` with the
      side-effect capability catalog from descriptors. Title + intro prose
      explicitly use **"capabilities."** Columns `Capability` (canonical
      signature, 40ch-capped) + `Description` (behavior + return summary) +
      `Safety`. **No availability/status column** and no claim any capability is
      enabled. Include **every** descriptor incl. all overloaded signatures.
      Communicate the catalog-wide constraints: documentation-only / no
      invocation; only an external orchestrator invokes; filesystem writes
      restricted to configured mutation root; network restricted by deny-all
      host allowlist; Markdown mutations honor auto-rehash; membership ≠
      authorization/availability. Perform **no** capability execution, policy
      probe, filesystem mutation, or network request.
- [x] **Inline code everywhere:** route prose, list items, table cells,
      signatures, variable refs, operators, and examples in all four reports
      through the Phase 2 inline-code helper.
- [x] **Lists everywhere:** route every unordered list through the Phase 2
      `UnorderedList` helper (removes the manual `• {item}` / `- ` prefixing in
      `render_expr_blocks`).
- [x] **Output paths:** primary report content via the data-output path
      (`log::data`); usage hints via the message path (`log::message`). Preserve
      established section ordering (no content before the report).

### ✅ Phase 3 checkpoint

- [x] `claudine` builds with no remaining Markdown parsing or `include_str!` of
      catalog docs.
- [x] All four reports render manually for a width ≥140 and a narrow width
      without panic.
- [x] Canonical Darkmatter names/signatures shown without Claudine renaming.

---

## Phase 4 — Tests & validation

**Depends on:** Phase 3.

**Goal:** Full coverage per the spec's Required Tests, on the Claudine side
(Darkmatter-side parity already covered in Phase 1).

### Catalog contract tests (Claudine ↔ descriptors) `⇄ PARALLEL`

- [x] Default + values reports emit exactly one row per context descriptor.
- [x] Expression report includes every expression-function descriptor.
- [x] Side-effects report includes every side-effect descriptor and supported
      signature (incl. overloads).
- [x] Deterministic category + entry ordering in rendered output.
- [x] Assert no report depends on Markdown headings/tables/lists (e.g. grep-style
      guard / absence of parser symbols).
- [x] `clap` rejects combined `--values` + `--expressions` + `--side-effects`.

### Rendering tests `⇄ PARALLEL`

- [x] Render every report **below, at, and above** the 140ch cap.
- [x] Visible row width never exceeds `min(terminal width, 140)`.
- [x] Both 1ch margins counted within the 140ch limit.
- [x] Every context section uses the same computed `Property`/`Type` widths.
- [x] `Property` and `Type` widths are **not** forced equal (construct a fixture
      where they differ).
- [x] Function + capability first-column widths ≤ 40ch.
- [x] Narrow output wraps without panic and retains descriptive content.
- [x] Inline code: inverse styling in styled output, visible backticks in plain
      output, correct in prose / list items / table cells; multiple spans;
      unmatched backticks do not panic.
- [x] Unordered lists use `- `, hanging indentation, 1ch right margin, exactly
      one blank line each side.
- [x] Corrected interpolation/condition wording asserted; no consecutive
      colon-terminated intro lines.

### Command tests `⇄ PARALLEL`

- [x] Each report selection renders its intended catalog.
- [x] `--values` performs **one** capture and includes every context row
      (null/unavailable rows present, not dropped).
- [x] Side-effects report describes **capability** not availability and performs
      no mutation or network access.
- [x] Footer hints never claim side effects are enabled, allowlist-exempt, or
      configuration-free.
- [x] Update the existing L2 PTY suite (`claudine/cli/tests/level2_context_pty.rs`)
      and `context_command.rs` for the new output; remove assertions tied to the
      old Markdown-parsed layout.

### ✅ Phase 4 checkpoint

- [x] Full Claudine test suite passes
      (`just -f claudine/justfile test` / `cargo nextest run -p claudine-cli`).
- [x] Darkmatter test suite still green.

---

## Phase 5 — Documentation & closure

**Depends on:** Phase 4.

**Goal:** Drift-free docs and a clean Definition-of-Done pass.

- [x] Document the new **public Darkmatter metadata API** (the three descriptor
      catalogs) where Darkmatter's public surface is described; note that the
      Markdown topic docs (`context-variables.md`, `darkmatter-expressions.md`,
      `side-effects.md`) remain **explanatory only** and are no longer an API.
- [x] Update Claudine docs for changed `claudine context` behavior: skill
      `.claude/skills/claudine/cli-reference.md` (and `SKILL.md` summary line if
      affected) and any repo topic doc referencing the context command.
- [x] Update `docs/dependencies.md` / per-area dependency docs **only if** crate
      dependencies changed (likely none).
- [x] Refresh any edited skill's `hash:` frontmatter via `md hash <file>`.
- [x] **DoD verification:**
  - [x] Typed metadata APIs exist in Darkmatter; Claudine's four reports satisfy
        the spec using them.
  - [x] Markdown catalog parsers and the side-effects placeholder are removed.
  - [x] Parity, layout, inline-code, list, narrow-terminal, command, and
        no-side-effect tests pass.
  - [x] No open question resolved implicitly in code (surface any that arose).
  - [x] Repository-prescribed checks for the changed **Darkmatter** and
        **Claudine** crates pass: `just lint`, `just build`, `just test`,
        `just doctest` for both areas.

### ✅ Phase 5 checkpoint (feature complete)

- [x] All 14 Acceptance Criteria from `spec.md` demonstrably satisfied.
- [x] Lint + build + test + doctest green for Darkmatter and Claudine.

---

## Dependency graph

```
Phase 1 (Darkmatter catalogs) ──┐
   A ⇄ B ⇄ C → D                │
                                ├─→ Phase 3 (wire reports) → Phase 4 (tests) → Phase 5 (docs/closure)
Phase 2 (Claudine render infra)─┘
   (parallel with Phase 1)
```

## Risks & notes

- **Demand-driven capture vs. full catalog parity.** Context capture is
  lazy/group-gated; the Phase 1 parity test must compare descriptors against the
  *complete* known key set (the `for_key`/`all_keys` enumerator), not the keys
  present in one capture. Repo-scope variables only populate in a monorepo —
  `--values` must still emit their rows as `null` when absent.
- **Overloaded effect signatures** (`ensure_file/1` vs `/2`) are distinct
  catalog entries on both the descriptor and the report; the parity test must
  treat arity as part of identity.
- **`frontmatter` is a read function, not a side effect** — it belongs to the
  expression catalog (Track B), not the effect catalog (Track C). Keep the
  read/write split intact.
- **Save-path note:** the planning brief's literal target path
  (`…/claudineclaudine/…`) is missing a separator; this plan is saved beside
  `spec.md` at `…/rusty-biscuit/claudine/claudine/features/_unscheduled/more-context-variables/plan.md`.
```