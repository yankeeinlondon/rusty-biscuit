---
agent: "claude/"
total_phases: 11
created: 2026-07-15
phase: 1
yolo: "true"
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

## Source Map (verified against HEAD)

| Concern | Location | Finding |
|---------|----------|---------|
| `detect_timezone_with_options(probe_ntp)` / bare `detect_timezone()` (delegates to `false` — must restore `true`) | `sniff/lib/src/os/time.rs:429`, `:508` | 1 |
| Darkmatter local-only caller (keep `false`) | `darkmatter/lib/src/markdown/compose/context/capture/datetime.rs:129` | 1 |
| OSC 10 text-color cache; OSC support session cache | `biscuit-terminal/lib/src/discovery/osc_queries/mod.rs:72` (`TEXT_COLOR_CACHE`), `:104`; `.../osc_queries/support.rs:11` | 2 |
| macOS color-mode / prose-theme probe | `darkmatter/lib/src/markdown/highlighting/themes.rs:412` `detect_prose_theme`, `:473` `detect_color_mode`; terminal build `biscuit-terminal/lib/src/terminal.rs:51` | 21 |
| Compose CLI shared terminal `OnceCell` | `darkmatter/cli/src/commands/compose.rs:191` (`term_cell`) | 3 |
| TOC newline offset table + `partition_point` | `darkmatter/lib/src/markdown/toc/mod.rs:193` `newline_offset_table`, `:210` `line_at_offset` | 4 |
| Frontmatter interpolation fixpoint; per-iteration ref extract + seed clone | `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs:362` `interpolate_frontmatter_impl`, `:431` `collect_deferred_key_references`, `:469` `extract_frontmatter_key_refs`, seeds `:405`/`:421`/`:442` | 11 |
| Expression `Option<ResolutionContext>` owned clone | `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs:19`, `:128` `resolution_context()`; `functions/mod.rs:45` `ContextFn` | 12 |
| Text `replace:` matcher + char-index vector | `darkmatter/lib/src/markdown/compose/replacement.rs:88` `apply_replacements`, `:90` `build_replacement_rules`, `:96` `scan_and_replace`; stage `inline/replacement.rs:19` | 13 |
| Literal `{{{` conversion + guard | `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs:69` `convert_literals`, `:197` guard; `frontmatter_interpolation.rs:217` `convert_frontmatter_literals` | 14 |
| `options_hash` / `effective_state_hash` / `context_hash` / overlay combine | `darkmatter/lib/src/markdown/compose/cache/hashing.rs:135`/`:87`/`:103`/`:220` | 7,16,AD-B,35 |
| `ComposeOptions` owning module | `darkmatter/lib/src/markdown/compose/context/options.rs:44` | AD-B,7,16 |
| Transclusion reuse + runtime cache options_hash | `.../transclusion/engine.rs:1341`; `.../cache/runtime.rs:72` (used `:761`,`:957`) | 7,16 |
| Preflight cached-directive reuse | `.../compose/preflight/mod.rs:80` (`canonical_key`) | 7,16 |
| Shell 10ms polling loops (**two**: `:242` and `:574`) | `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:242`, `:574` | 17 |
| Shell policy rule clones per directive | `.../shell_expansion/policy.rs:161`/`:234`/`:257`/`:271` `normalize_command` | 32 |
| Syntect theme per code block; theme set + resolve | `darkmatter/lib/src/markdown/output/code_block.rs:62`/`:84`; `highlighting/themes.rs:377` `THEME_SET`, `:442` `detect_code_theme` | 23 |
| Cleanup two-stage pipeline; placeholder + line passes; reflow | `darkmatter/lib/src/markdown/cleanup/mod.rs:233`; `strip_incidental_newlines` calls; `cleanup/reflow.rs:123` | 25 |
| Directory-hash vendor exclusion | `darkmatter/lib/src/markdown/fs.rs:8` `SKIPPED_VENDOR_DIRS`, `:24` skip in `read_dir` | 22 |
| Remote discovery per-expression prefix rescan | `darkmatter/lib/src/markdown/compose/remote.rs:287` `discover_remote_urls_from_expressions`, `:307` loop calling `byte_offset_to_line` | 33 |
| Heading offsets / releveling | `.../transclusion/engine.rs:59` `find_preceding_heading_level`, `:76` `relevel_with_overflow`, `:310` memoized offset table; `markdown/mod.rs:947` `relevel` | 35.2 |
| Fetched response body (`String` → `Arc<str>`; `Arc` already imported) | `darkmatter/lib/src/markdown/compose/remote_fetch.rs:38` `Ready { body }` | 35.3 |
| `::toc-linking` target reads (multiple `read_to_string`) | `darkmatter/lib/src/markdown/compose/toc_linking/mod.rs:148`, `:161`; `parser.rs:141` | 35.4 |
| `md hash --diff` / `--save` | `darkmatter/cli/src/commands/hash.rs:33`,`:141` `run_hash_diff`, `:164` `run_hash_save`; lib `plan_hash_save`/`apply_hash_save` | 35.5 |
| `normalize_body_rhythm` (ANSI-strip per line) | `darkmatter/lib/src/layout/page.rs:1423` (called `:950`) | 35.6 |
| Link/image URL/title policy application | `.../compose/link_normalization.rs:29` `normalize_links`; `markdown/mod.rs:1016` `build_markdown_image_literal`, `:1039` `escape_markdown_title` | 35.7 |
| `md delta` command | `darkmatter/cli/src/commands/mod.rs:175`; lib `markdown/delta/mod.rs` | 35 (ctx) |

### Confirmed constraints from the read-through

- Two independent 10 ms polling loops exist in `executor.rs` (`:242` and
  `:574`); the Finding 17 fix must replace **both**.
- `remote_fetch.rs` already imports `Arc`; the Finding-35.3 `Arc<str>` change is
  localized to `RemoteFetchOutcome::Ready { body }` plus its `Ready.body`
  consumers.
- `toc_linking/mod.rs` re-reads targets with `read_to_string` in multiple spots
  — the candidate for Finding-35.4 run-cache routing.
- The existing `cache::hashing::options_hash` is the incumbent that
  Architecture Decision B must **replace or delegate**, not run parallel to.

---

## Standing Contracts (apply to every phase)

These are not a phase; they gate every checkpoint. Each optimization task must
honor them or record an explicit disposition.

- **Compatibility invariants (spec §Compatibility and Correctness Invariants
  1–8):** compose Markdown, validation results, rendered output, graph/CLI JSON,
  diagnostics, and exit status stay byte-for-byte and error-for-error
  compatible; the Finding 29 `Arc<Value>` exception is the *only* public Rust
  API shape change; caches include every semantic input and are bounded or
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
- **Cross-platform gate honesty (spec §Verification Matrix):** OS-divergent
  paths (F17 shell wait primitive, the F2/F3/F21 PTY helper, F22 traversal)
  **require** a real non-macOS behavioral run, not just a cross-compile.
  OS-identical paths state that identity in their disposition and treat Windows
  compile evidence + the macOS behavioral run + ordinary Linux CI as sufficient.
  Classify from the code actually changed, not the finding number.
- **No write-mode formatter** is authorized. `cargo fmt --check` (read-only)
  and `git diff --check` only.

---

## Phase 1 — Evidence infrastructure & command/TOC closeout (AD-A + Finding 4)

Establish the feature-local evidence home and the single fixture manifest that
every later checkpoint consumes. Reconstruct the reproducible historical
command/TOC closeout that Review 3 rejected for using different, unhashed
fixture bytes. **Blocks every measured checkpoint** in later phases.

- [ ] Create `darkmatter/features/2026-07-15-performance-followup/results.md` as the disposition + evidence index (one row per open finding/sub-item; disposition, evidence location, and cross-platform classification columns). (AD-A)
- [ ] Create a sibling `benchmarks/` directory holding the fixture **manifest** as the single authority for fixture identity, plus either committed fixtures or a checked-in **deterministic generator** (record generator version + exact command). (AD-A, Work 3)
- [ ] Populate the manifest per spec: generator version/command; exact byte size + structural counts for every fixture; Darkmatter frontmatter/body hash identities for Markdown fixtures; `biscuit-hash` xxHash whole-file identity where byte identity is required; commands, release profile, host facts, TTY mode, warm-up, sample count, raw-result locations; predeclared improvement and no-regression thresholds. (Work 3)
- [ ] Fixture coverage must include at minimum: `md --help`, render, hash, trivial compose, schema/transclusion compose, the three TOC size tiers, and the code-heavy render cases. (Work 3)
- [ ] Record the three runner contracts in `results.md`: existing Criterion recipes for library microbenchmarks, a release CLI runner for command-level measurement, and (built in Phase 3) the checked-in PTY/L2 helper. Each records commands, raw samples, environment; each consumes the shared manifest for file fixtures. Do **not** force CLI/PTY evidence through `just bench`. (AD-A)
- [ ] Historical F4 closeout: build the **before** binary from pre-optimization baseline `83aaecc8f` and the **after** binary from audit commit `51c1f16e10ffe825b56987573ba4eabc659c768e`; run both against the **same immutable fixture directory**; record threshold pass/fail. Note in `results.md` that these pins reconstruct the accumulated 2026-07-12 result only — they are **not** the baseline/candidate pair for this follow-up's own changes. (Work 3)
- [ ] Add TOC unit/property coverage confirming line/span behavior over the manifest fixtures (guards the non-quadratic `line_at_offset` path). (Work 3, verification matrix F4)

**Parallelizable:** manifest authoring, the deterministic generator, and the
historical before/after binary builds are independent once the `benchmarks/`
directory layout is agreed. The Criterion and CLI runner contracts can be drafted
concurrently; the PTY runner contract is finalized in Phase 3.

### Checkpoint 1
`results.md` and `benchmarks/manifest` exist and are internally consistent
(recomputed hashes match recorded ones). The F4 historical closeout reproduces
on identical bytes and meets its predeclared thresholds, with raw samples
retained. No production source changed yet.

---

## Phase 2 — Compatibility corrections (Findings 1 & 22)

Revert the two forbidden behavior changes. Both are small, independent, and
high-priority; landing them early re-establishes invariants 3 and 4 before the
larger optimization work builds on top.

### Finding 1 — Restore the Sniff timezone compatibility boundary (Work 1)
- [ ] In `sniff/lib/src/os/time.rs:508`, restore bare `detect_timezone()` to delegate to `detect_timezone_with_options(true)` (full NTP-reporting convenience API). Align its rustdoc.
- [ ] Keep Darkmatter's explicit `detect_timezone_with_options(false)` call at `capture/datetime.rs:129` unchanged.
- [ ] Sniff tests: bare API selects `true`; configurable API respects both `true` and `false`.
- [ ] Darkmatter source-path test proving Darkmatter selects `false`; confirm no live network dependency in ordinary compose tests.
- [ ] Gates: Sniff `just test` + `just lint`; Darkmatter context tests + `just test` + `just lint`. (This is a filesystem/network-adjacent but OS-identical logic change — Windows compile + macOS run + Linux CI sufficient; state so in `results.md`.)

### Finding 22 — Restore directory-hash membership (Work 8)
- [ ] Remove the unconditional `node_modules` / `target` / `vendor` exclusion in `darkmatter/lib/src/markdown/fs.rs` (`SKIPPED_VENDOR_DIRS` skip at `:24`) so aggregate membership matches pre-optimization behavior.
- [ ] Add an **end-to-end CLI** test that freezes the aggregate hash, diagnostics, and exit status for a tree containing directories named `node_modules`/`target`/`vendor`.
- [ ] Confirm no hash-migration step is needed (the exclusion was never released; any aggregate under it is a private working-tree artifact). Record in `results.md` that a *future* opt-in ignore policy would require owner approval + migration semantics.
- [ ] Gates: Darkmatter `just test` + `just lint`. F22 traversal/path handling is OS-divergent → **requires a real non-macOS behavioral run** of the CLI aggregate test (Linux at minimum), plus Windows.

**Parallelizable:** Finding 1 (Sniff + Darkmatter caller) and Finding 22
(Darkmatter fs + CLI) touch disjoint code and can proceed concurrently.

### Checkpoint 2
Bare `sniff::detect_timezone()` reports full NTP status again; Darkmatter compose
still performs no NTP probe. `md hash <dir>` includes Markdown under
`node_modules`/`target`/`vendor` exactly as before the perf change. Both areas
green on `just test` + `just lint`; Linux evidence recorded for F22.

---

## Phase 3 — Requirement-matched terminal evidence (Findings 2, 3, 21) — Work 2

Build the checked-in L2 PTY helper that observes OSC requests independent of the
user's shell theme, and the CLI single-detection case. This is the evidence gap
Review 3 flagged as "wrong level". Depends on the Phase 1 evidence home for
recording latency artifacts.

- [ ] Add a checked-in L2 helper that runs under a supported real PTY and can observe OSC requests without depending on a user's shell theme. Unix-only PTY code **target-gated** so Windows continues to compile. (spec Work 2.6)
- [ ] Verify: two or more `Terminal` constructions in one process emit **one** OSC 10 query (`TEXT_COLOR_CACHE` reuse). (Work 2.1)
- [ ] Verify the cached response is genuinely reused, not merely equal by coincidence. (Work 2.2)
- [ ] Record repeated-construction latency with warm-up, sample count, and dispersion into the feature-local evidence index. (Work 2.3)
- [ ] Add a CLI case: one `md compose` invocation rendering verbose + performance + warning output performs **one** terminal detection (exercises the `compose.rs:191` `term_cell` `OnceCell`). (Work 2.4)
- [ ] Verify macOS appearance discovery (`detect_color_mode`/`detect_prose_theme`) does **not** spawn for fully redirected output. (Work 2.5, Finding 21)
- [ ] Report interactive (PTY) and piped (redirected CLI) measurements **separately**. No Level 3 input-protocol test. (spec Work 2)

**Parallelizable:** the PTY OSC-count helper (biscuit-terminal side) and the CLI
single-detection case (darkmatter-cli side) are independent once the L2 helper
skeleton lands.

### Checkpoint 3
Biscuit Terminal + Darkmatter CLI `just test` / `just test-l2` / `just lint`
green. The L2 artifact shows one OSC 10 request across N constructions and one
detection per `md compose`; interactive vs piped latencies recorded separately;
Windows still compiles (PTY target-gated). Linux L2 evidence recorded.

---

## Phase 4 — Shared `ComposeOptions` classification authority (Architecture Decision B)

Define the single crate-private, exhaustive field classification that both graph
provenance and compose caching derive from. **Blocks Phase 5** (cross-pass
reuse). Requires **coordination with the linked
[Opaque Reference Graph](../2026-07-15-reference-graph/plan.md) feature**, whose
`ReferenceGraphOptionsIdentity` must consume this same classification in the
coordinated change.

- [ ] In the `ComposeOptions` owning module (`compose/context/options.rs`), add a crate-private field-classification authority that destructures `ComposeOptions` **with no `..`** — a new field is a compile error until classified. (AD-B)
- [ ] Derive `ReferenceGraphOptionsIdentity` from the classification: conservative, fail-closed; may use weak/minimal instance handles for stateful callbacks/runtimes; may include output-irrelevant fields. (Coordinate: the reference-graph feature owns the type; this feature ensures the shared classification is its source.) (AD-B)
- [ ] Derive the **compose-cache value fingerprint** from the classification: only canonical value semantics relevant to the cached artifact, combined with the existing source, effective-state, context, directive-overlay, and pass-scope dimensions. (AD-B)
- [ ] Canonical value encoding uses field names, type boundaries, sorted unordered collections (`magic_paths`, `exclude_keys`, `env_path_whitelist`, `pre_approved_commands`, allowed hosts, captured context/env), and a versioned domain marker. It **must not** use `Debug` output. Hash with `biscuit-hash` xxHash. (AD-B)
- [ ] Process-local identity from a stateful field participates **only** in run-local reuse: a key that depends on pointer/instance identity must never read or write a persistent cache entry. When equivalence cannot be established, **reject reuse**. (AD-B)
- [ ] Replace or delegate the existing `cache::hashing::options_hash` — do **not** add a parallel third options fingerprint. Update its call sites (`cache/runtime.rs:72`, `transclusion/engine.rs`, preflight). (AD-B)
- [ ] Unit tests: classification identity equal across unordered-collection insertion orders; unequal across representative option families (scalar, collection, context, schema, transclusion, remote, shell); clone-stable including a set `Arc`-backed stateful field (shared instance equal, fresh instance unequal). The no-`..` destructure is the field-addition guard.

**Sequential within phase:** the classification lands first; the two derived
identity products and the `options_hash` replacement build on it.
**Parallelizable across features:** the reference-graph feature's consumption of
the classification proceeds concurrently once the authority signature is agreed.

### Checkpoint 4
Darkmatter `just test` + `just lint` green. Exactly one `ComposeOptions` field
inventory exists (the no-`..` classification); `options_hash` is gone or a thin
delegate; no `Debug`-based option encoding remains. Existing compose/cache
behavior byte-identical (no reuse-boundary change yet — identity products are
in place but Phase 5 wires the new reuse).

---

## Phase 5 — Cross-pass compose reuse (Findings 7 & 16) — Work 4

Finish the remaining validate/preflight/compose duplication using a cache key
whose identity contains **every** semantic input. Depends on Phase 4 (AD-B
compose-cache fingerprint).

- [ ] Audit the existing transclusion key (`options_hash` + source + effective state + context + directive-overlay identities) before changing any reuse boundary. Confirm it now derives from the AD-B classification, not the retired `Debug`-based encoding.
- [ ] Implement reuse in the spec's preferred order, stopping at the first safe level: (1) share parsed source + reference metadata; (2) share context-independent prepared representations; (3) share fully rendered content **only** if a complete semantic identity is demonstrated; (4) otherwise retain recomposition and record a same-fixture **no-win** disposition for narrower candidates. (Work 4)
- [ ] Preserve condition-aware behavior: do not reuse bodies whose output depends on parent state, directive position, conditions, or lifecycle decisions. (Findings 7/16)
- [ ] The cache is run-local or bounded; retains no unrelated contexts, graphs, callbacks, or runtimes. Because transclusion composes children **concurrently**, any shared prepared-content cache is **concurrency-safe or partitioned per compose run** — no data race, no lock held across child composition. (Work 4)
- [ ] Add a compose benchmark comparing immediate pre-change vs candidate on identical manifest fixtures; declare thresholds per the evidence contract.
- [ ] Verification (matrix F7/F16): reference, preflight, transclusion, condition, lifecycle, and cache-identity suites pass; compose benchmark recorded.

### Checkpoint 5
Darkmatter `just test` + `just test-l2` + `just lint` green. Compose/validation
output byte-identical. The reuse-cache benchmark shows a repeatable win **or** a
recorded no-win disposition with the speculative code removed. No lock is held
across concurrent child composition (assert via test where a real process is
involved). This is an OS-identical allocation/caching change → state identity;
Windows compile + macOS run + Linux CI sufficient (F12-style filesystem reach
does not apply here).

---

## Phase 6 — Frontmatter & expression rework (Findings 11–14) — Work 5

Four separate checkpoints sharing a fixture set. Independent of Phase 4/5 and of
each other's core logic (they touch distinct modules), so the four can be
implemented in parallel; each closes on its own benchmark.

### F11 — Incremental frontmatter interpolation fixpoint
- [ ] In `frontmatter_interpolation.rs`, extract each templated key's dependencies **once**; maintain unresolved-dependency counts + reverse edges; enqueue newly eligible keys. Avoid rebuilding the full seed map per successful key where mutation can be incremental. Preserve cycles, shell deferral, best-effort propagation, and key-scoped errors.

### F12 — Borrowed/shared `ResolutionContext`
- [ ] Add an internal borrowed/shared path for evaluators and expression functions (`resolve_ctx.rs`, `functions/mod.rs::ContextFn`), retaining the owned public facade where compatibility requires it. No public owned-return API change without an approved exception.

### F13 — Faster exact multi-pattern replacement
- [ ] Benchmark an exact multi-pattern matcher in `replacement.rs` against the current ordered rules. **Reject** any design changing first-rule precedence, overlap, cascading, Unicode indices, or empty-pattern handling. If no win, record a requirement-matched no-win result and remove speculative code.

### F14 — Reduced literal / interpolation rescans
- [ ] In `interpolation/rewrite.rs`, reduce repeated Markdown-aware scans and full-body copies when interpolation is present; construct output once per interpolation depth where practical. Nested interpolation keeps semantic fixpoint behavior; it does **not** authorize rescanning unrelated protected ranges. Benchmark nested and no-expression cases **separately**.

- [ ] Shared fixtures (spec Work 5): wide dependency graphs, deep dependency chains, cycles, shell-pending keys, best-effort errors, many replacement rules, Unicode, code fences, literal escapes, multiline indentation, nested interpolation. Register in the manifest.
- [ ] Verification (matrix F11–F14): focused units + compose integration + scale benchmarks per checkpoint. F12 can reach filesystem-backed expression functions → classify its cross-platform gate from the actual changed path, not categorically OS-identical.

**Parallelizable:** F11, F12, F13, F14 are four independent work streams over
distinct modules; they converge only on the shared fixture manifest entry.

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
- [ ] Replace or avoid **both** independent 10 ms `try_wait`/`sleep` loops in `shell_expansion/executor.rs` (`:242` and `:574`) with a blocking wait primitive or event-driven notification available on all supported OSes. Any platform split is **target-gated and tested**.
- [ ] Prove unchanged timeout boundaries, captured output, process cleanup, failure/error selection, and source-order execution. Arbitrary directive parallelism remains prohibited.

### F32 — Snapshot shell policy once per stage
- [ ] In `shell_expansion/policy.rs`, take one immutable stage snapshot (or share immutable collections) instead of cloning read-only rule collections per directive. Do **not** hold a policy mutex across command execution.
- [ ] Tests: all directives in a stage see the intended stable policy; a subsequent stage observes an allowed policy update.

- [ ] Verification (matrix F17/F32): cross-platform process/policy tests; timeout + cleanup tests; L1/L2 where a real process is required. Record Linux (and Windows) behavioral evidence for the wait primitive.

**Parallelizable:** F17 (executor wait mechanism) and F32 (policy snapshot) touch
disjoint files and can proceed concurrently.

### Checkpoint 7
Shell directives still execute in source order with identical timeout/output/
cleanup/failure semantics; no mutex held across execution. Darkmatter `just test`
+ `just test-l2` + `just lint` green. **Real non-macOS run of the wait primitive
recorded** (blocking-wait vs event-driven differs by OS); Windows target-gated
path tested.

---

## Phase 8 — Render & cleanup sub-items (Findings 23 & 25) — Work 7

Independent of Phases 4–7.

### F23 — Resolve code theme once per render snapshot
- [ ] Resolve code theme + relevant environment inputs (`detect_code_theme`, color mode) **once at the start of a render** and pass the snapshot to every code block (`output/code_block.rs`), instead of reading per block. Separate render invocations must still observe environment changes allowed by the existing contract.

### F25 — Cleanup pass fusion (profile-gated)
- [ ] First profile individual cleanup passes (`cleanup/mod.rs`, placeholder + line passes, `reflow.rs`) on representative documents. Combine line passes **only** when ordering and boundary behavior can be made exactly equivalent; preserve exact pass ordering and canonical output. A same-fixture no-win (fusion within noise, or added allocation/complexity without a repeatable end-to-end gain) is an acceptable disposition.

- [ ] Verification (matrix F23/F25): snapshot/golden output; L2 terminal frames where applicable; code-heavy render + cleanup benchmarks over manifest fixtures.

**Parallelizable:** F23 (render theme snapshot) and F25 (cleanup profiling) are
independent.

### Checkpoint 8
Terminal + browser render output and cleanup canonical output byte-identical
(existing snapshots pass untouched). F23 resolves theme once per render; F25 has
a threshold-meeting fusion or a recorded no-win. Darkmatter `just test` +
`just test-l2` + `just lint` green. OS-identical → state identity; Windows
compile + macOS run + Linux CI sufficient.

---

## Phase 9 — Remote discovery line positions (Finding 33) — Work 9

Independent of Phases 4–8.

- [ ] Retain the cheap no-HTTP guard in `remote.rs`. For documents that **do** contain remote expressions, replace the per-expression `byte_offset_to_line` prefix rescan (`:307` loop) with **one forward pass** or a shared offset table (reuse the TOC-style `newline_offset_table` approach).
- [ ] Verify byte offsets at LF, CRLF, Unicode, start/end-of-file, and multiple expressions on one line.
- [ ] Benchmark a remote-heavy input (immediate pre-change vs candidate, identical bytes).
- [ ] Verification (matrix F33): focused behavior tests + one target/control benchmark.

### Checkpoint 9
Remote-URL discovery produces identical line positions on all edge cases;
remote-heavy benchmark meets threshold or records a no-win. Darkmatter
`just test` + `just lint` green. OS-identical → state identity.

---

## Phase 10 — Finding 35 residual sub-items — Work 10

Seven independent sub-items. Each needs its **own** behavioral tests and
measurement disposition — a single aggregate benchmark may **not** conceal a
no-win or regression in an individual path. Independent of Phases 4–9.

- [ ] **35.1** Compute `effective_state_hash` **once per transclusion phase**, not once per `::file` directive (`cache/hashing.rs:87`, consumers in `transclusion/engine.rs`).
- [ ] **35.2** Build heading line offsets once and emit releveling spans/output **without copying the whole child once per heading** (`transclusion/engine.rs:59`/`:76`/`:310`; `markdown/mod.rs:947` `relevel`).
- [ ] **35.3** Store fetched response bodies as `Arc<str>` internally, preserving the owned public facade where required (`remote_fetch.rs:38` `Ready { body }` + `Ready.body` consumers; `Arc` already imported).
- [ ] **35.4** Route `::toc-linking` target reads through the **run cache** so one target is not read independently by graph discovery and composition (`toc_linking/mod.rs:148`/`:161`, `parser.rs:141`).
- [ ] **35.5** Reuse one document-hash computation across `md hash --diff` and `--save` (incl. explanation output), without changing stored hash semantics (`cli/commands/hash.rs`; lib `plan_hash_save`/`apply_hash_save`).
- [ ] **35.6** Make `normalize_body_rhythm` avoid allocating an ANSI-stripped string for every output-line check (`layout/page.rs:1423`).
- [ ] **35.7** Borrow link/image URL + title data through policy application, including the **empty-policy fast path**, while retaining owned public output nodes (`compose/link_normalization.rs:29`; `markdown/mod.rs:1016`/`:1039`).
- [ ] Per sub-item: behavioral tests + one target/control benchmark, each with its own disposition in `results.md` (implementation win or recorded no-win with code removed).
- [ ] Cross-platform classification per sub-item (all appear OS-identical — pure alloc/caching/hashing — but confirm from the changed path; 35.3/35.4 touch fetch/fs).

**Parallelizable:** all seven sub-items touch disjoint code and can be
implemented concurrently.

### Checkpoint 10
Every sub-item has an individual benchmark disposition (no aggregate masking).
Compose/CLI output byte-identical; `Arc<str>` and borrowing changes preserve
owned public facades. Darkmatter `just test` + `just test-l2` + `just lint`
green.

---

## Phase 11 — Documentation, cumulative closeout, cross-platform evidence, final gates

- [ ] Add a **dated correction/supersession notice** to the old plan/results (`../../reviews/2026-07-12-perf/`), linking to this feature's audit and final dispositions. Do **not** rewrite their original body or checkboxes — they remain the historical `codex/default` record. (AD-A, Documentation Deliverables)
- [ ] Link the original review to this active follow-up **and** to the opaque graph feature.
- [ ] Confirm `results.md` records one disposition + evidence location for **every** open sub-item (Findings 7, 11–14, 16, 17, 23, 25, 32, 33, all seven Finding-35 items).
- [ ] Document the restored Sniff and directory-hash compatibility behavior (rustdoc + README where behavior/supported construction changed).
- [ ] Update the audit table + `results.md` so every finding reflects its final honest disposition.
- [ ] Update the darkmatter skill (`.claude/skills/darkmatter/`) if any architecture/workflow changed; regenerate the skill `hash:` with `md hash <file>`.
- [ ] Update `darkmatter/docs/dependencies.md` (and per-area deps doc) if any crate was added/removed.
- [ ] **Cumulative closeout run:** run the **complete manifest** against the final feature head so the cumulative result includes every follow-up change (distinct from Phase 1's historical `83aaecc8f`→`51c1f16e…` reconstruction).
- [ ] **Cross-platform evidence:** record Linux **and** Windows evidence per the targeted gate — real non-macOS behavioral runs for F17, the F2/F3/F21 PTY helper, and F22; Windows-compile + macOS-run + Linux-CI dispositions for the OS-identical findings. macOS-only success is insufficient.
- [ ] Final gate matrix: `just test`, `just test-l2`, `just lint` in **every touched area**; root recipes for cross-package changes (Sniff + Darkmatter + Biscuit Terminal); `cargo check --workspace`; `cargo fmt --check`; `git diff --check`.
- [ ] Run GitNexus `impact` on each edited symbol before its change and `detect_changes({scope: "compare", base_ref: "main"})` before any commit; confirm the blast radius is confined to the expected compose/cache/shell/render/hash/CLI + Sniff-timezone + terminal-OSC scope, and report HIGH/CRITICAL risk if surfaced.

### Final acceptance (maps to spec Acceptance Criteria 1–8)
- [ ] Findings 1–4's compatibility/evidence gaps are closed.
- [ ] Findings 7, 11–14, 16, 17, 23, 25, 32, 33, and every Finding-35 sub-item has an implementation or an allowed evidence-backed disposition.
- [ ] Finding 22's membership change is reverted (no unapproved exception).
- [ ] No Finding 18 correctness work landed here; the opaque graph feature owns it with no duplication/conflict.
- [ ] Reproducible same-byte benchmark artifacts meet predeclared thresholds with raw samples retained.
- [ ] Behavioral, L1, L2, lint, workspace, formatting-check, and whitespace gates pass, with Linux and Windows evidence recorded.
- [ ] The audit table and original review documentation reflect every finding's final honest disposition.
- [ ] Architecture Decisions A and B are implemented: evidence is feature-local behind one fixture manifest + focused runners; graph provenance and compose caching derive purpose-specific identities from one exhaustive `ComposeOptions` field classification.
