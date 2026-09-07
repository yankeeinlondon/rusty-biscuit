---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-08-27T11:04:56+01:00
spec: 2026-08-26-finalized-references/spec.md
log: claudine/features/2026-08-26-finalized-references/log.md
implemented: true
implemented_by: codex/default
description: A **feature** review of `2026-08-26-finalized-references/spec.md`
feature: 2026-08-26-finalized-references/review-1.md
---

# Review 1: Finalized References

## Verdict

The feature is **not ready for production**.

The implementation now appears functionally aligned with the finalized grammar on
the macOS host. In particular, the post-review fix correctly rejects bare `C:` as
a scheme-shaped Windows drive-relative reference, and focused Level 1 verification
passes. Production readiness is still blocked by the specification's uncompleted
cross-platform gate: the final tree has not completed the affected package-area
suites on Linux, WSL Claudine, or native Windows, and the Windows-only junction /
reparse-point containment test has not run. Several active public docs and source
comments also still publish the removed `!` grammar and the superseded
repository-first ordering.

## Findings

### 1. High — The final cross-platform validation contract has not been executed

DECISION: Treat this as a non-blocker! The normal local OS build environments are 
currently under storage strain and can't be used productively. For that reason, we 
will rely on cicd to tease out cross-platform issues.

AC9 and AC10 require the final tree to pass `just test`, `just test-l2`, and
`just lint` in `biscuit-file/`, `darkmatter/`, and `claudine/` on macOS,
native Linux, WSL, and native Windows. The committed acceptance matrix records:

- no package-area gate completed on `build-linux`;
- biscuit-file and Darkmatter passed on WSL, but Claudine failed during
  compilation with `No space left on device`;
- no native-Windows gate ran because the capacity preflight failed; and
- `repository_containment_rejects_an_external_junction`, the native-Windows
  evidence for junction/reparse-point containment, has not executed.

The latest bare-`C:` regression was run on macOS after the earlier WSL grammar
run, so the final parser tree also lacks a completed non-macOS run. These are
environment failures rather than demonstrated implementation failures, but they
leave explicit acceptance criteria unsatisfied. Given the feature changes path
classification, containment, process environments, and public enums across three
package areas, macOS-only success cannot establish Windows and Linux production
readiness.

Required change: run the exact final tree on healthy Linux, WSL, and native-Windows
builders, including the native junction fixture, and record the commit/tree identity
and commands. All required rows must be green before setting `ready: true`.

Verification level: OS-specific parser and filesystem tests are Level 1, which is
the correct tier; the gap is that the required native legs did not execute. The
compose, proxy, sequence, and completion behaviors have appropriate Level 2 tmux
coverage on macOS. Level 3 is not applicable because the feature makes no claim
about OS keyboard, mouse, paste, IME, or terminal input encoding.

### 2. Medium — Active documentation still describes the retired grammar

The feature's documentation drift pass is incomplete. Current, non-historical
authorities and source comments still state that `!` is a package reference,
that implicit paths are repository-first, or that completion supports only the
old `@`/implicit root model. Examples include:

- `biscuit-file/lib/src/file_reference/mod.rs:533-539` — public `resolve_from`
  rustdoc names `!` and omits `&`/`^`;
- `biscuit-file/docs/tech-spec/file-reference-struct.md:427-443` — the active
  technical specification requires `!` package-area tests and the old `@` order;
- `darkmatter/lib/src/markdown/compose/transclusion/resolver.rs:70-78` — rustdoc
  advertises `!` as supported;
- `claudine/docs/topics/composition.md:86-96,125-128` — completion omits direct
  `&`/`^`, and compose is documented as repository-first with `!` support;
- `claudine/docs/topics/execution-flow.md:147-153`,
  `claudine/docs/topics/lifecycle.md:196-201`, and
  `claudine/docs/topics/flow-control/sequences.md:169-172` — active user docs
  repeat the removed grammar or obsolete candidate order; and
- `claudine/lib/tests/boundary_lint.rs:203-206` and
  `claudine/cli/tests/sequence_sources_cli.rs:11-13` — test documentation now
  contradicts the behavior it guards.

This violates the repository's behavior-change documentation discipline and
AC12's public-contract goal. A caller following these docs can select the wrong
reference spelling or reason incorrectly about collision precedence.

Required change: sweep active biscuit-file, Darkmatter, and Claudine rustdoc,
topic docs, technical specs, test comments, and completion docs for `!`,
`repository-first`, and old magic-root wording. Update them to D1/D3/D6/D8,
while leaving historical completed feature records unchanged. Refresh any
frontmatter hashes through Darkmatter where applicable.

Verification level: source/document review is sufficient; no runtime tier applies.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1 — grammar, candidate order, misses, remote typing, typed errors | Level 1 parser and resolution integration tests | Appropriate; focused final-tree bare-`C:` regression passes on macOS. |
| AC2 — CWD-first implicit resolution | Level 1 conflict matrices plus Level 2 tmux compose/proxy capture | Appropriate on macOS. |
| AC3 — removed `!` sigil and enum/provenance migration | Level 1 parser, exhaustive compile, and source-inventory tests | Runtime contract is covered; active docs still contradict it (Finding 2). |
| AC4 — reference-owned scope derivation and no ambient discovery | Level 1 topology, work-counter, parity, and second-repository tests; Level 2 compose | Appropriate; native matrix incomplete (Finding 1). |
| AC5 — caller materialization and provenance across routes | Level 1 schema/orchestration matrices plus Level 2 proxy and sequence capture | Appropriate. |
| AC6 — `ctx.cwd` and `AGENT_CWD` | Level 1 process integration and non-vacuous spawn-inventory tests | Appropriate; no terminal encoder behavior is involved. |
| AC7 — effective magic conventions | Level 1 collision/deduplication tests plus Level 2 completion execution | Appropriate, but documentation is stale (Finding 2). |
| AC8 — completion/execution parity | Level 1 root/round-trip tests plus Level 2 tmux completion-to-compose execution | Appropriate. |
| AC9 — cross-platform parsing and containment | Host-independent Level 1 fixtures on macOS; native-Windows junction fixture exists but has not run | **Gap:** required native evidence is absent (Finding 1). |
| AC10 — package-area validation matrix | macOS L1/L2/lint green; WSL partial; Linux and native Windows incomplete | **Gap:** explicit acceptance criterion is unsatisfied (Finding 1). |
| AC11 — repository containment | Level 1 lexical, symlink, deepest-ancestor, completion, and platform-gated junction tests | Correct tier; native-Windows junction evidence is missing. |
| AC12 — passive and public contracts | Level 1 passive-validation, corpus, exhaustiveness, and CLI integration tests | Runtime coverage is appropriate; public documentation remains inconsistent (Finding 2). |
| AC13 — ratification/document alignment | Level 1 grammar diagnostics and design-document review | Parser behavior is aligned after the bare-`C:` fix. |
| Keyboard, mouse, paste, IME, or hotkey behavior | No Level 3 test | Not applicable; no such behavior is specified. |

## Verification Performed

- Read the complete specification, acceptance matrix, consumer audit, current
  review target, implementation commits, and principal implementation/test surfaces.
- Used GitNexus concept search and compare-scope change detection. The implementation
  touches 678 indexed symbols across 157 files and affects 25 execution flows;
  GitNexus rates that review range `CRITICAL`, consistent with requiring the full
  cross-platform closure rather than inferring safety from one host.
- Ran the focused Level 1 Nextest regression
  `reserved_schemes_and_windows_device_prefixes_are_rejected`: 1 passed.
- Confirmed the finalized real-terminal tests use `TmuxHarness` for compose,
  proxy materialization, sequence task parameters, and completion-to-execution parity.
- Did not rerun the already-recorded full macOS L1/L2/lint gates or attempt the
  known-unhealthy remote builders during this review.
- No implementation code, formatting, or Git commit was performed.

## Production Readiness Closure

Production readiness requires a green final-tree Linux/WSL/native-Windows matrix,
including the native junction test, and correction of the active documentation drift.
No additional Level 3 test is required.
