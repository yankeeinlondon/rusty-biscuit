---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-21T21:27:57-07:00
spec: 2026-07-20-claudine-mega-merge/spec.md
implemented: false
description: A **fix** review of `2026-07-20-claudine-mega-merge/spec.md`
fix: 2026-07-20-claudine-mega-merge/review-1.md
previous: /
---

# Review 1: Claudine Mega Merge

## Verdict

The fix is **not ready for production**. The reviewed checkout contains the
integration specification and execution plan, but it does not contain the
mega-merge implementation. Neither frozen feature tip is in the ancestry of
the current `claudine` revision, the required Phase 0 evidence artifacts do not
exist, every plan task remains unchecked, and the known seed merge markers are
still present. Consequently none of the specification's merged-tree L1, L2,
L3, or native Windows acceptance obligations has implementation evidence.

## Findings

### 1. Critical: neither feature implementation has been merged

The current revision is `80e51c384c0ab969099c3d3e88804dbd42fa1158`.
Ancestry checks show that neither frozen input required by the specification is
an ancestor of this revision:

- `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
  (`error-prop-and-file-resolution`) has 243 commits not present in `HEAD`.
- `e348486c810969abe87a6b7209979034f5454b07` (`proxy-with`) has 123 commits
  not present in `HEAD`.

The named integration branch
`claudine-mega-merge-integration-20260721-phase1` is also behind the reviewed
checkout rather than containing either merge. Representative required types
such as `DiagnosticSnapshot` and `FileResolutionContext` occur only in specs
and merge records in the current tree, not in production source. A tree diff
against the foundation tip changes 457 files; a diff against the proxy tip
changes 169 files. This is not a partially implemented seam that can be
reviewed or repaired surgically—the implementation under review is absent.

This fails the first completion condition at `spec.md:709-712` and leaves every
architectural invariant I1–I12 unimplemented in the reviewed tree.

**Required change:** execute the reviewed merge plan on the integration branch,
preserve both feature histories through the required merge commits, resolve the
combined ownership boundaries, and fast-forward `claudine` only after the
merged candidate satisfies the complete acceptance ledger. Request another
review against that immutable candidate SHA.

### 2. High: Phase 0 has not produced its required safety and acceptance artifacts

The execution plan contains 176 unchecked tasks and no checked tasks. The
required `acceptance-ledger.md`, `sha-ledger.md`, `conflict-checklist.md`, and
marker-baseline artifact are absent from the fix directory. The current
worktree also still contains the unclassified untracked
`.claude/settings.local.json` noted by `git status`.

The specification requires the acceptance ledger before implementation edits
and makes its existence an explicit Phase 0 exit condition
(`spec.md:397-419`). Without it there is no stable mapping from the four source
specifications and 12 combined seam cases to a merged test, tier, platform,
owner, status, candidate SHA, and evidence location. It is therefore
impossible to distinguish missing coverage from a test that exists but was
blocked or skipped.

**Required change:** complete Phase 0 before merging code. Freeze and record
the actual execution SHAs, classify the dirty path without overwriting it,
capture branch baselines, produce the conflict and impact records, and create
the full acceptance ledger with every row initially open. Do not credit a
feature-branch result as evidence for the later merged candidate.

### 3. High: every required user-observable verification tier is missing on merged code

The specification assigns explicit evidence levels at `spec.md:598-604` and
`spec.md:662-670`. No acceptance candidate or merged-code test result exists,
so the strongest verification present for each user-observable contract is
none. The existing source-branch logs are baseline context only; the spec
explicitly prohibits treating them as merged acceptance evidence.

| User-observable requirement | Required verification | Strongest merged-tree verification present | Assessment |
|---|---|---|---|
| Typed diagnostic detail is identical in terminal output, lifecycle `err.*`, and snapshots; failures render exactly once in plain, color, and OSC 8 modes | L1 semantic/serialization tests plus L2 real-terminal captures | None | **Gap:** no merged implementation or capture exists. |
| Bare, explicit-relative, nested, direct, proxy, sequence, completion, and Darkmatter references share one request-scoped resolution model | L1 grammar/context tests plus L2 direct/proxy/sequence CLI evidence; native Unix and Windows path cases | None | **Gap:** the required production context is absent. |
| Sequence Plus task frames retain attribution, ordering, styling, idle flush, close behavior, and process teardown | L1 state/process tests plus L2 task-stream captures | None | **Gap:** no merged task-stream evidence exists. |
| Physical interruption and console-control behavior terminate the correct process tree | Guarded L3 OS keyboard injection on each claimed native platform | None | **Gap:** Level 1 or injected terminal bytes would not satisfy this physical-key requirement. |
| Proxy handoffs, `proxy.with`, retry/resume refresh, launch rebuild, schema ordering, overlay redaction, and dry-run behavior match direct execution | L1 coordinator/overlay/launch tests plus full L2 real-CLI proxy and diagnostic matrices | None | **Gap:** no merged proxy path exists. |
| Parallel proxy-in-sequence failures preserve task attribution and stdout/stderr ordering through a real terminal | L2 real-terminal capture, with L1 state/teardown support | None | **Gap:** mandatory seam case MM-S10 is only an unchecked plan row. |
| Windows drive, UNC, HOME, named-pipe, console-control, terminal, and descendant-termination behavior works natively | Native Windows L1/L2/L3 as applicable; cross-check remains compile-only | None | **Gap:** no candidate SHA or native Windows evidence is attached. |
| The 12 mandatory cross-feature seam cases behave as one execution model | Assigned L1/L2/L3 tests according to each observable boundary | None | **Gap:** all 12 remain unchecked plan entries, not tests with results. |

Per the review's rigor rules, each mismatch is production-blocking even if an
isolated feature branch previously passed its own tests. At minimum, all
required L1 and L2 rows must reach their assertions and pass, and the specified
L3/native Windows rows must have native evidence before readiness can be true.

### 4. High: the known seed merge markers still survive

`CLAUDE.md:121-122` still contains literal `<<<<<<< HEAD` and
`||||||| 4a7e2f8c6` lines; `AGENTS.md` exposes the same content through its
symlink. The specification identifies these exact lines as seed debt and
requires the first merge resolution to remove them (`spec.md:387-390`). It also
requires a clean final marker scan (`spec.md:721-722`).

Their continued presence is additional evidence that the integration has not
passed Phase 1. It also leaves the repository's active agent instructions in a
syntactically conflicted state.

**Required change:** record these lines in the Phase 0 marker baseline, remove
them while resolving the first merge's `CLAUDE.md`, and retain before/after
worktree and staged-blob scans as evidence. Classify any other marker-shaped
fixture separately rather than weakening the scan.

## Ergonomics and Performance

There is no merged Rust implementation to assess for ergonomics or
performance. Making API or optimization recommendations against either source
branch alone would risk endorsing one side of an unresolved architectural
conflict. The next review should evaluate these qualities only after the
coordinator, preparation service, request context, launch bundle, and sequence
containment have one final owner in the merged candidate.

## Verification Performed

- Resolved the requested `@` references with Biscuit File's file-reference
  resolver.
- Inspected the specification, execution plan, package architecture/test
  placement guidance, branch ancestry, current source tree, and fix artifacts.
- Confirmed both frozen feature-tip ancestry checks fail against `HEAD`.
- Counted 243 foundation-tip and 123 proxy-tip commits absent from `HEAD`.
- Confirmed all 176 plan tasks are unchecked and the Phase 0 evidence files are
  absent.
- Confirmed the known marker lines remain in `CLAUDE.md`/`AGENTS.md`.

No test suite was run because there is no merged implementation or acceptance
candidate to exercise. Running the seed's existing suites would establish only
pre-merge baseline health, which `spec.md:724-725` explicitly says cannot
satisfy completion.

## Closure Criteria

Complete all six plan phases, then request another review against the recorded
acceptance-candidate SHA. The next review needs the two merge commits in
ancestry, one coherent implementation for I1–I12, the complete acceptance
ledger, passing required L1/L2 assertions, guarded L3 evidence, native Linux
and Windows evidence, clean lint/generator/drift/source-scan gates, and a clean
marker audit. Until then, this fix remains a reviewed plan rather than a
production candidate.
