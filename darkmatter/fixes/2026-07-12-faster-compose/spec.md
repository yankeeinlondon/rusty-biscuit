---
clarified: claude/opus-4.8
status: draft
---

# Faster Compose — Demand-Driven Context Capture

`ComposeOptions::new()` eagerly runs a **full** `ComposeContext::capture()` on every
call. That capture populates *all* runtime-context groups — repository/git state,
git file-change diffs, language detection, document/skill filesystem scans, OS,
hardware, and GPU detection (all via `sniff`) — regardless of whether the document
being composed references any `ctx.*` variables at all. On a large working tree the
cost is seconds per call, and it is paid by every consumer that constructs options
the ordinary way, plus every test that does.

A demand-driven, cheap path already exists (`capture_runtime_context_for_content`
/ `ComposeContext::capture_for_content` / `capture_for_document`): it scans the
document for `ctx.*` references and populates only the groups actually referenced —
falling back to date/time only (zero I/O) when the document uses none. The eager
`capture()` used by `ComposeOptions::new()` simply bypasses it. This fix makes the
default compose path demand-driven and caches the process-stable detections, so
compose pays only for the context a document actually uses.

## Motivation / Evidence

This surfaced while bringing darkmatter's full L1 suite onto a completing CI run
(previously the suite was cancelled at a 30-minute wall-clock timeout before it
finished, hiding the cost). With the suite completing, a whole *class* of tests
times out at nextest's 30s terminate ceiling on the CI runner's large tree:

- `markdown::compose::cache::hashing::tests::options_hash_sensitive_*` — each built
  `ComposeOptions::new()` several times; 3× full capture exceeded 30s. (Worked
  around test-side by switching those tests to a cheap `capture_for_content("", …)`
  context, since `options_hash` never reads the context — but that is a symptom
  patch, not the cure.)
- `markdown::compose::preflight::acceptance_tests::approval_set_is_loop_stable`,
  `execution_is_a_subset_of_approval_across_states`,
  `execution_subset_of_approval_across_randomized_conditions` — these run real
  `compose_with(...)` passes; they time out at 30s (one even with a 90s allowance).

The pattern is not a set of individual slow tests — it is one shared cost
(`ComposeContext::capture()`) on the hot path of everything that composes. The same
cost is paid in **production**: every `ComposeOptions::new()` in the CLI and library
pays for full git + filesystem + hardware detection even when composing a document
that references no `ctx.*` variables.

## Current Architecture (as-is)

- `ComposeOptions` (`compose/context/options.rs`) stores a `context: ComposeContext`
  field. `ComposeOptions::new()` → `ComposeContext::capture()`; `new_with_context`
  accepts a pre-captured context.
- `ComposeContext::capture()` (`compose/context/runtime.rs`) → `capture_for_dir(cwd)`
  → `capture::capture_runtime_context(dir)` →
  `capture_runtime_context_for_groups(dir, ContextGroup::all())` — **all groups,
  unconditionally**.
- `capture::capture_runtime_context_for_content(dir, content)` already exists and is
  demand-driven: `scan_needed_groups(content)` + always-on `DateTime`; groups not
  referenced by the content are never captured (no I/O).
- The compose pipeline (`compose/pipeline/mod.rs`) reads `options.context()` — it
  consumes the context stored on the options; it does not re-capture per document.
  So whatever `new()` eagerly captured is exactly what compose uses.

The expensive groups (`Repo`, `FileChanges`, `Languages`, `Documents`) are `sniff`
git/filesystem scans; `Os`/`Hardware`/`Gpu` are `sniff` hardware probes. `DateTime`
and `Agent` are cheap.

## Proposed Design (root fix)

Three moves, in priority order:

1. **Make the default compose path demand-driven against the document.**
   The public compose entry points have the document in hand, so context capture
   should be scoped to what that document references. Options are composed at
   `compose_with`/`compose_preflight` time using `capture_for_document` (already
   demand-driven) rather than a pre-captured full context. Concretely: stop
   `ComposeOptions::new()` from eagerly capturing all groups; instead defer capture
   until the document is known, then populate only `scan_needed_groups(doc)`.

   > **Ruling (needs confirmation):** the cleanest shape is for `ComposeOptions` to
   > carry a *deferred* context (captured lazily against the document at compose
   > time) rather than an eagerly-captured one. Callers that legitimately need a
   > fully-populated context *before* compose (e.g. reading `ctx.repo.*` directly)
   > use an explicit `ComposeContext::capture()` / `with_context(...)`. `new()` must
   > no longer imply "detect everything about the host right now".

2. **Cache process-stable detections.**
   `Os`, `Hardware`, and `Gpu` do not change during a process's lifetime. Memoize
   them behind a `OnceLock` (or equivalent) so that even when a document *does*
   reference them, repeated captures across a compose-heavy test run or a
   multi-document CLI invocation pay the sniff probe exactly once.

3. **Bound the filesystem/git scans.**
   When `Repo`/`FileChanges`/`Languages`/`Documents` groups *are* needed, their
   `sniff` scans must exclude `target/`, `.git/`, `node_modules/`, and `_`-prefixed
   directories (aligning with the shared walker exclusions used elsewhere in the
   monorepo), so a large `target/` tree cannot dominate the walk. (This mirrors the
   sniff repo-scan cost fixes already applied to `git-status` and staged-package
   detection.)

## Backward Compatibility & Risks

- **Semantic change to `new()`.** Any consumer relying on `options.context()` being
  fully populated immediately after `ComposeOptions::new()` (without a document)
  must be migrated to an explicit full capture. Audit `context()` readers in the CLI
  and library; provide `ComposeContext::capture()` / `with_context(...)` for them.
- **`ctx.*` correctness must be preserved.** Demand-driven capture already backs the
  content path and is covered by
  `content_without_runtime_context_only_populates_datetime`; extend coverage to the
  document path (frontmatter + body scan) so no `ctx.*` reference silently resolves
  to empty.
- **Cache invalidation.** Only cache detections that are genuinely process-invariant
  (OS/hardware/GPU). Do **not** cache repo/file-change/document state — those change
  during a session and must stay live.

## Validation

- **Unit:** a compose over a `ctx.*`-free document performs zero git/fs/hardware I/O
  (assert empty capture timings, mirroring the existing datetime-only test).
- **Perf/CI:** the `preflight::acceptance_tests` and any other `compose_with`-based
  L1 tests complete well under the 30s nextest terminate ceiling on the
  `_area-ci.yml` darkmatter matrix (the run that exposed this). Target: darkmatter
  L1 green on the blocking Linux legs without per-test timeout bumps.
- **Regression:** `ctx.*` resolution across repo/os/hardware/documents groups still
  produces identical values for documents that reference them.

## Out of Scope

- Broader `sniff` scan-performance work beyond the exclusion bounds above.
- The Windows darkmatter test behaviour (tracked separately; only a compile fix has
  landed so far).
- Retiring the test-side `capture_for_content` workaround in the `options_hash`
  tests — harmless to keep; may be removed once `new()` is cheap.
