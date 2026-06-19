---
reviewed: true
status: "ready for planning and implementation"
---

# Compose Pipeline v2 — Pre-Flight Tech Design

This document defines an implementation-ready redesign of how Darkmatter discovers, approves, and executes shell commands during composition. It supersedes the implicit behavior currently spread across:

- `darkmatter/lib/src/markdown/compose/mod.rs` (`run_compose_pipeline_internal`)
- `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs` (`collect_shell_commands`)
- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs` (`prepare_directive`)
- `claudine/lib/src/composition/preflight.rs` (`resolve_shell_approvals`)
- `darkmatter/docs/darkmatter-compose-pipeline.md` and `darkmatter/docs/inline/preflight-checks.md`

The goal is to make pre-flight a first-class, well-defined stage with two clearly separated responsibilities — **approval** and **execution** — and to reconcile a spec/code drift discovered while reviewing the current implementation.

Reader's note: this review treats the companion
[`module-structure.md`](./module-structure.md) as part of the design surface.
The implementation contract below therefore names the same split that the module
structure defines: `compose/preflight` owns collection and validation,
shared shell primitives move toward `compose/shell`, and condition-aware
execution remains an inline stage with a run-local cache.

## Summary

Composition currently conflates three concerns that should be distinct:

1. **What commands *could* run** (used for approval).
2. **What commands *will* run** (used for execution).
3. **How many times a given command runs** (used for caching).

The current code derives all three from a single condition-aware walk and interleaves approval with execution per directive. This produces three problems: fragile behavior under state change/loops, a latent "evaluate a condition against an unresolved `$(...)` value" bug, and a duplicated discovery compose.

The v2 design separates the concerns:

- **Approval set** — condition-**blind**, computed once, up front, validated against blacklist/whitelist policy with a single batched prompt.
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
4. Solving dynamic command generation whose cardinality depends on runtime/shell state.
5. OS sandboxing or isolation.
6. Replacing the current shell directive grammar beyond the minimum option
   additions specified here.

## Background — The Drift

`darkmatter/docs/darkmatter-compose-pipeline.md` states that pre-flight requires **all** shell commands to be whitelisted "before starting," explicitly including commands in conditionally-excluded blocks. That is **condition-blind** approval.

The implementation drifted to **condition-aware** discovery:

- `collect_shell_commands` Phase 2 enables `PageBlocks` (`discovery.rs:169`), so commands in excluded page blocks are pruned before collection.
- The transclusion walk evaluates each `::file` directive's `when:` condition (`discovery.rs:366-375`) and skips children whose condition is false.

Only frontmatter `$(...)` ternaries remain condition-blind (`directive_reachable_pipelines` captures both branches).

This drift has a concrete failure mode beyond philosophy: discovery strips shell execution, so any `when:`/page-block condition that depends on a shell-produced value is evaluated against the unresolved literal `$(...)`. Discovery can therefore prune the **wrong** branch, then fail at execution. v2 eliminates this class by making approval condition-blind.

## Core Concepts

### Approval set (condition-blind)

Every command that *could* execute under *any* state. Built by walking
frontmatter, body, shell blocks, and the transclusion graph **without**
evaluating any `when:` or page-block condition. Both sides of every frontmatter
`$(...)` ternary, every `::block`, and every conditionally-transcluded document
contribute their commands. The set is deduped by normalized command string and
validated against the builtin blacklist, user blacklist, and whitelist; unknowns
trigger a single batched approval prompt (or a fast fail in non-interactive
contexts).

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

The reusable artifact is graph metadata, not fully rendered child Markdown:
resolved child paths/URLs, overlay state, pass-1 frontmatter snapshots, source
provenance, and any discovered shell entries. The final transclusion stage still
renders condition-aware content at the normal point in the pipeline. This matches
the module-structure decision to introduce a `TransclusionEngine` while avoiding
a second compose pass that can drift from the execution path.

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

The syntax intentionally follows each existing directive family rather than
forcing one spelling everywhere:

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
::shell-block no_cache=true
date +%s
uuidgen
::end-block
```

`--no-cache` is the chosen body-directive keyword. Frontmatter adds a boolean
`::no-cache` suffix alongside the existing `::timeout:N` suffix. Shell blocks
keep their documented key-value parameter grammar and use `no_cache=true`;
`::shell-block --no-cache` must remain a parse error with the same targeted
"shell blocks use key-value parameters" hint as other flag-style mistakes.
`--fresh` / `--volatile` were considered; `no-cache` is the most conventional
term and maps directly to the behavior.

### Discoverability guard

When the cache collapses a command whose executable is on a small known-volatile allowlist (`uuidgen`, `date`, `openssl`, …) and the command appears more than once, emit a compose **warning**: "`uuidgen` appears 3× and was executed once (cached); add `--no-cache` for a fresh value each time." This preserves the convenient default while catching the foot-gun.

## Worked Examples

### Conditional page block

```md
---
env: production
---
::block when="env == 'production'"
::shell ./deploy.sh --prod
::end-block

::block when="env == 'staging'"
::shell ./deploy.sh --staging
::end-block
```

- `approval_set` = `{ ./deploy.sh --prod, ./deploy.sh --staging }`
- `execution_set` (env=production) = `{ ./deploy.sh --prod }`

A later compose with `env=staging` runs the staging command with no new prompt — it is already approved.

### Loop / state mutation

```md
---
iteration: "{{ ctx.iteration }}"
---
::block when="iteration % 2 == 0"
::shell ./even-task.sh
::end-block
::block when="iteration % 2 == 1"
::shell ./odd-task.sh
::end-block
```

- `approval_set` (once) = `{ ./even-task.sh, ./odd-task.sh }`
- `execution_set` iter 0 = `{ ./even-task.sh }`; iter 1 = `{ ./odd-task.sh }`

Condition-blind approval front-loads both branches, so no iteration interrupts to prompt.

### Dead-branch safety

```md
---
cleanup: false
---
::block when="cleanup == true"
::shell ./purge.sh ./build-artifacts
::end-block
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

### Public API shape

Darkmatter should expose the document-side pre-flight result without forcing
callers through the full compose execution path:

- Add a library entry point equivalent to `Markdown::compose_preflight(options)`
  returning a `ComposePreflightReport` with the deduped approval set, source
  entries, warnings, and reusable transclusion graph metadata.
- Keep `ComposeOptions::with_pre_approved_commands(...)` as the execution
  membership channel.
- Add an internal execution mode that accepts a previously collected pre-flight
  report so `compose_with()` can avoid re-collecting after Claudine merges and
  returns the approved set.
- Keep the CLI's existing `md compose --shell` behavior, but route it through the
  same pre-flight collector so `--shell` reports condition-blind approval
  candidates rather than condition-aware execution candidates.

## Chicken-and-Egg Handling

A body command whose parameters interpolate a shell-produced frontmatter value is collected at pass-1 state in unresolved form:

```md
---
branch: $(git branch --show-current)
---
::shell git log {{ doc.branch }} -1
```

Collection sees `git log {{ doc.branch }}`; execution (post pass-2) sees
`git log main`. The two differ, so the execution gate would miss
`approval_set` → `NotPreApproved`.

v2 rejects this earlier instead of silently resolving it. The pre-flight
collector must detect any body `::shell` or `::shell-block` command whose
normalized command text depends on a frontmatter key still pending
frontmatter-shell expansion after pass 1. That diagnostic is a hard pre-flight
error and should explain the supported alternatives:

- Move the dynamic value fully into the frontmatter command and reference its
  output as document content rather than as a shell command argument.
- Use a stable command shape whose dynamic value is not part of the executable
  or argument vector being approved.
- Split the document into two explicit compose runs if the second run really
  must approve commands generated by the first run.

Two-phase collection was considered and rejected for v2. It restores correctness
for this case, but it creates a second approval point after frontmatter commands
have already executed, which conflicts with the single batched pre-flight
contract and complicates Claudine's harness-command merge.

## Error Model

Reuse existing `ShellExpansionError` variants where they already match the
failure. The execution-time membership check produces `NotPreApproved` (already
defined). Add only narrowly-scoped variants for new pre-flight-only failures.
Pre-flight surfaces:

- `Blacklisted` — builtin or user blacklist match during collection/validation.
- `Denied` / blacklist-persist — interactive approval outcomes.
- `ApprovalRequired` — non-interactive context with un-approved commands.
- `PreFlightDiscoveryFailed` — collection walk (interpolation/transclusion)
  failed; wraps the underlying Markdown or shell parse error with source
  context.
- `DynamicCommandShape` — a body command or shell-block command depends on a
  frontmatter-shell-expanded value and therefore cannot be approved in the
  condition-blind pass.

`NotPreApproved` should remain a bug sentinel after the collector rejects
dynamic command shapes. New tests should assert that user-authored dynamic
shapes fail as `DynamicCommandShape`, not as a late `NotPreApproved`.

## Module Structure Alignment

The companion [`module-structure.md`](./module-structure.md) is the agreed structural
target and is part of this feature's implementation plan (`plan.md`), with these constraints:

- Implement `compose/preflight` as the home of collect → validate → approve.
  It may reuse shell primitives, but it must not evaluate page-block or
  transclusion `when=` conditions.
- Move shared parser/tokenizer/policy/store code toward `compose/shell`, while
  keeping condition-aware execution under `compose/inline/shell_expansion`.
- Keep cache implementation local to shell execution for v2
  (`inline/shell_cache.rs`). Do not merge it with the existing
  content-hash transclusion cache unless implementation proves the shapes are
  actually the same.
- Extracting `TransclusionEngine` is useful because pre-flight and final
  transclusion need the same graph traversal rules. The extraction must be
  behavior-preserving before the condition-blind collection change lands.
- The module move must update Claudine imports in the same change; the
  no-shim decision is acceptable because these are monorepo-internal paths.

## Testing Strategy

1. **Approval is condition-blind** — a document with commands in `when:`-false page blocks and false-condition transclusions yields an `approval_set` containing all of them.
2. **Execution is condition-aware** — the same document executes only the reachable subset; dead-branch side-effecting commands do not run (assert via a sentinel file or recorded marker).
3. **Invariant** — property test: for randomized condition states, `execution_set ⊆ approval_set`.
4. **Loop stability** — re-compose across flipping conditions issues zero new prompts after the first.
5. **Cache default** — a pure command repeated N times executes once; output identical at all sites.
6. **Cache opt-out** — `--no-cache uuidgen` repeated yields distinct values; document-order preserved.
7. **Shell-block cache opt-out** — `::shell-block no_cache=true` repeats execute
   fresh, while `::shell-block --no-cache` remains a targeted parse error.
8. **Frontmatter cache opt-out** — `$(uuidgen)::no-cache` works alongside
   `$(cmd)::timeout:N`; invalid suffix combinations receive a precise parse
   error.
9. **Discoverability warning** — repeated volatile command without `--no-cache`
   emits the warning.
10. **Dynamic command shape** — frontmatter-shell-dependent body parameters
    produce `DynamicCommandShape`, not a silent mismatch or late
    `NotPreApproved`.
11. **Orchestrator merge** — harness commands plus document commands approve in
    one batch; execution membership-checks against the merged set.
12. **CLI shell listing** — `md compose --shell` reports commands from false
    `::block when=...` regions and false `::file when=...` transclusions.

## Documentation Updates

- Rewrite `darkmatter/docs/inline/preflight-checks.md` around the approval/execution split and cache-by-default.
- Update `darkmatter/docs/darkmatter-compose-pipeline.md`: pre-flight becomes step 4 (collect → validate → approve); execution stays in-stage; remove the contradictory line-103 note and replace it with the condition-blind-approval / condition-aware-execution statement.
- Update `darkmatter/docs/inline/shell-blocks.md` with `no_cache=true` and keep
  the warning that shell blocks do not accept flag-style parameters.
- Update CLI help for `md compose --shell` so "discovered" means
  condition-blind approval candidates.
- Add cache opt-out docs to the `::shell`, frontmatter `$(...)`, and
  `::shell-block` references, using each surface's syntax.

## Resolved Review Decisions

1. Dynamic loops whose command cardinality depends on runtime/shell state are
   explicitly unsupported for v2. Do not add a runtime approval fallback; it
   would violate the `execution_set ⊆ approval_set` invariant. Surface these as
   `DynamicCommandShape` or `NotPreApproved` bug sentinels, depending on where
   they are detected.
2. Chicken-and-egg body command parameters use the diagnostic path, not
   two-phase collection.
3. The known-volatile allowlist for the discoverability warning is a fixed
   builtin set for v2. Make it configurable only after real usage shows the
   builtin list creates noise.
