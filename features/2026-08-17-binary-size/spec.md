---
status: draft
created: 2026-08-17
area: workspace
packages:
    - claudine-cli
    - darkmatter-cli
    - biscuit-terminal-cli
---

# Reduce debug binary size across the workspace

## Summary

Debug binaries are large enough to be an operational problem, not a cosmetic
one. Measured on 2026-08-16 at `71ee67dc6`:

| Binary | macOS | Linux |
| --- | ---: | ---: |
| `claudine` | 244 MB | 373 MB |
| `md` | 152 MB | — |
| `bt` | 64 MB | — |

The Linux `claudine` image decomposes as:

| Section | Size | Share | Content |
| --- | ---: | ---: | --- |
| `.strtab` + `.symtab` | 145 MB | 39% | linker symbol table: mangled generic names |
| `.text` | 83 MB | 22% | unoptimized code |
| `.debug_*` | 63 MB | 17% | the line tables `line-tables-only` deliberately keeps |
| `.rodata` | 36 MB | 10% | embedded constants |

The workspace already sets `debug = "line-tables-only"` and dependency
`debug = 0`; that lever is spent. The dominant remaining cost is
monomorphization, which pays twice — once as `.text` bytes and once as long
mangled names in `.strtab`.

Cost today: CI cache entries at a saturated quota, slow cache restores, link
time on every incremental build, and cold-page-in stalls on first spawn of a
cache-restored binary (mitigated by the CI prewarm step in
`_package-ci.yml`, but the underlying mass remains). CI runs
`31944322558`/`31956960890` and the WSL2 starvation cluster document the
page-in symptom class.

## Scope

**In scope:** measurement tooling, symbol-table stripping (if backtraces
survive), monomorphization reduction in the worst offenders, and CI cache
impact verification.

**Out of scope:** release-profile changes, switching CI to release builds,
stripping the line-table DWARF (CI backtrace quality is a hard requirement),
and any behavior change to the CLIs.

## F1 — Establish the measurement baseline

Add a repeatable measurement path (a `just` recipe or script) that reports,
per top-level binary on macOS and Linux: total size, section breakdown
(`readelf -S` / `size -m`), and the top code contributors via
`cargo bloat --release=false -n 50` and `--crates`. Record the baseline for
`claudine`, `md`, and `bt` in this feature's directory so every later phase
diffs against fixed numbers.

**Acceptance.** One command produces the report on both OSes; baseline
checked in.

## F2 — Determine whether the symbol table can be dropped

39% of the Linux image is `.symtab`/`.strtab`. Rust backtrace symbolization
prefers DWARF, and the line-table DWARF is present — but whether our
`line-tables-only` output carries enough (function DIEs) for named frames is
an empirical question.

Experiment: `objcopy -R .symtab -R .strtab` on a copy of a test binary and a
CLI binary, force a panic, and compare backtraces against the unstripped
original on both Linux and macOS. If names and lines survive, wire the strip
into the dev profile (`strip = "symbols"` keeps `.debug_*`? verify — cargo's
`strip` values interact with platform defaults) or a CI-only
`CARGO_PROFILE_DEV_STRIP` env, and re-measure cache entry sizes.

If backtraces degrade, record the negative result here and close this arm.

**Acceptance.** A documented experiment with byte-for-byte backtrace
comparison; either the strip lands with green CI and equal backtrace
quality, or the arm is closed with evidence.

## F3 — Reduce monomorphization in the worst offenders

Use the F1 bloat report to rank instantiation factories. Expected suspects
(verify, do not assume): serde derives across the 42-field catalog types,
clap builders, generated schematic surfaces, and generic render/compose
plumbing instantiated per concrete type.

Standard reductions, applied only where the report shows material weight:

- box trait objects at wide boundaries instead of generic fan-out;
- move generic function bodies into non-generic inner functions
  (`fn inner(x: &dyn T)` behind `fn outer<X: T>`);
- de-generify internal APIs that only ever see one or two concrete types;
- feature-gate heavy corners that most binaries never exercise.

Each change must show its delta against the F1 baseline and keep `just test`
green in the affected package areas. No public API changes without a
recorded decision.

**Acceptance.** ≥25% `.text`+`.strtab` reduction on the Linux `claudine`
image (target, not contract — record the achieved number), no test or
behavior regressions, link-time delta recorded.

## F4 — Verify the operational wins

After F2/F3 land: measure CI cache entry sizes and restore times against a
pre-change run, confirm the prewarm step's duration drops proportionally,
and re-check the cross-run cache hit rate against the saturated quota
(`project_ci_cache_quota_saturated` context). Update
`docs/dependencies.md`/CI docs where behavior changed.

**Acceptance.** Before/after table for cache size, restore time, and prewarm
duration, linked to real run IDs.

## Verification matrix

- macOS and Linux: F1 report; F2 backtrace comparison on both.
- CI: one full run after each landing phase; failure-set diff must be empty
  relative to the pre-change run.
- No Level-2/Level-3 impact expected; the CLIs' rendered output must be
  byte-identical (spot-check one golden test per CLI).

## Success criteria

1. A checked-in, repeatable size report with baselines.
2. The symbol-table question answered empirically, in either direction.
3. Measured, recorded reduction of the Linux `claudine` image with green CI.
4. Cache and prewarm improvements demonstrated on real runs.
5. Backtrace quality in CI failures is not degraded — same file:line and
   frame names as before.

## Open questions

1. Is `cargo`'s `strip = "symbols"` on the dev profile compatible with
   keeping `.debug_line` on both ELF and Mach-O, or does F2 need
   platform-conditional `objcopy` in CI instead?
2. Does the WSL2 nextest archive (which ships test binaries wholesale)
   warrant the same treatment, and does the archive format preserve
   stripped sections?
3. Is `-Zsymbol-mangling-version` relevant here, or is the workspace already
   on `v0` via the toolchain default?
