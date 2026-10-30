---
$schema: feature-review.yaml
ready: false
findings:
    - "R2-F1 (low): Keep host cache inspection helpers inside the library"
observations: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-26T19:07:59-07:00
spec: 2026-10-25-host-function-catalog-leak/spec.md
implemented: true
description: A **fix** review of `2026-10-25-host-function-catalog-leak/spec.md`
fix: 2026-10-25-host-function-catalog-leak/review-2.md
next: 2026-10-25-host-function-catalog-leak/review-3.md
previous: 2026-10-25-host-function-catalog-leak/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/fix-pipeline-finalization/darkmatter/fixes/2026-10-25-host-function-catalog-leak/log.md
implemented_by: claude/opus
---

# Review of Host Function Catalog Leak

The ownership repair and revision replacement behavior match the specification.
One small API boundary violation remains: new host-cache test helpers are
exported with a feature even though the spec requires library-internal access.
That finding keeps `ready: false`; it does not indicate a remaining catalog leak.

## Previous Finding Verification

The requested `review-1.md` is absent. A repository file search found no relocated
copy, and `git log --all -- <requested path>` returned no history. The supplied
`implementation-log.md` contains implementation phases but no `### Finding Ledger`
or previous finding IDs. Consequently, no previous finding can honestly be marked
fixed, partially repaired, or recurring. The implementation was reviewed directly
against the spec instead.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| Unavailable: previous finding inventory | Cannot enumerate without the first review and its repair ledger | unresolved | Requested file is absent; repository search and Git history did not recover it; implementation log has no Finding Ledger | Supply the original review and ledger to verify its actual findings. No previous IDs or outcomes have been invented. |

This row records missing review evidence, not an inferred implementation defect.
The missing previous review's `next` property could not be updated; creating a
replacement would fabricate its findings and implementation claims. The new
finding below is independently grounded in the spec. Its relationship to any
unavailable earlier finding cannot be determined; `recurrence: false` records
that no recurrence could be established from the available artifacts.

## Unblocked Findings

### R2-F1 (low): Keep host cache inspection helpers inside the library

**Authority — spec:** “Test-only access to ownership and cache entries stays
inside the library (`cfg(test)` helpers or module tests), never a new public
API.” In the darkmatter package, the new
[`cached_host_functions` helper](../../lib/src/markdown/schemas/catalog.rs)
exposes a snapshot of a host cache entry to callers outside the crate when the
`work-counters` feature is enabled. Its enclosing `discovery_probe` module and
the containing `schemas::catalog` module are public. This is an API visibility
defect, rather than an ownership or runtime behavior defect.

The new exported surface consists of `CachedArtifact`, `CachedHostFunctions`,
`cached_host_functions`, `host_function_entries_under`, and
`DiscoveryProbe::host_function_builds`. All of their current uses are in the
library's `host_function_cache_tests` module. The existing downstream tests use
other discovery instrumentation; they do not need these new exports. By contrast,
the ownership probe in `expression::host` correctly uses `#[cfg(test)]` and
crate-restricted visibility.

The smallest repair is to restrict the new host-cache helpers and their result
types to the enclosing module and compile the inspection-only items under
`#[cfg(test)]`. Keep the existing lifecycle instrumentation's visibility and
feature behavior. Re-run the six host-cache module tests and check a
`work-counters` build so the narrower visibility does not leave warnings.
No new abstraction or additional public seam is needed. Default production
builds do not export these helpers, so the immediate harm is small; the repair
prevents an explicitly excluded API from becoming a dependency.

## Blocked Findings

None. The repair for R2-F1 requires no human decision.

## Implementation and Coverage Review

The review boundary was the ownership migration and its consumers in commit
`26d72d2a3`, the host cache replacement, and the supplied working-tree changes
and validation log. Existing unrelated working-tree changes were left intact.

| Spec requirement | Implementation evidence and verification | Appropriate level |
| --- | --- | --- |
| Reclaim catalog-only, artifact-only, and failing replacements | Six module tests in [host_function_cache_tests.rs](../../lib/src/markdown/schemas/catalog/host_function_cache_tests.rs) exercise production discovery with unique temporary paths, inspect one entry per path, and observe the old `CatalogData` through a weak reference. Catalog-only input includes inline examples and a display-only reason. | L1: ownership and cache state are in-process properties. |
| Replace successes and failures; recover from missing, invalid, stale, and unreadable facts | The same module checks success → error, unchanged error reuse, error → success, and valid → absent → invalid JSON → stale hash → non-UTF-8 → valid facts. It checks revisions, build counts, completeness, and diagnostics. | L1: public discovery results plus private cache inspection. |
| Held sets and guards survive; independent paths and shadowing remain intact | Tests retain the old scope while loading a replacement, compare registration parameter names and call-analysis signatures, drop newer guards, and finally observe reclamation. Distinct-root fixtures verify discovery order, shadowing, and independent cache entries. | L1: no terminal or browser behavior is involved. |
| Failed partial construction leaks nothing | [Catalog projection](../../lib/src/markdown/compose/expression/catalog/mod.rs) owns categories, aliases, strings, parameter arrays, compiled types, examples, reasons, and descriptor arrays. Both example projection helpers return owned values. The public host-load test rejects a later file-form example after an earlier valid inline example. No leak, unsafe lifetime extension, or forgotten allocation remains in these loading paths. | L1 plus source ownership inspection, as the spec requires. |
| Preserve static borrows and shared example semantics | `static_catalog_accessors_lend_permanent_borrows` pins the three static slice APIs and lifecycle string views at compile time. Catalog tests check first-inline-example selection, display-only reasons, error outcomes, file-only examples, missing examples, and source-independent ownership. | L1. |
| Preserve generated text, CLI output, editor output, and consumers | Exact generated-region tests, deterministic facts artifacts, CLI plain-output checks, pinned hover/completion text, and editor host-diagnostic integration tests are declared in running targets. The two generator examples are checked separately against the captured output. Claudine registration and descriptor consumers were inspected for borrow/clone changes. | L1 and hermetic CLI/LSP integration. The spec promises preserved text and diagnostics, not emulator rendering or keyboard behavior. |

Cache inspection confirms a `HashMap<PathBuf, HostFunctionEntry>` keyed by the
unchanged discovery path. Each entry stores one catalog digest, full artifact
state, and result. Comparison, construction, and replacement occur under the
same mutex. Changed failures overwrite earlier successes; unchanged errors are
reused. Neither retained old sets nor returned errors borrow artifact text.
The separately read catalog and artifact remain a non-atomic snapshot, as the
spec explicitly permits.

The borrowed `Described` methods and lifetime-parameterized example views match
the planned API. Static consumers recover permanent borrows from concrete static
owners. Documentation and skill guidance explain the owned model and per-path
retention without promising a fixed total cache size. No dependency was added.

## Input Robustness Matrix

This refactor changes the catalog loader's allocation and projection ownership;
it does not add a format, field validator, or new interpretation of invalid
input. Comparison with the parent commit confirms the field-reading and
validation decisions are preserved. The facts-artifact reader is unchanged;
its existing public-API robustness matrix remains selected in L1. The new cache
transition test separately proves that malformed or unreadable facts produce
the current incomplete classification and that a later valid revision recovers.
No additional format-by-field matrix is warranted for replacing borrowed fields
with owned fields.

## Validation

- `just test` in `darkmatter/`: **12,610 passed**, 10 skipped, 100.510 seconds.
  Nextest reported four slow tests; a passing suite is not a claim that every
  repository test meets the five-second budget.
- A focused nextest run of the host-cache module, expression catalog, and static
  borrow contract: **32 passed**, no slow tests, 0.131 seconds. This separately
  confirms the core ownership checks stay within the L1 budget.
- Focused Claudine nextest run selecting context reporting, host catalogs,
  schema consumers, and prewarm tests: **977 passed**, 10,356 excluded by the
  filter/tier selection, 71.605 seconds. One slow test was reported. The macOS
  linker also emitted its large unwind-table warning.
- `just check-tier-coverage darkmatter`: **no stranded tests**. The host tests,
  schema-generation tests, CLI schema-about tests, and editor diagnostics are
  declared in their consolidated L1 targets; the cache tests are compiled by
  the library module.
- `cargo check --color=never -p darkmatter --example expression_doc_generator
  --example lifecycle_schema_generator`: **passed**.
- Running both examples without write mode and comparing stdout to the captured
  baselines: **byte-identical**, 31,669 bytes for the expression table and
  75,172 bytes for the lifecycle schema. No baseline was changed.

Two attempted filtered `just test` commands failed in shell argument forwarding
before running tests; the unfiltered recipe above completed successfully.
Lint was not rerun for this documentation-only review. The implementation log's
lint and full Claudine-suite claims were read as historical claims, not counted
as newly executed checks.

No real-terminal or browser window was opened. Cross-OS execution is left to CI
and is not a readiness finding.

## Observations

None.
