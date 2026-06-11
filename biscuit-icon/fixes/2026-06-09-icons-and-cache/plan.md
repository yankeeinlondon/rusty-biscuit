---
agent: claude
phases: 6
created: 2026-06-10
start_phase: 1
supersedes: plan.archived-2026-06-09.md
packages:
  - biscuit-icon
  - biscuit-icon-cli
---

# biscuit-icon — Icons and Cache Reporting: Execution Plan

Derived from the current [`spec.md`](spec.md) (status: *ready for planning and
implementation*, last updated 2026-06-09).

## Why this plan was regenerated

A prior plan ([`plan.archived-2026-06-09.md`](plan.archived-2026-06-09.md)) was
written, fully executed, and reviewed ([`review-1.md`](review-1.md), "ready for
production"). Re-reading the **current** spec against the committed code shows
the spec has since gained an **Argument validation** section — `icon show` with
zero tokens must error, and every token must be at least three characters — that
the archived plan never covered and the code does **not** implement. The
old plan's "all green" state is therefore stale against the revised spec.

This plan is the spec's current executable form: it frames the already-shipped
spec areas as **verify-and-confirm** tasks (grounded in the files where the
behavior lives) and carries the genuine **net-new** argument-validation work as
the focus of Phase 2.

## Implementation status (verified 2026-06-10)

Confirmed **present and tested** in the worktree:

- v2→v3 migration with idempotent `ALTER TABLE sets` guards, `user_version = 3`
  (`lib/src/cache/store.rs`); migration tests pass.
- `Icon::css()` percent-encoded data-URI (`lib/src/icon.rs:161`) + tests.
- Offline-resize obligation test (`lib/src/icon.rs:414`), curated + cached paths.
- `catalog::{icon_meta, cached_icon_meta, suggestions, offline_icons, offline_sets}` + tests.
- Shared `ShowFlags` at top-level / `show` / `domain`; `CacheAction::{List, Clear{filter}}`;
  the bare `cache clear` subgroup is removed (`cli/src/args.rs`).
- `show` inexact-match dispatch, picker/list/auto, formats, meta + display tables (`cli/src/commands.rs`).
- `domain` sets table / variants table (`Icon` column + `--verbose` Iconify ID) / infallible single-icon.
- `cache list` (conditional `Display` column); `cache clear [filter]`.

Confirmed **missing** (net-new):

- **Argument validation** (spec §Argument validation; acceptance rows
  `icon show` (no args) and `icon show ab`). `run_show` currently lists offline
  icons on empty `ids` and applies no 3-character check; the existing tests
  `show_no_filter_lists_offline_only` and `bare_invocation_shows_help` encode
  the superseded behavior.

## Conventions for every phase

- Library in `biscuit-icon/lib`; CLI in `biscuit-icon/cli`; binary `icon`.
- en-US spelling in symbols/docs. No `# H1` in rustdoc.
- Targeted builds only (`-p biscuit-icon` / `-p biscuit-icon-cli`); prefer the
  package `justfile`: `just test` (L1), `just test-l2` (`--features image`),
  `just lint`, `just doctest`, `just build`.
- Error-UX standard (Design Decision 7): plain language, name the offending
  input, give a concrete next step + example, write to stderr, exit non-zero.
- Never run `cargo fmt` unless explicitly told. Comment-only churn stays out of
  behavior commits (repo Scope-discipline rule).

---

## Phase 1 — Library foundation: cache schema & offline guarantees

The CLI is unsound unless the library honors the v3 schema and the
offline-resize invariant. Everything downstream depends on this phase. All tasks
here are **verify-and-confirm** against existing, tested code.

- [ ] **1.1** Confirm the v2→v3 migration in `lib/src/cache/store.rs` adds
  `author_name`, `author_url`, `tags`, `category` to `sets`, bumps
  `PRAGMA user_version = 3`, runs in a single transaction, and guards each
  `ALTER` with a `PRAGMA table_info(sets)` check for idempotency (Design
  Decision 2).
- [ ] **1.2** Confirm `SetInfo` carries the four new fields and that `put_set` /
  `search_sets` / `all_sets` read and write all ten columns in stable order.
- [ ] **1.3** Confirm `Icon::css()` (`lib/src/icon.rs`) emits
  `url('data:image/svg+xml,…')` with `#`,`<`,`>`,`"`,`'` percent-encoded
  (`%23 %3C %3E %22 %27`) per Design Decision 3.
- [ ] **1.4** Confirm the **Offline Resize** invariant: once an `Icon` exists,
  `.width(w).height(h).svg()` re-assembles locally in `Style::assemble` with no
  `reqwest` / `IconifyClient` call (spec §Library Invariants).
- [ ] **1.5** Confirm the L1 **offline-resize test obligation**
  (`lib/src/icon.rs:414`) asserts both halves: (a) a curated `DomainIcon`
  resized to `64×64` keeps the original `viewBox`, emits `width="64" height="64"`,
  and constructs no `reqwest::Client` / `IconifyClient::fetch_body`; (b) a
  `wiremock`-fetched Iconify icon re-built from cache resizes locally on the
  second pass (exactly one mock request).
- [ ] **1.6** Confirm the `catalog` metadata helpers implement the set-level
  fallback mapping (Design Decision 1): `Set` = cached title or prefix,
  `Categories` = curated enum-set name or `N/A`, `Tags` = set-level tags; and
  that `suggestions` searches the unified offline catalog only (Design
  Decision 6).

**Validation checkpoint (V1):** `just test` (L1) green, including
`fresh_db_is_user_version_3_with_all_columns`, `v0_db_migrates_to_v2_preserving_data`,
`v1_db_migrates_to_v3`, the `css_*` tests, `offline_resize_obligation_test`, and
the `catalog` tests. `just lint` clean for the lib.

---

## Phase 2 — CLI argument model & validation (primary net-new work)

Depends on Phase 1. The argument-validation contract must be correct before the
per-command phases lean on it. **This phase contains the only build-from-zero
work; the rest is verification.**

- [ ] **2.1** Confirm the clap shape in `cli/src/args.rs`: shared `ShowFlags`
  flattened at top level / `show` / `domain`; `CacheAction::{List,
  Clear{filter: Option<String>}}`; the bare `cache clear` (no subgroup) gone so
  `cache clear` parses as `Clear { filter: None }`.
- [ ] **2.2** **Build:** zero-token validation in `run_show`
  (`cli/src/commands.rs`). When `ids` is empty for an explicit `icon show`, fail
  with an error-UX-compliant message stating at least one icon id or filter is
  required, with a concrete example (`icon show mdi:home` or
  `icon show <filter>`). Exit non-zero. (Replaces the current
  empty-ids → `show_offline_list` path.)
- [ ] **2.3** **Build:** 3-character minimum check on every `show` token. Reject
  any token shorter than three characters with an error that names the offending
  token, explains the 200k-icon rationale, and suggests a longer query or an
  exact id. Exit non-zero.
- [ ] **2.4** **Decide & document** the bare-`icon` vs explicit-`icon show`
  boundary: bare `icon` (no subcommand, no filter) keeps its `main.rs`
  help short-circuit; the new validation applies to the resolved `show` command.
  State this assumption explicitly in the PR description.
- [ ] **2.5** Confirm format-flag exclusivity (`check_format_exclusivity`): more
  than one of `--svg` / `--code-block` / `--css` errors with exit 1 and an
  error-UX message naming the rejected combination (spec §Formatting
  Exclusivity + Design Decision 7).
- [ ] **2.6** Confirm `render_error` (`cli/src/main.rs`) routes all errors
  through `Prose` (`<red><b>Error:</b></red> …`) to stderr, exit 1, and that the
  new validation errors carry an example invocation.
- [ ] **2.7** Update/replace the superseded tests
  `show_no_filter_lists_offline_only` and `bare_invocation_shows_help` to the
  new contract (explicit `icon show` with no ids now errors; bare `icon` still
  prints help).

**Validation checkpoint (V2):** new L2 tests — `icon show` (no args) →
error + example (exit ≠ 0); `icon show ab` → error naming the token (exit ≠ 0);
`icon show mdi:home` → normal behavior; mutually-exclusive format flags → error.
`just lint` clean.

---

## Phase 3 — `show` command behavior

Depends on Phase 2. Phases 3, 4, 5 are **mutually independent** and may proceed
in parallel once Phase 2 lands. Verify-and-confirm against existing code.

- [ ] **3.1** Confirm direct-id resolution (`resolve_ids` / `emit_resolved`):
  single id → single render; ≥2 ids → `ID` + `Display` table; any unresolved id
  → error to stderr with suggestions where applicable, exit 1 (spec §Multiple
  icons, §Inexact `:` miss).
- [ ] **3.2** Confirm inexact-match dispatch for a single `<filter>` without `:`
  (`show_inexact_match`): TTY + ≥2 → `choose_many` picker; TTY + 1 → auto-render;
  non-TTY → list one-per-line (`<render>  <id>`); 0 → error. `--list` forces list
  on TTY; `--pick` forces picker and errors in non-TTY; Esc/Ctrl-C → exit 130;
  pick-zero → exit 0.
- [ ] **3.3** Confirm `Format::render`: `--svg` raw text, `--css` data-URI,
  `--code-block` Darkmatter-highlighted fence, default → terminal ladder.
  **Verify no debug preamble leaks to stderr** for `--code-block` (acceptance:
  the old `CODE_RENDERER width=…` stderr line is gone).
- [ ] **3.4** Confirm `--meta` forces a metadata table at any arg count with
  columns `Set`, `Icon`, `Categories`, `Tags`, and `Author` / `License` when
  populated; the display column honors the active format flag.

**Validation checkpoint (V3):** L2 coverage for the acceptance rows
`icon show mdi:home`, `… mdi:home mdi:user`, `… --meta`, `… home` (picker / pipe
/ `--list`), `… homex` (error), `… uil:bad-name` (error + suggestions),
`… --svg` / `--css` / `--code-block`. `just test-l2` green.

---

## Phase 4 — `domain` command (offline-first, curated-enum-only)

Depends on Phase 2. Parallelizable with Phases 3 and 5. Verify-and-confirm.

- [ ] **4.1** Confirm `icon domain` (no args) → table of the 16 enum names with
  columns `Domain Set`, `Variant Count` (`domain_sets_table`).
- [ ] **4.2** Confirm `icon domain <enum>` → variants table with `Variant`,
  `Icon`, where the `Icon` cell renders through the same ladder as `show`
  (glyph → image → SVG code block → SVG text), the Iconify id is hidden by
  default, and `--verbose`/`-v` adds an `Iconify ID` column
  (`domain_variants_table`, Design Decision 8).
- [ ] **4.3** Confirm `icon domain <enum>:<variant>` → single infallible render;
  honors `--svg`/`--code-block`/`--css`; the Iconify id is **never** emitted,
  even with `--verbose`.
- [ ] **4.4** Confirm `icon domain <filter>` (no `:`) → list matching enum
  names; 0 → error; ≥1 → list. No picker, no network, no cache (offline-first).
- [ ] **4.5** Confirm flag parity: `--from` limits substring search;
  `--svg`/`--code-block`/`--css` mutually exclusive (same error UX);
  `--meta`/`--list`/`--pick`/`--nerd` accepted as silent no-ops for `show`
  symmetry.
- [ ] **4.6** Confirm `icon domain sport:baseball` → errors "not a curated enum"
  (a curated enum is required; an Iconify prefix is rejected by design).

**Validation checkpoint (V4):** L2 coverage for `icon domain`,
`icon domain emoji`, `icon domain emoji --verbose`, `icon domain emoji:happy`
(never prints id), `icon domain emoji:happy --svg`, `icon domain sport:baseball`
(error).

---

## Phase 5 — `cache` command

Depends on Phase 1 (v3 schema) and Phase 2 (action enum). Parallelizable with
Phases 3 and 4. Verify-and-confirm.

- [ ] **5.1** Confirm `icon cache list` → table `Set`, `Icon`, `Display`,
  `Categories`, `Tags`, where `Display` is **omitted** unless Nerd Font is
  enabled, the icon has a curated Unicode glyph, or the `image` feature + a
  terminal image protocol is present (Design Decision 4).
- [ ] **5.2** Confirm `icon cache clear` (no filter) → deletes all rows from both
  `icons` and `sets` (file preserved); prints a confirmation (Design
  Decision 5).
- [ ] **5.3** Confirm `icon cache clear <filter>` → deletes `icons` rows whose
  `prefix || ':' || name` contains the filter (case-insensitive); leaves `sets`
  intact; prints the deleted count (`clear_filtered`).
- [ ] **5.4** Confirm the removed bare-`cache clear` form parses as
  `Clear { filter: None }` (the old no-subgroup variant is gone).

**Validation checkpoint (V5):** L2 coverage for `icon cache list`,
`icon cache clear`, `icon cache clear home` (leaves `sets`), and that the old
top-level form is parsed as the new `Clear`. `just test-l2` green.

---

## Phase 6 — Acceptance sweep, drift maintenance, final validation

Depends on Phases 1–5. The final gate.

- [ ] **6.1** Walk every row of the spec §Acceptance Criteria and confirm a
  passing L1/L2 test (or add the missing one). Pay special attention to the
  net-new rows: `icon show` (no args), `icon show ab`, `icon mdi:home --svg`
  (top-level flag), `icon domain emoji:happy --svg`, and the `--code-block`
  no-stderr-preamble row.
- [ ] **6.2** Confirm the §Backwards Compatibility contract: bare
  `icon cache clear` removed; `icon <filter>` non-TTY still lists (script
  compatibility).
- [ ] **6.3** Update drift artifacts to match shipped behavior:
  - `biscuit-icon/README.md` (public behavior changes — incl. the new
    `show` argument-validation rules).
  - `.claude/skills/biscuit-icon/cli.md` — add the `show` argument validation +
    3-char minimum; **reconcile the `domain` section**, which still lists
    `Variant`, `Glyph`, `Iconify ID` (spec renames `Glyph` → `Icon` and hides
    the id by default behind `--verbose`).
  - `.claude/skills/biscuit-icon/cache.md` and `domain-icons.md` where behavior
    drifted.
  - `biscuit-icon/docs/dependencies.md` if any crate was added/removed.
- [ ] **6.4** Regenerate `hash:` frontmatter for every edited skill file with
  `md hash <file>` (Darkmatter hasher). Mirror to the global
  `~/.claude/skills/biscuit-icon/` copy if the workflow requires it.
- [ ] **6.5** Comment/doc pass over every symbol whose behavior changed in this
  effort (the new validation path) — fix or delete drifted `///`/`//` comments
  in the same change (repo Authoring-discipline rule).
- [ ] **6.6** **Final checkpoint:** from `biscuit-icon/`, run `just build`,
  `just lint`, `just test`, `just test-l2`, and `just doctest` — all green.

**Definition of done (V6):**

- Every §Acceptance Criteria row maps to a passing test or a demonstrated CLI
  behavior; any L2/manual-only row is noted.
- `just build && just test && just test-l2 && just lint && just doctest` pass
  for both packages.
- Out-of-scope items untouched: `icon sets`, shell completions, `domain` network
  lookups, `cache show`, and any `--meta` column beyond the listed set (spec
  §Out of Scope).
- No raw escape codes emitted (rendering goes through `Prose`/`Table`/`TerminalImage`).

---

## Dependency & parallelization summary

```
Phase 1 (library foundation, verify)
   └─> Phase 2 (CLI args + NET-NEW validation)
          ├─> Phase 3 (show)     ┐
          ├─> Phase 4 (domain)   ├─ parallelizable
          └─> Phase 5 (cache)    ┘
                 └─> Phase 6 (acceptance sweep + drift + final validation)
```

- **Serial spine:** Phase 1 → Phase 2 → (3 ∥ 4 ∥ 5) → Phase 6.
- **Parallel work:** Phases 3, 4, and 5 touch largely disjoint paths in
  `cli/src/commands.rs` and may run concurrently after Phase 2.
- **Highest-risk task:** the Phase 2 argument-validation build (2.2–2.3, 2.7).
  It changes existing behavior and supersedes two passing tests; land it before
  the per-command phases depend on the new contract.

## Notes / risks

- The bulk of this spec ships already; most tasks are verification. Treat a
  failed verification as a real defect (the archived plan + review-1 predate the
  current spec and cannot be trusted for the validation contract).
- `--code-block` previously leaked a `CODE_RENDERER width=…` line to stderr; the
  acceptance criteria require this to be gone — assert it explicitly in Phase 3.
- Image-in-table-cell alignment is a known cosmetic limitation; acceptance tests
  run off-TTY where the ladder degrades to glyph/identifier text.
