---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-26T19:20:58-07:00
spec: 2026-10-25-host-function-catalog-leak/spec.md
implemented: false
description: A **fix** review of `2026-10-25-host-function-catalog-leak/spec.md`
fix: 2026-10-25-host-function-catalog-leak/review-3.md
previous: 2026-10-25-host-function-catalog-leak/review-2.md
---

# Review of Host Function Catalog Leak

The fix is production ready. The previous API visibility finding is fully repaired,
including every instance in the repair ledger. No new findings or observations
remain. No human decision is required.

## Previous Finding Verification

Read [review-2.md](review-2.md) and the repair's
[Finding Ledger](log.md#finding-ledger) before inspecting the changes. The ledger's
completion statements were checked against source, call sites, and fresh builds.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F1: Keep host cache inspection helpers inside the library | All nine ledger rows, R2-F1.1 through R2-F1.9, detailed below | verified fixed | The five inspection items are module-private and test-only; counter storage and updates are test-only; lifecycle instrumentation is unchanged; both feature-enabled consumer builds pass; the six host-cache tests run in L1 | Nothing |

The complete ledger instance check follows. These are verification rows for the
previous finding, not new findings.

| Ledger instance | Required result | Verification evidence |
| --- | --- | --- |
| R2-F1.1 | `CachedArtifact` stays internal and test-only | `#[cfg(test)] pub(super) enum` in darkmatter's [discovery probe](../../lib/src/markdown/schemas/catalog.rs#L899) |
| R2-F1.2 | `CachedHostFunctions` and its fields stay internal and test-only | The struct is `#[cfg(test)] pub(super)`; each field is `pub(super)` in the same module |
| R2-F1.3 | `cached_host_functions` stays internal and test-only | `#[cfg(test)] pub(super) fn`; repository search finds callers only in the library's cache tests |
| R2-F1.4 | `host_function_entries_under` stays internal and test-only | `#[cfg(test)] pub(super) fn`; only the cache-test fixture calls it |
| R2-F1.5 | Host build-count inspection, storage, and updates stay test-only | `DiscoveryProbe::host_function_builds` is `#[cfg(test)] pub(super)`; its state field and increment are also under `#[cfg(test)]`. The internal notification function remains callable in every build configuration |
| R2-F1.6 | Preserve existing lifecycle instrumentation and downstream feature use | The repair diff leaves `install`, `builds`, `take_reads`, `fail_reads_of`, and `cached_outcome` unchanged. Discovery tests run in the L1 suite; `cargo check -p claudine --features work-counters --tests` passes |
| R2-F1.7 | Other ownership inspection seams stay library-internal | Inspection of the original implementation diff and current source confirms `owner_probe`, `CatalogOwnerProbe`, and its implementation are `#[cfg(test)]` and crate-restricted in darkmatter's [host catalog module](../../lib/src/markdown/compose/expression/host/mod.rs#L178). Other public additions are the spec's intended owned descriptor and borrowed example APIs |
| R2-F1.8 | Feature-enabled darkmatter build remains clean | `cargo check -p darkmatter --features work-counters --lib --tests` succeeds without warnings; it checks both the ordinary dependency and test configurations |
| R2-F1.9 | Keep all six host-cache tests working | The library declares `host_function_cache_tests` under `#[cfg(test)]`; its six tests are selected by the ordinary L1 recipe and pass |

The first review remains unavailable: this directory has no `review-1.md`, and
Git history for that path has no entries. The previous review explicitly recorded
this as missing historical evidence, not a finding. There is no earlier open
finding ID to carry or invent. This limits historical verification but does not
leave a known implementation defect unresolved.

## Unblocked Findings

None.

## Blocked Findings

None.

## Implementation and Coverage Review

The boundary is the owned catalog migration and its consumers in commit
`26d72d2a3`, plus the current repair to inspection visibility. Existing unrelated
Claudine working-tree changes were left untouched.

| Spec acceptance requirement | Evidence checked | Verification level |
| --- | --- | --- |
| Reclaim owners after catalog-only, artifact-only, and failing replacements | [Cache tests](../../lib/src/markdown/schemas/catalog/host_function_cache_tests.rs) use production discovery, unique temporary paths, and weak owner probes. The catalog-only fixture contains inline examples and a display-only reason. Dropping all old sets, clones, and guards releases the old owner | L1 plus ownership inspection |
| Keep one result per path, reuse unchanged errors, and recover across facts states | The cache maps a discovery path to one entry containing digest, full artifact state, and result. Tests check unchanged reuse, success → failure → success, and valid → absent → invalid JSON → stale hash → non-UTF-8 → valid facts, including incomplete classifications and reasons | L1 through public discovery with private cache observations |
| Preserve held values and independent paths | Tests retain old registrations and scope guards across replacement, compare call-analysis signatures, restore the old scope after dropping the new guard, and check two independent paths plus root shadowing | L1 |
| Release partial construction on error | [Catalog loading and projection](../../lib/src/markdown/compose/expression/catalog/mod.rs) own categories, aliases, names, descriptions, compiled types, parameter arrays, example text, display-only reasons, and descriptor arrays. Both example helpers return owned values. Public host-load coverage rejects a later non-embedded file example after a valid inline example. No leak, forgotten allocation, or unsafe lifetime extension remains in these paths | L1 plus source ownership inspection |
| Preserve permanent built-in borrows and output | Typed bindings in `static_catalog_accessors_lend_permanent_borrows` pin all three static descriptor slice APIs and lifecycle string views. Catalog tests verify example selection and metadata. Generated-region checks, CLI schema-about checks, pinned DMLS hover/completion text, and watched-file host diagnostics run in declared L1 targets | L1 and hermetic CLI/LSP integration |
| Compile consumers and examples without changing artifacts | The feature-enabled Claudine test build succeeds. Both generator examples build and run; their stdout matches the previously captured baselines byte for byte. Existing facts-artifact and generated-region checks remain selected | L1 and compile checks |

The cache preserves the existing path identity and root order. Comparison,
construction, and replacement remain under one mutex; a changed failing revision
replaces the successful result. Separately reading catalog and artifact is the
explicitly accepted snapshot contract. The cache retains one result per encountered
path, not a fixed total number of paths; the documentation states that bound.

The shared example views borrow their strings, including display-only reasons,
from their descriptor owner. Static consumers obtain permanent borrows from the
concrete static owner. The related documentation, framework README, and skill
guidance match these lifetimes. The repair also corrects the discovery-probe
comment so it no longer promises host-cache inspection to feature-enabled callers.

No requirement involves terminal-emulator rendering or keyboard encoding. L1
checks of text, ownership, discovery, and editor protocol results are appropriate;
L2, L3, and browser tests would not add relevant evidence.

## Input Robustness Matrix

No new file-format interpretation or field validator was introduced. The catalog
refactor replaces storage ownership, and this repair changes visibility and test
configuration only. Inspection of the implementation diff confirms validation
branches and parsing decisions are preserved. The facts reader is unchanged;
its public-result tests still run. The cache-transition tests independently check
invalid and unreadable facts and recovery. No additional configuration-shape
matrix is needed for this ownership and visibility change.

## Validation

- `just test` in `darkmatter/`: **12,610 passed**, 10 skipped, 133.420 seconds.
  Nextest reported 17 slow tests. This passing suite does not establish that
  every repository test meets the five-second budget; unrelated editor suites
  account for the slow cases noted during the run.
- Focused nextest run of the host-cache tests, expression catalog tests, and
  permanent-borrow contract: **30 passed in 0.148 seconds**, no slow tests.
- `just lint` in `darkmatter/`: **passed**, including the Zed WASM extension check.
- `cargo check --color=never -p darkmatter --features work-counters --lib --tests`:
  **passed without warnings**.
- `cargo check --color=never -p claudine --features work-counters --tests`:
  **passed without warnings**.
- `just check-tier-coverage darkmatter`: **zero stranded tests**.
- Both generator examples compiled and ran without write mode. Output compared
  directly against the supplied pre-refactor baselines: **byte-identical**,
  31,669 bytes for the expression table and 75,172 bytes for the lifecycle schema.
  No baseline or generated artifact was changed.

The repair log's earlier full Claudine test and lint claims are historical
evidence, not fresh runs by this review. This review independently checks its
feature-enabled test compilation because the narrowed instrumentation is the
only changed downstream boundary. Cross-OS execution remains CI's responsibility
and is not a readiness finding. No terminal or browser window was opened.

## Recurrence

No finding repeats or introduces a sibling of a prior defect. R2-F1 is verified
fixed in full; there is no partial repair or regression to carry forward.

## Observations

None.
