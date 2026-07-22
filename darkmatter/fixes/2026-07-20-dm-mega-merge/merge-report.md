---
status: blocked
phase: 7
generated_on: 2026-07-21
---

# Darkmatter and More-Is-More Merge Report

## Result

The merged working tree preserves the tested behavior of both pinned parents,
but it is **not ready for commit authorization**. The operator explicitly
prohibited staging, while Phase 7 requires a fully resolved staged index.
Consequently six marker-free working-tree resolutions remain unmerged in the
index. The required final GitNexus refresh also exceeded the non-interactive
60-second ceiling and was stopped; generated counts therefore remain the
temporary Darkmatter-parent values.

No commit, tag, push, worktree deletion, source-branch update, Level 3 run, or
native Windows/Linux run was performed or claimed.

## Pinned topology and recovery

| Item | Value | Result |
|---|---|---|
| Darkmatter parent / `HEAD` | `14dd391f45206d58383ba9d84adbf53c65520534` | Pass |
| More-is-more parent / `MERGE_HEAD` | `0584d8297f57f5eb30b52d03b1241ba55184bb44` | Pass |
| Computed merge base | `d672388dd0fed4196295e7f21514cac6fa59f0ae` | Pass |
| Darkmatter backup | `refs/dm-mega-merge/backups/darkmatter-14dd391f` | Pass |
| More-is-more backup | `refs/dm-mega-merge/backups/more-is-more-0584d829` | Pass |
| Integration branch | `dm-mega-merge-integration-20260721-phase1` | Pass |
| Integration worktree | `/Users/ken/.claudine/worktrees/rusty-biscuit/dm-mega-merge-integration-20260721-phase1` | Pass |
| External ledger | `/private/tmp/dm-mega-merge-preflight.CdC0FB` | Pass |
| Immutable manifest SHA-256 | `9c30d5f02b9fe6714b33f05b808988d7a016f668b5a1876a4efe60b0aba3affe` | Pass |
| Cargo target directory | `/private/tmp/dm-mega-merge-target.EhU7p8` | Pass |

## Conflict and shared-path resolutions

The six actual conflicts exactly matched the reviewed preview:

- `.claude/skills/darkmatter/SKILL.md`: semantic union retained both behavior
  catalogs; the Markdown-aware body hash is now
  `87f17662fa397abe-c0eb7c8a0924fdd4`.
- `.claudine/memory/commits.md`: retained non-interactive signing/hook safety
  and the incoming `--only` argument-order guidance.
- `CLAUDE.md`: retained both parents' authored guidance; GitNexus counts remain
  temporarily `136293 / 270769 / 300` because the final refresh did not finish.
- `darkmatter/cli/tests/level2_code_block_styling.rs`: retained the centralized
  Level 2 helper and rejected the duplicate local harness.
- `darkmatter/cli/tests/level2_errors.rs`: retained one canonical `md_shim`
  import and the Cargo-built binary path.
- `darkmatter/features/2026-07-15-performance-followup/review-8.md`: retained
  Review 8 and restored the Review 7 -> 8 -> 9 -> 10 chain.

The four shared production paths were clean semantic unions: Sniff's manifest
features, the schema facade, validator construction, and compose CLI error /
baseline behavior. The full per-path authority and evidence records are in
`resolution-record.md`. P11 records the additional test-fixture correction in
`level2_schema_about.rs`.

## Requirement-to-test mapping

Phase 7 changes no runtime behavior, parser, schema, template, prompt, or
configuration artifact. Its verification map is metadata idempotence,
change-intelligence reconciliation, topology/index integrity, and replay of
the already completed Phase 5/6 gates. No new regression test is appropriate.

The exact focused evidence is F5-01 through F5-11 in `resolution-record.md`:
53 schema/meta-schema, 98 invalid-frontmatter, 53 compose/provider, 170 Sniff
Git/remote, 23 reference-trust, 196 cleanup/formatting, 91 passive DMLS, 14
Claudine downstream, and 56 mechanism-guard tests. The real terminal seam
passed the complete Darkmatter area recipe: 18 Darkmatter, 69 Darkmatter CLI,
and 3 DMLS Level 2 tests. These tests cover shipped-artifact corpus and normal
invocation paths, the exact malformed/native/quoted/missing/boundary variants,
negative behavior, downstream state, and repeated persisted read/write/read
round trips required by the implementation request.

## Scoped area gates

All final Phase 6 commands exited 0 on macOS using the isolated target and
bounded jobs. Detailed command lines and logs are in the external ledger.

| Area | Build | Level 1 | Level 2 | Lint |
|---|---:|---:|---:|---:|
| `biscuit-file` | Pass | 624 lib + 61 CLI pass | Canonical no-op | Pass |
| `sniff` | Pass | 1,634 lib + 769 CLI pass | 2 pass | Pass |
| `darkmatter` | Pass | 6,084 lib + 643 CLI + 640 DMLS pass | 18 + 69 + 3 pass | Pass |
| `claudine` | Pass | 21 + 3,411 + 47 + 1,907 + 90 pass | 131 CLI pass | Pass |

Tier-filtered skips and the `biscuit-file` Level 2 no-op are intentional. One
pre-existing tmux case passed on configured retry. No selected final test or
lint failed. `biscuit-terminal` was not activated because both parent subtrees
are identical and the downstream Darkmatter real-terminal seam passed.

## Generated metadata

- `md hash ... --save` updated only the Darkmatter skill hash/date.
- The immediate `md hash ... --diff` exited 0 with “No semantic changes
  detected.”
- The repository-local GitNexus runner was absent. The global installation
  failed under Node 16, so the existing repository runner was invoked under
  Node 22 against this worktree.
- A normal and then forced verbose refresh parsed the integration corpus, but
  the latter exceeded the required non-interactive ceiling before graph
  persistence/registration. It was stopped with exit 130. `.gitnexus/` contains
  ignored partial generated data; no tracked generated path changed.
- Because `gitnexus status` still reported “Repository not indexed,” the
  temporary parent counts in `CLAUDE.md` were not replaced with invented or
  partial values.

## Change intelligence and parent preservation

The existing GitNexus service, explicitly pointed at this worktree, reported:

- `scope=all`: HIGH, 870 changed symbols, 206 indexed files, 13 processes.
- `scope=compare, base_ref=main`: CRITICAL, 4,591 changed symbols, 855 indexed
  files, 78 processes.

These are stale-index results and do not replace the blocked final refresh.
The all-scope result reconciles with the Phase 6 snapshot (869 / 205 / 13); the
single file/symbol increase is generated documentation metadata. The wider
`main` result reflects long-lived branch history.

Working-tree comparisons (excluding ignored `.gitnexus` data) report:

| Baseline | Paths | Short stat |
|---|---:|---|
| Darkmatter parent | 206 | 36,871 insertions, 964 deletions |
| More-is-more parent | 160 | 28,874 insertions, 1,968 deletions |
| Merge base | 348 | 65,529 insertions, 2,775 deletions |
| `main` | 924 | 131,844 insertions, 24,174 deletions |

The difference between 924 Git paths and 855 GitNexus files is non-indexed
documentation/assets. No path outside the frozen four-area scope introduced a
new runtime authority. Both parents' expected behavior is represented by the
focused evidence above.

## Repository and artifact audit

- Locked metadata succeeds with 72 workspace members.
- `Cargo.lock` remains untracked/ignored and byte-identical at SHA-256
  `52c2a58dc23331afe1cd82424ea5b295123ed4c1f40275fd35a400709d9ef286`.
- `git diff --check` passes.
- The only marker-shaped changed text is the reviewed equals-only Windows
  `route print` fixture in `sniff/lib/src/network/mod.rs`; no `<<<<<<<` or
  `>>>>>>>` text remains.
- The only snapshot delta is the reviewed Sniff OS-summary normalization pair;
  every host-dependent scalar is replaced by `<normalized>`, and the complete
  Sniff area gate passes.
- All external-only control artifacts remain outside the integration tree. The
  source Darkmatter plan is the sole authorized documentation delta requested
  for phase recovery; all other frozen dirty-file identities remain equal.
- Reviews 7 through 10 remain `ready: false` and explicitly retain the open
  quiet-host evidence requirement. Invalid-frontmatter Review 3 still states
  that timing and native Linux/Windows runtime evidence are absent.

## Completion audit

Specification criteria 1, 2, 4-6, 8-13, 16, and 17 pass. Criteria 3, 7, 14,
and 15 are
blocked or fail solely at final handoff state:

- criterion 3/7: six unmerged index entries remain because staging is forbidden;
- criterion 7: GitNexus counts remain stale after the bounded refresh failed;
- criterion 14: comparisons ran, but final refreshed-index reconciliation is
  unavailable;
- criterion 15: the index is intentionally incomplete and the working tree is
  intentionally not clean.

## Handoff

To complete Phase 7, a separately authorized operator must stage the reviewed
working-tree resolutions and evidence files, confirm `git ls-files -u` is
empty, rerun GitNexus with enough bounded resources/time to finish, update the
`CLAUDE.md` counts, rerun both change-detection scopes, and perform the final
cached/unstaged/status review. The merge must remain uncommitted until separate
commit authorization is given.

The final implementation-request replay (phase 8 of 8, corresponding to plan
Phase 7) reconfirmed 72 workspace members, the HIGH 870/206/13 all-scope result,
the CRITICAL 4,591/855/78 compare-main result, and a passing Darkmatter-area
lint gate. A duplicate full test replay passed 2,607 tests with zero failures
before the required non-interactive timeout; the completed Phase 6 area result
above remains the authoritative full test gate. No files were staged.
