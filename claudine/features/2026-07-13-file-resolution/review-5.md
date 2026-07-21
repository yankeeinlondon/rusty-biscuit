---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-21T03:33:03-07:00
spec: 2026-07-13-file-resolution/spec.md
implemented: true
description: A **feature** review of `2026-07-13-file-resolution/spec.md`
feature: 2026-07-13-file-resolution/review-5.md
previous: 2026-07-13-file-resolution/review-4.md
---

# Review 5: Unified File-Reference Resolution

## Verdict

The feature is **not ready for production**. Review 4's top-level source
re-anchoring, lexical proxy identity, context-containment, test-placement, and
primary Darkmatter reference-analysis findings are materially improved. The
three dedicated file-resolution captures and the formerly failing proxy-cycle
capture now pass in a real tmux terminal, and the new Darkmatter reference
tests preserve typed invalid-reference and I/O failures.

The migration is still incomplete at expression-evaluation boundaries.
Claudine lifecycle, loop, and sequence adapters, plus Darkmatter's reference
graph `when=` evaluator, construct a fresh expression `ResolutionContext` from
only a directory. Read-side file functions invoked there can therefore recapture
ambient HOME/environment and rediscover repository/package roots after the
request snapshot was established. Reference analysis also derives ordinary
contexts for every recursively visited source while the composition resolver
uses the trusted-external derivation, so an explicitly accepted external child
can compose successfully but fail graph/validation traversal. Finally, the
required full Claudine Level 1 gate is red because one unrelated CLI test timed
out through all retries and fail-fast left 809 tests unrun.

## Findings

### 1. High — Runtime expression adapters still bypass the request-scoped file-resolution snapshot

The main `ComposeOptions` path now carries `FileResolutionContext`, but several
Claudine-executed expression adapters discard it and instantiate Darkmatter's
lighter `ResolutionContext::new(base)` directly:

- lifecycle stack evaluation in
  `claudine/lib/src/composition/lifecycle/executor.rs:664-682`;
- loop conditions in
  `claudine/lib/src/composition/looping/expression.rs:131-138`;
- sequence source expressions in
  `claudine/lib/src/composition/sequence/expr.rs:78-80`;
- sequence shell preflight and task-value interpolation in
  `composition/sequence/preflight/mod.rs:739-747` and
  `composition/sequence/task/mod.rs:776-785`;
- reference-graph directive `when=` evaluation in
  `darkmatter/lib/src/markdown/reference/graph.rs:298-310`.

When no host `FileResolutionContext` is attached, Darkmatter's read-side file
functions build a new `biscuit_file::FileResolutionContext` at evaluation time.
That recaptures HOME/environment and may rediscover Git/package roots. A value
such as `file_exists('{{SNAPSHOT_ROOT}}/flag')`, a `~` reference, a configured
magic root, or `!package-file` can therefore disagree with body composition
after process state changes. The existing lifecycle test mutates CWD and proves
the explicit base directory is honored, but it does not mutate HOME/environment
or exercise configured magic/package roots; the graph `when=` tests likewise
cover only simple source-local paths.

This violates D2, D5, D10, D12 and Acceptance Criteria 6, 12, and 14. Its
appropriate verification level is Level 1 because the behavior is deterministic
in-process resolution; no terminal emulator is needed.

**Required change:** make the captured `FileResolutionContext` part of the
Claudine lifecycle/loop/sequence execution context and use one shared adapter
to derive Darkmatter expression contexts for the current source. Route the
reference graph's `when=` lookup through the same `ComposeOptions` builder.
Add ambient-mutation fixtures for environment interpolation, home, custom
magic roots, and package references across these adapters.

### 2. High — Reference analysis rejects accepted external nested documents that composition can traverse

The composition transclusion resolver deliberately derives a context with
`for_trusted_external_source` for a file-backed source already accepted by the
top-level resolver or a parent transclusion
(`darkmatter/lib/src/markdown/compose/transclusion/resolver.rs:131-135`). This
preserves the specification's non-goal that an explicitly authored `../` may
leave the repository and also supports documents accepted through home, magic,
or vault roots.

The newly migrated reference-analysis resolver instead always calls ordinary
`snapshot.for_source(source_path)`
(`darkmatter/lib/src/markdown/reference/mod.rs:68-86`). On the first external
target, resolution succeeds because the authoring context is still the parent
inside the repository. When graph traversal visits that target and resolves a
reference it authored, ordinary derivation validates the external target's
directory against the original repository and returns
`RepositoryRootNotContainingSource`. Composition and public
enumeration/graph/validation therefore disagree for the same accepted document
tree.

The new shared reference fixture covers an in-repository collision, ambient CWD
mutation, invalid syntax, and Unix permission I/O, but not a nested source
outside the request repository. This violates D2, D5, D12 and Acceptance
Criteria 6, 12, and 14. The missing evidence is Level 1.

**Required change:** carry provenance saying how each child source was accepted,
then select ordinary versus trusted-external derivation consistently in both
composition and reference analysis. Add one shared fixture in which an in-repo
root explicitly transcludes an external child and that child authors another
relative reference; assert composition, enumeration, graph construction, and
validation all reach the same grandchild. Include a configured magic/vault
external case so trusted derivation is policy-driven rather than unconditional.

### 3. Medium — Schema-root lookup retains a private prefix classifier and live ambient compatibility resolver

`darkmatter/lib/src/markdown/schemas/resolve.rs:304-321` decides whether a
schema reference is a bare name using path-separator checks and explicit
`starts_with` tests for `@`, `!`, `%`, and `vault:` before parsing through
`FileReference`. `try_bare_name_in_roots` then probes its synthesized `./name`
through `resolve_from` (`resolve.rs:334-357`). Although explicit-relative
probing makes the selected path deterministic, `resolve_from` still captures
live HOME and the process environment for every root.

This is one of the D12-inventoried schema callers and conflicts with D1's single
syntax authority and D12's no-late-ambient-read requirement. The behavior is
currently green because a synthesized `./name` does not consume the captured
values, but the architecture remains divergent and performs unnecessary
per-root snapshots.

**Required change:** parse once with `FileReference`, derive bare-name
eligibility from its public classification plus path shape, and probe each
schema root with an explicit context built from the existing request snapshot.
Keep the schema-root nearest-first policy, but do not route it through an
ambient compatibility API.

### 4. Medium — Comments still describe the removed document-first/launch-fallback behavior

Review 4 required stale contract narration to be removed, but material examples
remain. `claudine/lib/src/composition/lifecycle/executor.rs:664-671` says the
document base is primary and the launch area is an explicit fallback;
`composition/looping/expression.rs:98-102` makes the same claim; and
`claudine/cli/src/commands/sequence.rs:332-335` says file values resolve
document-first then launch-area. The implementation and specification now say
repository-first then source, with launch-area metadata diagnostic-only for
document-authored references.

These comments are especially misleading beside Finding 1 because they make a
remaining snapshot-plumbing omission look intentional. This violates
Acceptance Criterion 7 and the specification's Documentation and Migration
section.

**Required change:** rewrite the comments to distinguish top-level CLI values
that are genuinely launch-relative from references authored inside documents,
and state that `file_ref_fallback_dir` is diagnostic metadata where that is the
implemented contract.

### 5. Medium — The required Claudine Level 1 area gate did not complete

An isolated `claudine/just test` run passed the catalog-types and 3,837-test
library suites, then timed out in
`claudine-cli::wrap_compose_agent::direct_wrap_dry_run_delivers_prompt_for_every_provider`
on all four attempts. Nextest stopped after 1,354 of 2,163 CLI tests, leaving
809 unrun. The failure is not evidently file-resolution-related, and the
feature-focused tests that ran were green, but Acceptance Criterion 10 requires
the complete area gate to pass before production readiness.

**Required change:** make the timeout test reliable or establish and document
that it is an infrastructure failure, then rerun the full Claudine Level 1 gate
to completion. A targeted pass is not a substitute for the required area gate.

## Requirement Verification Levels

| User-facing or contract requirement | Strongest verification present | Assessment |
|---|---|---|
| Shared parsing, explicit/implicit precedence, effective interpolation kind, candidate provenance, fallible probing, home, package, and recursive behavior | Level 1 `biscuit-file` unit/integration tests | Appropriate and green. Context derivation now preserves the original request boundary and has explicit trusted-external APIs. |
| Top-level compose, inline-compose, and sequence re-anchor to the resolved source repository | Level 1 subprocess integration | Appropriate and green for the new cross-repository fixtures. |
| A completion value executes unchanged through the shared candidate builder | Level 1 subprocess integration | Appropriate and green. |
| Lifecycle, loop, sequence, and reference-graph read-side file expressions reuse the immutable request snapshot | Partial Level 1 tests using source-local paths/CWD mutation | **Gap:** the actual adapters still discard the snapshot; no HOME/environment/custom-magic/package mutation matrix exists (Finding 1). |
| Nested external documents retain their own authoring source consistently across composition and reference analysis | No discriminating Level 1 fixture | **Gap:** composition and reference analysis select different context derivations (Finding 2). |
| Bare motivating reference succeeds, explicit source-relative reference fails, and a no-match lists repository then source candidates | Level 2 tmux captures | Appropriate and green: all three dedicated captures passed. |
| Proxy routes share typed identity and target-initialize A→B→A cycles surface to the user | Level 1 lexical identity test plus Level 2 tmux lifecycle captures | Appropriate and green; Review 4's red cycle test is restored. |
| Exact candidate ordering and typed error presentation render correctly in a real terminal | Level 2 tmux capture | Appropriate and green. |
| macOS/Linux/Windows path classification and home semantics | Host-independent/target-gated Level 1 tests; macOS runtime execution | Appropriate for deterministic path semantics; native Windows execution was unavailable on this host. |
| Required package gates | Level 1 area tests, lint, and Claudine Level 2 | Not satisfied: `biscuit-file` and `darkmatter` L1 are green, all lint gates and Claudine L2 are green, but the full Claudine L1 CLI tier did not complete (Finding 5). |

Level 3 is not applicable. The feature claims no OS keyboard/mouse event,
paste, IME, hotkey, or terminal input-encoder behavior.

## Verification Performed

- Read the specification, Review 4, migration inventory, current implementation
  diffs, shared resolver/context code, Claudine orchestration adapters,
  Darkmatter reference/schema/expression surfaces, and relevant tests.
- Used GitNexus against the explicit worktree index to trace composition
  capture and the remaining reference-analysis call graph. The generic
  repository resource pointed at a sibling branch, so it was not used as
  implementation evidence.
- `biscuit-file/just test` passed: 377 library/integration tests selected with
  4 configured skips; 61 CLI tests passed.
- `darkmatter/just test` passed: the 5,650-test library target, 555 CLI tests,
  and 566 DMLS tests completed green with their configured skips.
- `claudine/just test` failed as described in Finding 5. The 21 catalog-types
  tests and the 3,837-test library target passed before the CLI timeout.
- `biscuit-file/just lint`, `darkmatter/just lint`, and `claudine/just lint`
  passed. Claudine's 18 error-guard tests also passed.
- `claudine/just test-l2` passed: 148/148 real-terminal tests, including the
  dedicated file-resolution captures, proxy route identity, and proxy-cycle
  regression.
- `git diff --check` passed. No formatting or Git commit was performed.
  Existing unrelated worktree changes were preserved.

## Production Readiness Closure

Production readiness requires Findings 1 and 2 to close with discriminating
Level 1 fixtures, schema-root lookup to stop using ambient resolution, stale
contract comments to be corrected, and the complete Claudine Level 1 gate to
pass. The existing Level 2 coverage is appropriate and green; no additional
Level 3 testing is required.
