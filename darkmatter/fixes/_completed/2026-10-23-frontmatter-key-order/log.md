---
implementation_1: "2026-10-29T14:35:49-07:00"
---

# Frontmatter Key Order: Review/Fix Log

## Implementation of Review Findings #1

> **started at:** 2026-10-29T14:35:49-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/fixes/2026-10-23-frontmatter-key-order/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was already `false`; it was left `false` until the cycle completed
- the review has one finding (R1-F1, low), no blockers, and `human_in_the_loop: null`
- the repair is a single documentation-test gate, so the orchestrator did it directly rather than dispatching a subagent; the sweep and verification below were done by the orchestrator

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Crate example fails with supported rendering-only features | fixed | every child row verified; see below |
| R1-F1.1 | `biscuit-hash/lib/src/lib.rs` crate-level example: imports `xx_hash` unconditionally; must compile and pass with `--no-default-features --features json` | fixed | wrapped the executable lines in hidden `# #[cfg(feature = "xx_hash")] # { … # }`; `cargo test -p biscuit-hash --doc --no-default-features --features json` → 2 passed |
| R1-F1.2 | Same example under `--no-default-features --features json,blake3` | fixed | 7 passed |
| R1-F1.3 | Default-feature control: the example still runs `xx_hash` where the feature is on | fixed | `--doc --features json` (default + json) → 8 passed; `--doc --all-features` → 18 passed |
| R1-F1.4 | Sibling: the same example under other no-xxHash combinations (`[]`, `blake3`, `argon2id`, `json,argon2id`) | fixed | before the gate these failed the same way; now 1, 6, 6, 7 passed |
| R1-F1.5 | Sibling: `json.rs` `canonical_json` doctest | clean | uses only `canonical_json` and `serde_json`, both present whenever `json` is; `json`-only run includes it and passes |
| R1-F1.6 | Sibling: `xx.rs`, `blake.rs`, `argon.rs` module and item doctests | clean | each module is declared behind its own `#[cfg(feature = …)]` in `lib.rs`, so its doctests are only collected when the items they import exist; covered by the runs above |
| R1-F1.7 | Sibling: feature-gated intra-doc links (`crate::xx_hash_bytes`, `crate::blake3_hash` in `json.rs`) | clean | sit on items behind the same `#[cfg]` as the linked item, so they only render when the target exists |
| R1-F1.8 | Sibling: README/docs pages transcluded into rustdoc | clean | no `include_str!` or `#[doc = …]` in `biscuit-hash/lib/src`, so no other Markdown is compiled as a doctest |

- starting the work on 'R1-F1: Crate example fails with supported rendering-only features' at 14:35:56
        - defect class: a documentation example imports an item that a non-default feature configuration removes, without gating the example on that feature
        - swept every doctest and doc link in `biscuit-hash/lib/src` (rows R1-F1.4 through R1-F1.8); the crate-level example was the only ungated site
        - chose a hidden `#[cfg(feature = "xx_hash")]` block, as the review suggested, so the rendered docs still show the default-feature usage unchanged
        - verified across nine feature configurations (see the ledger)
- work completed for 'R1-F1: Crate example fails with supported rendering-only features' at 14:36:45

### Completion Report

- **outcome:** complete
- **elapsed:** about 1 minute (14:35:49 to 14:36:45 local)
- **parent findings:** total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- **instance rows:** 8 (4 fixed, 4 clean)
- **open findings:** none
- **changed files:**
        - `biscuit-hash/lib/src/lib.rs`: crate-level example gated on `xx_hash` (R1-F1)
        - this log and the review's frontmatter (metadata only)
- **verification (macOS):**
        - `cargo test -p biscuit-hash --doc --no-default-features --features json`: 2 passed (R1-F1.1)
        - `cargo test -p biscuit-hash --doc --no-default-features --features json,blake3`: 7 passed (R1-F1.2)
        - `--doc --no-default-features --features json,xx_hash`: 8 passed; `--features json` (default on): 8 passed; `--all-features`: 18 passed (R1-F1.3)
        - `--doc --no-default-features` with `[]`, `blake3`, `argon2id`, `json,argon2id`: 1, 6, 6, 7 passed (R1-F1.4)
        - `just test` in `biscuit-hash`: 90 passed, 0 skipped
        - `just lint` in `biscuit-hash`: passed
        - darkmatter was not re-run: the change only touches a doc comment in biscuit-hash, so no darkmatter code path or output changes
        - no cross-OS check was run: the change is a feature gate on a doctest and carries no OS-specific risk
