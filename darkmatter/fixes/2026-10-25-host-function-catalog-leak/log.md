---
implementation_2: "2026-10-26T19:13:44-07:00"
---

# Host Function Catalog Leak: Review Fix Log

## Implementation of Review Findings #2

> **started at:** 2026-10-26T19:13:44-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-pipeline-finalization/darkmatter/fixes/2026-10-25-host-function-catalog-leak/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- inventory: one finding (R2-F1, low), no observations, no blockers; the review's "previous finding verification" row records missing review-1 evidence, not a defect, so it is not an implementable row
- review `implemented` was already `false`; left `false` before repairs

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R2-F1   | Keep host cache inspection helpers inside the library | fixed | every child row R2-F1.1–.9 verified; no sibling open |
| R2-F1.1 | `CachedArtifact`: `#[cfg(test)]`, not exported with `work-counters` | fixed | now `#[cfg(test)] pub(super) enum` in `lib/src/markdown/schemas/catalog.rs` |
| R2-F1.2 | `CachedHostFunctions`: `#[cfg(test)]`, not exported with `work-counters` | fixed | `#[cfg(test)] pub(super) struct`, fields `pub(super)` |
| R2-F1.3 | `cached_host_functions`: `#[cfg(test)]`, not exported with `work-counters` | fixed | `#[cfg(test)] pub(super) fn` |
| R2-F1.4 | `host_function_entries_under`: `#[cfg(test)]`, not exported with `work-counters` | fixed | `#[cfg(test)] pub(super) fn` |
| R2-F1.5 | `DiscoveryProbe::host_function_builds` (and its counter state): `#[cfg(test)]`, not exported with `work-counters` | fixed | method `#[cfg(test)] pub(super)`; `State::host_function_builds` field and the increment in `note_host_function_build` are `#[cfg(test)]`; the function itself stays in every config because production discovery calls it |
| R2-F1.6 | Existing lifecycle instrumentation (`install`, `builds`, `take_reads`, `fail_reads_of`, `cached_outcome`): visibility and feature behavior unchanged | clean | diff touches none of them; `discovery_tests` pass; `cargo check -p claudine --features work-counters --tests` (the downstream `work-counters` consumer) compiles clean |
| R2-F1.7 | Sibling sweep: other test-only seams added by `26d72d2a3` stay crate-internal | clean | `git diff 26d72d2a3^ 26d72d2a3 -- '*.rs'` adds four `cfg(test)` items, all `owner_probe` / `CatalogOwnerProbe` in `expression/host/mod.rs`, already `#[cfg(test)] pub(crate)`; other new `pub` items are the spec's intended API (`Example<'a>`, owned descriptor fields, `descriptors_for`, `function_names`) |
| R2-F1.8 | `work-counters` build of `darkmatter` has no new warnings | fixed | `cargo check -p darkmatter --features work-counters --lib`: 0 warnings; `--lib` and `--features work-counters --lib --tests` clean; `just lint` exit 0 |
| R2-F1.9 | The six host-cache module tests still pass | fixed | `cargo nextest run -p darkmatter --lib -E 'test(host_function_cache_tests)'`: 6 passed |

### R2-F1 work

- starting the work on 'R2-F1: Keep host cache inspection helpers inside the library' at 19:14:09
        - defect class: test-only inspection seams added by this fix are reachable outside the crate through a feature-gated public module
        - the five items are used only by `host_function_cache_tests.rs`; grep of `darkmatter/` and `claudine/` found no other user
        - delegated to one foreground `rust-developer` subagent; orchestrator reviewed the resulting diff line by line
        - the `discovery_probe` module comment no longer claims host-function-set counts or "the process caches" for `work-counters` builds; it now says the host cache inspection items are crate-private and test-only
        - smallest repair chosen: `#[cfg(test)]` plus `pub(super)` on the new items only; no new seam or abstraction
        - no `docs/`, skill, or README referenced the narrowed items, so no doc drift
- work completed for 'R2-F1: Keep host cache inspection helpers inside the library' at 19:19:12

### Completion Report

- **outcome:** complete; elapsed about 7 minutes (19:13:44 → about 19:21)
- **parent findings:** total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- **instance rows:** 9 (7 fixed, 2 clean), none open
- **open finding IDs:** none
- **changed files:**
        - `darkmatter/lib/src/markdown/schemas/catalog.rs` (R2-F1.1–.5, and the module doc comment)
- **verification (after the final change):**
        - `just test` in `darkmatter/`: 12,610 passed, 10 skipped (R2-F1 overall, R2-F1.6, R2-F1.9)
        - `just lint` in `darkmatter/`: passed, exit 0 (R2-F1.8)
        - `just check-tier-coverage darkmatter` from the repo root: 0 stranded (no test added or renamed)
        - `cargo check -p darkmatter --features work-counters --lib`: 0 warnings (R2-F1.8)
        - `cargo check -p claudine --features work-counters --tests`: compiles clean (R2-F1.6: the downstream `work-counters` consumer did not depend on the narrowed items)
        - focused nextest `test(host_function_cache_tests) | test(discovery_tests)`: 13 passed (R2-F1.9)
- `claudine` `just test` was not rerun: no Claudine source changed, and the only coupling, its `work-counters` feature on `darkmatter`, was checked by the compile above
- no cross-OS check was run: the change is visibility and `cfg(test)` only, with no OS-specific code
