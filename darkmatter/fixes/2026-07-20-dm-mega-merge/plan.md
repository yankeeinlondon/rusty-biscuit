---
agent: opencode/zai-coding-plan/glm-5.2
total_phases: 8
created: 2026-07-21
phase: 1
yolo: "true"
---

# Execution Plan: Darkmatter and More-Is-More Integration Merge

## References

- **Specification**: `darkmatter/fixes/2026-07-20-dm-mega-merge/spec.md`
- **Conflict report**: `darkmatter/fixes/2026-07-20-dm-mega-merge/conflict-report.md`
- **`darkmatter` branch log**: `darkmatter/fixes/2026-07-20-dm-mega-merge/darkmatter-log.md`
- **`more-is-more` branch log**: `darkmatter/fixes/2026-07-20-dm-mega-merge/more-is-more-log.md`
- **Research workbook**: `darkmatter/fixes/2026-07-20-dm-mega-merge/_research.md`

## Pinned Inputs

These revisions are the validity envelope for the entire plan. If any of them
move, work MUST stop and the merge base, branch deltas, shared-path inventory,
and `git merge-tree` preview MUST be regenerated before any further resolution
edits.

| Input | Revision |
|---|---|
| Merge base | `d672388dd0fed4196295e7f21514cac6fa59f0ae` |
| `darkmatter` head | `14dd391f45206d58383ba9d84adbf53c65520534` |
| `more-is-more` head | `0584d8297f57f5eb30b52d03b1241ba55184bb44` |

## Expected Package-Area Scope

Per spec R2.4, recorded with `sniff repo packages`:

| Area | Reason in scope |
|---|---|
| `biscuit-file` | Shared spans and the source-first YAML analysis/repair foundation |
| `sniff` | Git discovery, worktrees, conflict prediction, remotes, providers, credentials |
| `darkmatter` (`darkmatter`, `darkmatter-cli`, `dmls`) | Compose, expressions, schemas, cleanup, references, CLI, and DMLS |
| `claudine` (`claudine`, `claudine-cli`) | Downstream traversal/validation of container expressions and semantic schema values |
| `biscuit-terminal` | Shared terminal discovery/caching and the real-terminal Darkmatter harness boundary |

## Cross-Cutting Rules (apply to every phase)

- **R1 isolation**: Integration happens only in the disposable clean worktree.
  Source worktrees and refs are read-only until handoff.
- **R2 change intelligence**: Run `sniff repo packages` once to record scope;
  run GitNexus `impact({target, direction: "upstream"})` before editing any
  affected symbol; surface HIGH/CRITICAL risk before proceeding.
- **R5 authority boundaries**: No second parser, validator, formatter, remote
  executor, Git implementation, or terminal harness may be introduced for an
  already-owned concern.
- **R6 no whole-side resolution**: Whole-file `ours`/`theirs` is forbidden for
  production files unless a path-level audit proves one side is intentionally
  identical/obsolete and the resolution record contains that proof.
- **R9 evidence integrity**: Existing benchmark samples, fixture hashes, review
  records, and historical evidence MUST NOT be rewritten merely because the
  branches merged.
- **R10 cross-platform**: Code and tests remain designed for macOS, Windows,
  and Linux. Native non-macOS execution is NEVER a completion gate.
- **R11 resource isolation**: One validation owner; package-area gates run
  serially; real-terminal tests stay serialized; never run workspace-wide Cargo
  gates or unscoped root lifecycle recipes; never run `cargo fmt` in write mode.
- **Comment quality / formatting**: Match surrounding style by hand. Do not
  introduce stray reformats — they poison branch↔`main` merges.

## Resolution Record (open throughout the plan)

Maintain one entry per conflict and per auto-merged shared production path. Each
entry MUST include:

- path and conflict type;
- behavior contributed by `darkmatter`;
- behavior contributed by `more-is-more`;
- chosen merged structure and why it preserves the authority boundary;
- symbols and flows identified by GitNexus impact analysis, when applicable;
- focused tests or inspections used as evidence;
- any follow-up that is explicitly outside merge completion.

Suggested location: `darkmatter/fixes/2026-07-20-dm-mega-merge/resolution-record.md`
(one entry appended per resolved path; never edit a closed entry).

The ten required entries (six conflicts + four auto-merge audits):

1. `.claude/skills/darkmatter/SKILL.md` (conflict; hash regenerated in Phase 5)
2. `.claudine/memory/commits.md` (conflict)
3. `CLAUDE.md` (conflict; GitNexus count refreshed in Phase 5)
4. `darkmatter/cli/tests/level2_code_block_styling.rs` (conflict)
5. `darkmatter/cli/tests/level2_errors.rs` (conflict)
6. `darkmatter/features/2026-07-15-performance-followup/review-8.md` (modify/delete)
7. `darkmatter/lib/Cargo.toml` (auto-merge audit)
8. `darkmatter/lib/src/markdown/schemas/mod.rs` (auto-merge audit)
9. `darkmatter/lib/src/markdown/schemas/validate.rs` (auto-merge audit)
10. `darkmatter/cli/src/commands/compose.rs` (auto-merge audit)

Any newly overlapping path discovered after a refreshed preview MUST be added
to the same record before work continues.

---

## Phase 1 — Freeze Source State and Provision Integration Worktree

**Goal**: Establish a clean, recoverable foundation for the merge. Capture
pinned revisions, inventory source-worktree state, halt interfering processes,
and create the disposable integration worktree from the pinned `darkmatter`
head. (Maps to spec Phase 0 / R1.)

**Depends on**: nothing.

**Validation checkpoint**: Source worktrees are unchanged; the integration tree
is clean; pinned inputs are recoverable by name; the conflict inventory is
current; one validation owner is recorded.

- [ ] Designate a single validation owner for the duration of the integration.
  Record the owner identity and the host budget for `CARGO_BUILD_JOBS` and
  `NEXTEST_TEST_THREADS` in `resolution-record.md`.
- [ ] Confirm the three pinned revisions still equal the SHAs in the spec's
  "Pinned Inputs" table:
  - [ ] merge base `d672388dd0fed4196295e7f21514cac6fa59f0ae`;
  - [ ] `darkmatter` head `14dd391f45206d58383ba9d84adbf53c65520534`;
  - [ ] `more-is-more` head `0584d8297f57f5eb30b52d03b1241ba55184bb44`.
- [ ] If any pinned ref moved, STOP and regenerate merge base, branch deltas,
  shared-path inventory, and `git merge-tree` preview before any further work.
- [ ] Run `git status --short` in both source worktrees
  (`/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter` and
  `/Users/ken/.claudine/worktrees/rusty-biscuit/more-is-more`) and save the
  output to `resolution-record.md`. The `darkmatter` worktree currently has a
  modified `CLAUDE.md`, untracked `.claude/settings.local.json`, and the
  untracked merge report directory — these MUST NOT enter any merge commit.
- [ ] Halt every interfering Cargo, rustc, linker, Nextest, Criterion, and
  terminal-harness process across all related worktrees before proceeding.
- [ ] Capture pre-merge fixture hashes and retained benchmark artifact
  identities (per conflict-report "Additional Risk Reduction") so silent
  rewrites are detectable after the merge.
- [ ] Record the package-area scope with `sniff repo packages` and save the
  output to `resolution-record.md`.
- [ ] Capture pre-edit GitNexus impact data for the symbols in the four
  auto-merged production paths (Phase 3 targets) and the high-risk directories
  in spec R8. For each symbol, record direct callers, affected execution
  flows, and risk level. Surface any HIGH/CRITICAL risk in
  `resolution-record.md` before that symbol is edited.
- [ ] Create lightweight backup refs for both pinned heads, for example:
  - [ ] `git branch backup/dm-mega-merge/darkmatter 14dd391f45206d58383ba9d84adbf53c65520534`;
  - [ ] `git branch backup/dm-mega-merge/more-is-more 0584d8297f57f5eb30b52d03b1241ba55184bb44`.
- [ ] Create a dedicated clean integration worktree and branch from the pinned
  `darkmatter` head. Suggested path:
  `/Users/ken/.claudine/worktrees/rusty-biscuit/dm-mega-merge` on branch
  `dm-mega-merge`. Confirm `git -C <integration> status` is clean and that
  `git -C <integration> rev-parse HEAD` equals the pinned `darkmatter` SHA.
- [ ] Record the integration worktree path, branch name, and clean status in
  `resolution-record.md`.

---

## Phase 2 — Create the Unresolved Integration State

**Goal**: Perform the merge without committing, compare actual conflicts to the
predicted six, and freeze a pre-resolution snapshot. Every unmerged path MUST
be mapped to a requirement before any conflict marker is edited. (Maps to spec
Phase 1.)

**Depends on**: Phase 1.

**Validation checkpoint**: Every unmerged path is understood and mapped to a
requirement; the pre-resolution status and unmerged-path list are persisted in
the merge record.

- [ ] From the integration worktree, run
  `git merge --no-commit --no-ff 0584d8297f57f5eb30b52d03b1241ba55184bb44`.
  Do not commit. Do not abort unless Phase 1 recovery is required.
- [ ] Capture the live conflict inventory:
  - [ ] `git status --short > <integration>/pre-resolution-status.txt`;
  - [ ] `git diff --name-only --diff-filter=U > <integration>/unmerged-paths.txt`.
- [ ] Compare the actual conflicts with the six predicted paths from
  conflict-report §"Predicted Mechanical Conflicts":
  - [ ] `.claude/skills/darkmatter/SKILL.md`;
  - [ ] `.claudine/memory/commits.md`;
  - [ ] `CLAUDE.md`;
  - [ ] `darkmatter/cli/tests/level2_code_block_styling.rs`;
  - [ ] `darkmatter/cli/tests/level2_errors.rs`;
  - [ ] `darkmatter/features/2026-07-15-performance-followup/review-8.md`
    (modify/delete).
- [ ] If any unexpected conflict, modify/delete case, or newly overlapping
  path appears, STOP and update the conflict inventory and resolution record
  before continuing. Regenerate the `git merge-tree` preview if the
  discrepancy suggests the source trees have shifted.
- [ ] For each unmerged path, create a stub entry in `resolution-record.md`
  containing: path, conflict type, the contributing branch behaviors (per
  conflict-report), and the spec requirement(s) that govern resolution.
- [ ] Confirm that `git -C <integration> log -1 --pretty=%H` still equals the
  pinned `darkmatter` head (no commit should have been created).

---

## Phase 3 — Resolve Production Authority Seams

**Goal**: Resolve every production-code conflict and every auto-merged shared
production path by meaning, then audit the directory-level semantic unions.
Production code MUST be clean and metadata-valid before any test or
documentation work proceeds. (Maps to spec Phase 2 / R5 / R7 / R8.)

**Depends on**: Phase 2.

**Validation checkpoint**: No conflict markers remain in production code;
`cargo metadata --no-deps --format-version 1` succeeds and confirms the
intended package/features/dev-dependency shape; the authority-boundary audit
(R5) is complete for every seam; every Phase 3 path has a closed
`resolution-record.md` entry.

> **Parallelizable** (within this phase): the four production paths are
> textually independent and MAY be resolved in separate worktree sessions by
> different operators, as long as only one operator runs `cargo metadata` and
> the final directory audit. The directory-level R8 audits are also
> parallelizable by directory. Do NOT parallelize across worktrees running
> build/test/lint gates (R11).

### 3a — `darkmatter/lib/Cargo.toml` (auto-merge audit)

- [ ] Read the merged manifest and confirm all three independent changes are
  present:
  - [ ] `sniff` with the `remote` feature for provider functionality;
  - [ ] `git2` as a **dev-only** merge-prediction oracle (not in `dependencies`);
  - [ ] the `clean_hot_paths` benchmark target.
- [ ] Run `cargo metadata --no-deps --format-version 1` and confirm it
  succeeds and that the package/features/dev-dependency shape matches
  expectations. Save the command's exit status in `resolution-record.md`.
- [ ] Confirm Sniff's pure-Rust Git implementation remains the production
  authority — `git2` MUST NOT have entered production dependencies.

### 3b — `darkmatter/lib/src/markdown/schemas/mod.rs` (auto-merge audit; highest silent-merge risk)

- [ ] Run GitNexus `impact({target: "<affected symbol>", direction: "upstream"})`
  for each of: the `clean` analysis facade, `effective_for_with_override`,
  raw-validation entry points, `SchemaReference`, and the source-aware cursor
  and parser exports. Record results in `resolution-record.md` and surface any
  HIGH/CRITICAL risk before editing.
- [ ] Confirm the merged module preserves **all** of:
  - [ ] the extracted test-module layout (do not restore the removed god-file);
  - [ ] the public `clean` analysis exports and schema override / raw-validation
    seams from `darkmatter`;
  - [ ] `SchemaReference` classification and bounded reference resolution from
    `more-is-more`;
  - [ ] all source-aware cursor, declaration, value, span, and parser exports
    needed by DMLS;
  - [ ] the `PartialOrd`/`Ord` behavior used for deterministic clean
    diagnostics; and
  - [ ] the invalid-frontmatter `clean` facade, `effective_for_with_override`,
    raw validation, and ordered problem codes contributed by `darkmatter`.
- [ ] If any export, registration, or ordering is missing, restore it
  additively (do not choose a side); record the symbol(s) and invariant(s)
  preserved in `resolution-record.md`.

### 3c — `darkmatter/lib/src/markdown/schemas/validate.rs` (auto-merge audit)

- [ ] Confirm the `pub(super)` helper visibility required by schema-clean
  analysis (from `darkmatter`) is retained.
- [ ] Confirm the URL-scheme keyword and both `type-definition` and `schema`
  custom validator registrations (from `more-is-more`) are present.
- [ ] Verify the merged validator builder separately exercises raw and
  coercing validation paths so a schema-proven clean repair is not hidden by
  coercion.
- [ ] Append a closed entry to `resolution-record.md`.

### 3d — `darkmatter/cli/src/commands/compose.rs` (auto-merge audit)

- [ ] Confirm the merged file retains the shared `env_disables_baseline_schema`
  baseline-disable rule used by clean (from `darkmatter`).
- [ ] Confirm the merged file retains the removal of obsolete approval-error
  bindings (from `more-is-more`).
- [ ] Confirm focused provider/approval error classification and rendering is
  intact.
- [ ] Append a closed entry to `resolution-record.md`.

### 3e — Directory-level semantic audits (R8)

For each directory union, confirm new state and policy objects propagate
through every intended entry point (nested compose subtrees, frontmatter,
body, shell expressions, CLI commands, DMLS passive projections):

- [ ] `darkmatter/lib/src/markdown/compose/context` — confirm `ctx.branch`,
  `ctx.worktree`, `ctx.merge_conflicts` are demand-driven, share repository
  discovery, and degrade independently (R4.3).
- [ ] `darkmatter/lib/src/markdown/compose/expression` — confirm array/object
  literals, indexed-file functions, and postfix indexing are intact (R4.1,
  R4.2).
- [ ] `darkmatter/lib/src/markdown/schemas` (especially `clean`, `simplified`,
  `reference`, `resolve`, `format`, `validate`) — confirm shared grammar/AST
  authority, descriptors, serializer, portable carrier domain, custom
  validators, array postfix support, shared recursion limit, and base
  `$schema: schema` declaration (R4.9, R4.10).
- [ ] `darkmatter/dmls/src/{overlay,providers,diagnostics}` and the graph
  substrate — confirm DMLS remains catalog-driven and passive: no shell, no
  network, no repository mutation, no compose execution (R4.8).
- [ ] `darkmatter/cli/src/commands/clean*` and `commands/compose.rs` — confirm
  clean/compose parity, idempotency, and the baseline-disable rule (R3.4).
- [ ] `sniff/lib/src/filesystem/git` and `sniff/lib/src/remote` — confirm
  conflict prediction remains caller-repository anchored, committed-tip based,
  deterministic, and free of HEAD/ref/worktree/live-index/object-database
  mutation; confirm preferred-remote selection and provider/flavor discovery
  are Sniff-owned (R4.4, R4.5).
- [ ] Claudine lifecycle, validation, classification, and rendering paths —
  confirm Claudine continues to traverse, validate, classify, and render
  container expressions and semantic schema values (R4.12).

### 3f — Phase 3 closeout

- [ ] From the integration worktree, run
  `rg -n '^<<<<<<< |^>>>>>>> |^=======$' darkmatter/lib darkmatter/cli/src darkmatter/dmls/src`
  and confirm zero matches in production code.
- [ ] Run `cargo metadata --no-deps --format-version 1` one more time and
  confirm it succeeds.
- [ ] Confirm every Phase 3 path has a closed `resolution-record.md` entry
  with the required fields.

---

## Phase 4 — Resolve Tests and Focused Harness Behavior

**Goal**: Resolve the two Level 2 test conflicts around the centralized shared
helper and audit auto-merged tests for accidental duplication of the harness,
parser, validator, or formatter. (Maps to spec Phase 3 / R3.4.)

**Depends on**: Phase 3 (tests reference resolved production code).

**Validation checkpoint**: Focused tests compile far enough to confirm imports,
feature flags, and test-harness topology before broad package-area gates; no
duplicate tmux harness, parser, validator, or formatter was restored; the two
test-file `resolution-record.md` entries are closed.

- [ ] Run GitNexus impact analysis for any helper symbols referenced by the
  test files (e.g., the shared `common::level2` module, `md_shim`) and record
  the results.
- [ ] Resolve `darkmatter/cli/tests/level2_code_block_styling.rs`:
  - [ ] Use the `darkmatter` structural direction: keep the centralized helper
    in `tests/common/level2.rs` and the shared-harness imports.
  - [ ] Do NOT reintroduce the local tmux harness, sentinel loop, fixture
    writer, or `run_md_in_tmux` helper from `more-is-more`.
  - [ ] Port any unique incoming Cargo build-shim coverage and
    terminal-discovery coverage that is not already represented in the
    centralized helper.
  - [ ] Preserve serialization of real-terminal tests.
- [ ] Resolve `darkmatter/cli/tests/level2_errors.rs`:
  - [ ] Keep exactly one `md_shim` import in the canonical surrounding order.
  - [ ] Confirm the tests use the current shared helper and build shim rather
    than any obsolete local path.
- [ ] Audit auto-merged tests in:
  - [ ] schema, clean, and validate suites;
  - [ ] expression, context, and indexed-file suites;
  - [ ] DMLS overlay, providers, diagnostics, and session suites;
  - [ ] Sniff Git, conflict prediction, remote, and credential suites;
  - [ ] Claudine traversal, validation, classification, and rendering suites.
- [ ] Confirm no local duplicate tmux harness, parser, validator, formatter,
  Git implementation, or terminal discovery was restored anywhere in
  `darkmatter/`, `dmls/`, `sniff/`, `claudine/`, or `biscuit-terminal/`.
  Record the negative finding in `resolution-record.md`.
- [ ] Close the two test-file entries in `resolution-record.md`.

---

## Phase 5 — Resolve Documentation, Policy, and Generated Metadata

**Goal**: Resolve the remaining three predicted conflicts (skill, commit
guidance, review chain) and refresh generated metadata **once** from the
resolved tree. Generated artifacts describe the merged tree, not either source
branch. (Maps to spec Phase 4 / R6 / R9.)

**Depends on**: Phase 3 (production code resolved) and Phase 4 (tests
resolved) so generated counts reflect real behavior.

**Validation checkpoint**: `rg` finds no unresolved conflict markers anywhere
in the integration worktree; `git diff --check` passes; the review chain is
internally consistent; generated metadata (skill hash, GitNexus counts)
describes the merged tree.

> **Ordering constraint**: The skill body MUST be final before its hash is
> recomputed; the source tree MUST be final before GitNexus is refreshed.
> These two refreshes are the last actions in Phase 5.

### 5a — Skill content merge

- [ ] Open `.claude/skills/darkmatter/SKILL.md` and resolve the textual
  conflict (currently limited to frontmatter `hash` and `last_updated`).
- [ ] Retain both branches' non-duplicated guidance:
  - [ ] `darkmatter` body: corrected cleanup/list behavior, fresh vs. checked
    reference-graph validation, invalid-frontmatter analysis, later package
    architecture;
  - [ ] `more-is-more` body: Git context capture, conflict prediction,
    expression literals/functions, remote providers, meta-schema semantics.
- [ ] Read the merged body end-to-end and reconcile contradictions against
  the merged code (Phase 3 outcomes are authoritative).
- [ ] Update `last_updated` to the integration date.
- [ ] Leave the `hash` field as-is until 5e.

### 5b — Commit guidance merge

- [ ] Resolve `.claudine/memory/commits.md`:
  - [ ] Keep the `darkmatter` non-interactive signing/pinentry safety
    guidance as a bullet.
  - [ ] Keep the `more-is-more` prohibition on bypassing repository hooks as
    a separate bullet.
  - [ ] Preserve the incoming `--only` plus `-F -` argument-order guidance
    that auto-merges elsewhere in the file.
- [ ] Confirm neither rule is weakened by the consolidation.

### 5c — Review chain repair

- [ ] Resolve the `review-8.md` modify/delete conflict by **keeping** the
  `darkmatter` version. Do not accept the deletion.
- [ ] Restore the Review 7 → 8 → 9 → 10 chain:
  - [ ] Audit `review-7.md` (auto-merged) and restore its `next: review-8.md`
    pointer if `more-is-more` removed it.
  - [ ] Verify `review-8.md`'s `next:` pointer reaches `review-9.md`.
  - [ ] Verify `review-9.md`'s `next:` pointer reaches `review-10.md`.
  - [ ] Verify `review-10.md` has no `next:` pointer (or an explicit terminal
    marker).
- [ ] Confirm the performance-followup review chain still states its
  quiet-host evidence is **open** (R9.5). Do not relabel the gap as closed.

### 5d — Repository support files review

- [ ] Review root workflow changes for accidental duplicate jobs or reduced
  OS coverage (R9.1).
- [ ] Review the testing-strategy, review-schema, prompt, skill, and
  commit-guidance unions.
- [ ] Confirm existing benchmark samples, fixture hashes, review records, and
  historical evidence were not rewritten merely because the branches merged
  (R9.2).
- [ ] Confirm carried-forward evidence gaps (performance-followup quiet-host,
  invalid-frontmatter timing/native runtime) remain visible and unlabeled as
  closed (R9.5, R9.6).

### 5e — Refresh generated metadata (LAST in Phase 5)

- [ ] Recompute the Darkmatter skill Markdown-aware hash from the final body
  using `md hash .claude/skills/darkmatter/SKILL.md` (or the library
  equivalent) and write it into the file's frontmatter `hash` field.
- [ ] Refresh GitNexus once from the resolved source tree:
  - [ ] `node .gitnexus/run.cjs analyze` (or `npx gitnexus analyze` if no
    runner is present);
  - [ ] record the post-merge symbol/relationship counts;
  - [ ] update `CLAUDE.md` with the refreshed GitNexus counts (resolving the
    `CLAUDE.md` conflict by replacing both stale counts with the merged-tree
    count).
- [ ] Inspect every snapshot delta in the integration tree. Tie each changed
  snapshot to an intentional merged behavior (R9.4). Revert or document any
  snapshot that cannot be tied to a named acceptance criterion.

### 5f — Phase 5 closeout

- [ ] From the integration worktree, run
  `rg -n '^<<<<<<< |^>>>>>>> |^=======$'` and confirm zero matches across the
  whole tree.
- [ ] Run `git diff --check` and confirm it passes.
- [ ] Confirm every Phase 5 path has a closed `resolution-record.md` entry.

---

## Phase 6 — Focused Convergence Tests

**Goal**: Run the smallest deterministic tests first so any failure stays
attributable to a single seam. Each seam MUST have focused passing evidence
before the broader package-area gates begin. (Maps to spec Phase 5 / R3 / R4 /
verification matrix.)

**Depends on**: Phase 5 (clean tree, generated metadata refreshed).

**Validation checkpoint**: Every convergence seam listed below has at least
one focused passing test on record. Any failure MUST be diagnosed against the
`resolution-record.md` entry for the affected seam before broader gates run.

> **Parallelization**: Within a single integration worktree, focused library
> unit tests MAY be run concurrently with each other via nextest's default
> thread pool. Real-terminal tests MUST stay serialized. Do NOT run focused
> tests from a second worktree concurrently (R11).

- [ ] **Schema clean + meta-schema**: schema exports; nominal validators;
  raw and coercing validation separately; references / cycles / depth;
  source projection; DMLS schema consumers.
- [ ] **Compose + providers**: structured literals; indexed-file endpoints;
  demand-driven Git context (`ctx.branch`, `ctx.worktree`, `ctx.merge_conflicts`);
  remote-runtime propagation; fatal focused provider errors; exact-host
  deny-by-default policy.
- [ ] **Git safety**: `git2` and pure-Rust Git parity fixtures; conflict
  prediction direction sensitivity; before/after repository-state assertions
  proving no mutation of HEAD, refs, index, worktree, or object database.
- [ ] **Reference validation**: fresh path skips re-verification;
  `FileTree::ensure_built` skips compatibility/dependency rereads; caller-
  supplied graphs remain fail-closed and reject stale or mismatched
  descendants; `PreparedHeadingSnapshot` keeps fragment validation coherent.
- [ ] **Cleanup + DMLS formatting**: default / preserve / fixed-width list
  modes (ordered, unordered, task, nested, blockquoted); opaque mixed shell,
  page, and directive blocks; idempotency; library / compose inline-post /
  CLI stdout / CLI `--save` / DMLS formatting parity.
- [ ] **Sniff remote/provider**: preferred-remote selection; `branch_exists_on_remote`;
  `remote_vendor`; `pr`, `pr_list`, `cicd`, `cicd_list`; provider-aware
  credential isolation; Wiremock-only provider tests.
- [ ] **DMLS passive behavior**: no-side-effects suite; completion; hover;
  diagnostics; document links; LSP session behavior; last-good recovery.
- [ ] **Claudine downstream**: container traversal; semantic-schema
  classification, validation, lifecycle, and rendering; CLI context
  formatting for the new nominal schema types.
- [ ] **Terminal boundary**: single terminal discovery; centralized Level 2
  harness; build shim; stable code-block rendering.
- [ ] **Invalid frontmatter**: ratified repair matrix (A/S1–S4 normalization
  and quoting; B report-only findings; C safety/spans; D file/stdin/`--save`/
  verbose/JSON/zero-work; E LF/CRLF/CR/BOM/UTF-8/final-newline forms; F
  pinned YAML Test Suite, mutation/property, parse-count invariants).
- [ ] For each focused suite, record the nextest filter used and the pass /
  fail count in `resolution-record.md`.

---

## Phase 7 — Scoped macOS Package-Area Gates

**Goal**: Run one affected package area at a time using its own `just`
recipes, with exact package selectors where supported. Build, Level 1, Level
2, and lint gates MUST pass serially on the available macOS host. (Maps to
spec Phase 6 / R2 / R11.)

**Depends on**: Phase 6 (focused tests already green for every seam).

**Validation checkpoint**: Every area retained by R2 has passing `just build`,
`just test`, `just test-l2`, and `just lint` evidence in
`resolution-record.md`, with the recorded host budget held constant across
runs.

> **Serial only** (R11): never run two package areas concurrently; never run
> gates from a second worktree; cap `CARGO_BUILD_JOBS` and
> `NEXTEST_TEST_THREADS` to the recorded host budget; never use a bare root
> `cargo build`/`check`/`test`, `--workspace`, or an unscoped root lifecycle
> recipe; real-terminal Level 2 tests keep their existing serialization.

Run the areas in this order (chosen to surface failures near the changed
authorities first):

### 7a — `biscuit-file` area

- [ ] `cd biscuit-file` (or use the exact package selector) and run, in order:
  - [ ] `just build`;
  - [ ] `just test`;
  - [ ] `just test-l2`;
  - [ ] `just lint`.
- [ ] Record the gate results in `resolution-record.md`. If R2 evidence
  supports removing `biscuit-file-cli` from the gate scope, document the
  reduction with both sniff and GitNexus evidence.

### 7b — `sniff` area

- [ ] `cd sniff` and run, in order:
  - [ ] `just build`;
  - [ ] `just test`;
  - [ ] `just test-l2`;
  - [ ] `just lint`.
- [ ] Record the gate results in `resolution-record.md`.

### 7c — Darkmatter area (`darkmatter`, `darkmatter-cli`, `dmls`)

- [ ] Run each gate across the three Darkmatter packages, in order:
  - [ ] `just build`;
  - [ ] `just test` (Level 1);
  - [ ] `just test-l2` (Level 2, including real-terminal coverage with
    existing serialization);
  - [ ] `just lint`.
- [ ] Use exact package selectors where the recipe supports a narrower
  recorded scope (e.g., `-p darkmatter`, `-p darkmatter-cli`, `-p dmls`).
- [ ] Record the gate results in `resolution-record.md`.

### 7d — Claudine area (`claudine`, `claudine-cli`)

- [ ] `cd claudine` and run, in order:
  - [ ] `just build`;
  - [ ] `just test`;
  - [ ] `just test-l2`;
  - [ ] `just lint`.
- [ ] Record the gate results in `resolution-record.md`.

### 7e — `biscuit-terminal` area + Darkmatter real-terminal Level 2

- [ ] `cd biscuit-terminal` and run, in order:
  - [ ] `just build`;
  - [ ] `just test`;
  - [ ] `just test-l2`;
  - [ ] `just lint`.
- [ ] Re-confirm Darkmatter's real-terminal Level 2 coverage still passes
  against the shared terminal implementation (single discovery, stable
  code-block rendering).
- [ ] Record the gate results in `resolution-record.md`.

### 7f — Scope reduction audit

- [ ] For every package removed from the gate scope, document that both
  sniff discovery and GitNexus impact evidence support the reduction (R2.5).
  Otherwise, restore the package to the scope and run its gates.

---

## Phase 8 — Final Audit and Handoff

**Goal**: Prove the merged result is ready for an explicitly authorized
commit. Both source refs and worktrees MUST remain recoverable. The merge
report makes no unsupported performance, platform, or production-readiness
claim. (Maps to spec Phase 7 and the completion criteria.)

**Depends on**: Phase 7 (all gates green).

**Validation checkpoint**: GitNexus change detection matches the recorded
scope; the final status and staged diff contain only intended integrated
work; all ten shared paths have closed resolution entries; all carried-forward
evidence gaps are still documented as open; a concise merge report is
produced and the integration result is ready for an explicitly authorized
commit.

- [ ] Run GitNexus `detect_changes({scope: "compare", base_ref: "main"})`
  from the integration worktree and compare the affected symbols and execution
  flows with the recorded Phase 1 scope. Investigate any drift.
- [ ] Inspect `git -C <integration> status` and confirm it shows only intended
  changes (no source-worktree-local files, no local settings, no generated
  junk, no bulk snapshot update).
- [ ] Inspect the complete `git -C <integration> diff` and the staged diff
  end-to-end.
- [ ] Confirm all ten shared paths have closed resolution/audit entries in
  `resolution-record.md` (six conflicts + four auto-merge audits).
- [ ] Confirm no unrelated source-worktree file, local setting, generated
  junk, or wholesale snapshot update is present.
- [ ] Confirm all carried-forward evidence gaps are still documented and
  have not been mislabeled as closed:
  - [ ] performance-followup quiet-host captures (acceptance criteria 5 and
    6) remain open;
  - [ ] invalid-frontmatter common-case timing and native Linux/Windows
    runtime evidence remain visible.
- [ ] Walk the spec's completion criteria (15 items) and record pass/fail
  for each in `resolution-record.md`.
- [ ] Produce a concise merge report at
  `darkmatter/fixes/2026-07-20-dm-mega-merge/merge-report.md` covering:
  - [ ] pinned inputs (restate the three SHAs);
  - [ ] actual conflicts vs. predicted (Phase 2 inventory);
  - [ ] resolutions and auto-merge audits (link to `resolution-record.md`);
  - [ ] focused convergence evidence (Phase 6);
  - [ ] package-area gate results (Phase 7, with host budget);
  - [ ] generated metadata refreshed (skill hash, GitNexus counts, snapshots);
  - [ ] any non-blocking follow-up (quiet-host captures, invalid-frontmatter
    timing/native runtime);
  - [ ] explicit statement that no Level 3 evidence is used or required and
    that native Windows/Linux execution is not a completion gate.
- [ ] Confirm both source refs (`darkmatter@14dd391f...`,
  `more-is-more@0584d82...`) still equal their pinned SHAs and that the
  backup refs from Phase 1 are still present.
- [ ] Confirm the integration worktree's HEAD is unchanged from the pinned
  `darkmatter` head (no commit yet) and that the working tree contains only
  the intended integrated diff, ready for an explicitly authorized `git
  commit`.
- [ ] Stop. Do NOT commit, tag, push, or update either source branch. Hand
  off the integration worktree path, branch name, and merge report to the
  authorizing operator.

---

## Dependency Graph

```
Phase 1 (Freeze + integration worktree)
   │
   ▼
Phase 2 (Merge --no-commit + conflict inventory)
   │
   ▼
Phase 3 (Resolve production authority seams) ──► 3a / 3b / 3c / 3d parallel;
   │                                              3e directory audits parallel;
   │                                              3f closeout serial
   ▼
Phase 4 (Resolve tests + harness)
   │
   ▼
Phase 5 (Resolve docs/policy/generated metadata) ──► 5a-5d parallel;
   │                                                  5e refresh LAST;
   │                                                  5f closeout serial
   ▼
Phase 6 (Focused convergence tests)
   │
   ▼
Phase 7 (Scoped macOS package-area gates) ──► 7a / 7b / 7c / 7d / 7e serial
   │
   ▼
Phase 8 (Final audit + handoff)
```

## Parallelization Notes

- **Phase 3 (3a–3d)**: the four production paths are textually independent
  and MAY be resolved in parallel sessions, as long as only one session runs
  `cargo metadata` and the final directory audit (3e). The directory audits
  in 3e are also parallelizable by directory.
- **Phase 5 (5a–5d)**: the skill, commit-guidance, review-chain, and
  support-file resolutions are independent and MAY proceed in parallel. The
  generated-metadata refresh (5e) is strictly last because it MUST read from
  the final source tree.
- **Phase 6**: focused library unit tests within a single integration worktree
  MAY run concurrently via nextest's default thread pool. Real-terminal tests
  and any L2 suites MUST stay serialized (R11). Never run gates from a second
  worktree.
- **Phase 7**: strict serial by area. The suggested ordering surfaces
  failures near the changed authorities first.
- **Phase 8**: strictly serial; no parallel work allowed during final audit.

## Out-of-Scope Reminders (per spec §Non-goals)

- No redesign or extension of any merged feature.
- No refactor of adjacent code merely to make the merge look cleaner.
- No change to public APIs, CLI schemas, serialized shapes, diagnostics, exit
  codes, or feature acceptance criteria beyond what the two pinned branches
  already do.
- No replacement of Sniff's Git/provider authority, `biscuit-file`'s YAML
  authority, or Darkmatter's expression/schema authority.
- No closure of the performance-followup quiet-host evidence gap.
- No closure of the invalid-frontmatter timing / native runtime gaps.
- No Level 3 testing.
- No native Windows/Linux hardware requirement.
- No workspace-wide Cargo gates and no unscoped root lifecycle recipes.
- No `cargo fmt` or `rustfmt` in write mode.
- No modification, rebase, force-update, or deletion of either source branch.
