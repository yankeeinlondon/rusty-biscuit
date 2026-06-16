---
status: ready for planning and implementation
reviewed: true
depends-on: 2026-06-15-monorepo-unification
---

# Monorepo CLI Wiring Cleanup

Tighten the wiring between the sniff **library** topology model and its
**consumers** (the `sniff` CLI and downstream claudine). The
[improved-monorepo-capture](../_completed/2026-06-15-improved-monorepo-capture/spec.md)
and [monorepo-unification](../2026-06-15-monorepo-unification/spec.md) features
landed a correct, unified `MonorepoStandard` / `MonorepoLayer` /
`Package.standard` / `Package.provenance` model and removed every legacy type.
This spec does **not** change detection. It addresses a small set of wiring
issues found in a post-unification review: a piece of selection logic that leaks
into presentation code, a join-key type mismatch, and two minor drift/dead-code
items. It also upgrades the `sniff repo is-monorepo` leaf (D5) to surface which
standard a repo uses, building on the primary-layer accessor from D1.

- Use the `sniff`, `cli`, and `rust` agent skills.
- The CLI owns reporting; the library owns all detection/business logic.
- D1–D4 are **not** a JSON contract break: the `sniff repo` / `sniff repo
  structure` JSON stays byte-identical (serde ids are unaffected; see each item
  for its surface impact). Their **text** does intentionally change, however:
  the monorepo standard is now rendered with the unified per-standard label and
  shared `{orchestrator_label} (using {authority_label})` template instead of
  `display_name` (e.g. `Cargo Workspace` → `cargo`,
  `pnpm Workspaces <dim>+ Nx</dim>` → `Nx (using pnpm workspaces)`). D5
  deliberately changes the focused `sniff repo is-monorepo` leaf's text, JSON
  body, and exit-code contract — that break is scoped to that focused leaf and
  is the point of D5. Bare aggregate `sniff repo --json` remains
  byte-identical and continues to expose the existing unwrapped
  `"is-monorepo": bool` member keyed by subcommand name.

## Background: what the review found

The library/CLI boundary is otherwise healthy — business logic lives in the
library, JSON serialization delegates to library serde
(`serde_json::to_value(repo)`), and the legacy `MonorepoTool` /
`workspace_tools` / `discovery_sources` surface is fully gone (only
absence-asserting tests remain). claudine migrated cleanly to
`monorepo_standard` / `monorepo_orchestrators` with a derived deprecated
`monorepo_tool` alias.

Four wiring issues remain (D1–D4, in priority order below); a fifth decision
(D5) upgrades the `is-monorepo` leaf and depends on D1's primary-layer accessor.

## Goals

1. **Stop re-deriving "the primary layer/authority" in every consumer.** Move
   that selection rule into the library as a documented accessor.
2. **Unify the layer→package join-key type** so `MonorepoLayer.packages` and
   `Package.relative` are the same type and the documented 1:1 join needs no
   conversion.
3. **Fix a stale comment** that still names a removed field.
4. **Remove (or correctly gate) a dead fallback** that encodes a false
   invariant about monorepos with zero layers.
5. **Make `sniff repo is-monorepo` carry the standard.** Upgrade the leaf from
   a bare `yes`/`no` predicate to a human-readable authority/orchestrator label
   plus a structured JSON payload, with the exit code acting as the predicate.
6. **One consistent human representation of the monorepo standard across all
   CLI surfaces.** Both `sniff repo` / `sniff repo structure` and `sniff repo
   is-monorepo` render a monorepo standard identically, via a single shared
   label + template helper (the library owns the per-standard label; the CLI
   owns the template composition), retiring `display_name` from the monorepo
   summary.

## Reviewed decisions

These are the proposed designs. Planning/review should refine them, not reopen
the underlying topology model.

### D1 — Add a library accessor for the primary membership layer

**Problem.** There is no library concept of "which layer represents the repo."
Three consumers independently reach into `monorepo_layers.first()` and each
attaches its own meaning:

- sniff text one-liner: `format_monorepo_summary` →
  `layer.authority.spec().display_name`
  (`sniff/cli/src/output/filesystem/repo.rs:151`).
- claudine environment: `layer.authority.spec().id`
  (`claudine/lib/src/events/environment.rs:276`).
- sniff multi-layer listing: `render_repo_section` iterates
  `monorepo_layers` (`sniff/cli/src/output/filesystem/repo.rs:537`).

`.first()` is **not** a defined "primary" concept. `build_monorepo_layers`
fills a `BTreeMap` keyed by root path
(`sniff/lib/src/filesystem/repo/topology.rs:34`), so `.first()` is the
lexicographically-smallest root path; when two authorities share one root
(e.g. Cargo + uv at the repo root) the winner is detector push-order. That
selection rule is a business decision currently encoded by accident inside
presentation code, and duplicated across consumers.

**Decision.** Add to the library (on `RepoInfo`):

```rust
impl RepoInfo {
    /// The layer that best represents the repository as a whole.
    ///
    /// ## Returns
    ///
    /// Selection rule:
    /// 1. The layer whose `root` equals the repo root, if one exists.
    /// 2. Otherwise the layer with the shallowest root (fewest path
    ///    components).
    /// 3. Ties are broken by `MonorepoStandard` enum-declaration order
    ///    (Cargo first … Unknown last) — **not** detector push-order, so
    ///    reordering detectors cannot silently change the primary layer.
    ///
    /// For the canonical Cargo + uv-at-repo-root case this selects Cargo,
    /// matching today's `.first()` output.
    pub fn primary_layer(&self) -> Option<&MonorepoLayer> { ... }
}
```

The sniff CLI and claudine both call `primary_layer()` instead of
`monorepo_layers.first()`. Presentation (display name vs kebab id, `<dim>`
markup) stays in each consumer; only the *selection* moves to the library.

**No `primary_authority()`.** Both consumers (sniff CLI
`format_monorepo_summary`, claudine `environment.rs`) read the authority id
**and** that layer's orchestrators off the *same* layer, so a
`primary_authority()` convenience removes no real duplication. Expose
`primary_layer()` only.

- **Surface impact:** none to JSON. The *selected layer* is unchanged for
  single-root repos (the overwhelming common case): the documented rule selects
  the same layer `.first()` does today for those repos, and the multi-root
  behavior becomes defined rather than incidental. (The CLI *text* rendered from
  that layer does change separately — D4 switches the summary to the unified
  label — but that is the label change, not a change in which layer is chosen.)
  claudine template values are unchanged for single-root repos. Note that
  claudine's `environment.rs` reads `.first()` *unconditionally* (no
  `layer_count` guard), so this rule now defines claudine's `monorepo_standard`
  on multi-root repos; on the single-Cargo-workspace rusty-biscuit repo it is
  unchanged.

### D2 — Unify the layer→package join-key type

**Problem.** `MonorepoLayer.packages: Vec<PathBuf>`
(`sniff/lib/src/filesystem/repo/standard.rs:394`) but the canonical join
target is `Package.relative: String`
(`sniff/lib/src/filesystem/repo/types.rs:84`). The layer value is literally
`PathBuf::from(&pkg.relative)`
(`sniff/lib/src/filesystem/repo/topology.rs:56`) — a `String` round-tripped
through `PathBuf`. The monorepo-unification acceptance criteria require each
layer entry to "resolve to exactly one `Package.relative`," yet a Rust
consumer must convert types to perform that join, and `PathBuf` imports
OS-path semantics this data explicitly does not want (it is always a logical,
`/`-separated repo-relative path).

It serializes safely today (`PathBuf::from("a/b")` keeps `/` on Windows), so
JSON consumers are unaffected — but the Rust type misrepresents intent.

**Decision.** Change `MonorepoLayer.packages` to `Vec<String>` to match
`Package.relative`. Update `build_monorepo_layers` to push `pkg.relative.clone()`
directly.

- **Surface impact:** JSON is **byte-identical** (both serialize as the same
  `/`-separated strings). The change is internal Rust ergonomics + the public
  Rust type. Update the one construction site and any fixtures/tests that build
  `MonorepoLayer` literals with `PathBuf`.
- **Alternative weighed:** a dedicated `RepoRelativePath` newtype shared by
  both fields. Rejected for now as over-engineering for a single-use logical
  path; revisit only if more APIs need to carry repo-relative paths.

### D3 — Fix the stale `structure_value` comment

`sniff/cli/src/output/repo_json.rs:563` documents the filtered clone as
preserving *"workspace tools, monorepo flags, root path…"*; `workspace_tools`
was deleted in the unification. Update the comment to name the fields that
actually survive (monorepo flags, root, `monorepo_layers`,
`monorepo_standards`, dependency rollups). Comment-only change, per the repo's
drift/scope rules.

### D4 — Remove the dead "Unknown" fallback branch

`format_monorepo_summary` is **not** dead code: it renders the single-layer
summary on the `sniff repo` / `sniff repo structure` path, and is called only
when `layer_count == 1`. Only its `"Unknown"` else-arm
(`sniff/cli/src/output/filesystem/repo.rs:152-154`) is unreachable — it returns
`"Unknown"` when `monorepo_layers` is empty, but its only caller is gated by
`if !repo.is_monorepo { return }`
(`sniff/cli/src/output/filesystem/repo.rs:483`), and
`is_monorepo == layers_imply_monorepo(layers)`
(`sniff/lib/src/filesystem/repo/detection.rs:306`) — so a monorepo always has
at least one layer. The else-arm asserts a false invariant.

**Decision.** Remove the dead `"Unknown"` branch. Once D1 lands, route the
function through `primary_layer()`; keep the function and its `String` return.
The `let Some(...)`/`None` becomes provably total via the upstream `is_monorepo`
gate, so no user-facing panic is introduced. If a guard is desired, a
`debug_assert!` documenting the `is_monorepo ⇒ ≥1 layer` invariant is
acceptable, but the prior "`debug_assert` vs `expect`" ambiguity is dropped —
the branch is simply removed.

`format_monorepo_summary` does **more** than the dead-branch removal, however:
it also stops rendering `layer.authority.spec().display_name` and instead
composes the unified label via the shared CLI helper introduced in D5 (the
per-standard label + `{orchestrator_label} (using {authority_label})` template,
or `{authority_label}` alone when no orchestrator). So its output **changes**
(e.g. `Cargo Workspace` → `cargo`; `pnpm Workspaces <dim>+ Nx</dim>` →
`Nx (using pnpm workspaces)`). The dead-branch removal still stands, and the
*selected layer* is unchanged (it remains the primary layer) — only the standard
naming switches from `display_name` to the unified label. See D5 for the shared
helper and the multi-layer listing.

- **Surface impact:** `sniff repo` / `sniff repo structure` single-layer
  summary **text** changes (`display_name` → unified label); JSON is untouched.

### D5 — Redesign the `sniff repo is-monorepo` leaf

This is a deliberate behavior **and** JSON-contract change on an *existing*
leaf. Today the leaf prints `yes`/`no` and emits `{ "is-monorepo": bool }`,
sourced from the lightweight `RepoIdentity`
(`sniff/cli/src/output/repo_json.rs:374-375`,
`sniff/cli/src/output/repo_json.rs:635`). The leaf is upgraded to carry which
standard the repo uses.

**Text output → STDOUT** (main content, per repo convention):

- When the repo **is** a monorepo: print the human-readable label —
  `{authority_label}` alone, or `{orchestrator_label} (using {authority_label})`
  when an orchestrator is present.
- When the repo is **not** a monorepo: print `false`.

**Exit code is the predicate:** non-zero (error) when **not** in a monorepo,
`0` when it is. This enables `if sniff repo is-monorepo; then …`.

**`--no-error` flag:** add a command-local flag to `sniff repo is-monorepo`
matching the existing locator-leaf pattern (`worktree`, `version`, `package`,
`package-area`). It suppresses **only** the "not a monorepo" predicate. With
`--no-error`, the not-a-monorepo outcome prints `false` to STDOUT and exits `0`
instead of non-zero. It does **not** turn the command into "never fails":
genuine failures — the path is not a git repo, detection itself errors, or a
bad/nonexistent path — STILL exit non-zero and report to STDERR even under
`--no-error`. For callers that want the predicate value without an error exit.

The flag applies equally in text and `--json` modes: JSON is printed first as a
valid object on STDOUT, then the process exits with the predicate status
(`1`/non-zero outside a monorepo, `0` inside, or `0` for the
not-a-monorepo predicate when `--no-error` is set).

**`--json` → STDOUT:** shape is

```json
{ "is_monorepo": false }
```

or

```json
{ "is_monorepo": true, "authority": "<id>", "orchestrators": ["<id>", …] }
```

`orchestrators` is an **array** (it mirrors the `Vec<MonorepoStandard>` model —
e.g. Nx + Lerna), omitted or empty when none. Identifiers are the existing
kebab `MonorepoStandard::spec().id` values (e.g. `cargo-workspace`,
`pnpm-workspaces`, `nx`) — the **same** vocabulary claudine emits as
`monorepo_standard`. Note the JSON key is snake_case `is_monorepo`, a **rename**
from today's `is-monorepo`.

**Aggregate JSON is not changed by D5.** Bare `sniff repo --json` is an
aggregate keyed by participating child command names, not a direct replay of
each focused child's object shape. To preserve the explicit D1–D4 non-break
contract, the aggregate keeps `"is-monorepo": true|false` as an unwrapped bool.
The standard/orchestrator details are already available in
`structure.monorepo_layers` for aggregate consumers that need them. Do not add
`authority` / `orchestrators` next to the aggregate boolean in this feature.

**Multi-layer repos report the PRIMARY layer** (per D1) for the single
`authority` / `orchestrators` fields.

**No `unknown`-authority rendering branch.** When `is_monorepo == true`, the
primary layer's `authority` is **always** a real `DefinesMembership` standard —
never `Unknown`. This is structurally guaranteed, not merely conventional:

- `UNKNOWN_SPEC` has empty `roles`
  (`sniff/lib/src/filesystem/repo/standard.rs:959`), so
  `MonorepoStandard::defines_membership()` is always false for `Unknown`.
- `build_monorepo_layers` (`sniff/lib/src/filesystem/repo/topology.rs:50-60`)
  only assigns a layer `authority` from outcomes that pass
  `defines_membership()`, and no detector emits `Unknown` as a membership
  authority.
- `Unknown` is only ever a NON-layer sentinel: an observability entry in
  `monorepo_standards` (a bare orchestrator with no authority), a
  `Package.standard` placeholder for manifest-scan packages, and a provenance
  fallback.

Therefore the `{orchestrator_label} (using {authority_label})` template can
never produce "using unknown", and D5 does **not** special-case an
"unknown authority" text or JSON rendering. The two outcomes the leaf
distinguishes are exhaustive: **not** a monorepo → prints `false`, non-zero exit
(or `0` with `--no-error`); **is** a monorepo → always a real
`DefinesMembership` authority label.

**Documented contract:** *the primary layer authority always defines membership
(never `Unknown`).* A `debug_assert!(layer.authority.defines_membership())` in
`build_monorepo_layers` is an acceptable optional guard against future
regression, but no user-facing handling is needed.

**New per-standard human label.** Add a natural-language label to each
`MonorepoStandard` spec in the library (examples: cargo → `"cargo"`, pnpm →
`"pnpm workspaces"`, nx → `"Nx"`, turborepo → `"Turborepo"`). This is a new,
distinct field — separate from the existing `display_name` (`"Cargo
Workspace"`). A label **must** be defined for every standard, including
`unknown` — the `unknown` label exists for its non-layer sentinel/observability
uses (it never appears as a primary-layer authority; see the invariant above),
not because the `is-monorepo` leaf renders an "unknown authority". Selection
stays in the library (the label is a reusable domain concept); the CLI owns only
the template composition
(`{orchestrator_label} (using {authority_label})`). This preserves the existing
D1/D5 boundary: library owns the per-standard label, CLI owns the template.

**Shared CLI label helper (unifies all surfaces).** The template composition
lives in a single shared CLI helper rather than being inlined in the
`is-monorepo` leaf. That helper is used by **both** the `is-monorepo` leaf
**and** `format_monorepo_summary` (the `sniff repo` / `sniff repo structure`
single-layer summary, per D4), so the two surfaces phrase a monorepo standard
identically and there is exactly one source for the phrasing. The helper takes
the per-standard label(s) (library-owned) and applies
`{orchestrator_label} (using {authority_label})`, or `{authority_label}` alone
when no orchestrator is present.

**Multi-layer listing also adopts the label.** The multi-layer listing
(`format_monorepo_layer` / `render_repo_section`,
`sniff/cli/src/output/filesystem/repo.rs:166`, `:537`) likewise renders standard
**names** via the new per-standard label — each layer's authority and
orchestrators use the label, not `display_name`. The per-layer **layout**
(package counts, list structure) is unchanged; only the standard naming
switches. The exact multi-layer template / orchestrator-joining wording is a
planning detail to confirm, but it must draw from the label vocabulary, not
`display_name`.

**Data source.** The leaf currently uses `detect_repo_identity`, and
`RepoIdentity` carries no layers / standards / root. Reporting
authority/orchestrators requires sourcing from the full `RepoInfo`
(`detect_repo_structure`) or otherwise obtaining the primary layer — an
implementation requirement of this decision.

Reader note: this is the intentional cost/shape trade-off for D5. The old leaf
could answer a cheap predicate from `RepoIdentity`; the new leaf promises
standard identity, so it must read the topology model. Keep the request scoped
to repo structure only; do not run network-primary paths or deep git/package
analysis just to answer `is-monorepo`.

- **Surface impact:** breaking on this leaf only. Text (`yes`/`no` → label /
  `false`), JSON key (`is-monorepo` → `is_monorepo`), JSON shape (bare bool →
  object with `authority`/`orchestrators`), and exit code (was always `0` → now
  predicate-driven, with `--no-error` to opt back into always-`0`). The `sniff
  repo` aggregate / `sniff repo structure` JSON is untouched.

## Implementation scope (provisional)

1. Add `RepoInfo::primary_layer()` (D1) with unit tests covering single-root,
   shared-root (Cargo+uv), and multi-root topologies, plus a doc comment
   stating the selection rule.
2. Repoint `format_monorepo_summary` and claudine `environment.rs` at
   `primary_layer()`; delete the local `.first()` derivations.
3. Change `MonorepoLayer.packages` to `Vec<String>` (D2); update
   `build_monorepo_layers` and all literal constructions in fixtures/tests.
4. Fix the `structure_value` comment (D3).
5. Remove the dead `"Unknown"` fallback branch in `format_monorepo_summary`
   and route it through `primary_layer()` (D4).
6. Add the new natural-language label field to `MonorepoStandardSpec` and define
   it for every standard, including `unknown` (D5).
7. Add the shared CLI label helper that composes
   `{orchestrator_label} (using {authority_label})` (or `{authority_label}`
   alone), and unify the structure renderer onto it: switch
   `format_monorepo_summary` (single-layer summary) and `format_monorepo_layer`
   / `render_repo_section` (multi-layer listing) from `display_name` to the
   per-standard label via this helper, keeping the multi-layer layout unchanged.
   Deliberately re-baseline the affected `sniff repo` / `sniff repo structure`
   text snapshots, with explanation and approval (D4/D5).
8. Redesign the `is-monorepo` handler (D5): source from `RepoInfo` /
   `primary_layer()` instead of `detect_repo_identity`; add the clap `--no-error`
   flag and update `RepoAction::IsMonorepo` to carry it; emit the new STDOUT
   text via the shared label helper (`{authority_label}` /
   `{orchestrator_label} (using {authority_label})` / `false`); emit the new
   focused `--json` shape (`is_monorepo` + `authority` + `orchestrators` array,
   kebab ids); make the exit code the predicate in both text and JSON modes
   (non-zero outside a monorepo, `0` inside, `--no-error` forces `0` only for
   the not-a-monorepo predicate).
9. Leave the bare `sniff repo --json` aggregate `"is-monorepo": bool` entry
   unchanged and document why the focused leaf and aggregate intentionally
   differ. Update the CLI README/help examples so users see the new focused
   contract without implying the aggregate shape changed.
10. Update documentation alongside the code (per the repo's drift-maintenance
    rules). Explicitly:
    - **`sniff/docs/cli/repo_is-monorepo.md`** — rewrite for the D5 contract:
      replace the `yes`/`no` text behavior with the label /
      `{orchestrator_label} (using {authority_label})` / `false` output; document
      the new predicate-driven exit code (non-zero outside a monorepo, `0`
      inside) and the `--no-error` flag (including its limited scope — genuine
      failures still exit non-zero); replace the `{ "is-monorepo": bool }` JSON
      section with the new `{ "is_monorepo": ... }` object shape (snake_case key,
      `authority`, `orchestrators` array, kebab ids); note the aggregate
      `sniff repo --json` still emits the legacy unwrapped `"is-monorepo"` bool.
    - **`sniff/docs/cli/repo_structure.md`** — update the monorepo summary /
      heading rendering (Default Behavior and Package Listing sections) to show
      the unified per-standard label and shared
      `{orchestrator_label} (using {authority_label})` template instead of
      `display_name` / `<dim>+ {orchestrator}</dim>`; confirm the `--json`
      example stays byte-identical (`monorepo_layers[].packages` is still an
      array of `/`-separated strings after the D2 `Vec<String>` change).
    - The Sniff skill (`.claude/skills/sniff/`) for the focused `is-monorepo`
      contract, the aggregate exception, and the unified monorepo label wording.

## Testing and acceptance criteria

- A library test asserts `primary_layer()` selects the documented layer for
  single-root, shared-root, and multi-root fixtures.
- A regression test asserts `primary_layer()` reproduces today's `.first()`
  output on (a) the rusty-biscuit repo (single Cargo workspace) and (b) a
  synthetic Cargo + uv-at-root fixture (must select Cargo via the enum-order
  tiebreak).
- No consumer (`sniff/cli`, `claudine`) calls `monorepo_layers.first()` for the
  primary-layer decision; both call `primary_layer()` (`git grep` clean).
- `MonorepoLayer.packages` is `Vec<String>`; every entry still resolves to
  exactly one `RepoInfo.packages[].relative` entry.
- Existing `sniff repo` / `sniff repo structure` **JSON** snapshots are
  unchanged (byte-identical; serde ids are unaffected). The **text** snapshots
  **will** change — the monorepo standard renders via the unified label instead
  of `display_name` (e.g. `Cargo Workspace` → `cargo`; `pnpm Workspaces
  <dim>+ Nx</dim>` → `Nx (using pnpm workspaces)`) — and must be deliberately
  re-baselined with explanation and approval, never silently. Any **JSON** diff
  (which is not expected) must likewise be explained and approved.
- Bare aggregate `sniff repo --json` remains byte-identical for the
  `is-monorepo` member: it is still an unwrapped bool under the existing
  hyphenated subcommand key. Focused `sniff repo is-monorepo --json` is the only
  JSON surface that switches to `{ "is_monorepo": ..., ... }`.
- Both `sniff repo` / `sniff repo structure` and `sniff repo is-monorepo`
  render the same monorepo standard via the **same** shared label + template
  helper (one source for the phrasing). `git grep` shows no `display_name` use
  for the monorepo summary (single-layer or multi-layer).
- claudine `{{project.monorepo_standard}}` / `monorepo_orchestrators` /
  deprecated `monorepo_tool` template values are unchanged on the rusty-biscuit
  repo.
- Terminal output continues to use `biscuit-terminal` renderables; `stdout` for
  main content, `stderr` for diagnostics only.
- CLI help, shell argument parsing tests, README examples, and the `sniff`
  skill document the focused `is-monorepo` output as label/`false` text plus the
  new JSON object shape; they also document that aggregate `sniff repo --json`
  keeps the legacy unwrapped `"is-monorepo"` bool.
- The two affected CLI docs are updated and no longer describe the old behavior:
  - `sniff/docs/cli/repo_is-monorepo.md` describes the D5 contract — label /
    `false` text, predicate exit code, `--no-error` flag, and the
    `{ "is_monorepo": ..., "authority": ..., "orchestrators": [...] }` JSON
    object — with **no** remaining `yes`/`no` text or `{ "is-monorepo": bool }`
    JSON example for the focused leaf.
  - `sniff/docs/cli/repo_structure.md` describes the monorepo summary/heading via
    the unified per-standard label and shared template, with **no** remaining
    reference to `display_name`-style naming or the `<dim>+ {orchestrator}</dim>`
    form; its `--json` example remains byte-identical.
- **D5 — `sniff repo is-monorepo`:**
  - Text output examples: `cargo`; `pnpm workspaces`;
    `Nx (using pnpm workspaces)`; `false` when not a monorepo.
  - Exit-code behavior: non-zero outside a monorepo, `0` inside; `--no-error`
    forces `0` for the not-a-monorepo predicate only (still printing `false` /
    the label).
  - `--no-error` scope: genuine failures (path is not a git repo, detection
    itself errors, bad/nonexistent path) STILL exit non-zero and report to
    STDERR even with `--no-error`; `--no-error` does not make the command
    "never fail" — it only converts the not-a-monorepo outcome to `false` + exit
    `0`.
  - `--json` shapes for both branches: `{ "is_monorepo": false }` and
    `{ "is_monorepo": true, "authority": "<kebab-id>", "orchestrators":
    ["<kebab-id>", …] }`, with `orchestrators` an array (omitted/empty when
    none) and ids drawn from `MonorepoStandard::spec().id`.
  - `--json` preserves valid JSON on STDOUT even when the predicate exits
    non-zero; diagnostics for genuine failures go to STDERR and do not corrupt
    STDOUT.
  - Multi-layer repos report the primary layer (per D1) in `authority` /
    `orchestrators`.
  - A per-standard natural-language label is defined for **every**
    `MonorepoStandard`, including `unknown`.
  - Invariant: when `is_monorepo == true`, the primary layer's `authority`
    always defines membership (never `Unknown`); the leaf renders no
    "unknown authority" branch, and the `{orchestrator_label} (using
    {authority_label})` template can never emit "using unknown". (Optional
    `debug_assert!(layer.authority.defines_membership())` guard in
    `build_monorepo_layers`; no user-facing handling required.)

## Out of scope

- Any change to detection, the `MonorepoStandard` enum, provenance tiers, or
  the `is_monorepo` predicate — owned by the prior two features.
- Enriching the existing `sniff repo is-monorepo` leaf is now **in scope** (see
  D5). New *additional* focused CLI leaves (`sniff repo standards`, `sniff repo
  layers`) remain deferred.
- A `RepoRelativePath` newtype (D2 alternative) — deferred unless more APIs
  need it.

## Resolved decisions

Both former open questions are now decided (see D1):

- **The "primary" selection rule** is: layer rooted at the repo root if one
  exists; otherwise the shallowest-root layer; ties broken by
  `MonorepoStandard` enum-declaration order (Cargo first … Unknown last), **not**
  detector push-order. Verified to reproduce today's `.first()` output on the
  rusty-biscuit repo and a synthetic Cargo + uv-at-root fixture (acceptance
  criteria above).
- **`primary_authority()` is not added.** Both consumers read the authority id
  and orchestrators off the same layer, so it removes no real duplication;
  `primary_layer()` is the only accessor.
- **Unified label across all CLI surfaces.** Both `sniff repo` / `sniff repo
  structure` (single-layer summary and multi-layer listing) and `sniff repo
  is-monorepo` render the monorepo standard via the new per-standard label and
  the shared `{orchestrator_label} (using {authority_label})` template (or
  `{authority_label}` alone). A single shared CLI helper owns the template
  composition; the library owns the per-standard label. `display_name` is
  retired from the monorepo summary. Consequence: the `sniff repo` / `sniff repo
  structure` **text** changes (and its snapshots are re-baselined with
  approval); its **JSON** stays byte-identical.
- **Focused leaf vs aggregate JSON.** Focused
  `sniff repo is-monorepo --json` changes to the new snake_case object shape and
  predicate exit semantics. Bare aggregate `sniff repo --json` keeps the
  existing unwrapped `"is-monorepo": bool` member because that surface is
  explicitly outside the D5 break; consumers that need standard identity should
  read `structure.monorepo_layers` from the aggregate.
