---
$schema: feature-review.yaml
ready: false
agent: codex/gpt-5.6-sol
created: 2026-09-07T20:58:51-07:00
spec: 2026-08-26-finalized-references/spec.md
implemented: true
implemented_by: claude/default
next: 2026-08-26-finalized-references/review-3.md
log: claudine/features/2026-08-26-finalized-references/log.md
description: A **feature** review of `2026-08-26-finalized-references/spec.md`
feature: 2026-08-26-finalized-references/review-2.md
previous: 2026-08-26-finalized-references/review-1.md
---

# Review 2: Finalized References

## Verdict

The implementation is not ready for production. The finalized grammar and most resolver behavior are implemented and exercise a broad set of platform-neutral and platform-specific cases, but two acceptance blockers remain: the subprocess spawn inventory can produce false negatives, and AC10's required final native-platform verification is neither complete nor green in the durable feature evidence. One active Darkmatter CLI page also still describes an incomplete version of the public grammar.

## Findings

### High — The spawn inventory guard does not prove that every executable child is governed

AC6 requires an inventory guard that fails for any production `Command` construction whose child can execute without the shared environment helper. The scanner in `claudine/cli/tests/spawn_inventory.rs` lines 153–228 counts command constructors and helper calls per function, then treats every constructor as governed when the helper-call count is at least the constructor count or when a single helper call follows all constructors.

That heuristic does not associate a helper call with a command binding, an execution call, or a control-flow path. It therefore accepts counterexamples such as:

```rust
let mut safe = Command::new("safe");
let mut leaked = Command::new("leaked");
contribute_child_environment(&mut safe);
contribute_child_environment(&mut safe);
leaked.output();
```

It also accepts a helper call made after `spawn`, `status`, or `output`. The existing scanner fixtures cover the simple one-constructor/one-helper case but not these adversarial cases. The current production call sites may be correctly governed, but the acceptance criterion explicitly requires a guard that preserves the invariant as the code evolves.

Required change: make the inventory analysis binding- and execution-aware, proving that the same command receives the helper before every terminal execution on every reachable path. A factory or wrapper type that cannot execute until context is applied would be an even stronger design. Add fixtures for duplicate helper calls on one of two commands, helper-after-execution, wrong receivers, and divergent branches.

Verification level: Level 1 is appropriate for this source-inventory invariant; the gap is semantic coverage, not test tier.

### High — The final cross-platform acceptance matrix is incomplete and includes failed legs

AC10 requires `just test`, `just test-l2`, and `just lint` for all three package areas on macOS, native Linux, WSL, and native Windows before hosted CI. The latest implementation log records meaningful progress, including native Windows junction execution and numerous green Ubuntu, macOS, Windows, and WSL jobs. However, its final referenced run was still in progress and included a failed Claudine CLI Windows wall-clock test and a failed macOS Level 2 timeout, while the Darkmatter WSL result was not final.

The durable acceptance matrix and plan remain open for Linux, Windows, and parts of WSL. A failure believed to be unrelated or flaky can be triaged separately, but it does not make the required final tree green. The earlier decision to treat remote execution as a non-blocker for implementation work does not satisfy the specification's production-readiness gate.

Required change: run the exact final tree through every AC10 platform/tier/lint leg, resolve or formally exclude failures through an authorized specification change, and record a complete green matrix in the feature artifacts.

Verification level: Level 1 and Level 2 are both required by AC10. Level 3 is not applicable because the feature has no OS-keyboard-input requirement.

### Medium — An active Darkmatter CLI page still publishes an incomplete grammar

`darkmatter/docs/cli/index.md` lines 46–53 describes file references as relative paths, absolute paths, and `@` magic paths for repository, package, and home locations. That omits `&`, `^`, package-area roots, registered prepend/append forms, and the finalized resolution order. This is active public documentation, not a historical record, so it conflicts with AC12 and the finalized public contract.

Required change: replace the abbreviated list with the complete finalized catalog and precedence, or link prominently to the canonical `biscuit-file` reference documentation while accurately summarizing the introducers here. The dated historical timeline entry can remain historical.

Verification level: documentation review is appropriate; no terminal test level applies.

## Requirement Verification Matrix

| Requirement | Strongest evidence reviewed | Assessment |
|---|---|---|
| AC1 — grammar and parser contract | Level 1 parser/resolver tests | Appropriate and passing locally. |
| AC2 — implicit reference precedence | Level 1 collision tests; recorded Level 2 compose/proxy checks | Appropriate. |
| AC3 — remove legacy `!` grammar | Level 1 grammar, compiler, and inventory tests | Appropriate. |
| AC4 — reference-owned scopes and topology | Level 1 catalog/topology/no-discovery tests; recorded Level 2 nested compose checks | Appropriate. |
| AC5 — `ctx.cwd` and materialization | Level 1 schema/orchestration tests; recorded Level 2 proxy/sequence checks | Appropriate. |
| AC6 — subprocess context propagation | Level 1 subprocess tests and source inventory | Correct tier, but the inventory guard is incomplete; see the first finding. |
| AC7 — candidate aggregation and deduplication | Level 1 collision/deduplication tests; recorded Level 2 compose checks | Appropriate. |
| AC8 — completion/execution parity | Level 1 completion and execution tests; recorded Level 2 magic-reference checks | Appropriate. No encoder-sensitive input requirement exists. |
| AC9 — cross-platform syntax | Level 1 platform-specific parser/filesystem tests | Correct tier, but final native-platform closure is incomplete. |
| AC10 — final quality gates | Local Level 1 green; recorded macOS Level 2/lint green; incomplete/red remote matrix | Not satisfied; see the second finding. |
| AC11 — containment and junction safety | Level 1 lexical, symlink, deepest-ancestor, completion, and Windows-junction tests | Appropriate; native Windows execution is reported, but belongs in a completed final matrix. |
| AC12 — documentation and corpus migration | Level 1 passive/public/corpus checks plus manual documentation review | Runtime checks are appropriate; active documentation drift remains. |
| AC13 — reserved syntax | Level 1 grammar tests and design review | Appropriate. |

No acceptance requirement concerns modifier visibility, hotkey encoding, paste, IME, mouse input, or another behavior that needs Level 3 OS keyboard/mouse injection.

## Verification Performed

- `biscuit-file`: `just test` passed 813 tests, plus 6 no-default-features tests.
- `darkmatter`: `just test` passed 7,709 tests; 51 higher-tier tests were skipped by their gates.
- `claudine`: `just test` passed 6,840 tests; 11 higher-tier tests were skipped by their gates.
- The implementation log records green local lint runs for all three package areas and prior macOS Level 2 runs. Those longer suites were not repeated for this documentation-only review update.
- `git diff --check` was used to check the working changes.

The passing Level 1 suites establish local behavior but do not override the AC6 semantic gap or replace AC10's explicitly required native-platform and Level 2 matrix.
