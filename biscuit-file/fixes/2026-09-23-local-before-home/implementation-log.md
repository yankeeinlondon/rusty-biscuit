---
spec: /Volumes/coding/wt/rusty-biscuit/feat-better-static-analysis/biscuit-file/fixes/2026-09-23-local-before-home/spec.md
plan: biscuit-file/fixes/2026-09-23-local-before-home/plan.md
implemented_by: opencode/zai-coding-plan/glm-5.3
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - biscuit-file/lib/src/file_reference/context.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/src/lib.rs
    - biscuit-file/lib/tests/magic_local_roots.rs
    - biscuit-file/lib/tests/finalized_reference_resolution.rs
    - biscuit-file/lib/tests/implicit_relative.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - claudine/lib/src/composition/resolve.rs
    - claudine/lib/src/composition/resolve/tests.rs
    - claudine/lib/src/invocation_context.rs
    - claudine/lib/src/invocation_context/tests.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/error/render/provider.rs
    - claudine/lib/src/harness/error.rs
    - claudine/cli/src/completion/scopes.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/completion/operation_file/recovery_tests.rs
    - claudine/cli/tests/l1/sequence_magic_reference.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
    - biscuit-file
    - claudine
    - claudine-cli
    - darkmatter
---

# Implementation Log for 2026-09-23-local-before-home (4 phases)

## Phase 1

Phase 1 is read-only analysis plus rulings and frozen assertions. No
implementation code was changed in this phase.

### Task 1.1 — Trace contexts (spike, read-only)

Traced every place a launch `@` scope is captured, derived, or replaced.

**Launch capture (three entry points, all routing through
`with_prompt_magic_roots`):**

1. Canonical CLI path: `InvocationContext::capture_with_observation`
   (`claudine/lib/src/invocation_context.rs:842`) discovers the launch
   repository once, then `build_file_resolution_context`
   (`invocation_context.rs:1877`) builds the launch context from
   `FileResolutionContext::from_snapshot(cwd, home, env)` + scope catalog (or
   bare `with_repository_root`) and registers Claudine's prompt conventions.
   Exposed as `launch_file_resolution_context()`. Consumers: compose prep
   (`claudine/cli/src/commands/compose/prep.rs:131`), sequence
   (`claudine/cli/src/commands/sequence.rs:269,366`), system-prompt resolve,
   wrap/harness orchestration.
2. Composition compatibility path: `capture_file_resolution_context`
   (`claudine/lib/src/composition/resolve.rs:61`) — ambient CWD/Git/topology
   reads, same registration.
3. CLI completion: `ScopeContext::discover` → `file_resolution_context`
   (`claudine/cli/src/completion/scopes.rs:235`) — completion process CWD
   (never chdir'd, so it *is* the launch directory), same registration.
   Completion never derives source contexts, so it has no nested-source gap of
   its own; its `resolve_repo_dir_walk_root` already falls back to `ctx.cwd`
   when no repository exists, evidence the "local root = launch dir" notion is
   precedented in completion but absent from the `@` chain.

**Where a fresh source context replaces launch state (the integration gap
confirming ruling 2):**

- `InvocationContext::derive_source` (`invocation_context.rs:991`): builds a
  brand-new context for the source's parent via
  `repository_for_base(&base_dir)` + `build_file_resolution_context`. The
  launch repository/package anchors and launch magic registrations are fully
  replaced by the *source's* anchors.
  - Prompt from `~/.claudine` (ordinary layout, no repo at HOME):
    `repository_for_base` finds no repository, so the derived context has no
    repository/package/area anchors; nested `@x.md` searches only
    `~/.claudine/prompts` (prepend), `~/.claudine` (append), and `$HOME`
    (intrinsic). The launch tree is never searched.
  - Prompt from a second repository: the derived context anchors on that
    repository and registers *its* conventions; nested `@x.md` searches the
    other repository first and the launch tree never.
- `derive_request_context_for_source` (`composition/resolve.rs:118`): the
  compatibility twin — fresh Git discovery at the source parent, fresh
  `from_snapshot(source_parent, …)`. Currently **has no callers** in the repo
  (exported public API only), but is listed by the plan/spec as a path to fix.
- biscuit-file `for_source`/`for_base`/`for_trusted_external_*`
  (`biscuit-file/lib/src/file_reference/context.rs:408-462`): these *clone*
  magic paths (launch registrations survive) but call
  `recompute_repository_scopes()` which re-anchors
  repository/package/area on the new base via the scope catalog — an external
  base yields `RepositoryScope::default()` (all anchors dropped). So even the
  in-context derivations lose launch intrinsic anchors for external sources.
  Downstream users of `request_context.for_source(...)`:
  `harness/resolve.rs:142`, `composition/sequence/source.rs:73`,
  `composition/lifecycle/executor.rs:1552`, `composition/schema/supplied.rs`,
  darkmatter `document_resolution_context` (`compose/util.rs:72`).

**Smallest API sketch that transfers an immutable launch `@` scope** (for
task 2.1; matches ruling 2 as recorded in the spec): capture a request-scoped
launch `@` snapshot on `FileResolutionContext` — the construction directory
plus the repository/package/package-area roots selected for it — updated only
by the direct builder methods (`with_repository_root`,
`with_repository_scope_catalog`, `with_package_root`, `with_package_area`)
and *not* by `recompute_repository_scopes` during `for_source`/`for_base`
derivation. `@` resolution reads the snapshot; `&`, `^`, bare, and
explicit-relative references keep reading the source-derived anchors. The
`Magic` arm of `collect_roots` becomes the single consumer. Claudine then
needs no new plumbing for the launch context itself (it is already built from
the launch directory); `derive_source` must additionally *preserve* the
launch snapshot when rebuilding for an external/other-repo source — e.g. seed
`build_file_resolution_context` with the launch context's captured `@` scope
instead of re-deriving it. Completion builds from the launch directory, so it
captures the snapshot naturally; it must not rediscover it for derived
tokens.

### Task 1.2 — Audit identities (spike, read-only)

**`RootProvenance` exhaustive matches that a new `LocalRoot` variant must
extend** (adding a variant breaks these compiles — that is the safety net):

| Site | Shape | Notes |
|---|---|---|
| `claudine/lib/src/composition/error/render/provider.rs:83-90` | 8-arm match → human labels | Defect 3 renderer; the `@` branch is replaced in Phase 3 but other reference kinds keep labels |
| `claudine/lib/src/composition/error/render/mod.rs:331-340` | 8-arm match → machine slugs (`caller_root_provenance_slug`) | JSON detail |
| `darkmatter/lib/src/markdown/compose/context/options.rs:2453-2462` | 8-arm match → cache/graph codes | Codes: Repository=0, Source=1, PackageRoot=2, Home=3, Magic=4, Vault=5, Absolute=6, PackageArea=7. `LocalRoot` must **append** as 8; never renumber |
| `biscuit-file/lib/src/file_reference/mod.rs:696-711` | `!= RootProvenance::Source` comparisons (`AuthoringBaseFirst`) | Not a match; `LocalRoot` naturally is *not* boosted — exactly why the spec wants a distinct variant |
| `darkmatter/lib/src/markdown/compose/schema_validation.rs:1141` | `unwrap_or(RootProvenance::Absolute)` | Default only; no change |

Comparisons/assertions in tests (`invocation_context/tests.rs:326-332`,
`harness/resolve/tests.rs:352-356`, `composition/error/tests.rs`,
biscuit-file integration tests, darkmatter `compose/tests/schema.rs:1025-1037`)
also name variants but only the three matches above are exhaustive.

**Darkmatter graph identity:** `encode_file_resolution_context`
(`compose/context/options.rs:2249`) encodes `source_path`, `base_dir`,
`request_base_dir`, `repository_root`, `package_area`, `home_dir`,
trusted-external flag, sorted env, prepended/appended magic paths, and vault
roots. It **omits `package_root`** (spec confirms) and would omit any launch
`@` scope or tier override. It is called by *both* identity domains — graph
(`enc`, `OPTIONS_IDENTITY_DOMAIN`, incl. per-`caller_input_records` and
`caller_file_provenance` origins) and compose-cache (`cenc`,
`CACHE_OPTIONS_DOMAIN`) — so one encoder extension covers both. The separate
compose cache key (`cache/hashing.rs`, `options_hash` →
`cache_value_fingerprint`) also encodes `ComposeOptions.magic_paths` itself
(options.rs:2775) with Start=0/End=1 position tags; a tier override added to
`magic_paths` plumbing must be encoded in both the options vector and the
context encoder.

**Configured `magic_paths` consumers (darkmatter):** all funnel through
`document_resolution_context` (`compose/util.rs:72`) — `for_source`/`for_base`
on the captured snapshot plus `add_magic_path` per configured root — or
directly through `snapshot.for_base(...)` (`expression/resolve_ctx.rs:360-411`,
used by `file()`/`file_exists()` expression functions), `link_resolve.rs:165`,
`transclusion/resolver.rs:154,545`. `ComposeOptions` carries
`magic_paths: Vec<(PathBuf, PathPosition)>` (options.rs:1968) copied into
transclusion contexts (1323, 1377, 1435). Because these add configured roots
via today's `add_magic_path`, ruling 1's inferred-by-default tiering keeps
them source-compatible; only Claudine's `~/.claudine` roots need the explicit
user-tier override.

**Windows normalized path spelling / containment:** all lexical comparisons
funnel through `normalize_components` (`biscuit-file/lib/src/file_reference/resolve.rs:1108`)
which collapses `.`/`..` and then reduces `\\?\` verbatim → legacy via
`dunce::simplified` (`simplify_root`), with Windows-only tests
(`normalize_components_reduces_verbatim_paths`,
`diff_paths_bridges_verbatim_and_legacy_spellings`). R2's containment
classification should reuse exactly this function — one captured spelling per
path. 8.3 short-name spellings (`PROGRA~1`) do *not* compare equal to long
spellings lexically and cannot be repaired lexically; per the spec this is
acceptable (ordering rule, not a security boundary), and tests must use a
single spelling family per root (host-absolute temp paths or per-OS literals
per `2026-08-30-path-spelling`). Constrains ruling 1: inference must compare
`normalize_components(root)` against `normalize_components(local_root)` with
`starts_with`, and the explicit override exists precisely for the cases
lexical comparison cannot distinguish (launch dir equal to `$HOME`).

### Task 1.3 — Record rulings

This session is non-interactive, so the human review the plan requires cannot
happen here. Both recommendations are recorded as the operative rulings in
`spec.md` (new "Rulings" section), verbatim from the open questions:

- **Ruling 1 (overlapping home and local trees):** infer tiers from
  containment in the local root by default, with an explicit user-tier
  override for Claudine's `~/.claudine/prompts` and `~/.claudine`
  registrations. Ordinary `add_magic_path` keeps its signature and inferred
  meaning.
- **Ruling 2 (nested prompts from another tree):** immutable, request-scoped
  launch `@` snapshot on `FileResolutionContext`, separate from source
  repository/package/authoring-base anchors; preserved through `for_source`,
  `for_base`, and trusted-external derivations; `./`, bare, `&`, `^` keep
  source semantics; source derivation and completion must not rediscover the
  scope.

`spec.md` frontmatter now sets `human_review: true` with both rulings spelled
out as review items (options, pros/cons, recommendation) so the author can
confirm or override **before Phase 2 begins**. If a ruling changes, Phase 2's
dependent tasks (2.1 override API, 2.2 classification) and acceptance checks
must be revised first, per the plan's "Necessary Rules".

### Task 1.4 — Freeze assertions

Acceptance cases that must distinguish the fix from the reported failures
(frozen for Phases 2–4; full layouts in the spec's Tests section):

1. **`@x.md` local-wins (Defect 2 row 1):** launch in plain repo
   `<home>/config/sh` with both `<repo>/x.md` and `~/.claudine/prompts/x.md`
   present → repository file wins. Fails today (reproduced 2026-09-23 in the
   spec).
2. **`@prompts/x.md` local-wins (Defect 2 row 2):** both
   `<repo>/.claudine/prompts/x.md` and `~/prompts/x.md` present → repository
   file wins. Fails today (`<repo>/.claudine` is an append so it follows the
   home intrinsic root).
3. **Plain-directory lookup (Defect 4):** non-repository `<home>/scratch`
   containing `prompts/x.md`; both `@prompts/x.md` and `@x.md` resolve to
   that local file even with `~/.claudine/prompts/x.md` present. Fails today
   (launch dir is never an `@` root outside a repository).
4. **Home overlap:** repo-under-home (reported case), launch == `$HOME`
   without a repository, and repository == `$HOME`; under ruling 1 the
   `~/.claudine` registrations stay user-tier via the explicit override while
   same-tree convention roots stay local. New coverage — no test exercises
   these layouts today.
5. **Nested external prompts (ruling 2):** a prompt loaded from `~/.claudine`
   or a second repository resolves nested `@x.md` against the launch `@`
   snapshot (launch tree first), while its `./x.md`, bare `x.md`, `&x.md`,
   and `^x.md` keep source anchors. New coverage.
6. **Miss report (Defect 3 / R4):** `@prompts/missing.md` renders the payload
   once plus ordered search roots with `(*)` only on configured roots, no
   joined candidate paths, no provenance labels; structured probe record
   stays intact; a bare or absolute miss keeps its current diagnostic.

Frozen clarifications recorded (already settled in the spec, restated here as
the review contract):

- **Path-shaped nonmatching joins (e.g. `<root>/prompts/prompts/x.md`) remain
  legitimate probes.** They are the natural join of a bare root plus the
  authored payload; resolution still attempts them, and only the
  *human-readable* error presentation changes (R4). No resolver filtering.
- **Defect 1's five CLI tests stay green** — verified on this host (macOS)
  before any code changes:
  - `claudine-cli::l1 compose_prompt_tiers::{path_shaped,concise}_reference_reaches_user_tier_from_plain_repository_under_home`,
    `both_reference_forms_reach_user_tier_outside_any_repository`,
    `repository_prompt_wins_over_user_tier_for_both_forms`,
    `repository_claudine_tier_wins_over_user_tier_for_path_shaped_form` —
    5/5 PASS.
  - `completion_compose::compose_path_shaped_magic_offers_user_tier_from_plain_repo_under_home`
    — PASS.
  - `claudine composition::resolve::*` unit tests (incl. the three Defect 1
    registration-shape tests) — 24/24 PASS.
- **Other reference forms keep their anchors and diagnostics:** `&`, `^`,
  bare, `./`, `!`, `~`, `vault:`, and absolute references are out of scope
  for reordering (spec "Out of scope"); only the `@` chain and its
  human-readable miss report change.

### Phase 1 verification

- `cargo nextest run -p claudine-cli --features test-fixtures -E 'test(compose_prompt_tiers) or test(compose_path_shaped_magic_offers_user_tier_from_plain_repo_under_home)'` — 6/6 pass.
- `cargo nextest run -p claudine -E 'test(composition::resolve)'` — 24/24 pass.
- No source files changed in Phase 1 (read-only spikes plus documentation of
  the rulings), so no package test/lint cycles were required beyond the
  evidence runs above.

## Phase 2

Biscuit-file resolution foundation: launch `@` scope capture (2.1), unified
tier-ordered `@` chain with `RootProvenance::LocalRoot` (2.2), and the R1–R3
parity/regression suite (2.3). All three tasks implemented; `just test`
(856 tests) and `just lint` pass in the biscuit-file package area, and
cross-check legs pass on Linux, native Windows, and WSL2 (macOS = local
host). No Phase 3 consumer files were touched.

### Task 2.1 — Capture launch scope

**New public API on `biscuit_file::FileResolutionContext`
(`biscuit-file/lib/src/file_reference/context.rs`):**

- `LaunchMagicScope` — the immutable, request-scoped snapshot: `request_dir`
  plus the repository/package/package-area roots selected for it, with
  accessors (`request_dir`, `repository_root`, `package_root`,
  `package_area`, `local_root`). `local_root()` is the repository root when
  one exists, else the request directory.
- `launch_magic_scope()` accessor and `with_launch_magic_scope(scope)`
  builder. The builder is the seeding API a request uses when it *rebuilds*
  its context around an external or other-repository source (ruling 2's
  cross-repository half): current anchors may follow the source while `@`
  keeps the launch tree. Proven by
  `seeded_launch_scope_keeps_at_local_while_sigils_follow_the_source`.
- `MagicPathTier { Inferred, User }` (mod.rs) with
  `add_magic_path_with_tier(path, position, tier)`; `add_magic_path` keeps
  its signature and means `Inferred`.
- `MagicPathRegistration` + `magic_path_registrations()` — the tier-aware
  registration view (path/position/tier) for Darkmatter's cache identity.
- `magic_search_roots() -> Vec<MagicSearchRoot>` — the R4 diagnostics
  exposure (ordered, deduplicated roots with provenance).

**Snapshot lifecycle:** initialized from the construction directory by
`new`/`from_snapshot`; synced by the direct builders `with_repository_root`,
`with_repository_scope_catalog` (post-recompute), `with_package_root`,
`with_package_area`; *never* touched by `for_source`/`for_base`/
`for_trusted_external_*` (they clone it verbatim). `recompute_repository_scopes`
no longer implicitly moves launch state — `with_repository_scope_catalog`
explicitly syncs afterward. Internal `ResolutionContext` carries
`launch_magic_scope: Option<LaunchMagicScope>` (`Some` via `from_context`,
`None` for ambient).

**Relative configured roots** resolve against the snapshot's `request_dir`
lazily at chain-build time (ambient: the context CWD, which for
`resolve_from(base)` is `base`). Behavior change documented on
`add_magic_path` (context + `FileReference`), matching R2.

### Task 2.2 — Unify root ordering

- `RootProvenance::LocalRoot` added (mod.rs) for the request-directory local
  root when no launch repository exists; documented as distinct from
  `Source` so `AuthoringBaseFirst` cannot boost it (asserted by
  `authoring_base_first_does_not_boost_the_local_root`).
- `build_magic_chain(&MagicChainInputs)` in resolve.rs is the single `@`
  ordering authority: local prepends → package root → package-area root →
  local root → local appends → user prepends → home → user appends. Tier =
  explicit `User` override, else normalized lexical containment
  (`normalize_components`, so Windows verbatim/legacy spellings collapse) in
  the local root. All roots normalized before ordering; dedup after ordering
  keeps first-seen provenance (a configured root equal to an intrinsic root
  keeps `Magic`).
- One chain, four consumers: `collect_roots`'s Magic arm (direct candidates
  and `%@` traversal via `build_candidates`/`build_search_roots`),
  `candidate_plan`, actual resolution, and completion —
  `completion_roots`' Magic arm now appends the typed scope segment only
  after the prebuilt chain is selected (R3). `complete_partial` (ambient)
  builds the same chain from live anchors; without a repository its local
  root is the base (Defect 4's completion side).
- `resolve_core`/`candidate_plan`: for Magic with a captured launch scope,
  the resolution's `repository_root` is the snapshot's root (or its
  absence) — the source-derived anchor never re-anchors `@`, and
  `DetailedResolution::repository_root` reports what `@` actually used.
- Non-`@` kinds (`&`, `^`, bare, `./`, `!`, `~`, `vault:`, absolute) keep
  their exact prior root construction; only the Magic arm changed.

### Task 2.3 — Prove resolver parity

**New test binary `biscuit-file/lib/tests/magic_local_roots.rs` (17 L1
tests; auto-discovered target, run by `just test`; no tier markers needed).
Requirement-to-test mapping:**

| Requirement (spec/plan) | Test |
|---|---|
| R2 headline tier order, every anchor + local/user root at each position | `tier_order_with_every_anchor_present` |
| Repo nested under `$HOME` | `repository_nested_in_home_precedes_home` |
| Defect 4 / no repo under home, `LocalRoot` provenance | `no_repository_local_root_is_the_request_directory` |
| Containment classification incl. external `/opt/configs` fallback | `tier_is_decided_by_containment_in_the_local_root` |
| Ruling 1: launch == `$HOME`, no repo (override beats inference) | `launch_equals_home_user_override_wins_over_inference` |
| Ruling 1: repository == `$HOME` | `repository_equals_home_user_override_wins_over_inference` |
| Ruling 2: trusted external source keeps launch scope; `./`/bare keep `Source` | `trusted_external_source_keeps_the_launch_scope` |
| Ruling 2: seeded scope, `&`/bare follow source repo | `seeded_launch_scope_keeps_at_local_while_sigils_follow_the_source` |
| Relative configured root vs captured request dir, across derivation | `relative_configured_root_anchors_to_the_captured_request_directory` |
| Ambient `resolve_from(base)` relative-root base change | `ambient_resolve_from_interprets_relative_roots_against_base` |
| Recursive `%@` walks local-before-user once, first-seen provenance | `recursive_magic_walks_local_roots_once_in_tier_order` |
| R3 parity, all four completion forms, configured tiers | `completion_roots_match_the_chain_for_every_entry_form` |
| R3 parity, no-repository layout (completion vs `LocalRoot` plan) | `completion_without_repository_enumerates_the_launch_directory_first` |
| R4 ordered root-list exposure | `magic_search_roots_exposes_the_ordered_chain` |
| `LocalRoot` not boosted by `AuthoringBaseFirst` | `authoring_base_first_does_not_boost_the_local_root` |
| Defect 2 row 1 + Defect 4 first-match with real duplicate files | `local_first_match_wins_over_user_tier_with_duplicate_files` |
| Defect 2 row 2 (local append beats home, path-shaped) | `local_append_root_beats_home_for_path_shaped_magic` |

Synthetic cases use `from_snapshot` with per-OS absolute literals (`abs()`
helper) — no ambient CWD/HOME dependence; first-match cases stage real files
under `TempDir`. The `&`/`^` parity case uses real (empty) directories
because completion validates repository containment through real
canonicalization.

**Existing tests updated to the governing rule (both previously encoded the
old ordering):**

- `finalized_reference_resolution.rs`:
  `magic_intrinsic_chain_is_between_registered_roots` →
  `magic_chain_orders_local_tiers_before_user_tiers` (outside-repo Start
  root now correctly follows the local tier; adds local-tier Start/End
  coverage).
- `implicit_relative.rs`: `magic_without_repo_only_returns_home_root` →
  `magic_without_repo_returns_base_then_home_roots` (Defect 4 completion
  side).

### Verification

- `just test` (biscuit-file area): 856 passed, 0 failed (includes the 17
  new tests, the `--no-default-features` `test-minimal` gate, and
  biscuit-file-cli's 62).
- `just lint`: clean for both packages. `just doctest`: clean. New
  rustdoc intra-doc links verified with `RUSTDOCFLAGS="-D warnings" cargo
  doc` (the two remaining doc errors — `analyze_yaml`, `PolicyClient` —
  predate this change).
- Cross-check: `just cross-check biscuit-file --os linux|windows|wsl` —
  all pass (WSL note: no receipt published, patched-tree run; tests green).
- Tier placement: all new tests are plain L1 `#[test]`s in auto-discovered
  targets; no stranded tier markers.

### Expected consumer breakage (Phase 3 scope, verified by `cargo check`)

Adding `RootProvenance::LocalRoot` and the accessor signature change break
exactly three sites, all owned by Wave 2 tasks:

1. `claudine/lib/src/composition/error/render/provider.rs` — 8-arm
   exhaustive match (task 3.2 rewrites the `@` branch anyway).
2. `claudine/lib/src/composition/error/render/mod.rs` — provenance slug
   match (task 3.2).
3. `darkmatter/lib/src/markdown/compose/context/options.rs` — the provenance
   cache-code match (task 3.3; `LocalRoot` appends as code 8) **and** an
   `E0308` at the `encode_file_resolution_context` call site because
   `prepended_magic_paths()`/`appended_magic_paths()` now return owned
   `Vec<PathBuf>` instead of `&[PathBuf]`. The owned return is what lets
   the encoder consume `magic_path_registrations()` (tier-aware) in the
   same pass; claudine-cli inherits the claudine breakage transitively.
   All other biscuit-file dependents in the workspace (sniff, messenger,
   …) still compile.

### Environment note (build-linux)

The first `cross-check --os linux` failed compiling dependencies with
`output file … is not writeable` — the documented kache-hardlink poisoning
of the standing clone's `target/` (docs/kache-strategy.md, 2026-09-09
incident; 372 read-only files present). Repaired by deleting only the
non-writable files in the standing clone's `target/` (regenerable build
artifacts) over `ssh -o BatchMode=yes build-linux`; the leg then passed in
35 s. No source or repository files were touched on the host.

## Phase 3

Consumer integration: Claudine scope wiring (3.1), the R4 search-root miss
report (3.2), and Darkmatter identity (3.3). All three tasks implemented;
`just test` and `just lint` pass in the claudine (7341 tests), darkmatter
(8497), and claudine-cli (2776, `test-fixtures`) package areas, and
`cargo check --workspace --all-targets` is clean. A cross-platform OS note:
all Phase 3 logic is lexical path handling rendered through
`to_portable_string`; task 4.3 owns the multi-OS evidence runs.

**Session note.** This phase started from an uncommitted, non-compiling
work-in-progress tree (a previous session had begun 3.1 and 3.3 — the
user-tier registration, launch-root fallback, `derive_source` scope
carrying, and the Darkmatter identity encoder). This session completed,
corrected, and verified that work and implemented all of 3.2.

### Task 3.1 — Wire Claudine scopes

- `with_prompt_magic_roots` (`claudine/lib/src/composition/resolve.rs`) is
  the single registration point and now (a) takes the launch **local root**
  (repository root, else the launch directory) instead of a repository
  `Option`, so a plain directory registers the same convention rows a
  repository does (R5 / Defect 4); (b) registers the two `~/.claudine` rows
  (`~/.claudine/prompts` Start, `~/.claudine` End) through
  `add_magic_path_with_tier(..., MagicPathTier::User)` — the ruling-1
  override — and skips an inferred local twin naming the same directory in
  the launch-equals-home layouts, where local-first dedup would otherwise
  reclassify the user row. `prompt_magic_roots`/`prompt_magic_fallback_roots`
  signatures changed from `Option<&Path>` to `&Path` for the local root.
- All three registration callers pass the launch local root:
  `capture_file_resolution_context` (composition compat),
  `build_file_resolution_context` in `invocation_context.rs`
  (`InvocationContext` + `derive_source`), and CLI completion's
  `file_resolution_context` (`claudine/cli/src/completion/scopes.rs`,
  `repository_root.unwrap_or(ctx.cwd)`).
- Ruling 2 carrying: `build_file_resolution_context` gained a
  `launch_scope: Option<&LaunchMagicScope>` parameter. The launch context
  passes `None` (captures its scope naturally); `derive_source` and the
  compat `derive_request_context_for_source` pass the launch context's
  captured `launch_magic_scope()`, register conventions against its local
  root, and seed the snapshot **last** via `with_launch_magic_scope`, so
  `./`, bare, `&`, and `^` keep the source-derived anchors while `@` keeps
  the launch tree. Completion never derives source contexts (Phase 1
  finding), so its wiring is the local-root change alone.
- `composition/sequence/expr.rs` reviewed: its `@magic.flag` fixture uses a
  configured root inside the repository (local-tier Start) — no changed
  winner; composition and sequence preflight tests stage unique files per
  root, likewise unchanged. Two stale WIP call signatures in
  `composition/resolve/tests.rs` (`Some(&repo)` → `&repo`) were corrected.
- **Governed expectation changes** (ruling 2 flips the nested-`@` winner):
  - `invocation_context/tests.rs`:
    `nested_sources_rebuild_their_own_prompt_convention_roots` →
    `nested_sources_keep_launch_at_conventions_while_reanchoring_source_scopes`
    — both derived contexts keep the launch (alpha) `@` conventions and
    launch scope, while `package_root()` still re-anchors per source.
  - `claudine/cli/tests/l1/sequence_magic_reference.rs`:
    `sequence_magic_reference_uses_source_doc_location_not_cwd` →
    `sequence_magic_reference_follows_launch_scope_and_relative_stays_source_anchored`
    — a nested `@fixtures/steps.yaml` launched from an unrelated repository
    now resolves against the **launch** tree (3 steps), while
    `./fixtures/steps.yaml` stays anchored beside the source document
    (1 step), preserving the original CWD-hijack regression under the
    reference kind that now owns it.

### Task 3.2 — Render search roots

- `ResolutionDetail` (`claudine/lib/src/harness/error.rs`) stores the
  ordered `@` root list beside its probe record: new
  `magic_search_roots: Vec<MagicSearchRoot>` (empty except for direct magic
  misses), `with_magic_search_roots` builder, and `kind()`/`
  `magic_search_roots()` accessors.
- `CompositionError::from_detailed_no_match(detailed, context)` now takes
  the resolving context and records `context.magic_search_roots()` when the
  reference kind is Magic. Both production call sites (composition
  `resolve_composition_source_in_context`, CLI sequence YAML fallback) and
  the two recovery-test call sites pass it.
- `render/provider.rs` splits the `FileReferenceNoMatch` body: a direct
  magic miss renders the R4 shape — the payload once (the authored
  reference minus `%`/`@` prefixes), the search-root directories in
  priority order via `UnorderedList`, `(*)` only on
  `RootProvenance::Magic` roots, the footnote
  `(*) searched in addition to the standard `@` roots, for this context`,
  and the existing "Did you mean:" block when suggestions exist. No joined
  candidate paths, no provenance labels. Every other reference kind keeps
  the exact prior "Cannot resolve … Tried:" body. The structured
  `detail["candidates"]` record (concrete paths, dispositions, provenance)
  is unchanged for both branches.
- Exhaustive `RootProvenance::LocalRoot` arms added:
  `provider.rs` candidate labels ("local root"),
  `render/mod.rs::caller_root_provenance_slug` ("local-root"),
  `harness/error.rs::root_provenance_slug` ("local_root"). These were the
  three remaining compile breakers from Phase 2; the workspace now builds.
- **Focused renderer tests** (biscuit-terminal `StatusBlock`/`Prose`/
  `UnorderedList` through `report_block_error`, plain 500-col terminal):
  - `magic_no_match_report_lists_search_roots_in_priority_order` — the
    reported layout (plain repo under `$HOME`), the exact reported input
    `@prompts/missing.md`: payload-once (which also proves no joined
    candidates, since every join repeats the payload as a suffix), no
    `@`-prefixed raw reference, no provenance labels, six representative
    root lines in local-before-user priority order, `(*)` count ==
    configured roots + footnote, footnote present, and structured
    candidates/dispositions/provenance intact.
  - `magic_no_match_without_repository_lists_the_launch_directory_without_marker`
    — plain `$HOME/scratch`, no repository: the launch directory is listed
    first and **unmarked**, precedes the home root, and the machine detail
    reports the new `local_root` candidate provenance.
  - `bare_no_match_keeps_the_candidate_report` — a bare miss keeps
    "Cannot resolve … Tried:" and never sees the `@` sentence.
- Registration-shape tests in `composition/resolve/tests.rs` (from the WIP,
  verified/corrected here): launch-directory convention rows without a
  repository, `.claudine` rows registered user-tier exactly once under
  launch-equals-home, and the observable
  `user_tier_prompt_row_stays_behind_local_files_when_launch_is_home`
  first-match proof.

### Task 3.3 — Update Darkmatter identity

- `encode_file_resolution_context`
  (`darkmatter/lib/src/markdown/compose/context/options.rs`) now encodes
  `package_root`, the full captured launch `@` scope
  (`launch_magic_scope`: request dir, repository, package, package-area),
  and the tier-aware `magic_path_registrations()` (path, position tag, tier
  tag) in place of the tier-less prepend/append lists. One encoder serves
  both identity domains, so the separate compose-cache key
  (`options_hash` → `compose_cache_fingerprint`) covers the same
  winner-changing inputs alongside its existing raw `magic_paths` encoding.
- `RootProvenance` cache-code match: `LocalRoot` appends as code **8**
  (Repository=0 … PackageArea=7 untouched; codes are persisted with graph
  identities).
- Identity proofs added (all assert `graph_value_fingerprint` **and**
  `compose_cache_fingerprint` differ):
  `identities_distinguish_launch_magic_scope`,
  `identities_distinguish_magic_tier_override`,
  `identities_distinguish_context_package_root`.
- `ComposeOptions.magic_paths` callers reviewed (compose util
  `document_resolution_context`, transclusion resolver, link resolver,
  expression contexts, reference graph/validation, type tests): all add
  configured roots through the inferred-tier `add_magic_path`, which
  ruling 1 preserves verbatim — no signature or behavior change needed.
  The `with_magic_path` doc comment now states the tier semantics.
- `compose/tests/schema.rs` `@collision/spec.md` expectation reordered to
  the governing chain (local Start root → package area → repository →
  local End root → home). `AuthoringBaseFirst` retention for bare
  references stays proven by biscuit-file's
  `authoring_base_first_does_not_boost_the_local_root` plus the passing
  darkmatter schema-validation tests that use the order.

### Verification

- `just test` / `just lint` per area: claudine 7341 passed / clean;
  darkmatter 8497 passed / clean (incl. wasm32-wasip2 zed-dmls check);
  claudine-cli 2776 passed (`test-fixtures`) / clean.
- `cargo check --workspace --all-targets`: clean.
- Defect 1 retention: the five `compose_prompt_tiers` tests plus
  `completion_compose::compose_path_shaped_magic_offers_user_tier_from_plain_repo_under_home`
  — 6/6 PASS.
- Full completion suite (470 tests) passes with the `scopes.rs` local-root
  wiring.
- Tier placement: new/renamed tests are plain L1 — claudine lib `#[test]`s
  and the `l1` test binary (module declared in `tests/l1/main.rs`); all ran
  in the suites above. No new tier markers, no stranded tests.
- Cross-OS evidence deferred to task 4.3 per the plan's wave structure.

### Requirement-to-test mapping (Phase 3 scope)

| Requirement | Test |
|---|---|
| R5 registration against launch local root, no repository | `prompt_magic_roots_without_repository_register_launch_conventions`, `prompt_magic_fallback_roots_are_local_then_home_claudine`, CLI `both_reference_forms_reach_user_tier_outside_any_repository` (retained) |
| Ruling 1 explicit user tier for `~/.claudine` rows | `with_prompt_magic_roots_marks_the_claudine_home_rows_user_tier`, `user_tier_prompt_row_stays_behind_local_files_when_launch_is_home` |
| Ruling 2 launch `@` scope through `derive_source` | `nested_sources_keep_launch_at_conventions_while_reanchoring_source_scopes`, CLI `sequence_magic_reference_follows_launch_scope_and_relative_stays_source_anchored` |
| Ruling 2 `./` keeps source anchoring | same CLI test's explicit-relative half |
| R4 payload-once, ordered roots, `(*)` only on configured | `magic_no_match_report_lists_search_roots_in_priority_order` |
| R4 no-repository intrinsic local root, unmarked | `magic_no_match_without_repository_lists_the_launch_directory_without_marker` |
| R4 structured record preserved; other kinds unchanged | same tests' `detail` assertions + `bare_no_match_keeps_the_candidate_report` + `recovery_enriches_explicit_no_match_without_selecting_suggestion` (retained) |
| R2 consumer order (local before user, both Defect 2 rows) | `path_shaped_prompt_reference_keeps_closest_tier_first`, `prompt_magic_candidates_interleave_conventions_and_intrinsic_scopes_once`, CLI `repository_prompt_wins_over_user_tier_for_both_forms` / `repository_claudine_tier_wins_over_user_tier_for_path_shaped_form` (retained) |
| Darkmatter identity: scope/tier/package_root | `identities_distinguish_launch_magic_scope`, `identities_distinguish_magic_tier_override`, `identities_distinguish_context_package_root` |
| Darkmatter chain order change | `compose/tests/schema.rs` `@collision/spec.md` row |
