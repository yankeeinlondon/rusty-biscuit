# Test-Audit Tooling

`tools/test-audit` is the shared TypeScript tool behind every test-suite
audit and test-performance fix: JUnit gates, inventory reconciliation, cost
attribution, local alternating-run measurement, and work-count evidence. One
implementation, one validated configuration file per area. Load this page
when a fix's plan tells you to capture listings, gate CI artifacts, reconcile
an inventory, attribute cost, or compare work counters. The audit *method* is
in [test-suite-audits.md](test-suite-audits.md); this page is the tooling.

## Install and verify

```sh
just -f tools/test-audit/justfile install   # pnpm install --frozen-lockfile (root workspace)
cd tools/test-audit && just check           # tsc --noEmit + vitest (160 tests)
just run <command> [options]                # or: pnpm exec tsx src/cli.ts <command>
```

Node 22 or later and pnpm 10. The package is a root pnpm-workspace member
pinned through the root `pnpm-lock.yaml`. Ordinary Rust test runs never touch
it; CI's `ci-tooling` leg runs `just check` when `tools/test-audit/**` or the
root pnpm files change (`scripts/ci/affected_scope.py`, `CI_TOOLING_PREFIXES`).
Exit codes everywhere: 0 clean, 1 violations or malformed evidence, 2 usage.
Every report is stamped with the tool version; reports from different
versions are not silently comparable.

## Configuration

An area keeps `audit.config.json` in its active fix directory (paths relative
to the file). `just run config validate --config <file>` checks the schema and
the cross-references. The file declares, and the engine never guesses:

- `packages`: Cargo names, paths, source roots for the scan;
- `routes`: every execution route (`just` recipe, CI step, bench, fuzz,
  `manual`, `absent`) with `executes`; a route row is a declaration that the
  route exists, never evidence that it ran;
- `selections`: each package × feature selection a route hands nextest, one
  capture per label;
- `cohorts`: populations a recipe actually runs, `population: local|ci`
  (darkmatter's local L1 excludes `slow_`; its CI L1 keeps it: two cohorts);
- `environments`: CI legs with their own `cells`, `kind: native|wsl`, and
  `pending` with a `reason`; legs are not rectangular;
- `junit`: `requiredTests`, per-leg `platformExclusions`, `timeoutFloors`;
- `counters`: work-count `signals` with `coverage: instrumented|fixture|pending`,
  `compatibilityKeys`, and `boundaries` no comparison may span.

Shipped: `darkmatter/fixes/2026-09-07-faster-darkmatter-tests/audit.config.json`,
`sniff/fixes/2026-09-07-faster-sniff-tests/audit.config.json`, and the
Claudine compatibility file
`claudine/fixes/2026-09-07-faster-claudine-tests/audit.config.json`.

## Canonical commands (Darkmatter example)

```sh
cd tools/test-audit
CFG=../../darkmatter/fixes/2026-09-07-faster-darkmatter-tests/audit.config.json
just run capture   --config $CFG                        # listings per selection → enumeration/
just run fetch     --config $CFG --run 34008778001      # CI staging trees → baseline/<run>/<env>/
just run junit     ../../darkmatter/fixes/2026-09-07-faster-darkmatter-tests/baseline/34008778001 --config $CFG --markdown
just run sources   --config $CFG --markdown             # source-side population + diagnostics
just run reconcile --config $CFG                        # needs families.json + inventory.md
just run measure parse ../../darkmatter/fixes/2026-09-07-faster-darkmatter-tests/baseline/local/test-l1-local.log
just run attribute <run.log|report.xml>... --config $CFG  # per-family cost, joined to families.json (nextest logs or JUnit XML)
just run attribute budgets --config $CFG --runs budgets-pending.json  # refuses local provenance and <3 green CI runs per leg
just run counters compare baseline.json candidate.json --config $CFG
```

Sniff uses the same commands with its own file; its configuration keeps
`remote`, `network`, and `test-fixtures` selections apart (the `lib-bare`
and `lib-network` captures exist to show which identities those selections
omit), times the sanity cohort separately, and declares the
`production-caching-2026-07-22` boundary that no counter comparison may span.
Claudine's existing entry points (`junit-metrics.ts`, `inventory-reconciler.ts`,
`attribution.ts`, `measurement.ts`, `measurement-runner.ts`) are wrappers over
the same commands and accept the legacy `--expect` expectations shape.

## Artifacts

| Command | Primary input | Output |
|---|---|---|
| `capture` | `cargo nextest list --message-format json` | `<enumeration>/<label>.json` + `.err`, `captures.json` (revision, dirty files, `--note` provenance, toolchain, platform, command, identity count) |
| `fetch` | GitHub `junit-<pkg>-<tier>-<env>` and `status-*` artifacts via `gh` | `<env>/<tier>/<pkg>.xml`, appended `manifest.jsonl`, `status/`, `missing.jsonl`, `fetch.json` |
| `junit` | staging tree (`_stage_junit` layout) | per-cell table; `--baseline` comparison matched/added/removed per environment |
| `sources` | `.rs` under the package source roots | records with attributes, cfg gates, module path, ignore reason, `fromMacro`/`macroExpanded`, plus `parse-error` diagnostics |
| `reconcile` | captures × `families.json` × `inventory.md` × source scan | family counts; violations: `missing-capture`, `duplicate-identity`, `unassigned-identity`, `double-assigned-identity`, `stale-family`, `inventory-drift`, `missing-disposition`, `undeclared-exclusion`, `stale-exclusion` |
| `attribute` / `measure` | nextest console logs (bounded adapter, `.log.gz` accepted) or — `attribute` only — nextest JUnit XML (`fetch` staging trees) | per-family or per-run costs; `attribute budgets` derives per-leg family budgets from ≥3 green CI runs (local provenance refused) |
| `counters` | evidence JSON (`readings[]` with `signal`, `value`, `revision`, `environment`, `os`, `platformKind`, `requestShape`, `counterVersion`, `phase`, `collector`, `boundaries`) | validation; per-signal deltas for compatible pairs, `incompatible-comparison` / `spans-boundary` otherwise |

## Reading the numbers

- **Three costs, never summed into one.** Build/setup = manifest
  `duration_s` − runner time; runner elapsed = nextest wall time; summed
  duration = Σ per-test time. The sum exceeds elapsed under parallelism and is
  the only column comparable across core counts. A sum is not cohort wall time.
- **Counts travel with timings.** Identities, failures, skips, retries, slow
  marks, and timeouts sit beside every duration so lost coverage cannot read as
  a speed-up. A run with a non-passing result is rejected, not averaged.
- **Pending is not passing.** A `pending` environment, a harness the host lacks
  (`just test-l3` refuses to run unattended and exits 1 without
  `BISCUIT_L3_TAKE_FOCUS=1`), or a `pending` counter signal is reported as
  pending. Never let a clean skip satisfy a gate.
- **Compatibility before comparison.** `junit --baseline` compares an
  environment only against itself; `counters compare` pairs readings only when
  every compatibility key matches (native Windows is not WSL; counter version;
  request shape; phase) and refuses pairs that straddle a declared boundary.
  Record the revision and dirty state of every input; a source state whose
  captures predate the tree reports the drift as `undeclared-exclusion`, which
  is the gate working. When you retake captures, move the superseded set into a
  revision-named subdirectory (`enumeration/<rev>/`) rather than overwriting
  it — the reconciler ignores subdirectories, and the old listings are what the
  already-published cost numbers were derived from. Capture with
  `--note` whenever the listing is not from a committed revision, so
  `captures.json` says *why* its `dirty` list is non-empty.
- **Source scan is discovery, not coverage.** `sources` resolves
  `macro_rules!` templates, literal `#[test] fn` items inside macro bodies
  (`proptest!`), and marks `#[rstest]` cases `macroExpanded` (the runner names
  them). tree-sitter-rust 0.24 cannot parse `&raw`/`raw` as an identifier or
  `unsafe extern` blocks; those surface as `parse-error` diagnostics on the
  line and the item still parses (proved by the Claudine replay: per-package
  source counts match the old lexer exactly). A `//` line comment
  between two attributes (`#[test]`, `// …`, `#[allow(…)]`, `fn`) breaks the
  attribute chain and the test surfaces as *runner-only* in `reconcile` rather
  than as a scan miss; darkmatter's `schemas/coerce.rs` carries four such tests.

## Reusing validation

Reuse a `capture`, `fetch`, or gate result while its inputs are unchanged:
the revision and dirty list in `captures.json` and `fetch.json`, the feature
selections, and the configuration file. A new failure or a relevant source
change invalidates the affected evidence, not every prior gate. The tool's own
suite (`just check`) is the proof of parser behavior; do not rerun Rust suites
to test report parsing.

## Onboarding another area

1. Copy an `audit.config.json` into the area's active fix directory and edit
   `packages`, `routes`, `selections`, `cohorts`, and `environments` from the
   area's justfile and `[package.metadata.ci.tests]`; run `config validate`.
2. `capture` at the baseline revision; `fetch` a compatible `main` run
   (an ancestor whose area sources are identical; record shared-input deltas).
3. Gate with `junit`; scan with `sources`; write `families.json` and the
   `inventory.md` family index; `reconcile` until it exits 0.
4. Add area-specific regressions to `tools/test-audit/tests/` only for
   behavior the shared suite does not already prove.
