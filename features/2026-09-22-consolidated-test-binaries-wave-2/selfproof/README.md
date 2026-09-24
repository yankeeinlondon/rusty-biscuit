# Toolkit self-proof (Phase 2)

This is the acceptance gate for the Phase 2 extension of
`scripts/ci/consolidation.py` (rulings R1, R2, R4, R14, R18, R19). It ran before
any source file of the ten packages moved.

- Host: macOS `aarch64-apple-darwin`, cargo-nextest 0.9.136.
- Revision: `9d44e7988`. All ten crate trees were clean
  (`provenance.crate_tree_dirty: false` in every capture).

## Results

| Proof | Command (from the repository root) | Result |
|---|---|---|
| No-op identity | `capture` the ten packages twice, then `compare --before capture-a --after <second run> --require-identical-digests` | `identical`: 0 failures and 0 notes across 21 feature-set captures and 487 selector cells. Every raw-listing digest is equal. Output: `noop-comparison.{json,md}` |
| Capture determinism | `shasum -a 256` of both runs | Byte-identical. The second run is kept only as `capture-b.SHA256SUMS`, which matches `capture-a/SHA256SUMS` |
| Frozen inventory | `inventory --package <ten> --listings capture-a` | `inventory.{json,md}`: 236 workspace integration-test targets, 136 in the ten packages (Phase 1's census) |
| Every new guard goes red on its mutation | `python3 features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/mutation-check.py` | 17 of 17 `OK`: the original passes and the mutant fails (`mutation-check.txt`). Wave 1's `mutation-check.py` still reports 8 of 8 `OK` |
| Port matches wave 1 (`check-metadata`) | `check-metadata --manifest <each wave-1 *-migration.json>` | `PASS`, with output identical to `acceptance/metadata-check.py`'s (`wave1-metadata-check.txt`) |
| R18 seed location | scratch crate, deliberately failing property | proptest wrote `tests/proptest-regressions/always_fails.txt`, which is exactly `proptest_regression_path`'s prediction (`r18-proptest-scratch.txt`) |
| `check-proptest` on real data | `check-proptest --manifest <wave-1 darkmatter manifest> --before-rev 048e44f7a` | Red on the two darkmatter seed files wave 1 left beside their modules, which is the defect filed as `proptest-regressions-after-consolidation` (`wave1-darkmatter-proptest-check.txt`) |
| `plan` projects the ruled table | `ShippedWave2PlanTests` in `scripts/ci/test_consolidation.py`, over `inventory.json` and `capture-a` with each capture's recorded filters | 18 targets. Aliases are exactly R5's three. Override rewrites are exactly R10's two. R4, R14, and R19 are applied. There are 10 shared `common` tests in `biscuit-terminal-cli` |
| `move` on the real tree | `dry-run.md` | All ten packages moved in a throwaway worktree. `check-proptest` and `body-diff` are clean for all ten. Every `check-attributes` failure is an S2-predicted disposition |

Platform-absent is `not-evaluated` in every cell, because these are
single-host captures. It is never reported as empty.

## Using `capture-a` later

`capture-a/` is the complete macOS before-side for Phases 3–5. It includes
every `.config/nextest.toml` override selector, so comparing against it needs
no `--common-selectors`. A package whose before-state changes before its
migration must be recaptured, because `plan` refuses a capture taken under a
different filter.

## Reproduce

```bash
python3 scripts/ci/consolidation.py capture \
    --package tree-hugger --package claudine --package sniff --package biscuit-file \
    --package schematic-gen --package biscuit-terminal-cli --package claudine-gen \
    --package dmls --package sniff-cli --package biscuit-tui-cli --out /tmp/capture-b
python3 scripts/ci/consolidation.py compare \
    --before features/2026-09-22-consolidated-test-binaries-wave-2/selfproof/capture-a \
    --after /tmp/capture-b --require-identical-digests
```

On this host the first capture took 6 min 56 s (21 feature-set builds with a
warm `kache`), and the second took 5 min 40 s.
