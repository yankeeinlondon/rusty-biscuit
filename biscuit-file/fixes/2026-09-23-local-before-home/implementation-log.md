---
spec: /Volumes/coding/wt/rusty-biscuit/feat-better-static-analysis/biscuit-file/fixes/2026-09-23-local-before-home/spec.md
plan: biscuit-file/fixes/2026-09-23-local-before-home/plan.md
implemented_by: opencode/zai-coding-plan/glm-5.3
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - biscuit-file
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
