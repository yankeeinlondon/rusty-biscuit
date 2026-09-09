# test-audit

Shared test-suite audit tooling for rusty-biscuit package areas: JUnit gates,
inventory reconciliation, cost attribution, local alternating-run measurement,
and work-count evidence. One implementation, driven by a validated per-area
configuration file; no product behavior lives here.

It is a TypeScript member of the root pnpm workspace (`pnpm-workspace.yaml`),
pinned through the root `pnpm-lock.yaml`. Rust test runs never depend on it;
only the `ci-tooling` CI leg provisions Node for its own checks.

## Install and verify

```sh
just -f tools/test-audit/justfile install    # pnpm install --frozen-lockfile at the root
cd tools/test-audit
just check                                   # tsc --noEmit, then vitest
just run <command> [options]                 # any command below
```

Node 22 or later. The tool version (`package.json` name@version) is stamped
into every report so reports produced by different versions are never silently
compared.

## Configuration

Each area keeps an `audit.config.json` in its active fix directory (paths are
relative to the file). It declares what the engine may not guess:

| Key | Meaning |
|---|---|
| `packages` | Cargo package names, paths, and source roots for the scan |
| `routes` | Every execution route (`just` recipe, CI step, bench, fuzz, manual, absent) with `executes` — a route row is a declaration, never evidence that it ran |
| `selections` | Each package × feature selection a route hands nextest; captured as `<enumeration>/<label>.json` |
| `cohorts` | Populations a recipe actually runs (local-default L1 vs CI-selected L1, sanity, L2, …) with `population: local\|ci` |
| `environments` | CI legs with their own `cells` (`{tier, package}`), `kind: native\|wsl`, and `pending` + `reason` |
| `junit` | `requiredTests`, per-leg `platformExclusions`, `timeoutFloors`, `enforceTimeoutFloors` |
| `counters` | Work-count `signals` (`coverage: instrumented\|fixture\|pending`), `compatibilityKeys`, and `boundaries` a comparison may not span |
| `families`, `inventory`, `enumeration` | Evidence file locations |

`test-audit config validate --config <file>` runs the schema and referential
checks (unknown packages, routes, selections; pending legs without a reason;
absent routes claiming to execute). Shipped configurations:

- `darkmatter/fixes/2026-09-07-faster-darkmatter-tests/audit.config.json`
- `sniff/fixes/2026-09-07-faster-sniff-tests/audit.config.json`
- `claudine/fixes/2026-09-07-faster-claudine-tests/audit.config.json` (compatibility)

## Commands

| Command | Purpose | Exit |
|---|---|---|
| `config validate\|show --config <cfg>` | Validate or display a configuration | 0 / 2 |
| `capture --config <cfg> [--only a,b] [--dry-run]` | `cargo nextest list --message-format json` per selection → `<enumeration>/<label>.json`, `.err`, `captures.json` (revision, dirty files, toolchain, platform, command, identity count) | 0 / 1 |
| `fetch --config <cfg> --run <id> [--out <dir>]` | Assemble a CI run's `junit-*`/`status-*` artifacts into per-environment staging trees via `gh` | 0 / 1 (missing artifacts) |
| `junit <staging-dir> --config <cfg> [--expect <json>] [--baseline <dir>] [--markdown\|--json]` | Gate a staging tree: malformed XML, invalid durations, missing cells/tests, excluded tests that ran, duplicates, failures, manifest disagreement; compare matched/added/removed per environment | 0 / 1 / 2 |
| `sources --config <cfg> [--json\|--markdown]` | Source-side test population via tree-sitter (attributes, cfg gates, module path, ignore reason, macro forms) plus parse diagnostics | 0 |
| `reconcile --config <cfg>` | Runner universe from captures × `families.json` × `inventory.md` family index × source scan: total, non-overlapping, every difference declared | 0 / 1 |
| `attribute <log>... --config <cfg>` / `attribute budgets --runs <json>` | Per-family cost from nextest console logs; budgets refuse local provenance and fewer than three CI runs per leg | 0 / 1 |
| `measure report\|parse\|run` | Local alternating-run report (three costs kept apart, drift bracket, cohorts, ten-execution targets), single-log parse, and the alternating runner (`--plan`) | 0 / 1 |
| `counters validate\|compare --config <cfg>` | Work-count readings as data; comparisons are rejected across differing compatibility keys or a declared boundary | 0 / 1 |

Exit codes everywhere: 0 clean, 1 violations or malformed evidence, 2 usage.

### Inputs, in order of preference

1. **Nextest JSON listings** (`capture`) and **JUnit XML + `manifest.jsonl`**
   staging trees (`fetch`, or the local `target/nextest/ci-reports` tree) —
   structured, the primary inputs.
2. **Nextest console logs** — accepted by `attribute` and `measure` through one
   bounded adapter (`src/nextest-log.ts`: nextest 0.9.x `PASS [ 0.123s] (n/m)
   binary name`, `TRY n`, `Starting N tests across M binaries (K skipped)`,
   `Summary [ … ]`, gzip accepted) for captures that cannot be regenerated. A
   log without a `Summary` line, or whose result lines disagree with it, is
   rejected.

### Evidence integrity

A gate that prints a miss and exits 0 is not a gate. Every command rejects
malformed input, keeps failures/skips/retries visible, and keeps three costs
apart: build/setup (manifest `duration_s` − runner time), runner elapsed
(nextest wall time), and summed test duration (Σ per-test, comparable across
core counts, never a wall time). A `pending` environment or `pending` counter
signal is reported as pending; it never satisfies a passing gate. Native
Windows and WSL are distinct `platformKind`s and never compared.

### Source scan limits

`sources` uses `web-tree-sitter` with the `tree-sitter-rust` 0.24 grammar. It
resolves three macro forms: `macro_rules!` templates naming the function by a
parameter, literal `#[test] fn` items inside a macro body (`proptest!`), and
attribute-parameterized tests (`#[rstest]` + `#[case]`), the last flagged
`macroExpanded` because the runner derives their names. Grammar gaps (`&raw`
and `raw` as an identifier, `unsafe extern` blocks) produce `parse-error`
diagnostics on the line; the item still parses. Source discovery is never a
claim of executable coverage — `reconcile` treats the runner listing as the
authority and demands a declared exclusion for every difference.

## Layout

```
src/cli.ts            dispatcher (commands load lazily)
src/config.ts         zod schema, referential validation, path resolution
src/capture/ fetch/   listing and CI-artifact acquisition
src/junit/            parse (fast-xml-parser), manifest, collect, compare, render
src/reconcile/        listings, families, inventory, sources (tree-sitter), gate
src/attribute/        family attribution and budgets
src/measure/          report, runner
src/counters/         work-count evidence
src/nextest-log.ts    the console-log compatibility adapter
tests/                vitest; *-claudine-compat.test.ts replay the preserved Claudine inputs
```

The Claudine fix directory keeps thin wrappers (`junit-metrics.ts`,
`inventory-reconciler.ts`, `attribution.ts`, `measurement.ts`,
`measurement-runner.ts`) that forward to these commands with its
`audit.config.json`; their previous implementations and tests live here now.
