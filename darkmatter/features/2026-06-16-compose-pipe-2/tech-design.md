# Compose Pipeline v2 — Pre-Flight Tech Design

This document defines an implementation-ready redesign of how Darkmatter discovers, approves, and executes shell commands during composition. It supersedes the implicit behavior currently spread across:

- `darkmatter/lib/src/markdown/compose/mod.rs` (`run_compose_pipeline_internal`)
- `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs` (`collect_shell_commands`)
- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs` (`prepare_directive`)
- `claudine/lib/src/composition/preflight.rs` (`resolve_shell_approvals`)
- `darkmatter/docs/darkmatter-compose-pipeline.md` and `darkmatter/docs/inline/preflight-checks.md`

The goal is to make pre-flight a first-class, well-defined stage with two clearly separated responsibilities — **approval** and **execution** — and to reconcile a spec/code drift discovered while reviewing the current implementation.

## Summary

Composition currently conflates three concerns that should be distinct:

1. **What commands *could* run** (used for approval).
2. **What commands *will* run** (used for execution).
3. **How many times a given command runs** (used for caching).

The current code derives all three from a single condition-aware walk and interleaves approval with execution per directive. This produces three problems: fragile behavior under state change/loops, a latent "evaluate a condition against an unresolved `$(...)` value" bug, and a duplicated discovery compose.

The v2 design separates the concerns:

- **Approval set** — condition-**blind**, computed once, up front, validated against the whitelist with a single batched prompt.
- **Execution set** — condition-**aware**, resolved lazily in-stage; a command runs only when its branch is actually reached.
- **Caching** — identical commands execute **once per compose by default**, with a `--no-cache` opt-out for volatile commands.

The governing invariant is `execution_set ⊆ approval_set`, so the execution-time gate is a pure membership check that never prompts and never encounters an un-approved command — this run or any future loop iteration.

## Goals

1. Approve every command that could execute under any state, exactly once, before anything runs.
2. Never execute a command that the document's current state excludes.
3. Make the approved set independent of runtime state, so loops and re-composes are stable.
4. Execute each unique command once per compose by default; allow opt-out for volatile commands.
5. Remove the duplicated discovery-compose interpolation pass.
6. Reconcile the spec, the pipeline docs, and the code on condition handling.

## Non-Goals

1. Changing the shell tokenizer, blacklist contents, or whitelist file formats.
2. Persisting cache results across compose runs (cache is per-compose only).
3. Cross-run or shared approval stores.
4. Solving dynamic command generation whose cardinality depends on runtime/shell state (see Open Questions).
5. OS sandboxing or isolation.

## Background — The Drift

`darkmatter/docs/darkmatter-compose-pipeline.md` states that pre-flight requires **all** shell commands to be whitelisted "before starting," explicitly including commands in conditionally-excluded blocks. That is **condition-blind** approval.

The implementation drifted to **condition-aware** discovery:

- `collect_shell_commands` Phase 2 enables `PageBlocks` (`discovery.rs:169`), so commands in excluded page blocks are pruned before collection.
- The transclusion walk evaluates each `::file` directive's `when:` condition (`discovery.rs:366-375`) and skips children whose condition is false.

Only frontmatter `$(...)` ternaries remain condition-blind (`directive_reachable_pipelines` captures both branches).

This drift has a concrete failure mode beyond philosophy: discovery strips shell execution, so any `when:`/page-block condition that depends on a shell-produced value is evaluated against the unresolved literal `$(...)`. Discovery can therefore prune the **wrong** branch, then fail at execution. v2 eliminates this class by making approval condition-blind.

## Core Concepts

### Approval set (condition-blind)

Every command that *could* execute under *any* state. Built by walking frontmatter, body, and the transclusion graph **without** evaluating any `when:` or page-block condition. Both sides of every ternary, every page block, and every conditionally-transcluded document contribute their commands. The set is deduped by normalized command string and validated against the blacklist/whitelist; unknowns trigger a single batched approval prompt (or a fast fail in non-interactive contexts).

### Execution set (condition-aware)

Every command that *will* execute given the document's firm state (post frontmatter interpolation pass 2). Resolved lazily, in-stage, where conditions are actually decidable: page blocks have pruned excluded regions and frontmatter is final.

### The invariant

```
execution_set ⊆ approval_set      (always)
```

Because approval is a superset of anything reachable under any state, the execution-time gate degrades to a membership check: the command is already approved, so it runs with no prompt. A miss indicates either a bug or the chicken-and-egg case (see below), surfaced as `NotPreApproved`.

## Pipeline Integration

Inline Pre ordering (unchanged steps keep their existing numbers):

1. Frontmatter Snapshot (external-state merge, `set_overrides`, pre-interpolation snapshot)
2. Frontmatter Interpolation — pass 1
3. Schema Validation
4. **Pre-Flight** (new, replaces ad-hoc per-directive approval)
   - **4a. Collect** — condition-blind walk of frontmatter `$(...)`, body `::shell`/`::shell-block`, and the transclusion graph recursively → `approval_set`.
   - **4b. Validate** — check `approval_set` against builtin blacklist, user blacklist, whitelist.
   - **4c. Approve** — single batched interactive prompt for the remainder; deny/blacklist aborts.
5. Frontmatter Shell Expansion — execute reachable frontmatter commands (membership-checked against `approval_set`, cached)
6. Frontmatter Interpolation — pass 2 (post-shell)
7. Build EffectiveState (firm document state)
8. Body stages: Text Replacement → Page Blocks → Interpolation → Shell Expansion → Shell Blocks → Link Resolve. Shell stages **execute reachable commands only** (page blocks have pruned dead regions), membership-checked, cached.
9. Transclusion → Inline Post → Finalization.

Key change: approval (step 4) is a single, condition-blind, up-front stage. Execution stays distributed across steps 5 and 8, condition-aware, but only ever runs commands already in `approval_set`.

### Reusing the collection walk

Collection (4a) walks the transclusion graph. The discovered child set and their resolved-at-pass-1 frontmatter should be cached and reused to seed graph composition (step 9), so the graph is traversed for structure once rather than re-discovered. This removes the separate discovery compose that exists today.

## Caching

Identical commands (same normalized string) execute **once per compose**; the result is memoized and reused at every other call site. Caching is the **default**.

### Rationale for cache-by-default

- The destructive non-idempotent commands (`rm`, `mv`, `dd`, `install`, `echo … >>`, package installs, `git push`, …) are already blacklisted and cannot run, so caching cannot silently skip a destructive side effect.
- The common multi-reference commands are pure (`git rev-parse HEAD`, `cat VERSION`, `basename "$PWD"`), where caching is faster and yields one consistent value across the document.
- The residual risk is **read-only but non-deterministic** commands (`uuidgen`, `date`, `openssl rand`), which is exactly what the opt-out targets.

### Cache semantics

- **Key:** normalized command string.
- **Scope:** per root compose run. Not persisted; each composition starts cold.
- **Opt-out is a full bypass:** a `--no-cache` directive neither reads nor writes the cache. It executes fresh at each occurrence, in document order, even when a cached entry for that string exists elsewhere.
- **Ordering:** unflagged commands that are *not* deduped (distinct strings) preserve document-order execution within a stage, as today.

### Opt-out syntax

Body and block directives use the existing `--flag` option style; frontmatter uses the existing `::key:value` style (mirrors `::timeout:`).

```md
::shell uuidgen                 # cached (default)
::shell --no-cache uuidgen      # fresh each occurrence
```

```md
---
build_id: $(uuidgen)::no-cache
stamp:    $(date +%s)::no-cache
version:  $(cat VERSION)            # cached
---
```

```md
::shell-block --no-cache
date +%s
uuidgen
::end-block
```

`--no-cache` is the chosen keyword. (`--fresh` / `--volatile` were considered; `--no-cache` is the most conventional.)

### Discoverability guard

When the cache collapses a command whose executable is on a small known-volatile allowlist (`uuidgen`, `date`, `openssl`, …) and the command appears more than once, emit a compose **warning**: "`uuidgen` appears 3× and was executed once (cached); add `--no-cache` for a fresh value each time." This preserves the convenient default while catching the foot-gun.

## Worked Examples

### Conditional page block

```md
---
env: production
---
::page when="env == 'production'"
::shell ./deploy.sh --prod
::end-page

::page when="env == 'staging'"
::shell ./deploy.sh --staging
::end-page
```

- `approval_set` = `{ ./deploy.sh --prod, ./deploy.sh --staging }`
- `execution_set` (env=production) = `{ ./deploy.sh --prod }`

A later compose with `env=staging` runs the staging command with no new prompt — it is already approved.

### Loop / state mutation

```md
---
iteration: "{{ ctx.iteration }}"
---
::page when="iteration % 2 == 0"
::shell ./even-task.sh
::end-page
::page when="iteration % 2 == 1"
::shell ./odd-task.sh
::end-page
```

- `approval_set` (once) = `{ ./even-task.sh, ./odd-task.sh }`
- `execution_set` iter 0 = `{ ./even-task.sh }`; iter 1 = `{ ./odd-task.sh }`

Condition-blind approval front-loads both branches, so no iteration interrupts to prompt.

### Dead-branch safety

```md
---
cleanup: false
---
::page when="cleanup == true"
::shell ./purge.sh ./build-artifacts
::end-page
```

- `approval_set` = `{ ./purge.sh ./build-artifacts }`
- `execution_set` (cleanup=false) = `{}`

The purge is approved (vetted once) but never executes while `cleanup` is false. Blanket pre-flight execution would run it; condition-aware execution does not.

## Orchestrator Boundary

Pre-flight inside the pipeline sees only the document's commands. Claudine additionally approves **harness** commands (pre/post checks, handlers) discovered via `collect_auditable_commands` (`claudine/lib/src/composition/preflight.rs:71`). To keep a single approval prompt:

- The pipeline exposes the collected `approval_set` to the caller before execution.
- The caller (Claudine) merges in harness commands and performs the batched approval.
- The merged approved set is handed back to the pipeline as the membership source for execution (today's `pre_approved_commands` channel, retained).

This preserves the existing "Darkmatter discovers, Claudine authorizes" separation while making the document-side collection condition-blind.

## Chicken-and-Egg Handling

A body command whose parameters interpolate a shell-produced frontmatter value is collected at pass-1 state in unresolved form:

```md
---
branch: $(git branch --show-current)
---
::shell git log {{ fm.branch }} -1
```

Collection sees `git log {{ fm.branch }}`; execution (post pass-2) sees `git log main`. The two differ, so the execution gate misses `approval_set` → `NotPreApproved`.

v2 does not silently resolve this. Options, to be decided in the spec:

1. **Document as unsupported** — frontmatter-shell-dependent command parameters are rejected at collection with a clear diagnostic.
2. **Two-phase collection** — collect+approve+execute frontmatter commands, run pass 2, then collect+approve body commands. This restores correctness at the cost of a second approval round (still up front, before any body execution).

Recommendation: ship option 1 with a precise error, evaluate option 2 if real documents need it.

## Error Model

Reuse existing `ShellExpansionError` variants. The execution-time membership check produces `NotPreApproved` (already defined). Pre-flight surfaces:

- `Blacklisted` — builtin or user blacklist match during collection/validation.
- `Denied` / blacklist-persist — interactive approval outcomes.
- `ApprovalRequired` — non-interactive context with un-approved commands.
- `PreFlightDiscoveryFailed` — collection walk (interpolation/transclusion) failed.

## Testing Strategy

1. **Approval is condition-blind** — a document with commands in `when:`-false page blocks and false-condition transclusions yields an `approval_set` containing all of them.
2. **Execution is condition-aware** — the same document executes only the reachable subset; dead-branch side-effecting commands do not run (assert via a sentinel file or recorded marker).
3. **Invariant** — property test: for randomized condition states, `execution_set ⊆ approval_set`.
4. **Loop stability** — re-compose across flipping conditions issues zero new prompts after the first.
5. **Cache default** — a pure command repeated N times executes once; output identical at all sites.
6. **Cache opt-out** — `--no-cache uuidgen` repeated yields distinct values; document-order preserved.
7. **Discoverability warning** — repeated volatile command without `--no-cache` emits the warning.
8. **Chicken-and-egg** — frontmatter-shell-dependent body parameter produces the chosen diagnostic, not a silent mismatch.
9. **Orchestrator merge** — harness commands plus document commands approve in one batch; execution membership-checks against the merged set.

## Documentation Updates

- Rewrite `darkmatter/docs/inline/preflight-checks.md` around the approval/execution split and cache-by-default.
- Update `darkmatter/docs/darkmatter-compose-pipeline.md`: pre-flight becomes step 4 (collect → validate → approve); execution stays in-stage; remove the contradictory line-103 note and replace it with the condition-blind-approval / condition-aware-execution statement.
- Add `--no-cache` to the `::shell`, frontmatter `$(...)`, and `::shell-block` references.

## Open Questions

1. Dynamic loops whose command cardinality depends on runtime/shell state cannot be fully enumerated at pre-flight. Document the boundary; decide whether a runtime approval fallback is acceptable.
2. Chicken-and-egg: ship the diagnostic (option 1) or the two-phase collection (option 2)?
3. Should the known-volatile allowlist for the discoverability warning be configurable, or a fixed builtin set?
