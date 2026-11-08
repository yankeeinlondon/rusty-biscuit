---
spike: S13a
spec: 2026-10-27-release-process
question: Does anything meaningful depend on a package's version value: tests, snapshots, or embedded strings?
answer: yes — a release-only diff fails tests in at least four packages (queue-cli, unchained-ai-cli, sniff-detection-cli, tree-hugger-cli) and changes shipped output in about a dozen more
confidence: high (queue-cli reproduced; the other three read from code and fixtures, not run)
date: 2026-11-07
---

## Answer

Yes. A diff limited to version bumps, changelogs, and `Cargo.lock` is **not**
test-neutral today. Four test groups hard-code the current version and fail
when it moves; one of them fails in a *different* package from the one bumped.
Shipped behavior also changes, but only in version-reporting text and in
tree-hugger's cache keys, which is the intended effect of a release.

So the rule "a release-only diff inherits evidence" is only safe after these
tests stop pinning the literal, or with the affected packages re-tested on the
release tree.

## Evidence

Searches over the worktree (`rg`, excluding `target/`): `CARGO_PKG_VERSION`
and `crate_version!` (20 files), clap `#[command(... version ...)]` (19 CLIs),
`--version`/`-V` invocations in tests, `0.1.0` literals in `*.rs` tests,
`*.snap` (1,599 files), and golden fixtures.

### Would fail on a release-only diff

| Package bumped | Test | Why |
|---|---|---|
| `queue-cli` | `queue/cli/tests/cli_integration.rs:32` `cli_shows_version`, `:71` `cli_short_version_works` | `contains("queue 0.1.0")` / `contains("0.1.0")` against `queue --version` |
| `unchained-ai-cli` | `unchained-ai/cli/tests/cli.rs:26` `test_version` | `contains("0.1.0")` against `--version` |
| `sniff-detection-cli` | `sniff/cli/tests/l1/snapshots.rs:164` `help_output_snapshot` | snapshot `l1__snapshots__help_output.snap:5` holds `sniff 0.1.0`; the help template is `{name} {version}` (`sniff/cli/src/args/mod.rs:1126`) and the normalizer does not mask it |
| `tree-hugger` (library) | `tree-hugger/cli/tests/symbols/golden.rs` `json_symbols_over_multi_match_the_golden`, `jsonl_symbols_over_multi_match_the_golden` | `tests/fixtures/golden/symbols.json` and `.jsonl` hold `"extractor_version": "0.1.0+rev.55"` 94 times each; the value is `env!("CARGO_PKG_VERSION")` of the **library** (`tree-hugger/lib/src/shared/schema_v2/mod.rs:155`), and the golden canonicalizer only masks `root_dir`/IDs |

Reproduced for `queue-cli` in a scratch clone: setting
`queue/cli/Cargo.toml` to `0.1.1` and running
`cargo test -p queue-cli --test cli_integration` gave
`test result: FAILED. 4 passed; 2 failed` (`cli_short_version_works`,
`cli_shows_version`). The other three were not run.

The tree-hugger case means a bump of `tree-hugger` alone breaks
`tree-hugger-cli`'s tests, so "the bumped package's own tests" is not the
boundary either.

### Version-dependent but test-neutral

These compute the expectation from `env!("CARGO_PKG_VERSION")` or assert only a
binary name, so they move with the version:

- `--version` checks that assert only the name: `bh`, `clip`, `bt`, `hug`, `md`,
  `sniff`, `so-you-say`, `geo` (success only).
- tree-hugger cache tests: `tree-hugger/lib/tests/l1/symbols_cache.rs:556,559,4230`,
  `symbols_adapter.rs:466`, `schema_v2/mod.rs:760`,
  `tree-hugger/cli/tests/symbols/cache_upgrade.rs:36,343,368`.
  `tree-hugger/lib/tests/l1/cache_tests.rs` uses a literal `"0.1.0"` but only as
  self-contained input.
- `claudine/rendezvous` tests compare `daemon_version` with the same
  `DAEMON_VERSION` constant (`client/tests/local_round_trip.rs:46,67`,
  `daemon/src/service/tests/rpc.rs:18`).
- `darkmatter` layout snapshots contain `version: 0.1.0`, but it is literal
  sample YAML (`darkmatter/lib/tests/layout_matrix_support/mod.rs:213`).
- `sniff` repo tests asserting `"0.1.0"` run against synthetic monorepos
  (`create_cli_monorepo()`), not this workspace.
- `tools/test-toolkit` kache tests use a fake kache version.

### Shipped behavior that changes with the version

All expected for a release, none hidden:

- `--version` output of the 19 clap CLIs and `dmls` (`darkmatter/dmls/src/main.rs:132`).
- Protocol self-identification: `dmls` LSP `serverInfo.version`
  (`router.rs:278`), claudine's Codex `clientInfo.version`
  (`codex_app_server/commands.rs:26`) and wrapper `client_version`
  (`wrapper_exec.rs:129`, `harness_orch/attempt.rs:228`), `claudine_version` in
  stream-capture metadata (`stream_capture.rs:193`), rendezvous
  `DAEMON_VERSION`, homelab integration `DRIVER_VERSION` (three `types.rs`).
- HTTP user agent `geo/<version>` (`biscuit-location/lib/src/config.rs:44`).
- **tree-hugger cache identity**: the version is part of the extractor version
  and analyzer fingerprint (`cache/symbol_cache.rs:130`, `cache/mod.rs:561`),
  so every user's symbol cache misses once after upgrading. Intended, but a
  version-only release is not a no-op for `hug` users.
- `unchained-ai-gen` stamps its version into generated-file headers
  (`unchained-ai/gen/src/generator.rs:128`); those headers also carry a
  timestamp, so no byte-exact drift test can depend on them.

No `build.rs`, `vergen`, `built`, or `option_env!("CARGO_PKG_…")` use was
found. No test reads this repository's own `CHANGELOG.md` files.

## What this changes in the spec

- **Rules this spec must settle → "The release's own diff"**: the rule as
  written ("a diff limited to versions, changelogs, and the lockfile must
  inherit the evidence") is unsound against today's tests. Either:
  1. make version-pinning tests version-agnostic (derive from
     `env!("CARGO_PKG_VERSION")`, mask `{version}` in the sniff help snapshot
     and tree-hugger goldens), and add a guard that keeps them that way, or
  2. keep the rule but have the release tree re-run the tests of every
     package whose version moved **and of its reverse dependencies**
     (the tree-hugger case).
  Option 1 keeps "`ci-gate` passes with no tests" true; option 2 does not.
- **Packages**: a per-package release still crosses package boundaries
  through the library→CLI path above, so "evidence for its own closure" must
  include dependents' tests, not only its own.

## Surprises

- The tree-hugger golden failure lands in a *different* package
  (`tree-hugger-cli`) from the one bumped (`tree-hugger`), which moves the
  seam for "what a release diff must re-prove" from the package to its
  reverse-dependency closure, unless the goldens are fixed first.
- Ordering: if option 1 is chosen, fixing these four test groups becomes a
  prerequisite of the release-only-diff rule, ahead of the first evidence-
  inheriting release.
