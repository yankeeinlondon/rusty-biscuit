---
agent: "claude"
total_phases: 11
created: "2026-07-14"
phase: 1
yolo: "true"
---

# Execution Plan: Darkmatter Performance Review (2026-07-12)

Derived from [`spec.md`](./spec.md). The spec catalogs **1 critical, 5 high,
12 medium, 15 low** findings across `darkmatter/lib`, `darkmatter/cli`, and the
cross-package hot paths in `sniff` and `biscuit-terminal`.

## Strategy

- **Phase 1 builds the measurement harness first.** Every subsequent phase has a
  concrete before/after benchmark checkpoint. The spec explicitly recommends
  landing criterion benches before the allocation-tier work.
- **Phases 2–5 are mutually independent** (NTP / terminal / schema / toc touch
  disjoint subsystems) and may run in parallel by different implementers once
  Phase 1 lands. They are ordered by value, not dependency.
- **Phases 6–10 are the deeper architectural + allocation-tier work**, ordered
  so cache/dedup foundations land before the items that depend on them
  (e.g. schema-tier cleanups in Phase 9 depend on Phase 4).
- **Phase 11 is the closeout**: re-run the full baseline, confirm the quadratic
  toc regression is gone, and update docs/memory.

### Global validation gates (run at every phase checkpoint)

- `just test` (unit) and `just test-l2` (integration) from `darkmatter/`
- `just lint` from `darkmatter/`
- For cross-package phases, also run `just test <pkg>` from repo root for
  `sniff` and `biscuit-terminal`
- `cargo check --workspace` — new schema/enum variants have historically
  drift-broken claudine's exhaustive matches
- **Behavior invariant:** `md compose`, `md schema validate`, and render output
  must be byte-identical before/after every performance fix unless the spec
  explicitly notes a semantic change (only Finding 1 changes an observable:
  no more NTP probe).

---

## Phase 1 — Benchmark & Measurement Foundation

**Goal:** Reproducible before/after harness so every fix is validated, not
assumed. No production behavior changes.

- [ ] Capture the current baseline table from the spec locally on this host:
      run `hyperfine` for `md --help`, `md small.md`, `md large.md`,
      `md hash small.md`, `md compose small.md`,
      `md compose --no-trigger-schemas`, against a **release** build; record to
      `reviews/2026-07-12-perf/baseline.md`.
- [ ] Capture the toc scaling baseline: `md toc --json` at 81 KB / 326 KB /
      1.3 MB fixtures; record wall times (expect ~203 ms / 2.24 s / 45.3 s).
- [ ] Add a criterion bench for `compose_with` on a fixture that exercises
      frontmatter interpolation + `$schema` + one transclusion (the regressed
      path with no existing coverage).
- [ ] Add a criterion bench for `as_terminal` / `DarkmatterPage::render` on a
      code-heavy 100 KB document.
- [ ] Confirm `md compose --perf` attribution still reports the documented
      segments (capture context / validate references / build options / compose
      pipeline / schema validation) — this is the primary harness for validating
      Findings 1, 5, 6, 7.
- [ ] **Checkpoint:** benches compile and run under `just`; baseline artifacts
      committed to the review folder.

> Parallelizable: the two criterion benches can be authored concurrently.

---

## Phase 2 — Critical: Eliminate the per-compose NTP network probe (Finding 1)

**Goal:** Remove the live `sntp time.apple.com` round-trip from every compose.
Expected: compose drops ~127 ms → ~70 ms; removes the offline 3 s stall.

**Cross-package:** `darkmatter/lib` + `sniff/lib`.

- [ ] In `darkmatter/lib/src/markdown/compose/context/capture/datetime.rs:123`,
      call `sniff::os::detect_timezone_with_options(false)` (NTP not needed —
      darkmatter only uses timezone-abbreviation derivation; `ntp_status` is
      never surfaced by any `ctx.*` key).
- [ ] Fix the stale comment at `capture/mod.rs:42` ("zero-cost local
      computation") to match reality (drift: code is authority).
- [ ] Evaluate making `sniff::os::detect_timezone()` default to
      `probe_ntp: false` — a network probe is a surprising default for a
      "detect timezone" call. If changed, audit sniff callers and update
      `sniff` skill/docs. (Ken preference: strategic fix over tactical; flag as
      a separate reviewable change if blast radius is non-trivial.)
- [ ] **Checkpoint:** `md compose --perf` on a no-`ctx.*` document shows
      "capture context" drop from ~60 ms to near-zero; compose total ≈ 70 ms.
      Run `just test`/`just test-l2` for both `darkmatter` and `sniff`.

---

## Phase 3 — Terminal Detection Caching (Findings 2, 3, 21)

**Goal:** Detect the terminal once per process; stop round-tripping the tty and
spawning subprocesses on every `Terminal` construction.

**Cross-package:** `biscuit-terminal/lib` + `darkmatter/lib` + `darkmatter/cli`.

- [ ] **(F2)** Add a `TEXT_COLOR_CACHE: OnceLock` mirroring `BG_COLOR_CACHE` in
      `biscuit-terminal/lib/src/discovery/osc_queries/mod.rs:93-100`; ideally
      batch OSC 10 + OSC 11 in one raw-mode session.
- [ ] **(F21)** Cache `color_mode()` per process and skip the
      `defaults read -g AppleInterfaceStyle` spawn when not a TTY —
      `biscuit-terminal/lib/src/discovery/detection/color.rs:193-208`.
- [ ] **(F2, lib side)** Cache one detected `Terminal` per process in a
      `LazyLock<Terminal>` and clone it in
      `terminal_options_from_terminal_options` / `ambient_terminal_width`;
      stop `render_tree_terminal` building a fresh `Terminal::default()` on
      every render — `darkmatter/lib/src/render_tree/entrypoints.rs:574`,
      `layout/page.rs:1479-1481`.
- [ ] **(F3, CLI side)** Introduce a CLI-level `LazyLock<Terminal>` and thread
      it through `run_compose`'s verbose/`-vv`/warnings/deferred-report
      constructions — `darkmatter/cli/src/commands/compose.rs:294,505,522,580,592`,
      `main.rs:82,119`, `frontmatter.rs:153`.
- [ ] **(F3)** Move `Terminal::new()` inside the non-JSON branch of `toc` so
      `md toc --json` never builds an unused `Terminal` —
      `darkmatter/cli/src/commands/mod.rs:158-171`.
- [ ] **Checkpoint:** verify caching via an L2 test that constructs multiple
      terminals and asserts a single OSC query (see `biscuit-test-harness`
      patterns); confirm no SGR/behavioral regression in captured frames.
      Run `just test`/`just test-l2`/`just lint` for `biscuit-terminal` and
      `darkmatter`.

> Ordering: land the `biscuit-terminal` cache primitives first, then the
> darkmatter/CLI consumers. The two consumer sub-tasks are parallelizable.

---

## Phase 4 — Schema Stage Double-Work (Findings 5, 6, 9, 8)

**Goal:** Stop resolving the effective schema twice, coercing twice, and
recompiling uncached validators per compose. Expected: recovers a large share
of the measured ~47 ms baseline-schema delta.

**Package:** `darkmatter/lib` (schema stage). No behavior change.

- [ ] **(F5)** In `compose/schema_validation.rs:146`, validate through the
      already-held `EffectiveSchema` (`effective.validate_with_positions(...)`)
      instead of `schemas.validate()`, which re-runs `effective_for`
      (`schemas/mod.rs:510`). Removes one resolution pass and one redundant
      coercion pass (line 128 vs `mod.rs:616` idempotent second pass).
- [ ] **(F6)** Thread the `ValidatorCache` (or prebuilt `arm_validators` from
      `build_arm_validators`, `schemas/mod.rs:1118-1132`) into
      `coerce_frontmatter_with_pending` / `coerce_property_union` so
      `coerce.rs:313` and `:436` become cache hits, not cold `jsonschema`
      compiles.
- [ ] **(F9)** Reuse the cached `BASE_JSON_SCHEMA` (`schemas/mod.rs:161-167`)
      for the default darkmatter baseline via
      `with_baseline_json_schema(darkmatter_base_json_schema())` instead of
      `with_darkmatter_baseline()`'s deep-clone + full `to_json_schema`
      re-conversion — `compose/schema_validation.rs:80-81`. Hold layers as
      `Arc<Value>` where `effective_for` currently clones (`mod.rs:388`).
- [ ] **(F8)** Promote the `ValidatorCache` to a process-level `LazyLock` (key
      already includes `base_dir`); reuse the first `run()`'s trigger registry
      for the pipeline's second `run()` via `with_trigger_registry` instead of
      re-scanning + recompiling — `compose/schema_validation.rs:76,96`,
      `pipeline/mod.rs:215,294`. Fix the "process-wide" doc claim in
      `schemas/validate.rs:49-55` to match whichever design lands.
- [ ] **Checkpoint:** `md compose --perf` shows the baseline-schema segment
      collapse; add/extend a schema-stage unit test asserting `effective_for`
      and coercion each run once per `run()` (e.g. via a call counter or the
      perf attribution). Byte-identical `md schema validate` output. Run
      `cargo check --workspace` (schema surface touches claudine).

> Ordering within phase: F5 first (it exposes the held `EffectiveSchema` the
> others reuse), then F6/F9 in parallel, then F8 (cache-lifetime design change)
> as the follow-on.

---

## Phase 5 — TOC O(n²) → O(n log n) (Finding 4)

**Goal:** Kill the quadratic per-event prefix scan. Expected: 326 KB from
2.24 s → milliseconds; the 1.3 MB tier from 45.3 s → sub-second.

**Package:** `darkmatter/lib`.

- [ ] Replace the per-event `content[..range.start].lines().count()` in
      `markdown/toc/mod.rs:210-211` (and `:232`) with a precomputed sorted
      line-start offset table + binary search, or incremental line tracking
      (offsets are mostly increasing). Compute the line only in the event arms
      that actually consume it.
- [ ] Verify all downstream consumers still get correct line numbers: `md toc`,
      `md hash --kind structured/detailed`, `md hash --diff/--save`
      explanations, and the reference graph's heading indexes.
- [ ] **Checkpoint:** re-run the toc scaling test (81 KB / 326 KB / 1.3 MB) —
      the 4×-size→11×-time relationship must become roughly linear. Add a
      regression unit test asserting toc line numbers on a multi-heading fixture
      are unchanged. `just test`/`just test-l2`.

---

## Phase 6 — Compose Multi-Walk Deduplication (Findings 7, 16, 18)

**Goal:** Stop preparing the document tree multiple times per compose. Biggest
architectural item; benefits claudine most. Requires care around cache-key
correctness.

**Package:** `darkmatter/lib` (+ CLI compose entry).

- [ ] **(F7)** Reuse the preflight graph for reference validation (or share the
      `RunLocalCache`/prepared content across the validate → preflight → compose
      passes) so interpolation/page-block work runs once per node, not ~2× —
      `cli/commands/compose.rs:267-310,399-424,426`,
      `reference/graph.rs:804-825`.
- [ ] **(F16)** Carry prepared/interpolated content in the preflight graph so
      the terminal compose does not re-run frontmatter interpolation +
      replacement + body interpolation per visited doc; store children as
      indices or `Arc` instead of deep-cloning each `PreflightGraphNode` into
      both `edges[i].child` and `children` — `compose/preflight/collect.rs:241,
      355,375,392-393,509`.
- [ ] **(F18)** Add a `validate_graph(&graph, ...)` entry point so
      `md graph --validate` accepts the already-built graph instead of calling
      `build_reference_graph` internally (twice, three times with
      `--fragments`); cache prepared heading slugs per canonical target path for
      the run so `validate_cross_doc_fragment` stops re-reading/re-composing —
      `reference/file_tree/mod.rs:225-247`, `reference/validate.rs:315-318,605,
      616-625,687-689`.
- [ ] **Checkpoint:** `md compose --perf` on a multi-file composition shows
      per-node prepare work drop to ~1×; `md graph --validate` graph-build count
      drops. Assert composed output byte-identical on a multi-transclusion
      fixture. `just test`/`just test-l2`.

> This phase depends on nothing structurally but is best sequenced after
> Phase 4 so the schema-stage cache reuse is already correct when compose walks
> collapse.

---

## Phase 7 — Compose Allocation & Re-Parse Reductions (Findings 10–15, 17, 30–35)

**Goal:** Cut the ~8–10 full-body copies and ~4–6 redundant pulldown-cmark
re-parses per compose. Opportunistic; land against the Phase 1 benches.

**Package:** `darkmatter/lib` (compose subsystem).

- [ ] **(F10)** Memoize the env/agent-overridden ctx map once (`OnceLock`) at
      construction; serve `get_effective` by reference instead of cloning the
      whole map per `ctx.*` lookup/presence-test —
      `compose/context/runtime.rs:219-221,255-256`.
- [ ] **(F12)** Return `Option<&ResolutionContext>` (or `Arc`) from
      `resolution_context()` and fetch only after the fs-function name match —
      pure functions like `length()` must stop cloning the full ctx map —
      `compose/expression/mod.rs:687`, `context/effective_state.rs:374-376`.
- [ ] **(F11)** Precompute per-key expression refs before the frontmatter
      interpolation fixpoint loop and mutate one seed state incrementally to
      remove the O(k²) map clones + re-parses —
      `compose/frontmatter_interpolation.rs:457-529,565-566`.
- [ ] **(F13)** Replace the per-position `starts_with` rule probe with an
      `aho_corasick` leftmost-longest matcher built once from the keys; return
      `Cow<str>` from `apply_replacements` (no-op on zero matches) —
      `compose/replacement.rs:92,165-198`.
- [ ] **(F14)** Return expressions + literals from one body-interpolation scan;
      skip `convert_literals` when there are zero `{{{ }}}` literals; emit
      between spans instead of `replace_range` —
      `compose/interpolation/rewrite.rs:69-80,104-110,147`.
- [ ] **(F15)** Extract parent headings once per transclusion phase and
      binary-search by offset instead of re-parsing from byte 0 per
      `::file`/`::url` directive — `compose/transclusion/engine.rs:59-73`.
- [ ] **(F17)** Apply the frontmatter path's prepare-serial / execute-parallel
      split (rayon) to body `::shell` directives and replace the 10 ms sleep-poll
      with a blocking `wait` on a thread + channel; preserve output order —
      `compose/inline/shell_expansion.rs:41-58`,
      `shell_expansion/executor.rs:240-309`.
- [ ] **(F30)** Add a map-based `doc.*` resolver that walks by reference and
      clones only the leaf, instead of rebuilding the full effective state as a
      `Value::Object` per lookup — `context/effective_state.rs:182-184`.
- [ ] **(F31)** Stringify the first `get` result directly in the interpolation
      evaluator's non-array arm instead of discarding it and re-running the
      lookup — `compose/interpolation/evaluator.rs:242-247`.
- [ ] **(F32)** Check whitelist/blacklist rules under the shared lock (or
      snapshot once per stage) instead of cloning three rule collections per
      directive — `shell_expansion/mod.rs:188`, `types.rs:1060-1067`.
- [ ] **(F33)** Single forward pass for line computation in remote-URL
      expression discovery; skip discovery when a `memchr` probe finds no
      `http` — `compose/pipeline/mod.rs:72-78`, `compose/remote.rs:287-307`.
- [ ] **(F34)** Compare xxHash (via `biscuit-hash`) before/after — or have the
      cleanup passes report modification — instead of a full-body copy + compare
      for the `cleanup_changed` flag — `compose/pipeline/phases.rs:80,114`.
- [ ] **(F35)** Assorted single-copy costs: hash effective state once per
      transclusion phase (`transclusion/engine.rs:1316-1319`); avoid per-heading
      full-child copies and byte-0 line counts in `relevel_with_overflow` /
      `extract_headings` (`:149-154,164,197`); store `RemoteFetchRuntime`
      response bodies as `Arc<str>` (`remote_fetch.rs:408`); read
      `::toc-linking` targets through the run cache (`reference/graph.rs:919`).
- [ ] **Checkpoint:** `compose_with` criterion bench improves on the
      100 KB+ fixture; composed output byte-identical across the fixture suite.
      `just test`/`just test-l2`/`just lint`.

> Highly parallelizable: each finding is an isolated edit in a distinct module.
> Split across implementers; land behind the Phase 1 bench. F35 sub-items are
> independent of each other.

---

## Phase 8 — Render-Path Reductions (Findings 19, 20, 23, 24)

**Goal:** Trim the render path's redundant second parse and per-token/per-block
allocations. Renders are already fast in absolute terms — these are the largest
remaining single wins after Finding 2.

**Package:** `darkmatter/lib` (render tree + code emit) + syntect theme handling.

- [ ] **(F19)** Track fence state during the initial O(n) `scan_delimiters`
      pass (or require a plausible `==` delimiter pair before parsing) so a
      document containing `a == b` in code does not pay a full second
      pulldown-cmark parse for `protected_ranges` —
      `render_tree/inline_extension.rs:241-260,355`.
- [ ] **(F20)** Push the original `(event, range)` through unchanged when no
      disclosure directive matches, instead of re-allocating every
      `Event::Text` into a boxed `String` —
      `render_tree/fold.rs:514-519,768-769,890-891`.
- [ ] **(F23)** Hold `&'static SyntectTheme` in `CodeHighlighter` instead of
      `THEME_SET.get(name).clone()` per code block; hoist `code_theme_from_env`
      out of the per-block loop — `highlighting/themes.rs:527-531`,
      `render_tree/code_renderer.rs:243,365`, `entrypoints.rs:649-651`.
- [ ] **(F24)** Replace `format!` + `push_str` with `write!(output, ...)`
      straight into the buffer in the per-token code-block emission loop —
      `output/code_block.rs:101-104,136-142,273-279,310-316`,
      `highlighting/mod.rs:189-193`.
- [ ] **Checkpoint:** `as_terminal` criterion bench improves on the code-heavy
      100 KB doc; terminal/browser code-block output byte-identical (existing
      code-block equality tests must stay green). `just test`/`just test-l2`.

> Parallelizable: F19/F20 (fold path) and F23/F24 (code emit) are independent
> pairs.

---

## Phase 9 — Schema-Tier Cleanups (Findings 26, 27, 28, 29)

**Goal:** Remove the remaining per-lookup schema serialization/parse/clone
costs. **Depends on Phase 4** — the spec notes several of these are "subsumed by"
or "fall out of" Findings 5/8/9.

**Package:** `darkmatter/lib` (schemas).

- [ ] **(F26)** Replace the per-lookup full-schema serialize + SHA-256 in
      `canonical_hash` with an xxHash via `biscuit-hash` (repo convention;
      accidental-collision resistance suffices), hashed once per stable schema —
      `schemas/validate.rs:148,948-967`.
- [ ] **(F27)** Memoize imported namespaces per canonical path on
      `ImportEngine` so `A@types.yaml`/`B@types.yaml`/`C@types.yaml` read +
      parse `types.yaml` once, not three times (+ `canonicalize` syscalls) —
      `schemas/resolve.rs:817-824,871-873,897-899,909-929`.
- [ ] **(F28)** Content-hash memoize the `example()` returns-target validator
      and cache the example file read, matching the already-memoized envelope
      validation — `schemas/resolve.rs:1271-1283`, `schemas/example.rs:224-252`.
      (Largely subsumed once Finding 5 stops the double resolution.)
- [ ] **(F29)** Hold baseline JSON / trigger payloads / document JSON as
      `Arc<Value>` layers so `effective_for` stops cloning every layer per call —
      `schemas/mod.rs:386-393,477-479`. (Falls out of Findings 5/8/9; finish any
      residual clones.)
- [ ] **Checkpoint:** schema-bearing `md compose --perf` shows no residual
      per-lookup hashing/parse cost; `md schema validate` byte-identical.
      `cargo check --workspace`; `just test`/`just test-l2`.

---

## Phase 10 — CLI / IO & Cleanup Miscellany (Findings 22, 25)

**Goal:** Remaining CLI/IO and cleanup-pipeline wins not on the render or
compose hot paths.

**Package:** `darkmatter/lib` + `darkmatter/cli`.

- [ ] **(F22)** In `md hash <dir>`, skip well-known vendored dirs
      (`node_modules`, `target`, `vendor`) — or switch to `ignore`-based
      walking — instead of only pruning dot-directories —
      `markdown/fs.rs:20-29`. Confirm the aggregate fingerprint change is
      intended and document it (public behavior → update README/hash docs).
- [ ] **(F25)** Fuse the four back-to-back whole-string `String::replace` calls
      into one scan and fold the line-based cleanup passes into a single
      iterator, reducing ~10 sequential full-document passes —
      `markdown/cleanup/mod.rs:214-314`, `cleanup/emphasis.rs:111-115`.
      (Not on the render path — only `Markdown::cleanup*` / `md clean` /
      compose's Cleanup stage.)
- [ ] **(F35 residual)** `md hash --diff/--save` computes the document hash
      2–3× (each rebuilding the toc — Phase 5 already multiplies the win);
      `md delta` clones both full documents; `normalize_body_rhythm` allocates
      an ANSI-stripped copy per line; `apply_link_policy`/`apply_image_policy`
      clone URL+title per node even with an empty policy — address
      opportunistically (`cli/commands/hash.rs:151-186`,
      `hash/explain.rs:449-502`, `cli/commands/mod.rs:189-190`,
      `layout/page.rs:1244-1269`, `render_tree/build_context.rs:375-379,411-415`).
- [ ] **Checkpoint:** `md hash <dir>` no longer walks vendored trees;
      `md clean` output byte-identical; cleanup pass count reduced.
      `just test`/`just test-l2`/`just lint`.

---

## Phase 11 — Final Validation & Closeout

**Goal:** Prove the regressions are gone and record the outcome.

- [ ] Re-run the full Phase 1 baseline table (release build) and diff against
      the recorded baseline; confirm `md compose` ≈ 70 ms and no NTP probe fires
      (verify via `RUST_LOG` trace — no silent ~60 ms gap).
- [ ] Re-run the toc scaling test at all three tiers; confirm roughly linear
      scaling (the 1.3 MB tier must be sub-second, down from 45.3 s).
- [ ] Re-run both criterion benches (`compose_with`, `as_terminal`) and record
      before/after deltas in `reviews/2026-07-12-perf/results.md`.
- [ ] Confirm the "good patterns" from the spec were **not** disturbed (syntect
      one-time load, `HighlightLines` reuse, zero render-path regexes, non-TTY
      short-circuit, demand-driven capture, single-flight caches).
- [ ] Run `just test`/`just test-l2`/`just lint` for `darkmatter`, plus
      `just test sniff` and `just test biscuit-terminal` from repo root;
      `cargo check --workspace`.
- [ ] Update drift surfaces: any changed public behavior in READMEs, per-area
      `docs/dependencies.md` if `aho_corasick`/`biscuit-hash` deps were added,
      and the `darkmatter` skill if entry-point behavior changed.
- [ ] Move this review folder to `_completed` per repo lifecycle convention and
      record a memory note summarizing the fixes and measured wins.

---

## Dependency & Parallelization Summary

| Phase | Depends on | Parallel with | Subsystem |
|-------|-----------|---------------|-----------|
| 1 Benches | — | — | lib benches |
| 2 NTP | 1 | 3, 4, 5 | sniff + lib |
| 3 Terminal | 1 | 2, 4, 5 | biscuit-terminal + lib + cli |
| 4 Schema double-work | 1 | 2, 3, 5 | lib schemas |
| 5 TOC | 1 | 2, 3, 4 | lib toc |
| 6 Compose walks | 1 (best after 4) | — | lib compose + cli |
| 7 Compose alloc/reparse | 1 | 8 | lib compose |
| 8 Render path | 1 | 7 | lib render + syntect |
| 9 Schema-tier | **4** | 10 | lib schemas |
| 10 CLI/IO/cleanup | 1 (5 multiplies F35) | 9 | lib + cli |
| 11 Closeout | all | — | all |

**Fast path to the critical/high wins:** land Phase 1, then run Phases 2, 3, 4,
5 concurrently — this clears the 1 critical + 5 high findings (the quick-wins
table) before any of the opportunistic allocation work begins.
