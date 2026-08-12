---
agent: claude/
phases: 5
created: 2026-06-16
start_phase: 1
yolo: true
source_files_during_phase_1:
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/standard.rs
  - sniff/lib/src/filesystem/repo/topology.rs
  - sniff/lib/tests/integration.rs
  - sniff/cli/src/output/repo_json.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/cli/src/output/filesystem/repo.rs
  - sniff/cli/src/output/repo_json.rs
  - claudine/lib/src/events/environment.rs
  - claudine/lib/src/dispatch/template.rs
  - sniff/cli/tests/snapshots/snapshots__cargo_monorepo_structure_text.snap
  - sniff/cli/tests/snapshots/snapshots__cargo_pnpm_monorepo_structure_text.snap
  - sniff/cli/tests/snapshots/snapshots__pnpm_nx_monorepo_structure_text.snap
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_3:
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/output/filesystem/repo.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/mod.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/tests/cli.rs
docs_updated_during_phase_3:
  - sniff/features/2026-06-16-monorepo-cli/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_4:
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/tests/cli.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - claudine/lib/src/events/environment.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - sniff/cli/src/args/mod.rs
docs_updated_during_phase_5:
  - sniff/docs/cli/repo_is-monorepo.md
  - sniff/docs/cli/repo_structure.md
  - sniff/docs/cli/repo.md
  - sniff/docs/cli/repo_package-count.md
  - sniff/docs/cli/repo_version.md
  - sniff/docs/topics/json-output.md
  - sniff/cli/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_code:
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/standard.rs
  - sniff/lib/src/filesystem/repo/topology.rs
  - sniff/lib/tests/integration.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/output/filesystem/repo.rs
  - claudine/lib/src/events/environment.rs
  - sniff/cli/tests/snapshots/snapshots__cargo_monorepo_structure_text.snap
  - sniff/cli/tests/snapshots/snapshots__cargo_pnpm_monorepo_structure_text.snap
  - sniff/cli/tests/snapshots/snapshots__pnpm_nx_monorepo_structure_text.snap
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/mod.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/tests/cli.rs
documentation:
  - sniff/features/2026-06-16-monorepo-cli/plan.md
  - sniff/docs/cli/repo_is-monorepo.md
  - sniff/docs/cli/repo_structure.md
  - sniff/docs/cli/repo.md
  - sniff/docs/cli/repo_package-count.md
  - sniff/docs/cli/repo_version.md
  - sniff/docs/topics/json-output.md
  - sniff/cli/README.md
packages:
  - sniff
  - claudine
---

# Execution Plan — Monorepo CLI Wiring Cleanup

Derived from [`spec.md`](./spec.md). Implements decisions **D1–D5**: a library
primary-layer accessor, a `Vec<String>` join-key, a stale-comment fix, a
dead-branch removal, and a redesigned `sniff repo is-monorepo` leaf — all unified
behind one per-standard label + shared CLI template helper.

## Ground rules

- Use the `sniff`, `cli`, `rust`, and `biscuit-terminal` agent skills.
- Library owns detection/business logic and the per-standard **label**; the CLI
  owns the **template composition** and all presentation.
- **JSON non-break (D1–D4):** `sniff repo` / `sniff repo structure` JSON stays
  byte-identical. Bare aggregate `sniff repo --json` keeps the unwrapped
  `"is-monorepo": bool` member under its hyphenated key.
- **Intentional breaks (D5 only):** focused `sniff repo is-monorepo` text, JSON
  key/shape, and exit code change. CLI **text** snapshots for `repo` /
  `repo structure` re-baseline (label replaces `display_name`).
- Targeted builds only — `-p sniff`, `-p sniff-cli`, `-p claudine` (never bare
  `cargo build` at root). `stdout` = main content, `stderr` = diagnostics.
- No commits by sub-agents; no AI attribution trailers.

## Key file map (verified)

| Concern | Location |
|---|---|
| `RepoInfo` (add `primary_layer()`) | `sniff/lib/src/filesystem/repo/types.rs:48` |
| `MonorepoLayer.packages` field (D2) | `sniff/lib/src/filesystem/repo/standard.rs:394` |
| `MonorepoStandardSpec` (add label, D5) | `sniff/lib/src/filesystem/repo/standard.rs:256` |
| 17 `*_SPEC` consts (define label, D5) | `sniff/lib/src/filesystem/repo/standard.rs:635-1430` |
| `build_monorepo_layers` (D2 push site, optional debug_assert) | `sniff/lib/src/filesystem/repo/topology.rs:50-60` |
| lib layer fixture `layer_with` | `sniff/lib/src/filesystem/repo/standard.rs:2000` |
| topology test `layer_package_paths_resolve_to_canonical_catalog` | `sniff/lib/src/filesystem/repo/topology.rs:330-370` |
| `format_monorepo_standard` / `format_monorepo_summary` (D4) | `sniff/cli/src/output/filesystem/repo.rs:143-163` |
| `format_monorepo_layer` / `render_repo_section` (multi-layer) | `sniff/cli/src/output/filesystem/repo.rs:166,471,537` |
| `is_monorepo_outcome` (focused JSON, D5) | `sniff/cli/src/output/repo_json.rs:374` |
| `structure_value` stale comment (D3) | `sniff/cli/src/output/repo_json.rs:563` |
| aggregate `"is-monorepo"` bool (KEEP) | `sniff/cli/src/output/repo_json.rs:635` |
| `RepoAction::IsMonorepo` enum variant (D5) | `sniff/cli/src/args/repo.rs:196` |
| `RepoSubcommand::IsMonorepo` clap leaf (D5) | `sniff/cli/src/args/repo.rs:631` |
| `IsMonorepo` → `RepoAction` map (D5) | `sniff/cli/src/args/mod.rs:946` |
| `IsMonorepo` handler (D5) | `sniff/cli/src/commands/mod.rs:718-731` |
| claudine `.first()` derivation (D1) | `claudine/lib/src/events/environment.rs:275-287` |
| claudine layer fixture | `claudine/lib/src/events/environment.rs:547` |
| Docs to rewrite | `sniff/docs/cli/repo_is-monorepo.md`, `sniff/docs/cli/repo_structure.md` |

---

## Phase 1 — Library foundations (`sniff/lib`)

All detection-model changes land first; consumers depend on them. Touches
`types.rs`, `standard.rs`, `topology.rs`. **D2 and D5-label both edit
`standard.rs`** — coordinate (do them in one editing pass to avoid churn); **D1
is isolated in `types.rs`** and may proceed in parallel.

- [x] **(D1)** Add `RepoInfo::primary_layer(&self) -> Option<&MonorepoLayer>` to
  `sniff/lib/src/filesystem/repo/types.rs`. Implement the documented selection
  rule: (1) layer whose `root` equals `self.root`; else (2) shallowest root
  (fewest path components); ties broken by **`MonorepoStandard` enum-declaration
  order** (Cargo first … Unknown last), *not* `BTreeMap`/push order. Add the
  rustdoc from spec §D1 (no H1; `## Returns` section). [parallel with D5-label]
- [x] **(D1)** Add library unit tests for `primary_layer()` covering:
  single-root; shared-root **Cargo + uv at repo root** (must select Cargo via
  enum-order tiebreak); multi-root (shallowest wins); and empty-layers → `None`.
- [x] **(D2)** Change `MonorepoLayer.packages` from `Vec<PathBuf>` to
  `Vec<String>` (`standard.rs:394`); update the field rustdoc (still
  "matches `Package::relative`"). Remove the now-unused `PathBuf` import only if
  nothing else in the file needs it.
- [x] **(D2)** In `build_monorepo_layers` (`topology.rs:56`) replace
  `.map(|pkg| PathBuf::from(&pkg.relative))` with `.map(|pkg| pkg.relative.clone())`.
- [x] **(D2)** Fix `topology.rs` test `layer_package_paths_resolve_to_canonical_catalog`:
  iteration now yields `&String`; change `normalize_path_for_catalog` to take
  `&str` (or inline `rel.replace('\\', "/")`). Confirm `layer.packages.len()`
  callers (`membership_resolves_non_degenerately`, `format_monorepo_layer`) are
  unaffected — `.len()` is identical on `Vec<String>`.
- [x] **(D2)** Update the lib layer fixture `layer_with` (`standard.rs:2000`):
  `PathBuf::from(format!("packages/pkg-{i}"))` → `format!("packages/pkg-{i}")`.
- [x] **(D5-lib)** Add a new `label: &'static str` field to
  `MonorepoStandardSpec` (`standard.rs:256`), distinct from `display_name`. Add
  rustdoc: natural-language label for CLI rendering (e.g. `"cargo"`,
  `"pnpm workspaces"`, `"Nx"`). [parallel with D1]
- [x] **(D5-lib)** Define `label` on **all 17** `*_SPEC` consts. Suggested
  values: cargo→`"cargo"`, npm→`"npm workspaces"`, pnpm→`"pnpm workspaces"`,
  yarn→`"yarn workspaces"`, bun→`"bun workspaces"`, uv→`"uv workspace"`,
  go→`"go workspace"`, gradle→`"Gradle"`, maven→`"Maven"`, dotnet→`".NET solution"`,
  bazel→`"Bazel"`, pants→`"Pants"`, buck2→`"Buck2"`, rush→`"Rush"`, nx→`"Nx"`,
  turborepo→`"Turborepo"`, lerna→`"Lerna"`, **unknown→`"unknown"`** (required;
  exists for the non-layer sentinel — see invariant). Confirm exact wording with
  the matching acceptance examples (`cargo`, `pnpm workspaces`,
  `Nx (using pnpm workspaces)`).
- [x] **(D5-lib, optional)** Add
  `debug_assert!(outcome.standard.defines_membership())` at the layer-authority
  assignment in `build_monorepo_layers` (`topology.rs:50`), documenting the
  `is_monorepo ⇒ primary authority defines membership (never Unknown)` invariant.

**Validation checkpoint 1**
- [x] `cargo build -p sniff` and `cargo test -p sniff` (or `cargo nextest run -p sniff`) green.
- [x] New `primary_layer()` tests pass, including the Cargo+uv enum-order case.
- [x] `git grep -n "Vec<PathBuf>" sniff/lib/src/filesystem/repo/standard.rs`
  shows no `MonorepoLayer.packages` hit.

---

## Phase 2 — CLI label helper + structure renderer rewire (D3, D4, label unification)

Depends on Phase 1 (`primary_layer()`, the new `label`). Builds the **single
shared CLI helper** every monorepo surface routes through, then rewires the
`sniff repo` / `sniff repo structure` renderers and the claudine consumer.
claudine is a separate crate depending only on Phase 1 — **parallelizable** with
the sniff-cli tasks.

- [x] **(D4/D5)** Add a shared CLI label helper (in
  `sniff/cli/src/output/filesystem/repo.rs`, or a small shared module if cleaner)
  that composes the unified phrasing from library labels:
  `{orchestrator_label} (using {authority_label})` when an orchestrator is
  present, else `{authority_label}` alone. It reads each standard's
  `spec().label` (never `display_name`). This is the **one** source of monorepo
  phrasing for both the structure summary and the is-monorepo leaf.
- [x] **(D4)** Rewrite `format_monorepo_summary` (`repo.rs:151`): route through
  `repo.primary_layer()` instead of `monorepo_layers.first()`; **remove the dead
  `"Unknown"` else-arm** (provably total via the upstream
  `if !repo.is_monorepo { return }` gate); compose output via the shared helper.
  Keep the `String` return. Output changes from `display_name` (e.g.
  `Cargo Workspace`) to label (e.g. `cargo`); `pnpm Workspaces <dim>+ Nx</dim>`
  → `Nx (using pnpm workspaces)`.
- [x] **(D5)** Switch the multi-layer listing `format_monorepo_layer`
  (`repo.rs:166`) and its callers in `render_repo_section` (`repo.rs:537`) and
  `render_filesystem_section` (`repo.rs:944-951`) from `display_name` to the
  per-standard **label** via the shared helper/label lookup. Keep per-layer
  **layout** (package counts, list structure) unchanged — only standard naming
  switches. Confirm the multi-layer orchestrator-joining wording draws from the
  label vocabulary.
- [x] **(D4/D5)** Update `format_monorepo_standard` (`repo.rs:143`): either
  repoint it to `spec().label` or remove it in favor of the shared helper. Ensure
  `git grep "display_name"` in `sniff/cli/src/output/filesystem/repo.rs` shows no
  remaining monorepo-summary use.
- [x] **(D3)** Fix the stale `structure_value` doc comment
  (`repo_json.rs:563`): remove "workspace tools"; name the fields that actually
  survive the filtered clone (monorepo flags, `root`, `monorepo_layers`,
  `monorepo_standards`, dependency rollups). **Comment-only** — no behavior change
  in this edit (scope-discipline rule).
- [x] **(D1)** Repoint claudine `environment.rs:275-287`: replace
  `r.monorepo_layers.first()` with `r.primary_layer()`; keep reading
  `layer.authority.spec().id` and `layer.orchestrators`. Delete the local
  `.first()` derivation. Values must be unchanged on single-root repos.
  [parallel with sniff-cli tasks above]

**Validation checkpoint 2**
- [x] `cargo build -p sniff-cli` and `cargo build -p claudine` green.
- [x] `git grep -n "monorepo_layers.first()" sniff/cli claudine` is clean for the
  primary-layer decision (only non-primary uses, if any, remain).
- [x] `git grep -n "display_name"` shows no monorepo-summary usage in
  `sniff/cli/src/output/filesystem/repo.rs`.
- [x] Manual smoke: `cargo run -p sniff-cli -- repo structure` on this repo shows
  the label (`cargo`) instead of `Cargo Workspace` (snapshot rebaseline deferred
  to Phase 4).

---

## Phase 3 — `is-monorepo` leaf redesign (D5)

Depends on Phase 1 (`primary_layer`, label) and Phase 2 (shared helper). This is
the intentional behavior/JSON/exit-code break, scoped to the focused leaf only.

- [x] **(D5)** Add a `--no-error` flag to the `IsMonorepo` clap leaf
  (`args/repo.rs:631`), matching the locator-leaf pattern. **`--no-error` only**
  (no `on_error`): the predicate always prints `false` to STDOUT, so the
  `on_error`-message path does not apply here. Document the flag's limited scope
  in the help text (suppresses only the not-a-monorepo predicate).
- [x] **(D5)** Update `RepoAction::IsMonorepo` (`args/repo.rs:196`) to carry
  `no_error: bool`, and update the `RepoSubcommand::IsMonorepo` → `RepoAction`
  mapping (`args/mod.rs:946`) to thread it.
- [x] **(D5)** Rewrite the focused JSON builder (replace/extend
  `is_monorepo_outcome`, `repo_json.rs:374`) to emit the new shape with
  predicate-driven exit code:
  - not a monorepo → `{ "is_monorepo": false }`, exit non-zero (or `0` under
    `--no-error`).
  - monorepo → `{ "is_monorepo": true, "authority": "<kebab-id>",
    "orchestrators": ["<kebab-id>", …] }`, exit `0`. `orchestrators` is an
    **array**, omitted/empty when none. Note the key rename `is-monorepo` →
    snake_case **`is_monorepo`**. Ids come from `MonorepoStandard::spec().id`
    (the primary layer's authority + its orchestrators).
- [x] **(D5)** Rewrite the `IsMonorepo` handler (`commands/mod.rs:718-731`):
  - Source from the topology model — call `detect_repo_structure(&root)` (the
    fast structure-only path, same as `repo packages`) instead of
    `detect_repo_identity`. Do **not** trigger network/deep git/package analysis.
  - Genuine failures (not a git repo / `None` / detection `Err` / bad path) →
    report to **STDERR**, exit non-zero **even with `--no-error`**.
  - Text mode: when monorepo, print the shared-helper label
    (`{authority_label}` / `{orchestrator_label} (using {authority_label})`) to
    STDOUT, exit `0`; when not, print `false` to STDOUT, exit non-zero (or `0`
    with `--no-error`). Use `biscuit-terminal` renderables (respect `--plain`).
  - JSON mode: print the valid object to STDOUT **first**, then exit with the
    predicate status. STDOUT stays valid JSON even on non-zero exit.
  - Multi-layer repos report the **primary layer** (per D1) for
    `authority`/`orchestrators`.
  - No `unknown`-authority branch — structurally impossible when
    `is_monorepo == true` (see spec invariant).
- [x] **(D5)** Leave the aggregate `"is-monorepo"` bool entry
  (`repo_json.rs:635`) **unchanged**; add a short code comment noting the focused
  leaf and aggregate intentionally differ (focused = snake_case object,
  aggregate = legacy unwrapped bool).

**Validation checkpoint 3**
- [x] `cargo build -p sniff-cli` green; `cargo run -p sniff-cli -- repo is-monorepo`
  prints `cargo` and exits `0` on this repo; in a non-monorepo dir prints `false`
  and exits non-zero; `--no-error` makes that exit `0` while still printing
  `false`.
- [x] `sniff repo is-monorepo --json` emits
  `{"is_monorepo":true,"authority":"cargo-workspace","orchestrators":[]}`
  (orchestrators empty/omitted here) on STDOUT; non-monorepo emits
  `{"is_monorepo":false}`.
- [x] In a non-repo path, both modes exit non-zero with diagnostics on STDERR and
  no STDOUT corruption (valid JSON or nothing).

---

## Phase 4 — Tests, snapshots & regression

Depends on Phases 1–3. Lock in behavior and re-baseline the intentionally-changed
text.

- [x] **(D5)** Add/repoint focused unit tests for the new JSON shapes (both
  branches), the `is_monorepo` key rename, the `orchestrators` array, and
  predicate exit codes. Update the existing `is_monorepo_outcome_wraps_bool` test
  (`repo_json.rs:1509`) to the new shape; **keep** the aggregate tests
  (`repo_json.rs:2183`, `:2263`) asserting the legacy unwrapped `"is-monorepo"`
  bool unchanged.
- [x] **(D5)** Add CLI argument-parsing tests for the new `--no-error` flag on
  `is-monorepo` (flag present/absent threads into `RepoAction::IsMonorepo`).
- [x] **(D5)** Add exit-code / STDOUT-vs-STDERR integration coverage
  (`assert_cmd` + `predicates`): monorepo→0+label, not→nonzero+`false`,
  `--no-error`→0+`false`, genuine failure→nonzero+STDERR even under `--no-error`,
  `--json` valid on non-zero exit. (Tier per `rust-testing` skill; gate with
  `require_level!` if needed.)
- [x] **(text snapshots)** Re-run and **deliberately re-baseline** the affected
  `sniff repo` / `sniff repo structure` **text** snapshots (label replaces
  `display_name`; `Nx (using pnpm workspaces)` form). Diff each changed snapshot,
  confirm only the standard naming moved, and record the rationale.
- [x] **(JSON snapshots)** Confirm `sniff repo` / `sniff repo structure`
  **JSON** snapshots are byte-identical (including
  `monorepo_layers[].packages` as `/`-separated strings post-D2). Any JSON diff
  is unexpected — investigate before accepting.
- [x] **(D2)** Update the cli layer fixture (`repo_json.rs:2432-2435`):
  `PathBuf::from("pkg-a")` / `"pkg-b"` → plain `String`s. The claudine fixture
  (`environment.rs:553`, `packages: vec![]`) needs no value change but must still
  compile against `Vec<String>`.
- [x] **(D1 regression)** Add a regression test asserting `primary_layer()`
  reproduces today's `.first()` output on (a) the rusty-biscuit repo (single
  Cargo workspace) and (b) the synthetic Cargo+uv-at-root fixture.
- [x] **(D1)** Add/confirm a claudine test asserting
  `{{project.monorepo_standard}}` / `monorepo_orchestrators` / deprecated
  `monorepo_tool` template values are unchanged on the rusty-biscuit repo
  (extend the existing `environment.rs` test).

**Validation checkpoint 4**
- [x] `cargo test -p sniff -p sniff-cli -p claudine` (or `cargo nextest run`) all green.
- [x] `git grep -n "monorepo_layers.first()" sniff claudine` clean for the
  primary-layer decision.

---

## Phase 5 — Documentation, skill & final validation

Depends on Phases 1–4 (final contract settled). Drift-maintenance per repo rules.

- [x] **(D5 docs)** Rewrite `sniff/docs/cli/repo_is-monorepo.md` for the D5
  contract: replace `yes`/`no` with label /
  `{orchestrator_label} (using {authority_label})` / `false` text; document the
  predicate exit code (non-zero outside a monorepo, `0` inside) and `--no-error`
  (including its limited scope — genuine failures still exit non-zero); replace
  the `{ "is-monorepo": bool }` section with the new
  `{ "is_monorepo": ..., "authority": ..., "orchestrators": [...] }` object
  (snake_case key, kebab ids); note the aggregate `sniff repo --json` still emits
  the legacy unwrapped `"is-monorepo"` bool. **No** remaining `yes`/`no` or
  `{ "is-monorepo": bool }` for the focused leaf.
- [x] **(D4/D5 docs)** Update `sniff/docs/cli/repo_structure.md` (Default
  Behavior + Package Listing sections): show the unified per-standard label and
  shared `{orchestrator_label} (using {authority_label})` template instead of
  `display_name` / `<dim>+ {orchestrator}</dim>`. Confirm the `--json` example
  stays byte-identical (`monorepo_layers[].packages` still `/`-separated strings).
  **No** remaining `display_name`-style or `<dim>+ {orchestrator}</dim>` form.
- [x] **(docs)** Update the Sniff skill (`.claude/skills/sniff/`) for: the focused
  `is-monorepo` contract, the aggregate exception, and the unified monorepo label
  wording. Regenerate the skill's `hash:` frontmatter (`md hash <file>`) after
  editing.
- [x] **(docs)** Update CLI README / `--help` examples so users see the new
  focused `is-monorepo` contract without implying the aggregate shape changed.
- [x] **(drift)** If crate dependencies changed (none expected), update
  `docs/dependencies.md`. Confirm no other READMEs reference the old
  `is-monorepo` text or `display_name` monorepo naming.

**Final validation checkpoint**
- [x] `just test` and `just lint` (or `cargo clippy -p sniff -p sniff-cli -p claudine`)
  green; `just doctest` where applicable.
- [x] Acceptance sweep against spec §"Testing and acceptance criteria":
  - [x] `primary_layer()` fixtures (single/shared/multi-root) + `.first()` regression.
  - [x] No `monorepo_layers.first()` for the primary-layer decision (git grep clean).
  - [x] `MonorepoLayer.packages` is `Vec<String>`; each entry resolves 1:1 to a
    `Package.relative`.
  - [x] `repo` / `repo structure` JSON byte-identical; text re-baselined with
    rationale.
  - [x] Aggregate `repo --json` `"is-monorepo"` bool unchanged; focused
    `repo is-monorepo --json` is the only switched JSON surface.
  - [x] One shared label+template helper; no `display_name` in the monorepo summary
    (git grep clean).
  - [x] claudine template values unchanged on rusty-biscuit.
  - [x] D5 text examples (`cargo`; `pnpm workspaces`; `Nx (using pnpm workspaces)`;
    `false`), exit-code behavior, `--no-error` scope, and both `--json` branches
    all verified.
  - [x] A per-standard label defined for **every** `MonorepoStandard`, incl. `unknown`.
  - [x] Both CLI docs updated with no remaining old-behavior references.
  - `primary_layer()` fixtures (single/shared/multi-root) + `.first()` regression.
  - No `monorepo_layers.first()` for the primary-layer decision (git grep clean).
  - `MonorepoLayer.packages` is `Vec<String>`; each entry resolves 1:1 to a
    `Package.relative`.
  - `repo` / `repo structure` JSON byte-identical; text re-baselined with
    rationale.
  - Aggregate `repo --json` `"is-monorepo"` bool unchanged; focused
    `repo is-monorepo --json` is the only switched JSON surface.
  - One shared label+template helper; no `display_name` in the monorepo summary
    (git grep clean).
  - claudine template values unchanged on rusty-biscuit.
  - D5 text examples (`cargo`; `pnpm workspaces`; `Nx (using pnpm workspaces)`;
    `false`), exit-code behavior, `--no-error` scope, and both `--json` branches
    all verified.
  - A per-standard label defined for **every** `MonorepoStandard`, incl. `unknown`.
  - Both CLI docs updated with no remaining old-behavior references.

---

## Dependency & parallelism summary

```
Phase 1 (lib: D1 ∥ D2 ∥ D5-label)
   │
   ├──> Phase 2 (cli helper + D3/D4 rewire)  ∥  claudine D1 rewire
   │                     │
   │                     └──> Phase 3 (D5 is-monorepo leaf — needs Ph2 helper)
   │                                   │
   └───────────────────────────────────┴──> Phase 4 (tests + snapshot rebaseline)
                                                          │
                                                          └──> Phase 5 (docs + final validation)
```

- **Within Phase 1:** D1 (`types.rs`) is parallel-safe; D2 and D5-label share
  `standard.rs` — do them in one pass.
- **Within Phase 2:** claudine rewire is parallel-safe with the sniff-cli helper
  and renderer tasks (separate crate, depends only on Phase 1).
- **Phase 3** must follow Phase 2 (reuses the shared helper).
- **Snapshot rebaselining** is deliberately concentrated in Phase 4, after all
  rendering changes (Phases 2–3) are in, so text baselines move exactly once.
