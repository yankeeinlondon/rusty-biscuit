---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-20-lifecycle-handoff-gaps/spec.md`
fix: 2026-09-20-lifecycle-handoff-gaps/review-1.md
spec: 2026-09-20-lifecycle-handoff-gaps/spec.md
reviewed_by: codex/default
created: 2026-09-28T08:30:59-07:00
implemented: false
ready: false
findings:
    - "high: Composition runs reuse stale or incomplete context evidence"
    - "high: Terminal handoffs do not reach their document owners"
    - "high: Preflight and execution disagree on overlaid partial commands"
    - "high: Lifecycle failure downgrades do not determine the final outcome"
    - "high: Loop state and loop implementations disagree with the contract"
    - "high: Shell result values and lifecycle assignments are missing"
    - "high: Literal and authored-text rendering still changes prompt text"
    - "medium: The prompt guide and PR flow still carry the temporary behavior"
    - "medium: Required behavior has no matching acceptance tests or performance evidence"
human_review: false
recurrence: false
---

# Review 1: Lifecycle Handoff Gaps

## Assessment

**Not production ready.** This specification still declares `status: draft-spec`, `implemented: false`, and `fixed: []`. Source inspection confirms that most of its requested changes have not landed. One part of sequence proxy handling exists for `initialize`, but does not cover the reported `start` case. The spec's reported CLI reproductions are useful baseline evidence; this review did not independently rerun them. There is no new implementation to validate, so the findings below identify the complete remaining work and the sibling paths that each fix must cover.

## Findings

### Composition runs reuse stale or incomplete context evidence (high)

**Defect class:** Claudine retains changing repository observations beyond one composition run, while discovery does not reliably collect context requests from every executable file in that run.

[`InvocationContext::project_evidence`](../../lib/src/invocation_context.rs) still reads `file_changes` through a `OnceLock` on an invocation-owned repository entry. A later run therefore receives the first observation. The spec's same staged-file change is the reproduction for each vehicle below. An unedited first run reports zero, establishing the control; staging between runs must make the second run report one. Stable `ctx.cwd` and `ctx.repo_root` must remain identical across the same pair.

| Site / shape checked | Observed baseline | Required result |
| --- | --- | --- |
| Direct run, no intervening mutation | First observation is reused within the run; clean | One stable observation for that run |
| Proxy target, source mentioned `ctx.staged_files` | Spec reports `0` after staging | `1` in the target |
| Proxy target, source did not mention the property | Spec reports `1`; first mention changes the answer | `1`, independent of first mention |
| Later sequence step | Spec reports `0` after a shell step stages the file | `1` |
| Later loop iteration | Spec reports `0` after the prior iteration stages the file | `1` |
| Retry or resume attempt | Spec reports `0` after the prior attempt stages the file | `1` |
| Serial group task | Uses the same invocation cache | Fresh observation per task |
| Parallel group siblings | No group-wide capture and re-entry contract is established | Shared initial observation, then fresh evidence for a sibling that re-enters |
| Root-only `ctx` reference | Already discovers the request; clean | Continue to work |
| First `ctx` reference in a transcluded partial | Spec reports empty output | Capture the required property and share it with the parent |
| Nested or interpolated `::file` reference | No acceptance test for recursive discovery | Resolve the same property set without executing side effects during discovery |

Cover branch/worktree changes as well as staged files. The context capture tests must distinguish volatile requests from stable identity and host discovery, and cover the staged initialization rule: scan the root before `initialize`, then extend from an include without refreshing an already captured group. These are deterministic prompt results, so Level 1 fake-provider and direct API tests are appropriate.

### Terminal handoffs do not reach their document owners (high)

**Defect class:** A lifecycle proxy can be accepted in a lower layer without being carried to the loop or sequence coordinator that must adopt the target.

[`build_loop_iteration_output`](../../cli/src/commands/compose/loop_run.rs) builds success and failure outputs without copying a surfaced handoff. The loop engine has a handoff branch, but this builder never supplies it. In a sequence, [`run_step_proxy_loop`](../../cli/src/commands/wrap/sequence/iterate.rs) exists, yet its caller enters it only for `outcome.initialize_handoff`. Sweep the same two-document fixture, changing only the source event or owner:

| Site / shape checked | Observed baseline or source path | Required result |
| --- | --- | --- |
| Loop source, `initialize` proxy | Existing loop test covers adoption; clean | Target starts, source runs zero iterations |
| Loop source, `success` proxy | Spec reports another source iteration and a false cycle | Target starts; no next source iteration |
| Loop source, `failure` proxy | Same output builder drops terminal handoff | Target starts; no next source iteration |
| Loop source, `finalize` proxy | Same output builder drops terminal handoff | Target starts; `finalize` does not repeat |
| Sequence task, `initialize` proxy | Current sequence code follows it; partial implementation | Keep target within the same step |
| Sequence task, `start` proxy | Spec reports no owning coordinator; caller checks only `initialize_handoff` | Target runs and step completes once |
| Sequence task, terminal proxy | Same narrow caller check | Target runs; final target owns the step result |
| Direct provider wrapper | Its refusal is intentional; clean | Continue to refuse, since it has no document coordinator |

Also assert that refused resolution, overlay evaluation, cycle, and hop-limit checks leave the chain untouched; one accepted handoff adds exactly one entry even if its target later fails. Sequence output publication, setup, teardown, and `fail_fast` behavior need process-level tests. None of these requirements needs a real terminal: Level 1 fake-provider tests can observe the chain, output, and exit status.

### Preflight and execution disagree on overlaid partial commands (high)

**Defect class:** The preflight command set does not use the same effective overlay and file-local state as execution of a transcluded shell span.

The spec's `router.md` → `target.md` → `part.md` fixture reports a `NotPreApproved` warning for the command interpolated from `proxy.with`. [`preflight`](../../lib/src/composition/preflight.rs) and the target preparation paths need one resolver for the approved bytes and the executed bytes. Use the same real fixture and change only where `base` comes from:

| Site / shape checked | Observed baseline | Required result |
| --- | --- | --- |
| Command inline in target; proxy overlay | Approved; clean | Approved |
| Command in partial; caller value | Approved; clean | Approved |
| Command in partial; no interpolation | Approved; clean | Approved |
| Command in partial; proxy overlay | `NotPreApproved` warning; command does not supply its fact | Approved exact bytes, or a hard preparation failure |
| Command in partial; direct invocation and caller value | Approved; clean | Approved |
| Nested/conditional partial, local `set.*`, or an untaken branch | No matching acceptance case | Discovery follows executable-span and state precedence rules without executing a branch |

`NotPreApproved` is a broken invariant; `when_error` and lifecycle `no_error` must not turn it into missing data. A Level 1 public composition or CLI test should compare the approval set with the command actually launched for each row.

### Lifecycle failure downgrades do not determine the final outcome (high)

**Defect class:** An explicit lifecycle `error` can route failure events while the owning command still returns a successful result.

The spec's stub-provider fixture observes `failure` and `finalize` after an `error` in `success`, then exit code zero and no rendered diagnostic. Sweep the same action through each owner:

| Site / shape checked | Observed baseline or coverage | Required result |
| --- | --- | --- |
| `start` error | Nonzero exit and diagnostic; clean | Preserve |
| `success` error, first attempt | Zero exit and no diagnostic | Nonzero exit and one canonical diagnostic |
| `success` error after retry/resume | No matching process-level acceptance case | Eventual unrecovered error fails; successful recovery succeeds |
| `finalize` error | Nonzero exit and diagnostic; clean | Preserve |
| Sequence step under `fail_fast: true` or `false` | No matching acceptance case for downgraded success | Step failure follows its chosen policy |
| Loop iteration | No matching acceptance case for downgraded success | Failed iteration follows loop policy |

Use the existing effective-diagnostic path in [`loop_control`](../../cli/src/commands/wrap/harness_orch/loop_control.rs) for terminal text and machine output. Level 1 CLI tests must assert both exit status and exactly one rendered error, including a recovery attempt; checking only that failure hooks fired would miss this defect.

### Loop state and loop implementations disagree with the contract (high)

**Defect class:** Loop control is still split between a prechecked engine and a postchecked engine, and the gate's lifecycle context lacks the ambient values used by its condition.

[`execute_loop`](../../lib/src/composition/looping/engine.rs) and `execute_loop_with_config` remain public beside `execute_loop_with_lifecycle`; [`composition`](../../lib/src/composition/mod.rs) still exports both. Tests in `iteration_actions.rs`, `rate_limits.rs`, and `seed_state.rs` still call the old path. In [`run_loop_gate`](../../lib/src/composition/looping/engine.rs), the `ambient` parameter reaches the condition but is absent from the `StackExecutionContext` passed to loop notifications and actions.

| Site / shape checked | Observed baseline | Required result |
| --- | --- | --- |
| Postchecked production loop, condition after body | Existing engine; clean | Keep as the sole engine |
| Prechecked public engine and its three test modules | Still present | Remove engine; port and rederive iteration-count assertions |
| Gate condition reading `_loop_count` | Receives loop ambient; clean | Preserve |
| Gate `info`, `warn`, `message`, and stack reading any `_loop_*` value | Spec reports unknown root | Same just-finished iteration value as the condition |

The existing `lifecycle.md` example must run as written through a Level 1 CLI test. No physical key input or real terminal is involved.

### Shell result values and lifecycle assignments are missing (high)

**Defect class:** Frontmatter shell expressions expose only text, and lifecycle `set` does not execute a whole-value shell expression when its action runs.

[`FrontmatterShellSuffix`](../../../darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs) recognizes only `::timeout` and `::no-cache`; [`lifecycle executor`](../../lib/src/composition/lifecycle/executor.rs) still treats authored `$(…)` data as verbatim. Sweep one executable fixture through every expression shape and suffix, then through each lifecycle boundary:

| Site / shape checked | Observed source state | Required result |
| --- | --- | --- |
| Single command: `::ok`, `::exit-code`, `::result` | No suffix variant or result type | Typed boolean, number, or `{ok, code, stdout, stderr}` |
| `&&` / `\|\|` chain, including fallback | No result suffix | Last executed status and joined executed streams |
| Ternary selecting command, chain, or literal | No result suffix | Selected outcome; literal has status zero |
| Timeout, `::no-cache`, both suffix orders, concurrent duplicate | No result-aware cache | Policy-specific value and one execution for equivalent cached requests |
| Top-level frontmatter | Only unsuffixed stdout is supported | Typed value validated by `$schema` after expansion |
| Executed mapping `set` in `start` or later event | Stores `$(…)` verbatim | Execute after guard; commit all destinations atomically |
| `initialize`, dead branch, dry run, early failure route | Existing shell-free boundary | Reject forbidden shell surface and perform no lifecycle shell work |

The result cache must retain the full command outcome and separate approval lifetime from result lifetime. Keep executable bytes fixed at preflight after early binding. Level 1 API tests can observe types, cache counts, command bytes, and rollback; a CLI fake-command test must prove the later `when:` reads the typed result. The new suffix grammar is a load-bearing frontmatter reader, so its acceptance matrix must include absent suffix, each valid suffix, duplicate or incompatible suffix, invalid suffix, and trailing content; assert the public composition result rather than only parser output.

### Literal and authored-text rendering still changes prompt text (high)

**Defect class:** Two rendering paths alter authored syntax when they lack a real interpolation span or treat author text as markup.

[`prompts/_prompt.md`](../../../prompts/_prompt.md) still carries the artificial real span and explicitly documents the literal defect. [`dry_run`](../../cli/src/commands/wrap/composition/dry_run.rs) constructs `Prose` markup by inserting `description` into a markup string without escaping it. The spec reports leading underscores disappearing and a literal `</i>` reaching the header; the same class affects a diagnostic that quotes `_loop_count`.

| Site / shape checked | Observed baseline | Required result |
| --- | --- | --- |
| `md compose`, literal-only direct file | Triple braces survive | Double braces |
| `md compose`, literal-only included file | Triple braces survive | Double braces |
| `claudine compose`, literal-only direct file | Same uncorrected Darkmatter path | Double braces |
| `claudine compose`, literal-only included file | Same uncorrected path | Double braces |
| File with a real span and a literal | Converts; clean control | Preserve |
| Header description with `_pr/open.md`, inline code, and `</i>` | Spec reports stripped underscores and leaked markup | Exact authored text after stripping terminal styling |
| Diagnostic quoting `_loop_count` | Spec reports `loop_count` | Exact identifier |

Use [`biscuit-terminal` Prose](../../../biscuit-terminal/docs/components/prose.md) with author text escaped as text, and sweep every description or diagnostic site that interpolates author text into Prose markup. Text conversion belongs in Level 1 direct/CLI tests. The header's rendered glyphs and styling need a Level 2 capture in a real terminal with no focus change; Level 1 string assertions alone cannot establish that display requirement. No Level 3 keyboard test is required.

### The prompt guide and PR flow still carry the temporary behavior (medium)

**Defect class:** Shipped prompts still depend on a specific active spec path and retain workarounds for defects the spec requires the implementation to remove.

| Site / shape checked | Observed source state | Required result |
| --- | --- | --- |
| [`_prompt.md`](../../../prompts/_prompt.md), active spec | Literal `&claudine/fixes/.../spec.md` works only while file stays there | Resolve by directory identity |
| `_prompt.md`, relocated/completed spec | No relocation handling | Read status, hide completed warnings |
| `_prompt.md`, missing or ambiguous spec | `file_exists` silently omits the whole warning block | Report the lookup failure |
| `_prompt.md`, F6 and D2 warnings | Neither warning exists | Add relevant warnings or explain why prompt authors are unaffected |
| [`dirty.md`](../../../prompts/_pr/dirty.md) and [`fix.md`](../../../prompts/_pr/fix.md) | Stale staged-list messages remain | Remove after fresh `ctx` lands |
| [`push.md`](../../../prompts/_pr/push.md) | Inline git fact blocks remain | Use refreshed `ctx` and shared partial after approval parity lands |
| `_pr/fix.md`, duplicate `warn` | Still present | Remove after failure exit and diagnostic are fixed |

Keep `fixed:` synchronized when each underlying repair lands. Level 1 tests should compose a temporary copy with each lookup state and each `fixed:` value, then drive the PR prompt routes through a stub provider and disposable repository.

### Required behavior has no matching acceptance tests or performance evidence (medium)

**Defect class:** Existing tests exercise neighboring behavior but do not prove the changed public outcomes across the owners and files named in this specification.

The sibling gaps are recorded in the tables above: stale `ctx` across five run vehicles, terminal proxy across loop events and sequence events, approval parity with an overlaid partial, final exit after downgrade, loop-gate ambient values, result suffix shapes and lifecycle `set`, four literal-composition routes, header rendering, guide relocation, and PR routes. Existing tests for `initialize` proxy and ordinary shell approval are clean controls, not substitutes. Add the specification's performance spike before planning the context and transclusion scans: repeated warm and cold runs, wall-time variance, and request counters for the five workloads on the supported hosts. Cross-OS proof is acceptance/CI work and is **not** itself a reason for `ready: false` here.

For each new test, confirm its declared Cargo target and tier. In particular, a new `claudine-cli` file under `tests/` needs a `mod` declaration in its consolidated `tests/l1/main.rs` or `tests/level2/main.rs`. Level 1 is appropriate for the deterministic command, document, exit-status, and output requirements; the rendered header needs Level 2 real-terminal capture. No requirement in this fix involves a physical keypress, so Level 3 is unnecessary. No new file-format/configuration reader has been implemented yet; apply the input robustness matrix to any load-bearing reader added while implementing shell results or guide lookup.
