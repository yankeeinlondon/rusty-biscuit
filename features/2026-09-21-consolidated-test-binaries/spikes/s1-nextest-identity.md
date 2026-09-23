---
kind: spike-record
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-21
plan_phase: 1
status: complete
nextest_resolved: cargo-nextest 0.9.136 (1d5bf1ec9 2026-05-16)
toolchain: rustc 1.98.1 (48a229cea 2026-09-01)
builds_on: 2026-09-19-direct-cell-execution (spikes/s2-nextest-list.md)
---

# S1 — Nextest identity fidelity under consolidation

This spike reuses the listing facts in `2026-09-19-direct-cell-execution`'s
S2 spike. It re-proves only the questions that consolidation adds. All work
ran locally on macOS (`aarch64-apple-darwin`) against a scratch crate in the
system temp directory. No CI run was used and no package changed.

## Fixture

The scratch crate `s1-probe` sets `autotests = false` and declares both
shapes side by side, so one listing shows the before and after identities:

| Target | Path | Shape |
|---|---|---|
| `alpha` | `tests/alpha.rs` | old one-file target |
| `level2_errors` | `tests/level2_errors.rs` | old target whose **name** carries a tier marker; its tests (`plain`, `renders`) carry none |
| `l1` | `tests/l1/main.rs` | consolidated root: `#[path = "../common/mod.rs"] mod common;`, `mod alpha;`, `mod level2_errors;`, `mod neutral_errors;` (the same file under an alias), `#[cfg(target_os = "linux")] mod linux_mod;` |
| `level2` | `tests/level2/main.rs` | `required-features = ["extra"]` |

`alpha.rs` contains `plain`, `#[ignore]`d `ignored_by_attribute`,
`level2_marked`, `slow_marked`, `#[cfg(target_os = "linux")] linux_only`, and
`inner::nested_plain`. The tier filters are the strings printed by
`just _tier_filter` (`just/devops.just:1017`), copied exactly.

## Findings

### 1. Matched path shape: nested vs top-level module

The consolidated binary's testcase key is the full module path **below the
crate root**. The binary name is never part of that key:

```text
s1-probe::alpha  plain                  → s1-probe::l1  alpha::plain
s1-probe::alpha  inner::nested_plain    → s1-probe::l1  alpha::inner::nested_plain
```

This is exactly the spec's normalization: `before: <old-binary>::<path>` and
`after: <consolidated-binary>::<module>::<path>`, where `<module>` is the
module name (the former target name or its recorded alias). The comparator
identity stays `<binary-id>` + testcase key. `binary-id` is
`<package>::<target-name>`.

### 2. `test(...)` never matches the binary id, but it matches module segments

Under the Level 1 filter, the one-file target `level2_errors`'s tests
`plain` and `renders` **match** because the binary name is not tested. The same
file as `mod level2_errors;` inside `l1` yields `level2_errors::plain`, which
now **mismatches** L1 (`reason: expression`) and **matches** L2. Under
`--features extra -E 'test(/(^|::)level2_/)'`:

```text
s1-probe::l1  level2_errors::plain      ← newly selected by L2
s1-probe::l1  level2_errors::renders    ← newly selected by L2
```

The same file under the alias `neutral_errors` keeps its L1 verdicts. This
confirms R2's neutral-alias mechanism: a module name is safe only if applying
every `_tier_filter` expression to every projected path gives the same verdict
as the old path. This is a **real** hazard, not a hypothetical one. The
baseline inventory (`baseline/inventory.md` §Module-name hazard) finds 53
marker-named targets across the four packages. For 52 of them, keeping the
name is harmless because every test in them already carries the marker.
darkmatter-cli's `level2_harness_integrity` does not: it is gated on
`terminal-tests`, but its four tests run in L1 today, and the module name
would move them to L2. It needs a neutral alias (R2).

### 3. `#[ignore]`, filter-excluded, and `cfg`-removed tests

These are unchanged from the prior spike, and consolidation does not affect them:

| Case | Listing entry |
|---|---|
| `#[ignore]` | present; `ignored: true`, `filter-match: {status: mismatch, reason: ignored}` |
| excluded by tier filter | present; `ignored: false`, `filter-match: {status: mismatch, reason: expression}` |
| removed by `cfg` on this target | **absent**; a `#[cfg(target_os = "linux")] mod linux_mod;` removes every test in the module, as a per-function `cfg` does |

### 4. A `required-features` target that is not enabled

The target is **absent from `rust-suites` entirely**; there is no suite entry
with a `skipped` status. `s1-probe::level2` appears only under
`--features extra`. Consequence for `capture`: each listing must record the
feature set it was taken under. A missing consolidated suite must be compared
against the old targets that shared its `required-features`, never read as
"zero tests". `cargo nextest list --test level2` without the feature fails
with Cargo's `requires the features: extra` error (exit 101), so selecting a
gated target does not fail silently.

### 5. Same-named tests across former binaries

These do **not** collide after consolidation. Two files that both define
`plain` become `alpha::plain` and `level2_errors::plain`, because the module
segment separates them. Before the move, only `binary-id` separated them.
A true collision needs two old targets that project to the **same module
name**, which cannot happen for top-level files (the filesystem makes file stems
unique). It can happen only if:

- a former target is named like a crate-root item the new root declares
  itself (`common`, `main`), or
- a nested target (Darkmatter's `error_snapshots/main.rs`) is flattened into a
  root that already has a module of that name.

The migration manifest must therefore reject any module name that equals
another module name or a crate-root helper name in the same consolidated
target. The comparator must report a duplicate projected identity as a
failure, never merge the two.

### 6. Listing from an archive with `--workspace-remap`

`cargo nextest archive --features extra` was listed from a fresh copy of the
crate at a different path with
`--archive-file … --workspace-remap <copy> -E <L2 filter>`. The consolidated
and one-file identities and their verdicts were **identical** to the
workspace listing. Consolidation does not change archive transport.

### 7. Crate-level attributes inside a moved module (feeds S2 and `check-attributes`)

| Attribute at the top of a moved file | Behavior as a module |
|---|---|
| `#![cfg(target_os = "linux")]` | still valid: an inner `cfg` removes the module (its test is absent on macOS) |
| `#![allow(dead_code)]` | still valid, scoped to the module |
| `#![recursion_limit = "256"]` | **warning only** (`the #![recursion_limit] attribute can only be used at the crate root`); the setting is silently dropped |

So an inner `#![cfg]` does not *need* to move for correctness. The spec still
requires the platform condition on the module declaration (acceptance 4), so
reviewers see the reachability of each module in `main.rs`. A crate-root-only
attribute is the dangerous case, because it degrades to a warning. The
`check-attributes` detector list must flag it as "forces a separate target or
moves to the consolidated root" (R3).

### 8. Positional name filtering keeps the tier expression

`cargo nextest list -E '<L1 filter>' neutral_errors` returned exactly
`neutral_errors::plain` and `neutral_errors::renders`. The positional filter
is ANDed with `-E`. This is the §6 replacement for `--test <old-binary>`.
`-E 'binary_id(<pkg>::l1)'` selects the consolidated binary as a whole.

## The JSON shape `consolidation.py capture` / `compare` will parse

The document is the same as the prior spike, with the suite fields observed on
0.9.136 recorded in full:

```json
{
  "rust-build-meta": { "...": "producer locality; never identity" },
  "test-count": 18,
  "rust-suites": {
    "<package>::<target>": {
      "package-name": "…", "binary-id": "<package>::<target>",
      "binary-name": "<target>", "package-id": "…", "kind": "test" | "lib" | "bin",
      "binary-path": "…", "build-platform": "target", "cwd": "…",
      "status": "listed",
      "testcases": {
        "<module path>::<fn>": {
          "kind": "test", "ignored": false,
          "filter-match": {"status": "matches"} | {"status": "mismatch", "reason": "ignored" | "expression"}
        }
      }
    }
  }
}
```

The comparator reads `rust-suites.*.binary-id`, `kind`, and
`testcases.*.{ignored, filter-match}`. It must never read `test-count`,
`binary-path`, `cwd`, or `rust-build-meta`. The four sets per
(package, feature set, platform, tier) are:

- **selected** — `filter-match.status == "matches"`
- **excluded-as-other-tier** — `mismatch` with `reason == "expression"`
- **ignored** — `mismatch` with `reason == "ignored"` (equivalently `ignored: true`)
- **platform-absent** — not listable on the host. It is derived by
  **differencing hosts**: the union of the macOS, Linux, and Windows listings
  of one feature set, minus this host's listing. It is not a per-host field.

## Corrections to the plan's assumptions

- R2's alias rule is **confirmed**. See finding 2: the rule is load-bearing
  for any L1-contract file whose name carries a marker. The inventory shows
  whether such a file exists.
- Collision handling (plan S1 text: "nextest disambiguates only by binary,
  which the migration manifest must therefore forbid or alias") is
  **narrowed**. Test-name collisions across former binaries disappear under
  the module segment. Only module-name collisions (finding 5) need a manifest
  rule.
- Platform-absent is not observable on one host. `compare` needs the
  per-host captures from `just cross-check` to build that set, so Phase 3
  Wave 3's on-host comparison is required, not optional.
