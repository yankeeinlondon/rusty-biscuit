---
agent: claude
total_phases: 5
created: 2026-07-14
phase: 1
yolo: true
---

# Execution Plan — Faster Compose (Demand-Driven Context Capture)

Derived from [`spec.md`](./spec.md). Root fix: stop `ComposeOptions::new()` from
eagerly running a full `ComposeContext::capture()` (all git/fs/hardware groups) on
every construction. Make the default compose path **demand-driven** against the
document, **cache** process-stable host detections, and **bound** the remaining
filesystem/git scans.

## Grounding — Key Facts Established During Planning

These anchor the tasks below; verify against `main` before large edits.

- `ComposeOptions::new()` → `new_with_context(ComposeContext::capture())`; `capture()`
  → `capture_for_dir(cwd)` → `capture_runtime_context(dir)` → **all groups,
  unconditionally** (`context/options.rs:417`, `context/runtime.rs:117`,
  `capture/mod.rs:31`).
- A demand-driven path already exists: `capture_for_content` / `capture_for_document`
  → `capture_runtime_context_for_content` → `scan_needed_groups(content)` + always-on
  `DateTime` (`context/runtime.rs:155-174`, `capture/mod.rs:40`, `capture/groups.rs:59`).
- **Root vs child compose**: only `compose_with` / `compose_mut` call
  `run_compose_pipeline` (the root, `compose/mod.rs:200,221`). Children recurse through
  `run_compose_pipeline_internal` (`transclusion/engine.rs:990,1391`) and inherit the
  parent's already-resolved context via cloned options. → **Deferred capture must
  materialize exactly once, at the root, before the context is consumed.**
- `ComposeOptions::context()` returns `&ComposeContext` (`context/options.rs:477`). All
  non-test in-crate readers consume it *during* compose (pipeline, frontmatter shell
  expansion, transclusion engine, preflight/collect) — **except** `reference/graph.rs:281,289`
  and `cli/commands/compose.rs`, which are separate entry points (audit targets).
- The CLI **already** does the target pattern manually:
  `capture_for_document(&base_dir, &md)` + `new_with_context(...)`
  (`cli/commands/compose.rs:207-220`). The fix generalizes this into the library.
- `~484` `ComposeOptions::new()` call sites (overwhelmingly tests). The deferred approach
  is chosen precisely so these need **no** changes — `new()` becomes cheap and correct.
- `OsInfo`, `HardwareInfo`, `GpuInfo` all derive `Clone` (`sniff/lib/src/os/mod.rs:134`,
  `hardware/mod.rs:42`, `hardware/gpu.rs:57`) → `OnceLock` caching is viable.
- `options_hash` does **not** read the context (`cache/hashing.rs:135`), so a cheap
  deferred context does not change cache keys.
- **Move 3 reality (from sniff investigation):** none of `detect_repo_structure`,
  `detect_docs_with_packages`, `GitRepo::file_changes` accept an exclusion arg. sniff
  already skips `target`/`.git`/`node_modules`/… internally via
  `should_skip_directory_name` (`sniff/.../file_types/classify.rs:195`) for repo/language/
  nested walks. **Gaps requiring sniff changes:** (a) `_`-prefixed dirs are skipped
  nowhere in sniff; (b) the docs walk `collect_markdown_files` (`sniff/.../docs.rs:313`)
  honors only `.gitignore`, not the skip predicate. The `target`/`.git`/`node_modules` +
  `_`-prefix list the spec references lives in `claudine/cli/src/completion/walker.rs:55`
  (`SKIP_DIRS`), not sniff.

## Success Criteria (goal-backward)

1. A `compose_with` over a `ctx.*`-free document performs **zero** git/fs/hardware I/O
   (empty `capture_timings()`), mirroring
   `content_without_runtime_context_only_populates_datetime`.
2. `ComposeOptions::new()` performs no sniff probes at construction.
3. `ctx.*` values (repo/os/hardware/documents/…) resolve **identically** to today for
   documents that reference them — via body **and** frontmatter references.
4. `preflight::acceptance_tests` and `compose_with`-based L1 tests complete well under the
   30s nextest terminate ceiling on the `_area-ci.yml` darkmatter matrix, without per-test
   timeout bumps.
5. Repeated Os/Hardware/Gpu captures within one process pay each sniff probe at most once.
6. `just test` + `just lint` (darkmatter) green on the host; darkmatter L1 green on the
   blocking Linux legs.

---

## Phase 1 — Deferred-Context Mechanism in `ComposeOptions`

**Goal:** `ComposeOptions::new()` becomes cheap (datetime-only, zero sniff I/O) and marks
its context *deferred*; explicit context setters mark it *resolved*. No call-site changes.

- [ ] Run `impact` (GitNexus) on `ComposeOptions::new`, `new_with_context`, `with_context`,
      and `ComposeContext::capture` before editing; record blast radius and warn if HIGH/CRITICAL.
- [ ] Add an internal marker to `ComposeOptions` recording whether `context` is *deferred*
      (from `new()`) or *explicitly provided* (from `new_with_context` / `with_context`).
      Prefer a private `bool` field (e.g. `context_deferred`) over an enum to minimize churn;
      keep the existing `context: ComposeContext` field so `context()` still returns
      `&ComposeContext`.
- [ ] Change `new()` to construct a **cheap datetime-only** context (e.g.
      `ComposeContext::capture_for_content(Path::new("."), "")` — scans empty string → only
      `DateTime`, zero I/O) and set `context_deferred = true`. Do **not** call
      `ComposeContext::capture()`.
- [ ] Ensure `new_with_context(...)` and `with_context(...)` set `context_deferred = false`
      (explicit context is authoritative and never overwritten at compose time).
- [ ] Confirm `Default for ComposeOptions` (→ `new()`) and `compose_mut()` inherit the cheap
      path automatically. `Clone`/`Debug` need no change beyond the new field (add field to the
      hand-written `Debug` impl only if it aids diagnostics — optional, keep surgical).
- [ ] Verify `options_hash` (`cache/hashing.rs`) is unaffected (it does not read context);
      no change expected.

**Validation checkpoint (Phase 1):**
- [ ] New unit test: `ComposeOptions::new()` produces a context whose `capture_timings()` is
      empty and whose `values()` contain `now`/`today` but **not** `repo`/`os`/`gpu`.
- [ ] New unit test: `new_with_context(full)` / `with_context(full)` leave `context_deferred`
      false (assert via a demand-driven compose that a *provided* full context is used verbatim —
      see Phase 2 test, or a crate-internal accessor gated on `#[cfg(test)]`).
- [ ] `just test` (darkmatter lib) compiles and passes the touched modules.

---

## Phase 2 — Materialize Deferred Capture at Root Entry Points + Audit Readers

**Goal:** Every root compose/preflight/reference entry point resolves a deferred context
against the document (body + frontmatter) exactly once, using `capture_for_document`.
Explicit contexts pass through untouched. Depends on Phase 1.

- [ ] `run_compose_pipeline` (root, `pipeline/mod.rs:29`): if `options.context_deferred`, replace
      `options.context` with `ComposeContext::capture_for_document(base_dir, self)` before any
      consumer reads it. Use `base_dir = current_dir()` (fallback `"."`) to preserve today's
      `capture()` semantics — **do not** silently switch to the source-file dir (call it out if a
      change is desired; out of scope here).
- [ ] Confirm `run_compose_pipeline_internal` (child path) is **never** given a deferred context
      (children always inherit a resolved parent context via cloned options). Add a debug assertion
      or comment documenting this invariant so future edits don't regress it.
- [ ] `compose_preflight` (`preflight/mod.rs:281`, takes `&ComposeOptions`): before calling
      `collect::collect_shell_commands_with_graph`, materialize a resolved context when
      `context_deferred` is set (build a local resolved `ComposeOptions` clone via
      `with_context(capture_for_document(...))`). This is the fix for the timing-out
      `preflight::acceptance_tests`.
- [ ] Audit `reference/graph.rs:281,289` (`options.compose.context()`): reference-graph
      `when=`-condition evaluation reads the context directly. When the embedded `ComposeOptions`
      is deferred, either (a) materialize `capture_for_document(&md, ...)` at that entry, or
      (b) require callers to pass an explicit context. Choose (a) for behavior parity; document
      the decision inline.
- [ ] Audit all other `context()` readers (grep confirms: `frontmatter_shell_expansion.rs`,
      `pipeline/mod.rs`, `inline/replacement.rs`, `transclusion/engine.rs`, `preflight/collect.rs`)
      — verify each executes only *inside* a compose pass whose root already materialized the
      context. Record the audit result as a comment or PR note.
- [ ] (Optional, surgical) Simplify `cli/commands/compose.rs` only if trivial: it already
      captures `capture_for_document` for validation reuse; leave as-is unless the shared-context
      path becomes redundant. Do not expand scope.

**Validation checkpoint (Phase 2):**
- [ ] New integration/unit test (document path): `compose_with` over a document with **no**
      `ctx.*` (body and frontmatter) yields a report/context with empty `capture_timings()` →
      Success Criterion 1.
- [ ] New test: a document referencing `ctx.*` **only in frontmatter** still resolves those
      values through `compose_with` (guards the frontmatter-scan path, not just body).
- [ ] Regression: existing `ctx.*` tests (repo/os/hardware/documents) pass unchanged →
      Success Criterion 3.
- [ ] `preflight::acceptance_tests` (`approval_set_is_loop_stable`,
      `execution_is_a_subset_of_approval_across_states`,
      `execution_subset_of_approval_across_randomized_conditions`) pass and are visibly fast
      locally (no full capture per `ComposeOptions::new()`).

---

## Phase 3 — Cache Process-Stable Host Detections (Os / Hardware / Gpu)

**Goal:** Memoize the three process-invariant sniff probes so repeated captures (compose-heavy
test runs, multi-document CLI invocations) pay each probe once. Independent of Phase 4 —
**parallelizable** with it. Depends on Phase 1/2 landing the demand-driven path (so caching is
only exercised when a document actually needs these groups).

- [ ] Run `impact` on `ContextCapture::new` (`capture/snapshot.rs:49`) — the sniff probes live in
      its `thread::scope`.
- [ ] Introduce module-level `OnceLock` caches for the three probe results:
      `os::detect_os_with_request(&request)` → cache `OsInfo`; `hardware::detect_hardware_summary()`
      → cache `HardwareInfo`; `hardware::detect_gpus()` → cache the joined GPU-name string.
      The `OsRequest` used is a fixed constant (`full().include_locale(false)…`) so a single cache
      key is correct.
- [ ] Route the `os_handle`/`hw_handle`/`gpu_handle` bodies in `snapshot.rs` through the cached
      getters (first call probes + stores; subsequent calls clone the stored value). Preserve
      current error/`diagnostics` semantics: a failed first probe should **not** be cached as a
      permanent failure — cache only successful detections; let failures re-probe.
- [ ] Keep timings meaningful: a cache hit reports ~0 elapsed for that group (acceptable) — do
      not fabricate timings. Confirm `capture_timings()` consumers tolerate near-zero.
- [ ] **Do not** cache `Repo` / `FileChanges` / `Languages` / `Documents` — those are live
      session state (spec: cache invalidation constraint).
- [ ] Confirm cross-platform correctness: `OnceLock` is `std`, no platform-specific code added;
      the cached values are the same shapes on macOS/Windows/Linux.

**Validation checkpoint (Phase 3):**
- [ ] New unit test: two consecutive captures that include the Os/Hardware/Gpu groups return
      identical values and the second performs no fresh probe (assert via a probe counter behind
      `#[cfg(test)]`, or by asserting equality + a near-zero second-capture timing). →
      Success Criterion 5.
- [ ] Regression: `host.rs` existing tests (`gpu_only_population_does_not_require_hardware_capture`)
      still pass.
- [ ] `just test` darkmatter green.

---

## Phase 4 — Bound the Filesystem/Git Scans (Exclusions) — *scoped decision gate*

**Goal:** When `Repo`/`FileChanges`/`Languages`/`Documents` groups *are* needed, ensure large
`target/` / `_`-prefixed subtrees cannot dominate the walk. Independent of Phase 3 —
**parallelizable**. Lower priority than Phases 1–2 for the CI symptom (a `ctx.*`-free doc does
zero scans regardless of tree size).

> **Decision gate — resolve before coding.** The sniff investigation showed
> `target`/`.git`/`node_modules` are **already** excluded by sniff's internal
> `should_skip_directory_name` for repo/language/nested walks. The real gaps are:
> (a) `_`-prefixed directories are excluded nowhere in sniff; (b) the docs walk
> `collect_markdown_files` honors only `.gitignore`. Closing these requires **editing the
> sniff crate**, which is broader than a darkmatter-local fix. Confirm with the maintainer
> whether Phase 4 lands now (sniff change) or is deferred as a follow-up — Phases 1–3 already
> satisfy the CI success criteria (1,2,4,5).

- [ ] **Measure first:** on the large-tree CI runner (or a synthetic large `target/`), capture
      per-group timings for a document that *does* reference `repo`/`documents`. Confirm whether the
      docs walk (not skip-predicate-bounded) is the residual cost. If already fast, record the
      measurement and mark Phase 4 deferred (no sniff change).
- [ ] If proceeding: in `sniff` (`--manifest-path` per repo convention; sniff is a curated area),
      extend directory skipping to cover `_`-prefixed directories and make the docs walk
      (`sniff/.../docs.rs` `collect_markdown_files`) honor `should_skip_directory_name` in addition
      to `.gitignore`. Prefer extending the existing `should_skip_directory_name` predicate over a
      new caller-supplied exclusion API (simplest; matches how sniff already bounds walks).
- [ ] If sniff gains the `_`-prefix rule, run the sniff package's own tests (`just test` in
      `sniff/`) and update `sniff/lib` docs/skill if the skip behavior is documented.
- [ ] Update `darkmatter` only if a new sniff API surface is required (avoid if the predicate
      extension is internal).

**Validation checkpoint (Phase 4):**
- [ ] Measurement recorded (before/after per-group timing on a large tree) justifying the change
      or the deferral.
- [ ] If sniff changed: sniff tests green; a darkmatter capture that references `Documents` no
      longer walks `_`-prefixed / `target` docs subtrees (assert via a fixture tree or timing
      bound).
- [ ] If deferred: an entry added to the fix's `_unscheduled` follow-ups (or a note in the spec's
      Out-of-Scope) recording the residual.

---

## Phase 5 — Full Validation, CI, and Drift

**Goal:** Prove all success criteria end-to-end and update any drifted docs. Depends on
Phases 1–3 (and 4 if it landed).

- [ ] Run `just test` and `just test-l2` (if applicable) and `just lint` for `darkmatter` on the
      host; all green.
- [ ] Run `detect_changes({scope: "compare", base_ref: "main"})` (GitNexus) and confirm only the
      intended symbols/flows changed; investigate any surprise blast radius.
- [ ] Confirm the spec's timing-out tests pass without any per-test timeout bumps: the
      `options_hash_sensitive_*` tests and `preflight::acceptance_tests`. (The `options_hash` tests'
      `capture_for_content` workaround is now redundant but harmless — leave or remove per spec
      Out-of-Scope note; if removed, do it in an isolated commit.)
- [ ] Push to / observe the darkmatter `_area-ci.yml` matrix (Linux legs) and confirm L1 completes
      under the 30s terminate ceiling → Success Criterion 4. (If CI cannot be triggered
      non-interactively, record the local timing evidence and the expected CI outcome.)
- [ ] Drift pass: update the `ComposeOptions::new()` rustdoc (`context/options.rs:411-419`) — it
      currently says "captures runtime context … at creation"; reword to reflect the deferred,
      demand-driven semantics. Update the struct-level "## Construction" note likewise.
- [ ] Drift pass: update the `darkmatter` skill / any `docs/` note describing eager context capture,
      and the memory pointer if one exists, to reflect demand-driven default.
- [ ] Move this fix directory to `_completed/` per repo lifecycle convention once merged (separate
      operation; do not commit as part of the code change unless instructed).

---

## Parallelization Summary

- **Phase 1 → Phase 2** are strictly sequential (Phase 2 consumes the deferred marker).
- **Phase 3** and **Phase 4** are mutually independent and may run in **parallel** once
  Phases 1–2 land (both build on the demand-driven path but touch disjoint code:
  `capture/snapshot.rs` host probes vs. sniff walk exclusions).
- **Phase 5** is the join point — depends on all preceding phases that landed.

## Risk Register

- **R1 — Semantic change to `new()` (Backward Compat).** Any caller reading `context()` for
  repo/os/hardware *before* compose now gets datetime-only. Mitigation: Phase 2 audit +
  `reference/graph.rs` fix; provide the explicit `capture()` / `with_context(...)` escape hatch
  (already exists). *Blast radius: the ~484 `new()` sites are unaffected because they compose.*
- **R2 — `ctx.*` silently empty.** A missed frontmatter/body scan surface would resolve `ctx.*`
  to empty. Mitigation: Phase 2 frontmatter-path test + full regression suite (Criterion 3).
- **R3 — Cache staleness (R2 of spec).** Caching anything session-variable would corrupt output.
  Mitigation: Phase 3 caches only Os/Hardware/Gpu; explicit "do not cache repo/fs" guard + test.
- **R4 — Move 3 scope creep into sniff.** The exclusion bounds require a sniff-crate change and
  the `target/` symptom is already handled. Mitigation: Phase 4 decision gate — measure, then
  land-or-defer; Phases 1–3 already meet the CI criteria.
- **R5 — Cross-OS.** `OnceLock` + deferred capture are `std`-only, no platform branches; verify
  the darkmatter suite on the host (macOS) and rely on CI for Linux/Windows legs.
