# Cumulative closeout — declared contract

**Written before any measurement was captured**, per the standing Benchmark &
Evidence Contract. Results land in `summary.md` beside this file.

## Purpose

Run the **complete fixture manifest** against the **final feature head** so the
cumulative number includes every follow-up change. This is distinct from Phase
2's historical reconstruction (`83aaecc8f` → `51c1f16e1`), which stopped at the
audit commit and therefore excluded all of this feature's own work.

This is a **cumulative reporting** run, not a new optimization checkpoint. No
finding closes on it; every finding's own disposition already carries its own
target/control measurement. Its job is to show the accumulated effect and to
catch any regression the follow-up introduced.

## Target operation + control groups

Three binaries, one host, one session, interleaved by `hyperfine` per case:

| Pin | Meaning |
|---|---|
| `pre_83aaecc8f` | pre-optimization baseline (the commit `baseline.md` was captured from) |
| `audit_51c1f16e1` | 2026-07-12 review's accumulated result; this feature's audit commit |
| `head` | final feature head = working tree at `b425fb466` + this feature's uncommitted work |

`pre → head` is the cumulative claim. `audit → head` isolates **this
follow-up's** own contribution and is the regression check.

**Controls:** `help` (no document work at all) and `render_basic` (a tiny
document) are the control cases — no finding in this feature targets them, so a
shift there is host/build drift, not code. `hash_basic` and `render_code_heavy`
are near-controls (F35.5 and F23 touch them, both with declared sub-1 % or
no-win expectations).

## Case matrix (13 fixtures + 1 fixture-less command)

Commands are recorded explicitly here because the Phase-2 F4 run record stored
only `--command-name` labels, not the command lines — a reproducibility gap this
run closes for its own cases.

| case | command | fixture identity |
|---|---|---|
| `help` | `md --help` | none (fixture-less) |
| `render_basic` | `md render <f>` | manifest `render_basic` |
| `render_code_heavy` | `md render <f>` | manifest `render_code_heavy` |
| `hash_basic` | `md hash <f>` | manifest `hash_basic` |
| `compose_trivial` | `md compose <f>` | manifest `compose_trivial` |
| `compose_schema_transclusion` | `md compose <f>` | manifest `compose_schema_transclusion` (+ `compose_child`) |
| `compose_transclusion_heavy` | `md compose <f>` | manifest `compose_transclusion_heavy` |
| `compose_interpolation_heavy` | `md compose <f>` | manifest `compose_interpolation_heavy` |
| `replace_heavy` | `md compose <f>` | manifest `replace_heavy` |
| `remote_heavy` | `md compose <f>` | manifest `remote_heavy` |
| `toc_small` | `md toc <f>` | manifest `toc_small` |
| `toc_medium` | `md toc <f>` | manifest `toc_medium` |
| `toc_large` | `md toc <f>` | manifest `toc_large` |

All 13 fixtures are the committed bytes under `benchmarks/fixtures/`, whose
identities are frozen in `benchmarks/manifest.yaml` and re-verified by
`darkmatter/lib/tests/benchmark_fixtures.rs`. Every case exits `0` on both the
oldest and newest binary (verified before this contract was written), so no case
is measuring an error path.

`compose_child` is consumed by `compose_schema_transclusion` and is not a case of
its own. All four fixtures added after Phase 2 (`compose_transclusion_heavy`,
`compose_interpolation_heavy`, `replace_heavy`, `remote_heavy`) were registered
and hashed before their own checkpoint's baseline, per Architecture Decision A;
the older pins simply read them as ordinary Markdown.

## Build profile / environment

- `cargo build --release -p darkmatter-cli`, isolated `CARGO_TARGET_DIR` per pin
  (the two historical pins in throwaway `git worktree --detach` checkouts).
- Host: Apple M4 Max (`Mac16,5`), macOS 26.5.2, Darwin 25.5.0 arm64.
- Toolchain: rustc 1.96.0 (ac68faa20 2026-05-25), `rust-toolchain.toml` = stable.
- All three binaries built in the **same session**, measured in the **same
  session**, on the **same host**.
- `hyperfine 1.20.0`, `--shell=none`, `NO_COLOR=1`, stdout+stderr piped (non-TTY,
  so terminal detection short-circuits — matching the Phase-2 methodology).
- Load average at contract time: 6.65 (1 min). Phase 10 recorded that this host
  reaches load ~29–30 under concurrent indexing, where cross-run comparison
  becomes unsound; this run is captured in a low-load window and all three pins
  are interleaved **within each `hyperfine` invocation**, so all three see the
  same conditions.

## Warm-up / samples / statistic

- `--warmup 3`, `--runs 20` per case per pin.
- Reported statistic: **mean ± standard deviation** (matching Phase 2's F4 record
  so the two tables can be read together).
- Raw per-case JSON retained beside this file (`<case>.json`).

## Predeclared thresholds

Declared now, before capture:

1. **Regression gate (the one that can fail this checkpoint).** No case may
   regress `audit → head` by more than **5 %** *outside* dispersion. A delta
   within one standard deviation is noise and passes. A reproducible
   out-of-noise regression >5 % on any case is a **fail** and must be
   investigated, not narrated.
2. **Cumulative TOC claim.** `toc_large` must retain **≥ 90 %** improvement
   `pre → head` (the non-quadratic `line_at_offset` result Findings 4 and 35.2
   both depend on).
3. **Control honesty.** `help` and `render_basic` are expected to be **flat**
   `audit → head` (no finding touches them). If they move materially, the shift
   is build/inlining drift and **every** case's delta in this table is reported
   with that caveat — the Phase-9 precedent, where a −19 % control shift was
   discounted from the headline rather than banked.
4. **No new win is claimed here.** Per-finding wins are already evidenced in
   their own run records. This table may only *confirm* them cumulatively;
   a large cumulative number is not re-bankable as a separate result.
