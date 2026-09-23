# Phase 5 — Cross-pass compose reuse (F7/F16, 35.1, 35.4)

Run record for the compose-cache reuse checkpoint. Records the F35.1 compose
benchmark (immediate pre-change baseline vs candidate) and the target/control
dispositions.

## Identity & environment

- **Baseline commit:** `b425fb466` (immediate pre-Phase-5 HEAD), built in an
  isolated `git worktree` with `CARGO_TARGET_DIR=/tmp/dm-baseline-target`.
- **Candidate:** the Phase-5 working tree (35.1 hash hoist + 35.4 graph read
  reuse), `cargo build --release -p darkmatter-cli`.
- **Fixture:** `fixtures/compose_transclusion_heavy.md` (manifest id
  `compose_transclusion_heavy`, `xxhash64 3e1c55dcd8e0f84a`, 1180 bytes, 40
  `::file ./compose_child.md` directives in one transclusion phase, generator
  `1.1.0`). Both binaries composed the **same absolute path** so bytes are
  identical; only the code under test differs.
- **Host:** macOS (Darwin 25.5.0), single-host non-interactive session.
- **Harness:** `hyperfine --warmup 5 --runs 40 --shell=none`, `NO_COLOR=1`,
  release profile. Raw JSON: `compose-transclusion-heavy.json`.

## Target operation & control

- **Target:** `md compose <transclusion-heavy>` — the transclusion phase builds
  N=40 per-directive compose cache keys. 35.1 hoists the phase-wide
  `effective_state_hash` + `context_hash` out of that per-directive path.
- **Control:** the child (`compose_child.md`) is composed once via single-flight
  regardless; process startup + single child compose + I/O dominate wall-clock.

## Predeclared thresholds

- **Correctness (hard gate):** composed output byte-identical between baseline
  and candidate. **PASS** — `diff` empty; all 40 transclusions applied, child
  body count 40 in both.
- **Performance:** candidate must not regress wall-clock beyond the measured
  dispersion, and must show a repeatable reduction in the transclusion-phase CPU
  work the hoist removes (user time).

## Result — PASS (small repeatable win)

| metric | baseline | candidate | delta |
|---|---|---|---|
| wall-clock mean | 59.27 ms ± 1.42 | 58.17 ms ± 1.69 | **-1.9%** (1.02× faster; within ~1σ) |
| user CPU time | 204.4 ms | 186.9 ms | **-8.6%** (out of noise) |

The wall-clock delta is within one standard deviation (startup/IO-dominated at
this fixture scale), so wall-clock alone is a within-noise result. The **user
CPU time** — the metric the redundant-hash removal directly targets — drops a
repeatable 8.6%, confirming the phase now performs the state/context
canonicalize+hash once instead of 40×. The candidate is retained on this
structural work-reduction (not a speculative addition): reverting would restore
39 redundant hash computations per phase.

## Sub-item dispositions

- **F7 / F16 general reuse — no additional broadening (existing level held).**
  Audit confirmed the transclusion key
  (`options_hash` + source + `effective_state_hash` + `context_hash` +
  set-overlay) already derives `options_hash` from the AD-B classification
  (`ComposeOptions::compose_cache_fingerprint`, Phase 4) and already implements
  the spec's level-3 reuse (fully rendered child content via
  `RunLocalCache::get_or_compute_compose` single-flight, keyed by a complete
  semantic identity). No safe broadening remained; recomposition is retained
  where identity cannot be fully established (process-local stateful handlers →
  `persistent_cache_eligible() == false`). No speculative reuse code added, so
  nothing to remove.
- **35.1 hash hoisting — win (this run).** `PhaseStateIdentity::capture(state)`
  computed once in `pipeline/phases.rs` before the parallel resolve and threaded
  through `resolve_prepared_transclusion` → `render_markdown_transclusion`.
  Byte-identical cache keys (`phase_state_identity_matches_underlying_hashes`
  proves the hoisted value equals the two functions it replaces). Benchmark
  above.
- **35.4 read reuse — win (structural; L1-verified).** `generate_toc_link_references`
  now loads the target through the run's shared `ReferenceAnalysisRuntime` cache
  (`runtime.load_markdown`) instead of a second `Markdown::try_from`, so
  graph-discovery and the follow-mode traversal share one owner. This removes one
  disk read + parse per `::toc-linking` directive during graph discovery. It is
  on the `md graph` / reference-graph path, not `md compose`, so it is not
  exercised by the compose benchmark above; verified by
  `reference_integration::toc_linking_repeated_target_generates_all_heading_links`
  (repeated directives to one multi-heading target still generate every heading
  link, byte-identical) and the existing
  `toc_linking_dependency_and_generated_links_appear_in_composed_references`.

## Concurrency

No lock is held across concurrent child composition: the resolve stage locks
`runtime_mutex` only to `clone_for_child()` / `merge_child()`; the child compose
runs unlocked. Proven by the existing deterministic barrier tests
`cache::runtime::tests::single_flight_contention` and
`operation_single_flight_contention` (8 threads, ≤2 computations, lock released).
