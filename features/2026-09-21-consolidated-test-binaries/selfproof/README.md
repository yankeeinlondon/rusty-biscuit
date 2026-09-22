# Toolkit self-proof (Phase 2)

The acceptance gate for `scripts/ci/consolidation.py`, run before any source
file moved. Host: macOS `aarch64-apple-darwin`, cargo-nextest 0.9.136.
Revision `da6e3847d`, with all four crate trees clean
(`provenance.crate_tree_dirty: false` in every capture).

## Results

| Proof | Command (from the repository root) | Result |
|---|---|---|
| No-op identity | `capture` twice, then `compare --before capture-a --after <second run> --require-identical-digests` | `identical`: 0 failures and 0 notes across 18 feature-set captures and 404 selector cells, every raw-listing digest equal → `noop-comparison.{json,md}` |
| Capture determinism | `shasum -a 256` of both runs | byte-identical, so the second run is kept only as `capture-b.SHA256SUMS` (same hashes as `capture-a/SHA256SUMS`) |
| Reproduces the Phase 1 listings | `compare --before ../baseline/listings --after capture-a --common-selectors --require-identical-digests` | `identical`. 114 of 114 baseline listing digests reproduced, with locality fields excluded (S1). The 290 notes are the override selectors that the Phase 1 baseline did not capture |
| Reproduces the Phase 1 inventory | `inventory --listings ../baseline/listings` | equal to `../baseline/inventory.json` in every field except `generator` |
| Every oracle goes red on its mutation | `python3 features/2026-09-21-consolidated-test-binaries/selfproof/mutation-check.py` | 8 of 8 `OK`: the original passes and the mutant fails |

Platform-absent is `not-evaluated` in every cell, because these are
single-host captures (S1 §3). It is never reported as empty.

## Using `capture-a` later

`capture-a/` is the complete before-side for the pilot and the rollout. Unlike
`../baseline/listings/`, it includes every `.config/nextest.toml` override
selector (R5), so a comparison against it needs no `--common-selectors`.
Compare after-captures against `capture-a` so the override selectors are
checked too.

## Reproduce

```bash
python3 scripts/ci/consolidation.py capture --out /tmp/capture-b
python3 scripts/ci/consolidation.py compare \
    --before features/2026-09-21-consolidated-test-binaries/selfproof/capture-a \
    --after /tmp/capture-b --require-identical-digests
```

The capture takes about 11 minutes on this host (18 feature-set builds).
