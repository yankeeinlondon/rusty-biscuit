---
title: Phase 1 rulings, spikes, and current-state baseline
created: 2026-09-14
phase: 1
spec: fixes/2026-09-13-cicd-redundancies/spec.md
plan: fixes/2026-09-13-cicd-redundancies/plan.md
tree: aad933bdb (branch `fix/cicd-improvements`)
host: macOS 27.0.0, Apple Silicon; native Windows evidence from `$BUILD_WIN`
---

# Phase 1 — Rulings, Spikes, and Current-State Baseline

Every ruling below is settled against the tree at `aad933bdb`, with the command
or file that settled it. A later phase that contradicts a ruling amends this
record; it does not work around it.

Five rulings did **not** confirm as the plan proposed them. They are flagged
**AMENDED** and carry the evidence that forced the change. Three spikes
uncovered defects that are prerequisites for later phases; they are collected
in [Blocking findings](#blocking-findings).

---

## Blocking findings

These are not rulings. They are pre-existing defects the spikes surfaced, each
of which blocks a later phase until repaired. None was known when the plan was
written.

| # | Finding | Blocks | Detail |
|---|---|---|---|
| B1 | `scripts/drift.rs` does not compile — 6 × `no method named fallback_render` | Phase 3 (R2) | [S1](#s1--root-workspace-migration-dry-run) |
| B2 | `drift`'s `resolve_workspace_root_falls_back_to_metadata_workspace_root` runs >400 s (unbounded ancestor walk to `/`) | Phase 3 (R2) | [S1](#s1--root-workspace-migration-dry-run) |
| B3 | `F2 precondition HELD` never reaches captured output — the test rewires `STD_OUTPUT_HANDLE` to `CONOUT$` before printing it | Phase 7, and the plan's own success criterion | [S2](#s2--windows-console-test-under-nextest) |
| B4 | Retiring `biscuit-tui-captured-stdout` orphans `flags["biscuit-tui"]` and the `scope.biscuit_tui` output; the plan's Phase 8 does not mention either | Phase 8 | [R11](#r11--ci-gate-dependency-list-after-retirement) |
| B5 | `scripts/ci-rollup.rs`'s `const PLAN_SCHEMA_VERSION: u32 = 2` must bump with the Python constant; the plan's R9 does not name it | Phase 6 | [R9](#r9--which-schema-counters-bump) |
| B6 | `test-toolkit` fails `cargo clippy --all-targets -- -D warnings` today; nothing lints it because `gates = false` | Phase 3 | [Baseline](#test-toolkit-current-state) |
| B7 | `repo-deps` fails its own lint gate too — 3 clippy findings in `scripts/drift.rs`, invisible for the same reason as B6 | Phase 3 (found during it) | [Phase 3 addenda](#phase-3-addenda) |
| B8 | `osc8_file_link` emits a verbatim `\\?\` prefix and `\` separators on Windows, so its URI and label are both wrong | Phase 3 (found during it) | [Phase 3 addenda](#phase-3-addenda) |
| B9 | `the_real_planners_plan_rolls_up` fails on Windows: `python3` is a Microsoft Store alias stub that spawns OK and exits non-zero, so the `Err`-based skip guard never fires | Phase 3 (found during it) | [Phase 3 addenda](#phase-3-addenda) |
| B10 | Six `ci-rollup` fixtures resolve the tree through a compile-time `CARGO_MANIFEST_DIR`, which the `wsl2-ubuntu` archive run does not have | Phase 3 (found during it) | [Phase 3 addenda](#phase-3-addenda) |
| B11 | The console redirect also swallows every *post-child* failure diagnostic in `windows_captured_stdout.rs` — a panic after `establish_console()` writes to `CONOUT$`, leaving nextest to report `FAIL` with no message | Phase 7 (found during it) | [Phase 7 addenda](#phase-7-addenda) |

---

## Rulings

### R1 — `repo-deps` area and canonical recipes

**CONFIRMED as proposed.**

- `package_area("scripts")` → `root`. Verified by importing the real function:

  ```text
  scripts                      -> root
  tools/test-toolkit           -> tools
  renderable                   -> root
  claudine/rendezvous/core     -> claudine/rendezvous
  ```

  `renderable` is already a gating `root`-area package, so the path is
  exercised, not theoretical.

- **`repo-deps` needs no `scripts/justfile`, and `root` must not be added to
  the root justfile's `areas` variable.**
  - `_package-ci.yml` invokes `just _test "${{ inputs.package }}"` (line 438)
    and `just _lint "${{ inputs.package }}"` (line 631) from the repository
    root. `_lint` is `cargo clippy -p {{ pkg }} --all-targets`
    (`just/devops.just:115`) and `_test` is a package-scoped `cargo nextest`
    invocation (`just/devops.just:845`). Neither reads an area justfile.
  - `check-canonical` (`justfile:352`) iterates `areas` and fails an entry with
    no `<area>/justfile`. There is no `root/` directory, so adding `root` would
    fail the gate immediately — and `renderable`, the existing `root`-area
    package, is likewise absent from `areas`.

- Confirmed end to end in the S1 worktree: with **only** workspace membership
  changed, `python3 scripts/ci/affected_scope.py scripts/ci/schema.py` selects
  `repo-deps` in area `root` with gates `["lint","test"]`.

---

### R2 — `repo-deps` feature set in its L1 cell

**CONFIRMED on the feature decision; AMENDED with two blocking prerequisites.**

- **The L1 cell runs with default features.** `default = ["local-tools"]` links
  `biscuit-terminal`, `sniff`, `cargo_metadata`, and `ctrlc`, which the `drift`,
  `ci-plan`, and `repo-deps` bins require. The specification's ownership table
  says the L1 suite covers both `ci-rollup` **and** `ci-plan` tests, so default
  features are required. No `[package.metadata.ci.tests].features` entry.

- **The always-runs `--no-default-features --bin ci-rollup` build is left
  byte-unchanged.** Its two sites are `_area-ci.yml:252-256` and
  `just/devops.just:2199`; only the `--manifest-path` spelling changes (R5).

- **Test inventory measured** (S1 worktree, default features):

  | binary | tests |
  |---|---:|
  | `repo-deps::bin/ci-rollup` | 178 |
  | `repo-deps::bin/drift` | 32 |
  | `repo-deps::bin/ci-plan` | 9 |
  | **total** | **219** |

- **Cost measured** (Apple Silicon, cold target directory in a fresh worktree):
  cold compile of the default-feature test targets **1 m 45 s**; the
  `--no-default-features --bin ci-rollup` suite runs in **0.7–0.8 s**.

- **AMENDED — B1: `scripts/drift.rs` does not compile today.** Six call sites
  (lines 170, 223, 253, 1515, 1611, 1675) call `fallback_render(&term)`, a
  `TerminalRenderable` method that no longer exists; the surviving
  `fallback_render` belongs to `RenderableWrapper` and takes
  `(content, &Terminal)` (`biscuit-terminal/lib/src/utils/layout.rs:40`). This
  is **not** caused by the migration — `cargo build --manifest-path
  scripts/Cargo.toml --bin drift` fails identically on the unmodified tree. It
  is invisible today because no CI job compiles `drift`: `ci-tooling` builds
  only `--no-default-features --bin ci-rollup` and `--bin ci-plan`.

  **Ruling:** Phase 3 repairs it in the same change that promotes the package.
  The mechanical fix is `X.fallback_render(&term)` → `X.render(&term)`, verified
  in the S1 worktree — all 6 sites, after which the package compiles and 218 of
  219 tests pass. Per CLAUDE.md's drift rule the code is authoritative and the
  call sites are wrong; this is a repair, not a feature.

- **AMENDED — B2: one `drift` test does not terminate.**
  `tests::resolve_workspace_root_falls_back_to_metadata_workspace_root`
  (`scripts/drift.rs:2084`) builds a temp workspace under `env::temp_dir()` and
  asks `resolve_workspace_root_for_package_area` for an area that is not there.
  That function (`scripts/drift.rs:711`) walks **every** ancestor calling
  `sniff::detect_repo`, reaching `/`. Measured: killed at **400 s** still
  running (6 m 40 s of wall clock, 40 % CPU). Nextest's default profile
  (`slow-timeout = { period = "5s", terminate-after = 6 }`) kills it at 30 s, so
  promoted as-is the `repo-deps` L1 cell is **red on every environment**.

  **Ruling:** Phase 3 must land a fix before the cell is scheduled. Preferred:
  bound the ancestor walk in `resolve_workspace_root_for_package_area` (the
  defect is the unbounded walk, not the test). Do not resolve it by deleting or
  `#[ignore]`-ing the test — that reintroduces exactly the pattern Phase 7
  removes.

  The sibling `resolve_workspace_root_walks_ancestors_via_sniff_repo_detection`
  passes in milliseconds because it matches at the first candidate, which is why
  the defect is on the not-found path only.

- **Note for Phase 3:** `repo-deps` needs no `[package.metadata.ci.tests]` block
  at all. `package_ci_policy` (`affected_scope.py:632`) already defaults a
  package with no `[package.metadata.ci]` to `gates = true`, `tiers = ["L1"]`.
  The plan's "Declare CI metadata for `repo-deps`" task is explicitness-only;
  adding `tiers = ["L1"]` changes no behavior. Declare it anyway for symmetry
  with `test-toolkit`, but do not treat it as load-bearing.

---

### R3 — required environments for the two tooling packages

**DECIDED: both packages take the full default environment set. AMENDED — the
narrower alternative the plan offered does not exist.**

- **There is no declared-environment mechanism.** `CI_TEST_FIELDS`
  (`affected_scope.py:155`) is the closed vocabulary for
  `[package.metadata.ci.tests]`: `tiers`, `l2-backends`, `features`,
  `local-features`, `all-features`, `l1-include-slow`, `runner-tools`,
  `companion-suites`. There is no `environments` key. `package_cells`
  (`affected_scope.py:1464`) iterates the full `environments` list for L1 and
  `native_environments(...)` for `check`; the only per-package input is the tier
  list. A narrower set would therefore require a **new** mechanism, which
  Implementation Boundaries forbid ("No second scope calculator,
  package-discovery mechanism, …"; "No change to required package
  environments").

- **Ruling:** `repo-deps` and `test-toolkit` take the default set from
  `.github/ci/environments.json` — `ubuntu-latest`, `windows-latest`,
  `macos-latest`, `wsl2-ubuntu` — giving each **4 L1 cells + 1 `ubuntu-latest`
  lint cell**. Confirmed empirically in the S1 worktree: `job_estimate: 6`,
  `native_environments: ["ubuntu-latest","windows-latest","macos-latest"]`,
  `wsl: true`.

- **Reason.** These suites are the repository's CI contract. `test_ci_local.py`
  extracts and executes workflow shell, `test_constraints.py` resolves
  `Path.home()` (which is `USERPROFILE` on native Windows), and
  `ci_workflow_contracts.rs` parses workflow YAML — all of which have
  demonstrated OS-specific behavior in this repository. Narrowing them to Linux
  would make the packages that verify cross-OS scheduling themselves
  single-OS.

- No `check` cell is scheduled for either package: neither declares `example`
  or `bench` targets, and neither has unchanged direct reverse dependents
  (`reverse_dependencies: []` in the S1 probe).

---

### R4 — publication and release-plz

**CONFIRMED as proposed, on evidence stronger than the plan asked for.**

`repo-deps` keeps `publish = false`, and root-workspace membership does **not**
enrol it in release-plz.

- `release-plz.toml`'s `[workspace]` sets `publish = false` and
  `git_only = true` (no crates.io read or write) but leaves
  `git_release_enable = true` / `git_tag_enable = true` for members. So
  membership alone *could* have produced a `repo-deps-v0.1.0` tag.
- It does not, because release-plz honors each manifest's `publish = false`.
  Proof from this repository's own tag history:

  | package | member | `publish` | tag |
  |---|---|---|---|
  | `test-toolkit` | yes | (default true) | `test-toolkit-v0.1.0` exists |
  | `biscuit-browser-harness` | yes, since 2026-05-24 | `false` | **no tag** |
  | `biscuit-test-harness` | yes, since 2026-05-02 | `false` | **no tag** |

  Tagging ran on 2026-05-26, after both `publish = false` members joined, and
  skipped exactly them.

- **Phase 3 must not remove `publish = false` from `scripts/Cargo.toml`.** It is
  the mechanism, not decoration. No `[[package]] release = false` entry is
  needed in `release-plz.toml`.

- **Drift detected.** `release-plz.toml`'s `semver_check` comment calls
  `scripts/Cargo.lock` one of the "intentionally-untracked, gitignored
  Cargo.lock files at the nested workspace roots (tree-hugger, scripts,
  schematic/schema)". All three are in fact **tracked** (`git ls-files`).
  Retiring `scripts/Cargo.lock` in Phase 3 makes the comment more wrong; Phase 10
  corrects it. The code is right and the comment is wrong.

---

### R5 — `scripts/target` and `--manifest-path` call sites

**RULED. Complete call-site list below — this is the Phase 3 work order.**

Enumerated with `rg -n "scripts/Cargo\.toml|scripts/target"` across the tree,
excluding `fixes/`, `features/`, and lockfiles.

**`--manifest-path scripts/Cargo.toml` → `-p repo-deps`** (a package selector
suffices at every site):

| site | current |
|---|---|
| `.github/workflows/ci.yml:550` | `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features --bin ci-rollup` (job deleted in Phase 8) |
| `.github/workflows/ci.yml:574` | `cargo nextest run --manifest-path scripts/Cargo.toml --bin ci-plan` (job deleted in Phase 8) |
| `.github/workflows/_area-ci.yml:255` | `cargo build --release --manifest-path scripts/Cargo.toml --no-default-features --bin ci-rollup` |
| `just/devops.just:2199` | same shape, in `ci-diff` |
| `justfile:315` | `cargo run --manifest-path scripts/Cargo.toml --bin repo-deps` |
| `biscuit-file/justfile:225` | `cargo run --manifest-path ../scripts/Cargo.toml --bin drift -- biscuit-file …` |
| `homelab/justfile:475` | same shape |
| `queue/justfile:139` | same shape |
| `scripts/Cargo.toml:15` | comment quoting the invocation (Phase 10) |

**`scripts/target/…` → `target/…`:**

| site | current |
|---|---|
| `just/ci-local.just:292` | `BISCUIT_CI_PLAN_BIN:-scripts/target/release/ci-plan` |
| `just/devops.just:2206` | `./scripts/target/release/ci-rollup compare …` |
| `.github/workflows/_area-ci.yml:272` | `./scripts/target/release/ci-rollup rollup …` |
| `.github/workflows/_area-ci.yml:302` | `./scripts/target/release/ci-rollup verdict …` |

**Contract-test references** (`tools/test-toolkit/tests/ci_workflow_contracts.rs`
lines 3271, 3364, 3368) read `scripts/Cargo.toml` to assert the `local-tools`
split and the `ci-plan` bin. They keep reading the manifest at the same path —
the file stays, only its `[workspace]` stanza goes — but the assertions that
quote `--manifest-path` must move with the call sites.

**Corrections to the plan's R5.**

- **`scripts/*.sh` contains no such call site.** The plan lists it; `rg`
  over every shell script in `scripts/` returns zero `--manifest-path` hits.
- The three area justfiles (`biscuit-file`, `homelab`, `queue`) invoke `drift`
  through `../scripts/Cargo.toml` and are **not** in the plan's list. They break
  the moment the nested workspace is removed, and they are also the surface B1
  has been silently breaking.

**`scripts/target/` after migration.** Its *contents* are ignored
(`.gitignore:62`, `**/target/*`) but the directory itself is not, so nothing
enters the index either way. It becomes stale build output. Delete it by hand
and note the deletion in the phase log; do **not** add a cleanup step to a
recipe.

---

### R6 — Windows test tier and process model

**CONFIRMED as proposed, on native-Windows evidence (S2).**

- **Tier is L1**, per spec D4. `#![cfg(windows)]` (line 75) stays and is how the
  tier contract expresses "Windows-only".
- **Safety rests on nextest's process-per-test model.** The test calls
  `AllocConsole` and rewires process-wide std handles through `SetStdHandle`;
  one test per process is what makes that safe inside a parallel suite. `cargo
  test` — which `just _test` falls back to when nextest is absent
  (`just/devops.just:638`) — would **not** be safe. That fallback runs
  `--skip level2_ --skip level3_ …` and would run this test in-process with 390
  others.
- **The L1 filterset selects it and nothing else from the L2/L3 files.**
  `_tier_filter L1` (`just/devops.just:737`) is
  `!(test(/(^|::)level2_/) + test(/(^|::)level3_/) + test(/(^|::)browser_/) + test(/(^|::)real_/) + test(/(^|::)slow_/))`.
  Every test in `real_terminal_render.rs` is `level2_`-prefixed and every test
  in `level3_chord_select.rs` is `level3_`-prefixed, so both files contribute
  nothing. `captured_stdout_receives_only_value_no_tui_bytes` carries no tier
  prefix and is selected once `#[ignore]` is removed — **observed on
  `$BUILD_WIN`**, 392 tests run / 392 passed / 7 skipped, six consecutive
  runs.
- No feature plumbing is required: `biscuit-tui/cli/Cargo.toml:75-79` already
  declares `features = ["terminal-tests"]` with `local-features = []`.

---

### R7 — companion-suite environment and reuse blast radius

**DECIDED: each registry entry carries an explicit `environment`; only the
matching cell becomes non-reusable.**

- Today's registry is `COMPANION_SUITES = {"homelab-frontend": ("homelab",
  "test-frontend")}` (`affected_scope.py:150`) — a name → `(directory, recipe)`
  pair with **no environment**. The reuse rule is
  `reusable = not (record["companion_suites"] and capability(environment,
  "node_pnpm"))` (`affected_scope.py:1625-1630`).
- That is **capability-derived, not declaration-derived**. It happens to be
  correct today only because `node_pnpm: true` appears on exactly one
  environment:

  | environment | `node_pnpm` |
  |---|---|
  | `ubuntu-latest` | **true** |
  | `windows-latest` | false |
  | `macos-latest` | false |
  | `wsl2-ubuntu` | false |

  Flip `node_pnpm` on for macOS and every macOS L1 cell of every
  companion-owning package silently stops being reusable. The specification
  explicitly rejects package-wide non-reuse, so the coincidence must be replaced
  by a declaration.
- **Ruling:** each registry entry declares `{name, owner, recipe, environment,
  kind}`. `package_cells` marks a cell non-reusable **iff** some companion the
  package declares names that cell's environment. Capability remains a
  *validation* input — a suite declaring an environment without the capability
  its recipe needs fails contract validation — never the selector.
- All suites in this fix's ownership table declare `ubuntu-latest`, so the
  observable behavior is unchanged on this tree. The point is that it stops
  being accidental.

---

### R8 — `change_class` is retained, not replaced

**CONFIRMED as proposed, with one semantic the inventory must not inherit.**

- `classify_preflight` (`affected_scope.py:2198`) returns
  `(change_class, preflight_os, reason)` and `change_class` drives preflight
  breadth. It stays in the plan and in `legacy_scope_document`. The inventory is
  a new sibling field, purely additive.
- **`change_class` is derived from *gating* packages, not from path kinds.**
  Measured: `python3 scripts/ci/affected_scope.py tools/test-toolkit/src/lib.rs`
  reports `change_class: documentation` — because `test-toolkit` is
  `gates = false`, so `gating` is empty — even though the changed path is Rust
  source. Likewise `.github/workflows/ci.yml` and `Cargo.lock` both report
  `documentation`.
- **Ruling:** the change inventory is computed from the **changed paths**, never
  from `change_class`. The two answer different questions and must be allowed to
  disagree; a reader seeing `change_class: documentation` beside a `source`
  bucket containing a `.rs` file is seeing the truth, not a bug. Phase 9's
  renderers must not present `change_class` as a summary of the inventory.

---

### R9 — which schema counters bump

**CONFIRMED for the four Python constants; AMENDED to add a fifth, in Rust.**

| constant | file | now | after |
|---|---|---:|---:|
| `RESOLVED_PLAN_SCHEMA_VERSION` | `scripts/ci/schema.py:59` | 2 | **3** |
| `SCOPE_RECEIPT_SCHEMA_VERSION` | `scripts/ci/schema.py:66` | 1 | 1 (unchanged) |
| `RECEIPT_SCHEMA_VERSION` | `scripts/ci/schema.py:60` | 2 | 2 (unchanged) |
| `LEGACY_RECEIPT_SCHEMA_VERSION` | `scripts/ci/schema.py:71` | 1 | 1 (unchanged) |
| **`PLAN_SCHEMA_VERSION`** | **`scripts/ci-rollup.rs:70`** | **2** | **3** |

- **B5 — the Rust constant is not in the plan's R9.** `scripts/ci-rollup.rs:70`
  carries `const PLAN_SCHEMA_VERSION: u32 = 2;` with a doc comment naming
  `scripts/ci/schema.py::RESOLVED_PLAN_SCHEMA_VERSION` as its source of truth,
  and `ci-rollup.rs:1256` refuses a plan whose version differs. Bumping only the
  Python side makes **every** `ci-rollup rollup --plan` invocation fail. Phase 6
  bumps both in one commit.
- The one-time scope-receipt miss comes from `schema.py:743`, which compares the
  receipt's embedded `plan_schema_version` against
  `RESOLVED_PLAN_SCHEMA_VERSION` and rejects with the existing `scope-schema`
  reason (`schema.py:745`). `SCOPE_RECEIPT_SCHEMA_VERSION` therefore does not
  move — that check is exactly what produces the intended single miss.
- Validation receipts stay reusable where their existing cell and gate-input
  checks still qualify; nothing in this change touches `RECEIPT_SCHEMA_VERSION`.
- `.github/ci/schemas/contract.json` regenerates. Its top-level keys are
  `resolved_plan`, `receipt`, `scope_receipt`, `vocabulary`;
  `test_schema.py`'s drift assertion is the gate.

---

### R10 — documentation-class preflight matrix becomes empty

**CONFIRMED as proposed.**

- Today `classify_preflight` returns `("documentation", [SCOPE_HOST_OS],
  "no build/test packages affected; preflight runs on the scope host only")`
  (`affected_scope.py:2236-2240`).
- **Ruling:** return `[]` with a new reason string that states the *decision*
  rather than a host — e.g. `"no package work is scheduled; preflight has no
  prerequisites to establish"`. The old string names a host the job will no
  longer run on, so leaving it would be stale on day one.
- `area-ci`'s `needs: [scope, preflight]` edge still resolves promptly: its
  condition is `!cancelled() && needs.scope.outputs.has_packages == 'true'`
  (`ci.yml:455-457`), and a **skipped** dependency satisfies `!cancelled()`.
  On a documentation-only plan `has_packages` is `false` anyway, so `area-ci`
  skips on its own condition and never waits on preflight's matrix.

---

### R11 — `ci-gate` dependency list after retirement

**CONFIRMED as proposed, plus one deletion the plan omits (B4).**

- `needs` becomes exactly `[validation, scope, preflight, area-ci]`. Today it is
  those four plus `biscuit-tui-captured-stdout` and `ci-tooling`
  (`ci.yml:617-623`).
- The `RESULTS` env block (`ci.yml:629-635`) loses the two corresponding lines
  and nothing else. The fold body (`ci.yml:636-653`) is unchanged: it still
  accepts exactly `success|skipped` and blocks everything else.
- `name: ci-gate` is untouched, and the `protect-your-bacon` ruleset is not
  edited in this fix.
- `ci-reporting` is **not** added to `needs`. It is advisory and carries
  `continue-on-error: true`, which would convert its failure to `success` in the
  fold anyway — listing it would be misleading rather than merely redundant.
- **B4 — retiring `biscuit-tui-captured-stdout` orphans two things the plan does
  not name:**
  - `ci.yml:75` — the `biscuit_tui: ${{ steps.scope.outputs.biscuit_tui }}` job
    output, and `ci.yml:264`, which produces it.
  - `affected_scope.py:1958-1961` — `"biscuit-tui"` in the area-flag dict.

  Its only consumer is the retiring job's `if:` at `ci.yml:494`. Phase 8 must
  delete the output and the flag entry with the job, exactly as Phase 4 deletes
  `ci_tooling`. Leaving them is a flag nothing reads, which is the shape this
  whole fix exists to remove.

---

### R12 — empty-matrix guard shape

**DECIDED: yes — `preflight` gains an explicit scalar `if:`.**

- Current state: `preflight` (`ci.yml:383-390`) has **no** `if:` and relies
  entirely on `fromJSON(needs.scope.outputs.preflight_os)` being empty.
  `area-ci` (`ci.yml:455-457`) already carries
  `if: !cancelled() && needs.scope.outputs.has_packages == 'true'`.
- **Ruling:** `preflight` gains `if: needs.scope.outputs.preflight_os != '[]'`
  (with `!cancelled()` to match `area-ci`'s shape). Spec §8 requires a scalar
  plan output **before** matrix expansion; an explicit `if:` makes the skip a
  declared, greppable, contract-testable property instead of a consequence of
  GitHub's empty-matrix handling.
- **Neither job may carry a `name:`.** Both are skippable as a whole, and GitHub
  does not evaluate the matrix context for a skipped job, so a `name:` holding
  `${{ matrix.… }}` reaches the Checks tab as raw expression text. The comments
  at `ci.yml:376-382` and `ci.yml:446-452` explain this; Phase 8 keeps and
  updates them rather than deleting them.
- `preflight_os` is already emitted as a compact JSON scalar
  (`ci.yml:267`, `jq -c`), so `!= '[]'` is an exact string comparison against
  what `jq -c` produces for an empty list. No new output is required.

---

### R13 — `.github/workflows/**` selection and global escalation

**AMENDED — the plan's premise is false against this tree.**

The plan states that `GLOBAL_PATHS_ALL_GATES` already contains
`.github/workflows/ci.yml`, so a change to `ci.yml` "selects every package *and*
`test-toolkit`". The constant does contain it, but **global-path escalation no
longer selects any package.**

- `affected_scope.py:46-47`, on the constant itself: *"These input-analysis
  helpers remain covered for tooling diagnostics. Package scheduling itself is
  source-driven and does not consume global-input changes."*
- `plan()` derives full scope from the `--all` flag alone:
  `full_gates = set(GATES) if force_all else set()` (`affected_scope.py:1842`).
  `gate_triggers` / `global_trigger` are never consulted for scheduling.
- Measured against this tree:

  | changed path | packages | `full_scope` | `flags.ci_tooling` |
  |---|---|---|---|
  | `scripts/ci/schema.py` | `['repo-deps']` | false | true |
  | `.github/ci/ci-baseline.toml` | `[]` | false | true |
  | `.github/workflows/_area-ci.yml` | `[]` | false | true |
  | `.github/workflows/ci.yml` | `[]` | false | true |
  | `tools/test-audit/package.json` | `[]` | false | true |
  | `pnpm-lock.yaml` / `pnpm-workspace.yaml` | `[]` | false | true |
  | `Cargo.lock` | `[]` | false | false |
  | `docs/topics/ci-cd.md` | `[]` | false | false |
  | `darkmatter/README.md` | `[]` | false | false |
  | `tools/test-toolkit/src/lib.rs` | `['test-toolkit']` | false | false |

**Ruling.**

1. The new path-to-owner table is the **sole** package selector for these
   paths. There is no escalation to add it to. `workflow_dispatch` / `--all`
   remains the only full-workspace path, exactly as
   `rust-devops/ci-cd.md` states.
2. **A change to `.github/workflows/ci.yml` selects `test-toolkit` and nothing
   else.** This is a narrowing relative to the plan's stated intent, and it is
   the correct one: it is what the tree does today for every other workflow
   file, and Implementation Boundaries forbid "full-workspace selection merely
   because CI tooling changed".
3. `GLOBAL_PATHS_ALL_GATES` and `gate_triggers` are **not** touched by this fix.
   They remain diagnostic helpers with their own tests. Phase 4 deletes
   `CI_TOOLING_PREFIXES`, `CI_TOOLING_PATHS`, and `flags["ci_tooling"]` only.
4. **`scripts/**` needs no trigger entry.** `scripts/` is `repo-deps`'s own
   package directory, so `source_paths_by_package` already selects it through
   ordinary source ownership — measured above, with
   `selection_reason: "source change in package(s) repo-deps"`. Only the
   *non-package* inputs need explicit triggers:

   | trigger | owner | reason |
   |---|---|---|
   | `.github/ci/**` | `repo-deps` | the Python suites' policy inputs |
   | `.github/workflows/**` | `test-toolkit` | subject of `ci_workflow_contracts` |
   | `tools/test-audit/**` | `test-toolkit` | its companion suite |
   | `pnpm-lock.yaml`, `pnpm-workspace.yaml` | `test-toolkit` | how test-audit resolves |

   `tools/test-toolkit/**` likewise needs no entry — it is a member directory.
5. The previously hardcoded `CI_TOOLING_PATHS =
   {"tools/test-toolkit/tests/ci_workflow_contracts.rs"}` becomes redundant for
   the same reason and is deleted rather than translated.

This is the single most likely source of a silently-widened or -narrowed scope,
so the Phase 2.A fixtures must pin **the exact package list**, not merely
non-emptiness, for every row of the table above.

#### R13.6 — the owners' own manifests are triggers (added in Phase 4)

`oracles.md` left one fixture undecided: `test_the_owners_manifest_is_an_explicit_trigger`
pins `scripts/Cargo.toml` → `['repo-deps']`, while `changed_package_ids` states
repository-wide that *"`Cargo.toml` and lockfile edits select no package by
themselves; the next source edit exercises the resulting package graph."*
R13's table above covered only *non-package* inputs and so never reached the
question — `scripts/Cargo.toml` is inside a member directory but is excluded
from source ownership by `is_package_source_path`.

**Ruling: implement the trigger, for both tooling owners, and record why it is
not general.**

| trigger | owner |
|---|---|
| `scripts/Cargo.toml` | `repo-deps` |
| `tools/test-toolkit/Cargo.toml` | `test-toolkit` |

- These two packages exist *only* to verify CI. Their
  `[package.metadata.ci.tests]` blocks are where the suites in `SUITE_REGISTRY`
  are declared to run at all, so a manifest edit that selected nothing would
  ship a change to CI's own scheduling with nothing scheduled to check it.
- It is **not** generalized to every manifest, because the repository-wide rule
  exists for a measured reason (PR #39: one dropped dev-dependency scheduled
  every package on every OS) and the exception's whole justification —
  "this manifest declares CI's own verification" — is true of exactly these two.
- Symmetric by choice. An asymmetric table would invite the same question on
  the next reading and get a different answer.
- Blast radius is one package per path, and it is narrower than a source change:
  see R13.7.

#### R13.7 — a trigger selection carries no dependent seam (added in Phase 4)

`tools/test-toolkit/src/lib.rs` attributes **eleven** unchanged direct
dependents to test-toolkit's own `ubuntu-latest` check cell. A
`.github/workflows/**` edit says nothing about test-toolkit's public API, so
compiling those eleven would be pure waste presented as coverage.

**Ruling:** `calculate_scope` keeps `source_ids` and the new suite-owner ids
distinct. Reverse dependencies, the dependent seam, and the area flags are
derived from `source_ids` alone; `affected_ids` is their union. A package
selected only by a trigger therefore gets its own lint/L1 cells and, with no
attributed dependents and no declared `example`/`bench` targets, **no check
cell at all**. Measured on this tree: `.github/workflows/_area-ci.yml` →
`['test-toolkit']`, `reverse_dependencies: []`, no `dependent_seam`, no check
cell; `tools/test-toolkit/src/lib.rs` keeps all of them.

The area record says so too. `area_records` takes the suite-owner names and
renders `changed suite input owned by package(s) …` instead of claiming a
source change it did not see.

---

### R14 — what "command duration" means in producer status

**CONFIRMED as proposed; AMENDED to record that nothing is measured today.**

- **Ruling:** lint duration is measured around the `just _lint` invocation
  inside `_package-ci.yml` (step `id: clippy`, line 630) — not the job's total
  elapsed time, which includes checkout, toolchain setup, `rust-cache` restore,
  and optional Node provisioning. Each companion suite records its **own**
  command duration and counts. The report's normalized field stays the result
  cell's `duration_s`.
- **The producer status carries no duration field at all today.** Both status
  writers emit `{package, job, environment, result}` plus an optional
  `detail` and a single `companion` string:
  - test job, `_package-ci.yml:483-520`
  - lint job, `_package-ci.yml:644-679`

  `Status` in `scripts/ci-rollup.rs:677-682` matches that shape, with
  `companion: Option<String>`. So Phase 5 is **adding** a measurement, not
  relabelling one.
- `just _lint` already computes `SECONDS` and, under
  `BISCUIT_LINT_RECORD_TIMING=1`, appends it to a local
  `lint-<pkg>-timing.jsonl` (`just/devops.just:129-137`). That file is a local
  developer aid and is **not** the CI path — do not wire the status to it.
  Measure in the workflow step, where the boundary is unambiguous.
- Per AC13, a cell whose duration was not measured renders `not recorded` with
  a reason. It is never emitted as `0`.

---

## Spikes

### S1 — root-workspace migration dry run

De-risks Phase 3. Run in a throwaway detached worktree
(`spike-s1-repo-deps`, since removed): `[workspace]` stanza deleted from
`scripts/Cargo.toml`, `scripts/Cargo.lock` deleted, `"scripts"` appended to root
`members`.

**Kill criterion: NOT triggered.** No unrelated crate's version changed.

**Root `Cargo.lock` diff: 87 insertions, 4 deletions.** Entirely additive plus
reference disambiguation:

| crate | outcome |
|---|---|
| `quick-xml 0.38` | **unified** with the root's existing `0.38.4` |
| `toml 1.0` | **unified** with the root's existing `1.1.2+spec-1.1.0` |
| `rstest 0.23` | **duplicate** entry added beside `0.25.0` |
| `rstest_macros 0.23` | **duplicate** entry added beside `0.25.0` |
| `cargo_metadata 0.18.1` | **duplicate** entry added beside `0.19.2` |
| `ctrlc 3.5.2`, `nix 0.31.3` | new; no existing root consumer |
| `repo-deps 0.1.0` | the member itself |

The 4 deletions are disambiguation only — `rstest` → `rstest 0.25.0` in two
Claudine entries and `cargo_metadata` → `cargo_metadata 0.19.2` in one
tree-hugger entry. No `version = ` line of any pre-existing package changed.

`repo-deps`'s *own* resolved versions move from the retired `scripts/Cargo.lock`
to the root's: `anyhow 1.0.101 → 1.0.102`, `ctrlc 3.4.7 → 3.5.2`,
`toml 1.0.6 → 1.1.2`. That is the package adopting the workspace's existing
resolution, not a dependency bumped to promote it, so the boundary holds.

**Dual-directory validation (spec Validation 3): passes.**

```text
### from repository root
     Summary [ 0.803s] 178 tests run: 178 passed, 0 skipped
target_directory: <wt>/target

### from scripts/
     Summary [ 0.708s] 178 tests run: 178 passed, 0 skipped
target_directory: <wt>/target
workspace_root:   <wt>
scripts/Cargo.lock: does not exist
```

One lockfile, one target directory, one workspace root, one
`.config/nextest.toml` from either working directory.

**Cold-build wall time of a `repo-deps` L1 cell:** 1 m 45 s on this host
(cold target directory, default features). The `--no-default-features --bin
ci-rollup` path is unaffected and still runs in under a second.

**Nextest configuration is a migration consequence worth knowing.** The nested
`scripts` workspace has no `.config/nextest.toml`, so `repo-deps` runs today
with nextest defaults — **no timeout**. After migration it inherits the root
profile's `slow-timeout = { period = "5s", terminate-after = 6 }`. That is how
B2 surfaces: the hang exists now and is simply unenforced.

**Blocking findings B1 and B2 were discovered here** — see R2.

### S2 — Windows console test under Nextest

De-risks Phase 7. Run on the native Windows host `$BUILD_WIN` via
`scripts/cross-check.sh`, from a throwaway branch with only the `#[ignore]`
attribute removed. `BUILD_LINUX`, `BUILD_WIN`, and `BUILD_WSL` were declared on
this machine; `BUILD_MACOS` was not. This is **runtime evidence on native
Windows**, not cross-compile evidence.

**Discovery.** Under the L1 filterset with `--features terminal-tests`, the
test is listed and executed exactly once, and no `real_terminal_render` or
`level3_chord_select` test is selected.

**Stability at CI thread count** (`--test-threads 4`, matching a public-repo
`windows-latest` runner's 4 cores). Six consecutive clean runs:

| run | suite | captured-stdout test |
|---|---|---|
| 1 | 392 run, 392 passed, 7 skipped (2.766 s) | PASS 0.783 s |
| 2 | 392 / 392 / 7 (2.726 s) | PASS 0.783 s |
| 3 | 392 / 392 / 7 (2.712 s) | PASS 0.783 s |
| 4 | 392 / 392 / 7 (2.703 s) | PASS 0.782 s |
| 5 | 392 / 392 / 7 (2.785 s) | PASS 0.782 s |
| 6 | 392 / 392 / 7 (2.731 s) | PASS 0.788 s |

Zero failures, zero flakes. **Kill criterion: NOT triggered.** Three further
runs failed before reaching the test, all with
`ssh: connect to host github.com port 22: Connection timed out` during the
remote `git fetch` — a transient network fault on the build host, not a test
result; they are excluded rather than counted.

No terminal or browser window was opened or focused: every run executed over
SSH in a non-interactive session and the host desktop was not disturbed.

**Measured latency the two sleeps were covering: effectively zero.** Direct
probe, same host and thread count:

| first sleep | second sleep | test duration | outcome |
|---:|---:|---:|---|
| 750 ms (shipped) | 250 ms | 0.782–0.788 s | PASS ×6 |
| 50 ms | 250 ms | 0.084 s | PASS |
| 0 ms | 0 ms | 0.045–0.047 s | PASS ×4 |

The shipped 750 ms sleep **is** the test's runtime; the real work is ~45 ms. The
250 ms retry path never executes (a run that took it would show ≥1.03 s). The
console input buffer queues the injected record, so `inject_enter()` does not
need the child's event loop to be running yet — which is why 0 ms passes.

**Bound for Phase 7's readiness loop:** the observed requirement is 0 ms, so a
bounded retry with a **2-second deadline and ~25 ms polling** is ~40× the
measured need while still failing loudly and finishing ~370× faster than the
current fixed sleep in the failure case. Do not treat 0 ms as licence to drop
the loop: four samples on one host is not a race proof, and the loop is what
turns a timing assumption into an assertion.

**Negative case confirmed.** With `establish_console()` deliberately short-
circuited, the cell fails loudly and the diagnostic is fully visible:

```text
FAIL [0.014s] (117/392) biscuit-tui-cli::windows_captured_stdout captured_stdout_receives_only_value_no_tui_bytes
F2 precondition NOT met after console setup [SPIKE: console setup deliberately skipped]:
stderr.is_terminal()=false, CONOUT$ openable=true — this run does NOT verify the
captured-stdout-with-console contract
Summary [2.703s] 392 tests run: 391 passed, 1 failed, 7 skipped   (cross-check exit 1)
```

**B3 — `F2 precondition HELD` does not reach the log.** Run with
`--success-output immediate`, the only captured stdout for a passing run is the
harness's `running 1 test`; `grep -c "F2 precondition" → 0`. The cause is in the
test itself: `establish_console()` calls
`redirect_std_handle_to_console(STD_OUTPUT_HANDLE, &CONOUT)`
(`windows_captured_stdout.rs:183`) **before** `assert_console_precondition`
prints, so the line goes to the allocated console rather than to nextest's pipe.

This does not weaken the assertion — the `assert!` still fires, and a *failing*
run is fully visible because the panic reaches stderr while stderr is still a
pipe. But the plan's success criterion ("with `F2 precondition HELD` in the
log") cannot be met as the test stands.

**Recommended Phase 7 fix:** delete the `STD_OUTPUT_HANDLE` redirect. Its own
comment concedes it is unnecessary — *"stdout is rewired too for completeness,
though this test pipes the child's stdout rather than inheriting it"*
(`windows_captured_stdout.rs:181-182`). Removing it restores the log line and
removes a process-wide mutation the test does not need. Alternatives (duplicating
the original handle before redirect, or writing the line before
`establish_console()`) are strictly more machinery for the same result.

### S3 — companion registry shape probe

De-risks Phase 5. Every recipe hand-run on this host against the current tree.

**Answer to S3's question: no suite needs to change. Phase 5's counts task is
ONE task, not two.**

`repo-deps` companions — all ten Python suites, all green, 470 tests total,
~198 s wall:

| suite | tests | duration | exit |
|---|---:|---:|---:|
| `test_affected_scope.py` | 130 | 11.50 s | 0 |
| `test_ci_local.py` | 61 | 112.38 s | 0 |
| `test_constraints.py` | 39 | 2.49 s | 0 |
| `test_evidence_reuse.py` | 64 | 43.81 s | 0 |
| `test_local_evidence.py` | 20 | 10.71 s | 0 |
| `test_publish_gaps.py` | 19 | 0.44 s | 0 |
| `test_resolved_plan.py` | 43 | 14.22 s | 0 |
| `test_reuse_validation.py` | 15 | 0.003 s | 0 |
| `test_runner_loss.py` | 34 | 0.008 s | 0 |
| `test_schema.py` | 45 | 0.002 s | 0 |

Every suite ends with `unittest.main(verbosity=2)` and emits a stable
`Ran <N> tests in <X>s` line plus `OK` / `OK (skipped=N)` /
`FAILED (failures=N, errors=M)` on stderr. Counts are therefore obtainable
without touching any suite. **Prefer a shared runner wrapper** that imports the
module and runs it under a JSON-emitting `TestResult` over regexing that prose —
the regex is a drift surface of exactly the kind spec §3 tells us to avoid
("validate suite identities and owners, rather than comparing fragile shell
command strings"). Either way, no suite file changes.

Note `test_ci_local.py` at 112 s is over half the Python total, and
`test_evidence_reuse.py` another 44 s. That is the real cost the `repo-deps`
`ubuntu-latest` L1 cell takes on.

`test-toolkit` companions:

| suite | recipe | counts | duration | exit |
|---|---|---|---:|---:|
| test-audit typecheck | `pnpm --dir tools/test-audit typecheck` | **none emitted** | 0.86 s | 0 |
| test-audit Vitest | `pnpm --dir tools/test-audit test` | 218 total / 189 passed / 29 skipped | 0.96 s | 0 |

- **Vitest**: `vitest run --reporter=json --outputFile=<path>` yields
  `numTotalTests`, `numPassedTests`, `numFailedTests`, `numPendingTests`,
  `success`, `startTime` — machine-readable, no suite change, only an
  invocation flag. Verified: `{'numTotalTests': 218, 'numPassedTests': 189,
  'numFailedTests': 0, 'numPendingTests': 29, 'success': True}`.
- **typecheck**: `tsc --noEmit` is a pass/fail gate with **no test
  cardinality**. It has no counts to report and no flag can invent them. Per
  AC13 it renders `not recorded` with the reason *"typecheck gate reports no
  test counts"* — never `0`. This is the concrete case AC13 exists for.

Both are reached through the existing `node-environments` input, the same
mechanism `homelab-frontend` uses (`_package-ci.yml:446`, `:639`). `node_pnpm`
is true only on `ubuntu-latest`, so R7's declared environment for all four
companion entries is `ubuntu-latest`.

---

## Baseline capture

This section is the **Phase 8 work order**. Nothing in it may be dropped without
amending this record.

### `preflight` — 9 Python suite steps (`ci.yml:418-437`)

`test_schema`, `test_affected_scope`, `test_resolved_plan`, `test_local_evidence`,
`test_evidence_reuse`, `test_constraints`, `test_publish_gaps`,
`test_reuse_validation`, `test_runner_loss`.

Retained after Phase 8: checkout, `rustup show`, `just` install, nextest
install, "Verify toolchain and required tooling", "Cargo metadata without build
acceleration", "Validate canonical area recipes".

### `ci-tooling` — 13 steps (`ci.yml:503-612`)

**Correction to the plan.** The plan records "8 Python suites"; the job runs
**nine**: `test_reuse_validation`, `test_schema`, `test_affected_scope`,
`test_resolved_plan`, **`test_ci_local`**, `test_local_evidence`,
`test_evidence_reuse`, `test_constraints`, `test_publish_gaps` — plus
`cargo nextest run -p test-toolkit --test ci_workflow_contracts`,
`cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features
--bin ci-rollup`, `pnpm --dir tools/test-audit check`, and
`cargo nextest run --manifest-path scripts/Cargo.toml --bin ci-plan`.
`test_ci_local.py` is ci-tooling's alone, which is why the duplication is 8 and
not 9.

### The 8-suite duplication (spec P1, re-measured at `aad933bdb`)

Run by **both** jobs on the same commit in the same run:

```text
test_affected_scope.py   test_constraints.py    test_evidence_reuse.py
test_local_evidence.py   test_publish_gaps.py   test_resolved_plan.py
test_reuse_validation.py test_schema.py
```

`test_runner_loss.py` is preflight's alone. `test_ci_local.py`,
`ci_workflow_contracts`, `ci-rollup`, `ci-plan`, and the test-audit check are
ci-tooling's alone. Unchanged from the `08f536b08` measurement.

### Contract tests that must be rewritten, not deleted

The plan names 8 line references. All 8 verified correct. The enumeration below
is the **complete** set — 21 tests in
`tools/test-toolkit/tests/ci_workflow_contracts.rs` (3476 lines) reference a
retiring job, flag, or workflow.

| test | lines |
|---|---|
| `primary_ci_runs_a_bootstrap_preflight_before_fan_out` | 240–256 |
| `the_fan_out_has_no_stage_in_front_of_it` | 365–366 |
| `scope_job_emits_an_actionable_summary` | 521, 525, 529 |
| `the_l1_suite_runs_no_fail_fast` | 752 |
| `specialized_inventory_contains_only_surviving_workflows` | 795 |
| `active_ci_authority_matches_the_retirement_contract` | 836 |
| `ci_summarizes_the_first_actionable_failure_class` | 1005–1020 |
| `no_reusable_workflow_call_is_advisory` | 1882 |
| `junit_uploads_carry_the_whole_staging_directory_and_its_manifest` | 2029–2036 |
| `ci_gate_is_the_single_required_check` | 2066, 2069 |
| `every_top_level_job_is_either_folded_by_ci_gate_or_the_advisory_summary` | 2116–2124 |
| `post_merge_reuse_preserves_the_gate_and_normal_ci_fallback` | 2151–2178 |
| `only_ci_gate_makes_a_run_level_claim` | 2188–2200 |
| `ci_tooling_changes_schedule_the_tooling_leg` | 2559–2606 |
| `ci_tooling_leg_runs_the_workflow_contract_suite` | 2608–2620 |
| `a_reused_cell_reaches_its_area_summary_without_being_re_executed` | 2835, 2839 |
| `advisory_jobs_cannot_fail_the_run_and_gates_are_not_advisory` | 2907–2918 |
| `every_downstream_consumer_reads_the_same_ci_run_conclusion` | 3028 |
| `only_the_advisory_summary_is_excluded_from_the_run_conclusion` | 3030–3049 |
| `every_cargo_workflow_neutralizes_a_stray_rustc_wrapper` | 237 |
| module-level `GATED_JOBS` constant | 2030–2037 |

`GATED_JOBS: [&str; 6]` (line 2030) becomes `[&str; 4]`. The two `ci_tooling_*`
tests are **replaced**, not deleted: their new subject is the suite registry and
the path-to-owner table.

Also in scope for Phase 8: `WorkflowGateStepTests`
(`scripts/ci/test_ci_local.py:1840`) feeds `RESULTS` with six job names; it
reduces to four and keeps the negative test that a fold accepting `failure` is
rejected.

### Phase 5 anchor points, re-verified

| symbol | file:line |
|---|---|
| `Status::companion: Option<String>` | `scripts/ci-rollup.rs:682` |
| `companion_lint_downgrade()` | `scripts/ci-rollup.rs:2321` |
| R12 expectation check | `scripts/ci-rollup.rs:1987–1992` |
| `COMPANION_SUITES` registry | `scripts/ci/affected_scope.py:150` |
| companion reuse rule | `scripts/ci/affected_scope.py:1625–1630` |
| test-job companion step | `.github/workflows/_package-ci.yml:446` |
| lint-job companion step | `.github/workflows/_package-ci.yml:639` |

Existing fixture family to extend (`scripts/ci-rollup-tests.rs`):
`a_skipped_companion_downgrades_a_green_report`,
`a_skipped_companion_downgrades_a_green_lint`,
`a_companion_with_no_reported_outcome_downgrades_a_green_report`,
`companion_suites_are_expected_only_on_node_capable_environments`.

### `test-toolkit` current state

- Exclusion record: `tools/test-toolkit/Cargo.toml:48-55` —
  `gates = false`, `exclusion-class = "promotion-pending"`,
  `owner = "@yankeeinlondon"`, `expiry = "2026-10-31"`, plus the comment above
  it. Phase 3 deletes the block and the comment.
- Its stated blocker is satisfied: `just check-canonical tools` **passes**
  (all 12 canonical recipes present), and `tools` is already in the root
  justfile's `areas`.
- **Correction to the spec and the plan.** Both say "63 L1 tests". Measured:
  **169**, all passing under `just _test test-toolkit` (169 run, 2 filtered):

  | binary | tests |
  |---|---:|
  | `ci_workflow_contracts` | 93 |
  | unit tests in `src/` | 34 |
  | `backend_requirements` | 28 |
  | `junit_staging_contracts` | 8 |
  | `audio_spool` | 5 |
  | `nextest_config_verification` | 1 |

  Because `ci_workflow_contracts` (93) runs under the ordinary L1 recipe, the
  `ci-tooling` job's dedicated
  `cargo nextest run -p test-toolkit --test ci_workflow_contracts` step is
  genuinely redundant once `test-toolkit` gates — it is not merely relocated.
  Phase 3 updates the "63" figure wherever it survives.
- `EXCLUSION_CLASSES` (`affected_scope.py:143`) keeps `promotion-pending` as a
  **legal** class; only this package's use of it is retired.
- **B6 — `test-toolkit` does not pass its own lint gate today.**

  ```text
  error: this `if` has identical blocks
      --> tools/test-toolkit/tests/ci_workflow_contracts.rs:1938:69
       = note: `-D clippy::if-same-then-else` implied by `-D warnings`
  ```

  This is the package's only clippy finding, it is committed at `HEAD`
  (`b1a8346fb`, 2026-09-13), and the working tree is unmodified — it is not a
  Phase 1 regression. It is invisible because `gates = false` schedules **no
  cells at all** (`matrix: []`), and `ci-tooling` runs
  `cargo nextest run -p test-toolkit --test ci_workflow_contracts` without ever
  linting the package.

  **Ruling:** Phase 3 repairs it in the same change that deletes the exclusion
  record. Promoting the package creates an `ubuntu-latest` lint cell that runs
  `cargo clippy -p test-toolkit --all-targets -- -D warnings`
  (`just/devops.just:129`), so the promotion is red on arrival otherwise. The
  block in question is inside `ci_workflow_contracts.rs`, which Phase 8 rewrites
  anyway; collapse the duplicated arms there rather than adding an `#[allow]`.

  This is the same class of defect as B1 and B2 and has the same cause: a
  package excluded from gating accumulates breakage no job can see. It is the
  concrete cost the specification's P2 is about.

### `ci.yml` job inventory today

`validation`, `scope`, `preflight`, `area-ci`, `biscuit-tui-captured-stdout`,
`ci-tooling`, `ci-gate`, `summary` — eight. Target: six
(`validation`, `scope`, `preflight`, `area-ci`, `ci-gate`, `ci-reporting`).

---

## Phase 3 addenda

Recorded during Phase 3's implementation. Four further defects surfaced, all of
the same class as B1/B2/B6 and all invisible for the same reason: a package
that gates nothing is linted by nothing and run on no OS but the author's.

**B7 — `repo-deps` does not pass its own lint gate.** Three findings in
`scripts/drift.rs`, all committed at `HEAD`: `if_same_then_else` (the two arms
of `print_markdownish_line`'s `had_newline` branch were identical),
`while_let_on_iterator`, and a `format!` inside `format!` args. Repaired in
place; the collapsed branch is a behavior-preserving simplification because
both arms already called the same function with the same arguments.

**B8 — `osc8_file_link` is wrong on Windows.** Two defects in one function.
`fs::canonicalize` returns the verbatim `\\?\C:\…` spelling, which
`percent_encode_path_for_uri` turned into `file://%5C%5C%3F%5C…`; and the
label came from `to_string_lossy()`, so it carried `\` separators. `file_uri`
now normalizes separators, strips the verbatim prefix (mapping `\\?\UNC\` to
the URI authority), and adds the leading `/` that makes `file:///C:/…`; the
label routes through the existing `path_to_slash_string`. The regression tests
assert the Windows spellings as string literals so the macOS and Linux cells
cover them too.

**B9 — `python3` is an App Execution Alias on Windows.** The stub spawns
successfully and exits non-zero, so the fixture's `let Ok(output) = … else
{ skip }` guard never fired and the assertion failed instead. Replaced with a
`--version` probe over `python3` then `python`. On the `$BUILD_WIN` host the
probe finds a real interpreter and the fixture now *runs* there.

**B10 — the `wsl2-ubuntu` archive has no checkout.** Six `ci-rollup` fixtures
resolved the tree through `env!("CARGO_MANIFEST_DIR")`. Reproduced in a Linux
container by emptying the baked path and relocating the tree: **6 failures**.
`repo_root()` now falls back to the run-time working directory and
`checkout_root()` returns `None` when no checkout is reachable, so the fixtures
run against a remapped tree and skip loudly without one. Same simulation after
the fix: **182 passed, 0 failed**, and 182 passed again with no tree at all.

**R3 note.** R3's "no `check` cell for either package" was measured from the
S1 probe's scripts-only diff. With `tools/test-toolkit/` also changed,
`test-toolkit` does take an `ubuntu-latest` `check` cell compiling its 11
unchanged dependents — the ordinary per-target-kind rule, not a contradiction.

**Evidence obtained.** macOS 231/231 and 175/175; native Windows
(`$BUILD_WIN`) `repo-deps` 231/231 and `test-toolkit` 174/174; Linux
(`linux/arm64` container) `repo-deps` 231/231. **WSL2 was not reached** — the
guest at `192.168.100.64` refused SSH on two attempts 20 minutes apart. The
archive-mode risk it would have covered was instead covered directly by the
container simulation above; the `wsl2-ubuntu` cell itself remains unverified
until CI or a reachable guest runs it.

## Phase 7 addenda

**B3 repaired as recommended.** The `STD_OUTPUT_HANDLE` redirect is gone from
`establish_console()`, and `F2 precondition HELD` now reaches the cell's
captured stdout on native Windows (`$BUILD_WIN`, L1 filterset,
`--test-threads 4`, `--success-output immediate`):

```text
PASS [0.043s] (125/392) biscuit-tui-cli::windows_captured_stdout captured_stdout_receives_only_value_no_tui_bytes
F2 precondition HELD: console attached, stderr.is_terminal()=true, CONOUT$ usable
  [AllocConsole=already-present-or-failed (Access is denied. (0x80070005)); std-redirect err=true in=true]
```

Five consecutive runs: 392 run / 392 passed / 7 skipped, suite 1.94–2.29 s,
this test 0.043–0.045 s (was 0.78 s under the fixed sleep). One further run was
excluded for the same transient `ssh: connect to host github.com port 22`
fault S2 recorded — a host network fault during `git fetch`, not a test result.

**B11 — the same redirect hid every *post-child* diagnostic, not just the
success line.** B3 is about stdout; stderr has the identical defect with a
worse consequence. `establish_console()` must point stderr at `CONOUT$` (that
is the whole precondition), so any assertion panicking *after* that point —
the submit timeout, the ESC-byte check, the value check — would write its
message to the attached console and leave nextest reporting `FAIL` with an
empty diagnostic. The precondition assertion itself is unaffected, because the
only way it fails is that the redirect did *not* take, which leaves stderr on
the harness pipe (confirmed by the negative probe below).

Repaired in the same change: the test captures its stderr handle before
`establish_console()` and restores it the moment the child exits, so
`submit_and_wait` reports its timeout as an `Err` the caller panics on *after*
the restore. This is what makes "fail loudly on timeout" true rather than
nominal.

**Negative case re-confirmed on the new shape.** With `establish_console()`
deliberately short-circuited, the cell fails and the diagnostic is fully
visible with no `--success-output` flag at all (failure output is shown by
default):

```text
FAIL [0.011s] (117/392) biscuit-tui-cli::windows_captured_stdout captured_stdout_receives_only_value_no_tui_bytes
F2 precondition NOT met after console setup [PROBE: console setup deliberately skipped]:
stderr.is_terminal()=false, CONOUT$ openable=true — this run does NOT verify the
captured-stdout-with-console contract
Summary [2.300s] 392 tests run: 391 passed, 1 failed, 7 skipped   (cross-check exit 1)
```

**Readiness bound as built.** 2 s deadline, 25 ms poll, re-inject at 500 ms —
S2's recommendation. No passing run needed a second injection.

**Timeout path probed directly** (`inject_enter()` short-circuited to `false`,
so the child can never submit). It fails at its own deadline rather than at
nextest's 90 s `ci` termination ceiling, and the diagnostic survives the
console — which is the B11 repair working:

```text
FAIL [2.338s] (1/1) biscuit-tui-cli::windows_captured_stdout captured_stdout_receives_only_value_no_tui_bytes
question did not exit within 2s of the first injected Enter (0/4 console-input writes
accepted) — the captured-stdout contract was NOT exercised
```

The child is killed and reaped on that path, so the cell reports `FAIL`, never
nextest's `LEAK`.

## Checkpoint

The Phase 8 work order above is complete and nothing in it may be dropped
without amending this record. Five rulings were amended against the plan (R2,
R3, R9, R13, R14), and six blocking findings (B1–B6) were added that the plan
did not anticipate. Phases 3, 6, 7, and 8 each acquire work as a result;
Phase 5 loses work (S3 shows no suite needs modification).

**Phase 3 is materially larger than the plan assumed.** Three of the six
blocking findings (B1, B2, B6) are pre-existing breakage in the two packages
being promoted, and all three are invisible today for the same reason: a package
that gates nothing is linted by nothing, compiled by nothing beyond the one bin
CI happens to build, and timed out by nothing. Promotion surfaces all of it at
once. Budget Phase 3 for repair, not only for manifest surgery, and expect the
first `repo-deps` and `test-toolkit` cells to be red until B1, B2, and B6 land.
