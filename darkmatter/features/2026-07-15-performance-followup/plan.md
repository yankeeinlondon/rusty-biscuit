---
agent: "claude/"
total_phases: 11
created: 2026-07-15
phase: 1
source_map_commit: bc1c148f26eae1bba36fc1f926298a52573d83bd
---

# Execution Plan — Performance Follow-up

Closes the delivery-contract gaps found by auditing all 35 findings of the
2026-07-12 performance review against `51c1f16e10ffe825b56987573ba4eabc659c768e`.
It restores two forbidden behavior changes (Findings 1, 22), builds the
requirement-matched terminal and command/TOC evidence the review lacked
(Findings 2/3/21, 4), finishes the deferred optimizations (Findings 7, 11–14,
16, 17, 23, 25, 32, 33, and the seven Finding-35 residuals), and implements
Architecture Decisions A (feature-local evidence behind one fixture manifest)
and B (one exhaustive `ComposeOptions` field classification driving
purpose-specific identities).

Opaque `ReferenceGraph` correctness (Finding 18) is **out of scope** — it is
owned by the linked [Opaque Reference Graph](../2026-07-15-reference-graph/plan.md)
feature. This plan only *coordinates* with it on the shared field-classification
authority (Architecture Decision B / Phase 4).

## Source Map (verified at `bc1c148f26eae1bba36fc1f926298a52573d83bd`)

Symbol names and paths are authoritative; line numbers are navigation hints at
the pinned commit and must be refreshed before each implementation phase.

| Concern | Location | Finding |
|---------|----------|---------|
| `detect_timezone_with_options(probe_ntp)` / bare `detect_timezone()` (delegates to `false` — must restore `true`) | `sniff/lib/src/os/time.rs:429`, `:508` | 1 |
| Darkmatter local-only caller (keep `false`) | `darkmatter/lib/src/markdown/compose/context/capture/datetime.rs:129` | 1 |
| OSC 10 text-color cache; OSC support session cache | `biscuit-terminal/lib/src/discovery/osc_queries/mod.rs:73` (`TEXT_COLOR_CACHE`), `:105`; `.../osc_queries/support.rs:11` | 2 |
| macOS color-mode / prose-theme probe | `darkmatter/lib/src/markdown/highlighting/themes.rs:412` `detect_prose_theme`, `:473` `detect_color_mode`; terminal build `biscuit-terminal/lib/src/terminal.rs:51` | 21 |
| Compose CLI shared terminal `OnceCell` | `darkmatter/cli/src/commands/compose.rs:191` (`term_cell`) | 3 |
| TOC newline offset table + `partition_point` | `darkmatter/lib/src/markdown/toc/mod.rs:193` `newline_offset_table`, `:210` `line_at_offset` | 4 |
| Frontmatter interpolation fixpoint; per-iteration ref extract + seed clone | `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs:362` `interpolate_frontmatter_impl`; repeated call sites at `:431`/`:469`/`:559`; helpers `extract_frontmatter_key_refs` at `:871`, `collect_deferred_key_references` at `:964` | 11 |
| Expression `Option<ResolutionContext>` owned clone | `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs:19`, `:128` `resolution_context()`; `functions/mod.rs:45` `ContextFn` | 12 |
| Text `replace:` matcher + char-index vector | `darkmatter/lib/src/markdown/compose/replacement.rs:88` `apply_replacements`, `:105` `build_replacement_rules`, `:165` `scan_and_replace`; stage `inline/replacement.rs:19` | 13 |
| Literal `{{{` conversion + guard | `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs:69` `convert_literals`, `:201` guard; `frontmatter_interpolation.rs:217` `convert_frontmatter_literals` | 14 |
| `options_hash` / `effective_state_hash` / `context_hash` / overlay hashes | `darkmatter/lib/src/markdown/compose/cache/hashing.rs:141`/`:87`/`:103`/`:150`/`:168` | 7,16,AD-B,35 |
| `ComposeOptions` owning module | `darkmatter/lib/src/markdown/compose/context/options.rs:44` | AD-B,7,16 |
| Transclusion key producer + persistent-key consumers | `.../transclusion/engine.rs:1335` (`options_hash` at `:1336`); `.../cache/runtime.rs:68` (`PersistentContext`, consumed by persistent read/write key assembly) | 7,16,AD-B |
| Preflight cached-directive reuse | `.../compose/preflight/mod.rs:122` (`canonical_key`), `:140` (`child_for_source`) | 7,16 |
| Shell 10ms polling loops (**two**) | `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:245`/`:309`, `:577`/`:634` | 17 |
| Shell policy snapshot cloned per directive | `.../shell_expansion/mod.rs:188` `shell_runtime.snapshot()`; `.../shell_expansion/types.rs:1060` `ShellExpansionRuntime::snapshot` | 32 |
| Per-code-block environment/theme resolution | `darkmatter/lib/src/markdown/render_tree/code_renderer.rs:176` `code_theme_from_env`, `:231` resolution chain; render entry points `render_tree/entrypoints.rs:612`, `:771` | 23 |
| Cleanup two-stage pipeline; placeholder + line passes; reflow | `darkmatter/lib/src/markdown/cleanup/mod.rs:214` `cleanup_content_internal`; `strip_incidental_newlines` calls; `cleanup/reflow.rs` | 25 |
| Directory-hash vendor exclusion | `darkmatter/lib/src/markdown/fs.rs:8` `SKIPPED_VENDOR_DIRS`, `:35` pruning condition | 22 |
| Remote discovery per-expression prefix rescan | `darkmatter/lib/src/markdown/compose/remote.rs:287` `discover_remote_urls_from_expressions`, `:311` loop calling `byte_offset_to_line` (`:446`) | 33 |
| Child-heading line lookup / repeated relevel copies | `.../transclusion/engine.rs:76` `relevel_with_overflow`, `:138` reverse replacement loop, `:181` `extract_headings` (`content[..start].lines()` at `:197`) | 35.2 |
| Fetched response body (`String` → `Arc<str>`; `Arc` already imported) | `darkmatter/lib/src/markdown/compose/remote_fetch.rs:38` `FetchSlot::Ready`, `:447` `get_content` clone; `cache/remote_cache.rs:58` fetch outcome handoff | 35.3 |
| `::toc-linking` target reads across graph + compose runtimes | `darkmatter/lib/src/markdown/reference/graph.rs:36`/`:49` (`ReferenceAnalysisRuntime` + run-local load), `:487` child load; `.../transclusion/engine.rs:1103`–`:1134` direct read + TOC-heading load | 35.4 |
| `md hash --diff` / `--save` | `darkmatter/cli/src/commands/hash.rs:33`,`:141` `run_hash_diff`, `:164` `run_hash_save`; lib `plan_hash_save`/`apply_hash_save` | 35.5 |
| `normalize_body_rhythm` (ANSI-strip per line) | `darkmatter/lib/src/layout/page.rs:1423` (called `:950`) | 35.6 |
| Link/image URL/title policy application | `darkmatter/lib/src/markdown/render_tree/build_context.rs:375` `apply_link_policy`, `:411` `apply_image_policy` | 35.7 |
| `md delta` command | `darkmatter/cli/src/commands/mod.rs:175`; lib `markdown/delta/mod.rs` | 35 (ctx) |

### Confirmed constraints from the read-through

- Two independent 10 ms polling loops exist in `executor.rs` (`try_wait` at
  `:245`/`:577`, sleeps at `:309`/`:634`); the Finding 17 fix must replace
  **both**.
- `remote_fetch.rs` already imports `Arc`; the Finding-35.3 `Arc<str>` change is
  centered on `FetchSlot::Ready { body }`, the owned `get_content` facade, and
  the `RemoteFetchOutcome` handoff from `cache/remote_cache.rs`.
- `toc_linking/mod.rs` re-reads targets with `read_to_string` in multiple spots,
  but its `process_toc_linking` helper is currently test-only. Finding 35.4's
  production duplication is between `ReferenceAnalysisRuntime::load_markdown`
  and the transclusion engine's direct source/hash + TOC-heading reads; optimize
  those paths rather than the legacy helper.
- The existing `cache::hashing::options_hash` is the incumbent that
  Architecture Decision B must **replace or delegate**, not run parallel to.
- `magic_paths` and `env_path_whitelist` are ordered vectors whose order can
  affect lookup/normalization behavior. They must never be sorted for identity;
  only genuinely unordered maps/sets are canonicalized by sorting.
- The existing OSC probe infrastructure already lives in
  `biscuit-terminal/lib/examples/discovery_probe.rs` and
  `biscuit-terminal/lib/tests/common/pty.rs`; Finding 2 extends that path rather
  than creating a second PTY abstraction.

---

## Standing Contracts (apply to every phase)

These are not a phase; they gate every checkpoint. Each optimization task must
honor them or record an explicit disposition.

- **Compatibility invariants (spec §Compatibility and Correctness Invariants
  1–8):** compose Markdown, validation results, rendered output, graph/CLI JSON,
  diagnostics, and exit status stay byte-for-byte and error-for-error
  compatible; this follow-up introduces no new public Rust API shape change and
  preserves Finding 29's already-approved ownership exception plus owned
  compatibility facade; caches include every semantic input and are bounded or
  run-local and concurrency-safe; internal borrowing never weakens an owned
  public facade without an approved exception; all code compiles and behaves on
  macOS, Linux, and Windows.
- **Benchmark & evidence contract (spec §Benchmark and Evidence Contract):**
  before measuring, every checkpoint declares target operation + control groups,
  fixture identity/size, build profile/commands/environment/host/TTY mode,
  warm-up/sample count/statistic/dispersion, and the minimum repeatable win +
  maximum permitted control regression. Baseline and candidate use identical
  source/fixture/harness bytes except the code under test. Raw samples retained.
  A no-repeatable-win finding closes through a recorded no-win disposition **and
  removal of the unnecessary code**.
- **Hashing authority:** Darkmatter Markdown-aware hashing (`md hash`) for
  Markdown identities; `biscuit-hash` xxHash for non-Markdown or whole-file
  byte identity. No ad hoc hashing.
- **Cache-identity encoding:** use a versioned, typed, length-delimited canonical
  encoder. Preserve ordered collections; sort only genuinely unordered values;
  distinguish `None` from empty values and field/type boundaries; never join
  unescaped `Debug`/display strings. A changed encoding uses a new cache-key
  domain so legacy persistent entries cannot be read under new semantics.
- **Test-tier contract:** L2 is reserved for behavior requiring a real terminal
  or PTY and runs only through `just test-l2`; spawning an ordinary child process
  remains L1. Browser-rendering behavior runs through the headless
  `just test-browser` tier. Do not add Level 3 host-input coverage.
- **Cross-platform gate honesty (spec §Verification Matrix):** OS-divergent
  paths (F17 shell wait primitive, the F2/F3/F21 PTY helper, F22 traversal)
  **require** a real non-macOS behavioral run, not just a cross-compile.
  OS-identical paths state that identity in their disposition and treat Windows
  compile evidence + the macOS behavioral run + ordinary Linux CI as sufficient.
  Classify from the code actually changed, not the finding number.
- **No write-mode formatter** is authorized. `cargo fmt --check` (read-only)
  and `git diff --check` only.

---

## Preflight — source freshness, ownership, and impact

- [ ] Record the current commit and working-tree state; preserve unrelated
  changes. Compare it with `source_map_commit`; treat the Source Map as a
  reviewed starting point, not a substitute for resolving each named symbol and
  confirming its current role immediately before its phase.
- [ ] Confirm the linked Opaque Reference Graph prerequisite/ownership boundary
  before touching `ComposeOptions` or cache identity. One feature owns the
  classification landing commit; this plan consumes it.
- [ ] Before editing any function, method, class, or other indexed symbol, run
  GitNexus upstream `impact` for that symbol and record direct callers,
  processes, modules, and risk. Warn and stop for owner direction if risk is
  HIGH or CRITICAL. Documentation/fixture-only changes do not require symbol
  impact analysis.
- [ ] Before each measured optimization, confirm its fixture entry, run-record
  location, target/control operations, threshold, and immediate baseline commit
  are frozen.

---

## Phase 1 — Compatibility corrections (Findings 1 & 22)

Revert the two forbidden behavior changes. Both are small, independent, and
high-priority; landing them early re-establishes invariants 3 and 4 before the
larger optimization work builds on top.

### Finding 1 — Restore the Sniff timezone compatibility boundary (Work 1)
- [ ] In `sniff/lib/src/os/time.rs:508`, restore bare `detect_timezone()` to delegate to `detect_timezone_with_options(true)` (full NTP-reporting convenience API). Align its rustdoc.
- [ ] Keep Darkmatter's explicit `detect_timezone_with_options(false)` call at `capture/datetime.rs:129` unchanged.
- [ ] Add a narrow Sniff-internal decision seam (for example, an injected probe function below both public entry points) so Sniff tests prove the bare API selects `true` and the configurable API respects both values without making a live NTP request.
- [ ] Add a Darkmatter-local injectable wrapper or equivalent seam around its Sniff call and prove the production path selects `false`; do not depend on Sniff's `cfg(test)` instrumentation crossing the crate boundary, use a brittle source-text assertion, or introduce a live network dependency in ordinary compose tests.
- [ ] Gates: Sniff `just test` + `just lint`; Darkmatter context tests + `just test` + `just lint`. (This is a filesystem/network-adjacent but OS-identical logic change — Windows compile + macOS run + Linux CI sufficient; state so in `results.md`.)

### Finding 22 — Restore directory-hash membership (Work 8)
- [ ] Remove the unconditional `node_modules` / `target` / `vendor` exclusion in `darkmatter/lib/src/markdown/fs.rs` (`SKIPPED_VENDOR_DIRS` skip at `:24`) so aggregate membership matches pre-optimization behavior.
- [ ] Add an **end-to-end CLI** test that freezes the aggregate hash, diagnostics, and exit status for a tree containing directories named `node_modules`/`target`/`vendor`.
- [ ] Confirm no hash-migration step is needed (the exclusion was never released; any aggregate under it is a private working-tree artifact). Record in `results.md` that a *future* opt-in ignore policy would require owner approval + migration semantics.
- [ ] Gates: Darkmatter `just test` + `just lint`. F22 traversal/path handling is OS-divergent → record behavioral runs of the CLI aggregate test on macOS, Linux, and Windows.

**Parallelizable:** Finding 1 (Sniff + Darkmatter caller) and Finding 22
(Darkmatter fs + CLI) touch disjoint code and can proceed concurrently.

### Checkpoint 1
Bare `sniff::detect_timezone()` reports full NTP status again; Darkmatter compose
still performs no NTP probe. `md hash <dir>` includes Markdown under
`node_modules`/`target`/`vendor` exactly as before the perf change. Both areas
green on `just test` + `just lint`; macOS, Linux, and Windows behavioral evidence
recorded for F22.

---

## Phase 2 — Evidence infrastructure & command/TOC closeout (AD-A + Finding 4)

Establish the feature-local evidence home and fixture-manifest schema that every
measured checkpoint consumes. Reconstruct the reproducible historical
command/TOC closeout that Review 3 rejected for using different, unhashed
fixture bytes. **Blocks every measured checkpoint** in later phases.

- [ ] Create `darkmatter/features/2026-07-15-performance-followup/results.md` as the disposition + evidence index (one row per retained partial/open/correction finding or sub-item, including evidence-only gaps; disposition, evidence location, and cross-platform classification columns). (AD-A)
- [ ] Create a sibling `benchmarks/` directory holding the immutable fixture **manifest** as the single authority for fixture identity, plus either committed fixtures or a checked-in **deterministic generator** (record generator version + exact command). (AD-A, Work 3)
- [ ] Define the manifest schema up front. Each fixture entry records generator version/command, exact byte size + structural counts, Darkmatter frontmatter/body hashes for Markdown, and a `biscuit-hash` xxHash whole-file identity where byte identity is required. Preserve ordered fixture collections. (Work 3)
- [ ] Keep per-run facts out of the immutable fixture identity: dated run records under `benchmarks/raw/<checkpoint>/<run-id>/` record baseline/candidate commits, commands, release profile, host, environment, TTY mode, warm-up, sample count, statistic/dispersion, thresholds, and raw-result files. `results.md` links each disposition to its run record. Declare the threshold before capturing the baseline. (AD-A, Work 3)
- [ ] Populate the Phase-2 fixture set with `md --help`, render, hash, trivial compose, schema/transclusion compose, the three TOC size tiers, and code-heavy render cases. Later phases may add checkpoint-specific fixtures only by registering and hashing them **before** that checkpoint's baseline is captured. (Work 3)
- [ ] Record the three runner contracts in `results.md`: existing Criterion recipes for library microbenchmarks, a release CLI runner for command-level measurement, and the existing Biscuit Terminal probe/PTY path extended in Phase 3. Each records commands, raw samples, environment; each consumes the shared manifest for file fixtures. Do **not** force CLI/PTY evidence through `just bench`. (AD-A)
- [ ] Historical F4 closeout: create isolated temporary worktrees for the pre-optimization baseline `83aaecc8f` and audit commit `51c1f16e10ffe825b56987573ba4eabc659c768e`; build with the same toolchain/lockfile policy and release profile; run both against the **same immutable fixture directory on the same host**; record threshold pass/fail. These pins reconstruct the accumulated 2026-07-12 result only — they are **not** the baseline/candidate pair for this follow-up's changes. (Work 3)
- [ ] Add TOC unit/property coverage confirming line/span behavior over the manifest fixtures (guards the non-quadratic `line_at_offset` path). (Work 3, verification matrix F4)

The manifest schema and deterministic generator can be prepared together. Do
not begin the historical builds until their fixture entries and run-record
contract are frozen.

### Checkpoint 2
`results.md`, the fixture manifest, and the Phase-2 fixtures exist and are
internally consistent (recomputed hashes match recorded ones). The F4 historical
closeout reproduces on identical bytes and meets its predeclared thresholds,
with raw samples retained. No production behavior changes in this phase; test
and benchmark-support edits are permitted.

---

## Phase 3 — Requirement-matched terminal evidence (Findings 2, 3, 21) — Work 2

Extend the checked-in Biscuit Terminal probe/PTY path so it observes OSC
requests independent of the user's shell theme, then add the CLI
single-detection case. This is the evidence gap Review 3 flagged as "wrong
level". Depends on the Phase 2 evidence home for recording latency artifacts.

- [ ] Extend `biscuit-terminal/lib/examples/discovery_probe.rs` and `biscuit-terminal/lib/tests/common/pty.rs`; do **not** add a second generic PTY abstraction. Put the assertions in a `level2_*` test binary and run them only through `just test-l2`. Unix-only `expectrl`/PTY code is target-gated so Windows compiles and records a clean unsupported/skip disposition. (spec Work 2.6)
- [ ] Run the cache proof in a dedicated child process so prior `OnceLock` state or test ordering cannot contaminate it. Construct two or more `Terminal` values, manufacture a response only for the first OSC 10 request, and assert exactly one request plus the same cached first response on later constructions. This proves reuse rather than coincidental equality. (Work 2.1/2.2)
- [ ] Record repeated-construction latency with warm-up, sample count, and dispersion into the feature-local evidence index. (Work 2.3)
- [ ] Add an isolated CLI probe case: one `md compose` invocation rendering verbose + performance + warning output performs **one** terminal detection (exercises the `compose.rs:191` `term_cell` `OnceCell`). Count the emitted detection/query requests rather than inferring from equal rendered output. (Work 2.4)
- [ ] Verify macOS appearance discovery (`detect_color_mode`/`detect_prose_theme`) does **not** spawn for fully redirected output. Keep this redirected-process assertion L1; it does not require a PTY. Serialize environment mutation with the repository test guard. (Work 2.5, Finding 21)
- [ ] Report interactive (PTY) and piped (redirected CLI) measurements **separately**. No Level 3 input-protocol test. (spec Work 2)

The probe protocol lands first. The Biscuit Terminal cache proof and Darkmatter
CLI case may then use it independently without sharing a process or global
cache state.

### Checkpoint 3
Biscuit Terminal + Darkmatter CLI `just test` / `just test-l2` / `just lint`
green. The L2 artifact shows one OSC 10 request across N constructions and one
detection per `md compose`; interactive vs piped latencies recorded separately;
Windows still compiles and records the Unix-PTY skip disposition. Linux provides
the required real non-macOS L2 behavior evidence.

---

## Phase 4 — Consume the shared `ComposeOptions` classification (Architecture Decision B)

Land Architecture Decision B exactly once. The linked
[Opaque Reference Graph](../2026-07-15-reference-graph/plan.md) Phase 1 owns the
crate-private exhaustive classification, the two purpose-specific identity
products, and the `options_hash` migration. This feature depends on that shared
prerequisite and must not implement a competing inventory. **Blocks Phase 5.**

- [ ] Land or merge the linked feature's shared prerequisite before this feature changes a compose reuse boundary. Record the prerequisite commit in `results.md`.
- [ ] Confirm the owning implementation destructures `ComposeOptions` **with no `..`** and derives both `ReferenceGraphOptionsIdentity` and the compose-cache fingerprint from that one field inventory. No third fingerprint or parallel field list is allowed. (AD-B)
- [ ] Confirm ordered vectors (`magic_paths`, `env_path_whitelist`, and any other order-sensitive sequences) retain order. Sort only genuinely unordered maps/sets such as `exclude_keys`, `pre_approved_commands`, allowed-host sets, and canonical context/env maps. (AD-B)
- [ ] Confirm the typed, length-delimited encoding distinguishes field/type boundaries, `None`, and empty values; uses a versioned domain marker; contains no `Debug` encoding; and hashes through `biscuit-hash` xxHash. Add delimiter-collision and `None`/empty regression tests. (AD-B)
- [ ] Confirm process-local state participates only in run-local reuse. The run-local key distinguishes independently constructed stateful instances but remains stable across clones of the same `Arc`, without increasing strong counts. Process-local identity bytes never enter a persistent key; when they are required, persistent reads **and** writes are disabled. When equivalence cannot be established, reject reuse. (AD-B)
- [ ] Replace or delegate `cache::hashing::options_hash` and migrate the direct producer in `transclusion/engine.rs` plus the persistent-key consumers in `cache/runtime.rs` under a new cache-key domain/version. Audit preflight-state participation through the shared classification rather than treating preflight as an `options_hash` call site. Prove a legacy persistent entry cannot be read under the new encoding. (AD-B)
- [ ] Tests cover equal identities across unordered insertion order; unequal identities across ordered-vector reordering and representative scalar/collection/context/schema/transclusion/remote/shell families; clone stability; fresh-instance inequality; and persistent-cache ineligibility for process-local state. The no-`..` destructure is the field-addition guard.

This phase is sequential with the linked feature's provenance work: the shared
prerequisite has one owner and one landing commit. Performance-follow-up work
begins only after that commit is present.

### Checkpoint 4
Darkmatter `just test` + `just lint` green. Exactly one `ComposeOptions` field
inventory exists (the no-`..` classification); `options_hash` is gone or a thin
delegate; no `Debug`-based option encoding remains; legacy cache entries cannot
cross the new domain; and stateful keys cannot touch persistent storage.
Rendered/diagnostic behavior remains byte-identical, while cache reuse may
become conservatively narrower by design.

---

## Phase 5 — Cross-pass compose reuse (Findings 7, 16, 35.1 & 35.4) — Work 4

Finish the remaining validate/preflight/compose duplication using a cache key
whose identity contains **every** semantic input. Depends on Phase 4 (AD-B
compose-cache fingerprint).

- [ ] Audit the existing transclusion key (`options_hash` + source + effective state + context + directive-overlay identities) before changing any reuse boundary. Confirm it now derives from the AD-B classification, not the retired `Debug`-based encoding.
- [ ] Implement reuse in the spec's preferred order, stopping at the first safe level: (1) share parsed source + reference metadata; (2) share context-independent prepared representations; (3) share fully rendered content **only** if a complete semantic identity is demonstrated; (4) otherwise retain recomposition and record a same-fixture **no-win** disposition for narrower candidates. (Work 4)
- [ ] Preserve condition-aware behavior: do not reuse bodies whose output depends on parent state, directive position, conditions, or lifecycle decisions. (Findings 7/16)
- [ ] The cache is run-local or bounded; retains no unrelated contexts, graphs, callbacks, or runtimes. Because transclusion composes children **concurrently**, any shared prepared-content cache is **concurrency-safe or partitioned per compose run** — no data race, no lock held across child composition. (Work 4)
- [ ] **35.1:** compute `effective_state_hash` once per transclusion phase and thread the value through directive cache-key construction. This belongs here because Phase 5 owns that key's assembly and measurement.
- [ ] **35.4:** route `::toc-linking` graph-discovery and composition reads through the same compose-run-owned source cache without broadening persistent reuse. `ReferenceAnalysisRuntime` currently constructs its own `RunLocalCache`; two caches of the same type do not satisfy this item. Thread one owner through the production graph and transclusion paths, while preserving authoritative-read and invalidation behavior. This belongs here because it is another cross-pass reuse boundary.
- [ ] Add a compose benchmark comparing immediate pre-change vs candidate on identical manifest fixtures; declare thresholds per the evidence contract.
- [ ] Record separate target/control dispositions for the general F7/F16 reuse, 35.1 hash hoisting, and 35.4 read reuse so an aggregate result cannot hide a regression.
- [ ] Verification (matrix F7/F16/F35): reference, preflight, transclusion, `::toc-linking`, condition, lifecycle, source-cache, and cache-identity suites pass. Use deterministic L1 concurrency tests with barriers/timeouts to prove concurrent child progress and lock release; an ordinary child process does not make this L2.

### Checkpoint 5
Darkmatter `just test` + `just lint` green. Compose/validation output remains
byte-identical. Each reuse item shows a repeatable win or a recorded no-win
disposition with speculative code removed. No lock is held across concurrent
child composition. Classify 35.4 from its filesystem implementation; do not
categorize the whole phase as OS-identical merely because its cache is
run-local.

---

## Phase 6 — Frontmatter & expression rework (Findings 11–14) — Work 5

Four separate benchmark checkpoints share one fixture set. F12 changes the
context path used by the interpolation work and lands first. F11 and F14 then
proceed as coordinated edits because both touch
`frontmatter_interpolation.rs`; F13 remains independent. Each closes on its own
baseline/candidate comparison.

### F11 — Incremental frontmatter interpolation fixpoint
- [ ] In `frontmatter_interpolation.rs`, extract each templated key's dependencies **once**; maintain unresolved-dependency counts + reverse edges; enqueue newly eligible keys. Avoid rebuilding the full seed map per successful key where mutation can be incremental. Preserve cycles, shell deferral, best-effort propagation, and key-scoped errors.

### F12 — Borrowed/shared `ResolutionContext`
- [ ] Add an internal borrowed/shared path for evaluators and expression functions (`resolve_ctx.rs`, `functions/mod.rs::ContextFn`), retaining the owned public facade where compatibility requires it. No public owned-return API change without an approved exception.

### F13 — Faster exact multi-pattern replacement
- [ ] Benchmark an exact multi-pattern matcher in `replacement.rs` against the current canonical precedence (descending key byte length, then ascending lexical order). **Reject** any design changing left-to-right non-overlapping matching, the choice at a shared start position, non-recursive replacement output, UTF-8 character-boundary behavior, empty-key omission, or scalar-value coercion. If no win, record a requirement-matched no-win result and remove speculative code.

### F14 — Reduced literal / interpolation rescans
- [ ] In `interpolation/rewrite.rs` and the frontmatter literal-conversion path, reduce repeated Markdown-aware scans and full-body copies when interpolation is present; construct output once per interpolation depth where practical. Nested interpolation keeps semantic fixpoint behavior; it does **not** authorize rescanning unrelated protected ranges. Benchmark nested and no-expression cases **separately**.

- [ ] Before any Phase-6 baseline, register and hash the shared fixtures: wide dependency graphs, deep dependency chains, cycles, shell-pending keys, best-effort errors, many replacement rules, Unicode, code fences, literal escapes, multiline indentation, and nested interpolation. (spec Work 5)
- [ ] Verification (matrix F11–F14): focused units + compose integration + scale benchmarks per checkpoint. F12 can reach filesystem-backed expression functions → classify its cross-platform gate from the actual changed path, not categorically OS-identical.

**Dependency order:** F12 → coordinated F11/F14. F13 may proceed independently
after the fixture set is frozen. Capture each immediate baseline before its
code change so one checkpoint cannot contaminate another's comparison.

### Checkpoint 6
All four checkpoints have either a threshold-meeting benchmark or a recorded
no-win disposition with speculative code removed. Compose output byte-identical.
Darkmatter `just test` + `just lint` green. Cross-platform disposition recorded
per checkpoint (F12 assessed from its changed path).

---

## Phase 7 — Shell polling & policy clones (Findings 17 & 32) — Work 6

OS-divergent — **requires a real non-macOS behavioral run** of the wait
primitive. Independent of Phases 4–6.

### F17 — Replace the 10 ms completion polling loops
- [ ] Replace or avoid **both** independent 10 ms `try_wait`/`sleep` loops in `shell_expansion/executor.rs` (`try_wait` at `:245`/`:577`) with a blocking wait primitive or event-driven notification available on all supported OSes. Any platform split is **target-gated and tested**.
- [ ] Preserve concurrent stdout/stderr draining while waiting; do not replace polling with a wait path that can deadlock on a full pipe. Prove unchanged timeout boundaries/granularity, saturated dual-stream capture, descendant/process cleanup, failure/error selection, and source-order execution for both executor variants. Arbitrary directive parallelism remains prohibited.

### F32 — Snapshot shell policy once per stage
- [ ] Move snapshot ownership to the stage orchestrator in `shell_expansion/mod.rs` and plumb one `ShellRuntimeSnapshot` from `shell_expansion/types.rs` through directive authorization. The matching helpers in `policy.rs` remain borrowed consumers. Do **not** hold the policy mutex across parsing, approval, or command execution.
- [ ] Define the visibility contract explicitly: all directives admitted to one stage see its opening immutable policy snapshot; approvals/persistence produced during that stage update the runtime but become policy input only for a subsequent stage. Add tests for both halves of that contract.

- [ ] Verification (matrix F17/F32): cross-platform L1 process/policy tests plus timeout, stream-saturation, and cleanup tests. An ordinary spawned command is not L2; add/run L2 only if the implementation introduces a real-terminal requirement. Record Linux **and Windows** behavioral evidence for the wait primitive.

**Parallelizable:** F17 (executor wait mechanism) and F32 (policy snapshot) touch
disjoint files and can proceed concurrently.

### Checkpoint 7
Shell directives still execute in source order with identical timeout/output/
cleanup/failure semantics; no mutex is held across execution. Darkmatter
`just test` + `just lint` green, plus `just test-l2` only if a terminal-specific
test was actually added. Real Linux and Windows behavioral runs of the wait
primitive are recorded.

---

## Phase 8 — Render & cleanup sub-items (Findings 23 & 25) — Work 7

Independent of Phases 4–7.

### F23 — Resolve code theme once per render snapshot
- [ ] Introduce a render-scoped theme/environment snapshot at the render-tree entry point and carry it in `TerminalCodeRenderer` (and the corresponding browser options/context) so `code_theme_from_env`, surface mode, and theme selection are resolved once per render. `output/code_block.rs` continues to receive an already-resolved highlighter; it is not the environment-discovery owner. Preserve explicit per-`CodeBlock` theme overrides. Separate direct `CodeBlock`, `DarkmatterPage`, terminal, and browser render invocations must still observe environment changes allowed by the existing contract.
- [ ] Serialize environment-mutating tests with the repository guard. Add a multi-block assertion proving one snapshot per render and a two-render assertion proving permitted environment changes are observed between renders.

### F25 — Cleanup pass fusion (profile-gated)
- [ ] First profile individual cleanup passes (`cleanup/mod.rs`, placeholder + line passes, `reflow.rs`) on representative documents. Combine line passes **only** when ordering and boundary behavior can be made exactly equivalent; preserve exact pass ordering and canonical output. A same-fixture no-win (fusion within noise, or added allocation/complexity without a repeatable end-to-end gain) is an acceptable disposition.

- [ ] Verification (matrix F23/F25): snapshot/golden output; headless browser computed-style/markup tests for F23; L2 terminal frames only where a real terminal is required; code-heavy render + cleanup benchmarks over manifest fixtures.

**Parallelizable:** F23 (render theme snapshot) and F25 (cleanup profiling) are
independent.

### Checkpoint 8
Terminal + browser render output and cleanup canonical output byte-identical
(existing snapshots pass untouched). F23 resolves theme once per render; F25 has
a threshold-meeting fusion or a recorded no-win. Darkmatter `just test` +
`just test-browser` + `just lint` green, plus `just test-l2` only for any
real-terminal assertion. Classify cross-platform behavior from the final
environment/terminal implementation rather than predeclaring the phase
OS-identical.

---

## Phase 9 — Remote discovery line positions (Finding 33) — Work 9

Independent of Phases 4–8.

- [ ] Retain the cheap no-HTTP guard in `remote.rs`. For documents that **do** contain remote expressions, replace the per-expression `byte_offset_to_line` prefix rescan (`:311` loop) with **one forward pass** or a shared offset table (reuse the TOC-style `newline_offset_table` approach).
- [ ] Verify byte offsets at LF, CRLF, Unicode, start/end-of-file, and multiple expressions on one line.
- [ ] Benchmark a remote-heavy input (immediate pre-change vs candidate, identical bytes).
- [ ] Verification (matrix F33): focused behavior tests + one target/control benchmark.

### Checkpoint 9
Remote-URL discovery produces identical line positions on all edge cases;
remote-heavy benchmark meets threshold or records a no-win. Darkmatter
`just test` + `just lint` green. Record the actual diff's cross-platform
classification; a pure byte/line scan may use the OS-identical disposition only
when no filesystem, URL-runtime, or `cfg`-specific path changed.

---

## Phase 10 — Remaining Finding 35 residual sub-items — Work 10

Five residual sub-items remain after 35.1 and 35.4 move into Phase 5. Each needs
its **own** behavioral tests and measurement disposition — a single aggregate
benchmark may **not** conceal a no-win or regression in an individual path.
Capture immediate baselines sequentially even where implementation files are
disjoint.

- [ ] **35.2** In `relevel_with_overflow`, compute heading line positions in one forward pass and apply all heading edits with one output construction rather than copying the whole child for every replacement (`transclusion/engine.rs:76`, `:138`, `:181`). Preserve byte-identical output, overflow-warning lines/order, and the unchanged/zero-adjustment fast paths.
- [ ] **35.3** Store fetched response bodies as `Arc<str>` internally, preserving the owned `get_content` facade where required (`remote_fetch.rs:38` `FetchSlot::Ready`, clone at `:447`, and the `cache/remote_cache.rs` outcome handoff; `Arc` already imported).
- [ ] **35.5** Within each mutually exclusive `md hash --diff` or `--save` invocation, compute each unique `(kind, effective MdHashOptions)` artifact once and pass it through comparison/planning and explanation output. Preserve `--save`'s legitimate distinction between the stored ignore-policy comparison and the selected current-policy baseline; cache by semantic hash identity rather than assuming one artifact can serve both. Do not change stored hash semantics (`cli/commands/hash.rs`; lib `compare_hash`/`explain_hash_diff`/`plan_hash_save`/`apply_hash_save`).
- [ ] **35.6** Make `normalize_body_rhythm` avoid allocating an ANSI-stripped string for every output-line check (`layout/page.rs:1423`).
- [ ] **35.7** Borrow link/image URL + title data through `render_tree/build_context.rs::apply_link_policy` and `apply_image_policy`, including the **empty-policy fast path**, while retaining owned public `RenderNode` output. Do not redirect this work to compose-time link normalization or Markdown image-literal escaping; those are different paths.
- [ ] Per sub-item: behavioral tests + one target/control benchmark, each with its own disposition in `results.md` (implementation win or recorded no-win with code removed).
- [ ] Cross-platform classification per sub-item. Do not preclassify 35.3 as OS-identical until its remote/runtime path is inspected; the other allocation/hashing changes still require confirmation from the actual diff.

35.3, 35.5, 35.6, and 35.7 have disjoint primary files; 35.2 follows Phase 5's
transclusion edits. Even when implementation work is independent, baseline and
candidate capture remains one checkpoint at a time.

### Checkpoint 10
Every sub-item has an individual benchmark disposition (no aggregate masking).
Compose/CLI output byte-identical; `Arc<str>` and borrowing changes preserve
owned public facades. Darkmatter `just test` + `just lint` green; run
`just test-l2` only if a remaining item adds or changes real-terminal behavior.

---

## Phase 11 — Documentation, cumulative closeout, cross-platform evidence, final gates

- [ ] Add a **dated correction/supersession notice** to the old plan/results (`../../reviews/2026-07-12-perf/`), linking to this feature's audit and final dispositions. Do **not** rewrite their original body or checkboxes — they remain the historical `codex/default` record. (AD-A, Documentation Deliverables)
- [ ] Link the original review to this active follow-up **and** to the opaque graph feature.
- [ ] Confirm `results.md` records one disposition + evidence location for every retained partial/open/correction item: Findings 1–4, 7, 11–14, 16, 17, 21–23, 25, 32, 33, and all seven Finding-35 items.
- [ ] Document the restored Sniff and directory-hash compatibility behavior (rustdoc + README where behavior/supported construction changed).
- [ ] Update the audit table + `results.md` so every finding reflects its final honest disposition.
- [ ] Update the darkmatter skill (`.claude/skills/darkmatter/`) if any architecture/workflow changed; regenerate the skill `hash:` with `md hash <file>`.
- [ ] Update `darkmatter/docs/dependencies.md` (and per-area deps doc) if any crate was added/removed.
- [ ] **Cumulative closeout run:** run the **complete manifest** against the final feature head so the cumulative result includes every follow-up change (distinct from Phase 2's historical `83aaecc8f`→`51c1f16e…` reconstruction).
- [ ] **Cross-platform evidence:** record Linux behavior for the Unix PTY helper and Windows compilation + clean skip/unsupported behavior; record Linux **and Windows behavioral runs** for F17's wait primitive and F22's directory/path CLI case; use Windows compile + macOS behavior + ordinary Linux CI only for findings demonstrated OS-identical by their final diff. macOS-only success is insufficient.
- [ ] Final targeted gate matrix: `just test` + `just lint` in every touched area; `just test-l2` only in Biscuit Terminal/Darkmatter areas containing the F2/F3/F21 PTY tests; Darkmatter `just test-browser` for F23; root `just test` selectors for the touched Sniff, Darkmatter, and Biscuit Terminal packages/areas; `cargo check --workspace`; `cargo fmt --check`; `git diff --check`. Do not invoke L2 directly through Cargo/Nextest.
- [ ] Run GitNexus `detect_changes({scope: "compare", base_ref: "main"})` before any commit; confirm the blast radius is confined to the expected compose/cache/shell/render/hash/CLI + Sniff-timezone + terminal-OSC scope.

### Final acceptance (maps to spec Acceptance Criteria 1–8)
- [ ] Findings 1–4 and 21's compatibility/evidence gaps are closed.
- [ ] Findings 7, 11–14, 16, 17, 23, 25, 32, 33, and every Finding-35 sub-item has an implementation or an allowed evidence-backed disposition.
- [ ] Finding 22's membership change is reverted (no unapproved exception).
- [ ] No Finding 18 correctness work landed here; the opaque graph feature owns it with no duplication/conflict.
- [ ] Reproducible same-byte benchmark artifacts meet predeclared thresholds with raw samples retained.
- [ ] Behavioral, L1, requirement-matched L2, headless Browser, lint, workspace, formatting-check, and whitespace gates pass, with Linux and Windows evidence recorded.
- [ ] The audit table and original review documentation reflect every finding's final honest disposition.
- [ ] Architecture Decisions A and B are implemented: immutable fixture identity and dated run records remain feature-local behind focused runners; graph provenance and compose caching derive purpose-specific identities from the one exhaustive `ComposeOptions` field classification owned by the linked prerequisite.
