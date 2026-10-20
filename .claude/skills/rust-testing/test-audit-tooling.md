# Test-Audit Tooling

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

`tools/test-audit` is the one shared TypeScript tool behind every suite audit
and test-performance fix: listing captures, CI JUnit gates, inventory
reconciliation, cost attribution, alternating-run measurement, and work-count
comparison. The audit *method* is [test-suite-audits.md](test-suite-audits.md);
the command, configuration, and artifact reference, including the list of
shipped configurations to copy, is `tools/test-audit/README.md`.

```sh
just -f tools/test-audit/justfile install   # pnpm install at the root workspace
cd tools/test-audit && just check           # tsc --noEmit, then vitest
just run <command> --config <audit.config.json> [options]
```

Commands: `config validate`, `capture`, `fetch`, `junit`, `sources`,
`reconcile`, `attribute`, `measure`, `counters`. Exit codes are 0 clean,
1 violations or malformed evidence, 2 usage.

## Patterns

- **The configuration declares; the engine never guesses.** An area keeps
  one `audit.config.json` in its active fix directory, validated with
  `config validate`. A `routes` row declares that a route exists, never that
  it ran; a `pending` environment needs a `reason`.
- **Pending is not passing.** A pending environment, a harness the host
  lacks, or a pending counter signal is reported as pending and never
  satisfies a gate.
- **Compare only like with like.** `junit --baseline` compares an environment
  only with itself, and `counters compare` pairs readings only when every
  compatibility key matches (native Windows is not WSL) and refuses pairs
  that span a declared boundary.
- **Keep the evidence trail.** Capture with `--note` whenever the listing is
  not from a committed revision, so `captures.json` explains its dirty list.
  When retaking captures, move the superseded set into
  `enumeration/<rev>/` instead of overwriting it: published cost numbers were
  derived from it, and the reconciler ignores subdirectories. A capture older
  than the tree surfacing as `undeclared-exclusion` is the gate working.
- **Source scan is discovery, not coverage.** `reconcile` treats the runner
  listing as the authority. A `//` comment between two attributes breaks the
  attribute chain, so that test shows as *runner-only* rather than as a scan
  miss; grammar gaps appear as `parse-error` diagnostics.
- **Reuse validation while inputs are unchanged:** a `capture`, `fetch`, or
  gate result stays valid until its revision, dirty list, feature selections,
  or configuration change. The tool's own `just check` proves its parsers; do
  not rerun Rust suites to test report parsing.

## Onboarding an area

1. Copy a shipped `audit.config.json` into the area's active fix directory,
   fill `packages`, `routes`, `selections`, `cohorts`, and `environments` from
   the area's justfile and `[package.metadata.ci.tests]`, and run
   `config validate`.
2. `capture` at the baseline revision, then `fetch` a compatible `main` run
   (an ancestor whose area sources are identical; record shared-input deltas).
3. Gate with `junit`, scan with `sources`, write `families.json` and the
   `inventory.md` family index, and `reconcile` until it exits 0.
4. Add area-specific regressions to `tools/test-audit/tests/` only for
   behavior the shared suite does not already prove.
