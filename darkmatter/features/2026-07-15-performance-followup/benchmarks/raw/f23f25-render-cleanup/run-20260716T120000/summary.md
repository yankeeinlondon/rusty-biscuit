# Run record — Phase 8 (Findings 23 & 25): render theme snapshot & cleanup passes

- **Run id:** run-20260716T120000 (2026-07-16 12:00 UTC)
- **Host:** Apple M4 Max (Mac16,5), macOS (Darwin 25.5.0), single-host
  non-interactive sandbox. Piped (non-TTY) process; no PTY measurement in this
  checkpoint.
- **Baseline commit:** `b425fb466` (branch `darkmatter` HEAD before this phase),
  reconstructed by `git checkout HEAD -- darkmatter/lib/src/markdown/render_tree/code_renderer.rs`.
- **Candidate:** this phase's F23 render-scoped theme/environment snapshot.
  Baseline and candidate share **identical** fixture, harness, and bench bytes —
  only `render_tree/code_renderer.rs` differs.
- **Build profile:** `release` (`cargo bench -p darkmatter --bench phase8_render`),
  stable toolchain per `rust-toolchain.toml`, workspace lockfile unchanged.
- **Fixtures (immutable, hashed in `manifest.yaml`, generator 1.2.0):**
  `render_code_heavy` (40 fenced blocks across rust/python/json, 4157 bytes,
  `xxhash64: 2f5d9cca8c854cfd`) for F23; `toc_large` (80936 bytes),
  `replace_heavy` (40944 bytes), and `render_code_heavy` for the F25 profile.
  No new fixture was registered — Phase 8 measures only existing manifest bytes.

## Predeclared thresholds (declared before the baseline was captured)

- **F23 target:** a repeatable, out-of-noise reduction in `as_terminal` /
  `as_html` over the 40-block fixture, **with byte-identical output**.
- **F23 control:** `code_block_direct` (the one-block, page-less surface, which
  has no per-render hoisting available) must not regress beyond noise. A control
  that moves with the target identifies build-to-build drift rather than a win.
- **F25 target:** an end-to-end `cleanup_content` win over the same fixtures from
  fusing line passes. **No-win rule (plan, Phase 8):** fusion within noise, or
  added allocation/complexity without a repeatable end-to-end gain, closes as a
  recorded no-win with no speculative code retained.

## Correctness gate (predeclared: byte-identical output)

Release `md` built from baseline vs candidate, run on the same fixture bytes:

| command | result |
|---|---|
| `md render render_code_heavy.md` (markdown, 4158 B) | **byte-identical** (`diff` empty) |
| `md render render_code_heavy.md --output html` (41534 B) | **byte-identical** (`diff` empty) |

This is the hard gate. The library suites add the rest: 5712 lib + 559 cli + 566
dmls L1 tests and 104 headless-browser tests pass unchanged.

## F23 — render-scoped code surface (Criterion, manifest fixture)

`cargo bench -p darkmatter --bench phase8_render -- --save-baseline f23-before`
then `-- --baseline f23-before`; warm-up 3 s, measurement 10 s, Criterion's
default 100 samples, statistic = mean with 95% CI.

| bench (40 blocks unless noted) | baseline | candidate | change (95% CI) | p |
|---|---|---|---|---|
| `as_terminal_code_heavy` | 1.6275 ms | 1.6170 ms | −0.61% [−1.26%, −0.05%] | 0.05 |
| `as_html_code_heavy` | 1.5166 ms | 1.5106 ms | −0.79% [−1.32%, −0.25%] | 0.00 |
| `code_block_direct` (**control**, 1 block) | 18.990 µs | 18.846 µs | −0.66% [−1.21%, −0.11%] | 0.02 |

**Verdict: no measurable win — threshold not met.** The control moved by the
same −0.66% as the targets, so the ~0.6–0.8% shift is build-to-build drift, not
hoisting: a single-block render has nothing to hoist, and its "improvement" is
indistinguishable from the 40-block ones. Net of the control, the 40-block
surfaces move ≈0.1% — far inside noise.

That result is expected from the code, and the size of the hoisted work explains
it: syntect themes and the syntax set are already borrowed `&'static`
(finding 23's earlier, already-complete half), so the remaining per-block work
is a theme-name match, two static map lookups, a luma comparison, and — on the
browser path only — one `HtmlOptions` clone. Against ~40 µs of actual
highlighting per block, 40 repetitions of that are ≈0.1–0.3% of the render.

F23 is nonetheless **retained**, not reverted: it is a plan- and spec-mandated
contract ("resolve environment/theme choice once per render snapshot rather than
reading it per block"), byte-identical, and carries its own required contract
tests (counted: 1 resolution + 1 environment read per render, verified to report
5 and 6 with the memo disabled). It adds no speculative machinery beyond the
snapshot the plan specifies. This matches the Phase-6 precedent for F11/F12 —
a byte-identical structural work-reduction whose wall-clock sits below the
measurement floor. The honest claim is *contract satisfied, no measurable
speed-up*.

Raw: `criterion-{as_terminal_code_heavy,as_html_code_heavy,code_block_direct}-{baseline,candidate}.json`.

## F25 — cleanup pass profile (the plan's "profile first" step)

Profiled with a temporary in-crate harness replicating `cleanup_content_internal`
step by step over three manifest fixtures (release build, 3 warm-ups, 25 samples,
median). The harness was removed after this capture; its verbatim output is
retained in `f25-cleanup-pass-profile.txt`.

`toc_large` (80936 bytes) — the largest fixture, where fusion has the most to win:

| stage | median | share of `cleanup_content` |
|---|---|---|
| **`cleanup_content` (full)** | **1262.5 µs** | 100% |
| `strip_incidental_newlines` (not an F25 line pass) | 278.3 µs | 22.0% |
| stage 1 parse (`into_offset_iter`) | 262.3 µs | 20.8% |
| stage 1 `add_text_language` + `align_tables` + `preserve_emphasis` + markers | 292.3 µs | 23.2% |
| stage 1 cmark serialize | 213.3 µs | 16.9% |
| **stage 2 line passes (all seven), net of clone overhead** | **≈282 µs** | **≈22.3%** |

Within those seven line passes, three carry essentially all the cost
(`normalize_list_spacing` 101.8 µs, `fix_blockquote_formatting` 104.1 µs,
`fix_list_indentation` 62.3 µs); the other four total ≈18 µs. The smaller
fixtures are more lopsided still — on `replace_heavy` the line passes are 8.8%
of cleanup and `strip_incidental_newlines` alone is 70.8%.

**Verdict: no-win — not implemented.** The reasons are structural, not marginal:

1. **The ceiling is small and the floor is far away.** Even a perfect fusion of
   the three heavy passes cannot remove their per-line work — only the repeated
   scan/rebuild overhead, a fraction of ≈268 µs, i.e. under ~7% of cleanup on the
   largest fixture. Cleanup is itself a small part of a compose run (Phase 5
   measured `md compose` at ~19 ms ± 0.5 ms), so the whole prize is ~0.5% of one
   compose — an order of magnitude below that fixture's run-to-run σ, and below
   the ~0.6% build drift this checkpoint's own F23 control just demonstrated.
2. **Exact equivalence is not available cheaply.** The passes are sequential
   rewrites, not independent filters: `normalize_list_spacing` inserts and
   removes lines, and `fix_blockquote_formatting`, `restore_list_markers`, and
   `fix_list_indentation` each consume the previous pass's *re-lined* output. A
   single fused scan would have to reproduce that re-lining internally, which the
   plan's "only when ordering and boundary behavior can be made exactly
   equivalent" condition does not license on this evidence.
3. **The blast radius is disproportionate.** GitNexus upstream impact on
   `cleanup_content_internal` is **HIGH** (35 impacted symbols; 9 direct;
   modules Cleanup, Composition, Wrap) — the plan requires warning and stopping
   for owner direction at HIGH risk, and the measured prize does not justify
   seeking it.

No speculative fusion code was written or retained; the profiler that produced
the numbers above was deleted. The `cleanup/mod.rs` two-stage pass order is
unchanged, and its canonical output is untouched.

Raw: `f25-cleanup-pass-profile.txt`.

## Cross-platform

Classified from the final diff, not the finding number:

- **F23:** OS-identical. The changed code is a struct field, a `RefCell`/`OnceCell`
  memo, and the same resolution chain called once instead of per block. No `cfg`,
  no filesystem, no terminal-detection, and no process branch was added or moved
  — `std::env::var` is read on the same code path as before, only fewer times, and
  the terminal-detection call sites are unchanged. Windows compile + the macOS
  behavioral run + ordinary Linux CI is sufficient. Headless-browser evidence for
  the browser half is the Darkmatter `just test-browser` tier (104 passing).
- **F25:** no production diff — nothing to classify.
