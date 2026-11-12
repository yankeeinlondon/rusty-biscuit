---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): Downstream freshness scheduling omits stored-digest preparation code"
    - "R1-F2 (high): Root regeneration replaces artifacts before full validation"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-11T18:24:48-08:00
spec: 2026-11-11-precompiled-shipped-schemas/spec.md
implemented: true
description: "A **feature** review of `2026-11-11-precompiled-shipped-schemas/spec.md`"
feature: 2026-11-11-precompiled-shipped-schemas/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-11-precompiled-shipped-schemas/log.md
implemented_by: claude/opus
next: 2026-11-11-precompiled-shipped-schemas/review-2.md
---

# Review 1: precompiled shipped schemas

The feature is **not production ready**. Two requirements remain incomplete:
downstream freshness checks must run when the code producing stored digests
changes, and regeneration must validate all candidates before replacing files.
Both repairs can proceed without an author ruling.

This review compared the spec, plan, implementation log, and implementation
commits through `08fac9d87`. Two parallel readers independently inventoried the
implementation decisions and the spec's delegated choices. There are no previous
review findings to carry forward.

## Findings

### R1-F1 (high): Downstream freshness scheduling omits stored-digest preparation code

**Authority: spec**, “Where the artifacts live and how they are refreshed”:
“Add exact cross-package code paths to the `source-inputs` list under
`[package.metadata.ci.tests]` when their preparation behavior affects output.”
The same section requires downstream freshness tests to spell their transitive
inputs in forms the test-input planner recognizes.

**Defect class:** incomplete dependency declarations allow a preparation-code
change to bypass the downstream tests that detect stale generated data.

Simplified Schema's [canonical identity revision, digest calculation, and file-dependency classification](../../../simplified-schema/lib/src/validate.rs)
determine the `CanonicalDigestRecord` values stored in Darkmatter's
baseline and Claudine's trigger payloads. Neither downstream manifest declares
that file for these tests, and neither `shipped_artifact_is_fresh` function
includes it. A normal maintenance edit to `CANONICAL_IDENTITY_REVISION`, for
example, changes both higher artifacts while leaving Simplified Schema's own
envelope-only artifact unchanged. Its passing freshness test therefore cannot
substitute for the two missing downstream runs. Existing artifacts may fall
back at runtime, but CI fails to enforce their regeneration.

**Reproduction:** run
`python3 scripts/ci/affected_scope.py simplified-schema/lib/src/validate.rs`.
The selected packages are `simplified-schema` and `test-toolkit`; neither
Darkmatter nor Claudine has an L1 cell. I also called the existing
`RealWorkspaceTestInputTests.plan` helper with `event="pull_request"` and
inspected the resolved cells and test filters, with the same result. No source
file was edited to obtain this evidence.

The sweep covers all three owners' freshness decisions, followed through the
planner to the cells and filters that actually run:

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Darkmatter freshness test and manifest | `simplified-schema/lib/src/validate.rs` source change | No Darkmatter L1 cell | Ubuntu L1 cell selecting `shipped_artifacts::shipped_artifact_is_fresh` |
| R1-F1.2 | Claudine freshness test and manifest | Same source change | No Claudine L1 cell | Ubuntu L1 cell selecting its freshness test |
| R1-F1.3 | Simplified Schema's own freshness test | Same source change | Clean: its full L1 suite is selected | Owner suite runs; this does not verify higher artifacts |
| R1-F1.4 | Both downstream tests | `simplified-schema/lib/src/shipped/wire.rs` source change | Clean: both narrowed cells include the freshness test | Both tests selected |
| R1-F1.5 | Both downstream tests | `simplified-schema/schemas/schema.yaml` change | Clean: both narrowed cells include the freshness test | Both tests selected |
| R1-F1.6 | Both downstream tests | Simplified Schema artifact-only change | Clean: both narrowed cells include the freshness test | Both tests selected |
| R1-F1.7 | Claudine freshness test | Darkmatter artifact-only change | Clean: its Ubuntu L1 filter includes the freshness test | Test selected |

**Recommended repair:** add the missing exact input to both manifests and both
freshness functions, then extend the existing planner regression test with this
case. Audit the curated list against the preparation chain, including the new
digest-producing code, while retaining narrowed cells. A new CI job or full
dependent-suite scheduling is unnecessary.

Relevant sites:
[Darkmatter manifest](../../lib/Cargo.toml),
[Darkmatter freshness test](../../lib/tests/l1/shipped_artifacts.rs),
[Claudine manifest](../../../claudine/lib/Cargo.toml),
[Claudine freshness test](../../../claudine/lib/tests/l1/shipped_artifacts.rs),
and [planner regression test](../../../scripts/ci/test_affected_scope.py).

### R1-F2 (high): Root regeneration replaces artifacts before full validation

**Authority: spec**, “Where the artifacts live and how they are refreshed”:
“Validate all outputs before replacing any file; do not write partially
generated JSON.” The implementation plan also requires a refused generation
to leave the existing artifacts unchanged.

**Defect class:** publication happens before the validation that can reject its
output, so a failed regeneration leaves rejected artifacts installed.

The root [regeneration coordinator](../../../scripts/shipped_schemas.py)
checks only JSON structure, owner names, dependency owner names, and final
line feeds before calling `replace_all`. It then invokes each generator's
full `--check`. A failed acceptance or freshness check exits 1 after all three
tracked artifacts have already been replaced. The existing
`a_failed_check_after_writing_is_reported` test asserts the error but never
asserts preservation of the previous files. It codifies the incomplete path.

**Reproduction:** use the coordinator's existing `--root` and `--generator`
interfaces with a temporary tree and the same stand-in design as its contract
tests: emit structurally valid candidate artifacts, and have one owner's
`--check` exit 1. I repeated the run once per owner. In every failure case the
coordinator exited 1 and all three artifact files differed from their originals.
The all-success control exited 0 and replaced all three. No tracked artifact
was modified by these reproductions.

The sweep covers each owner at the root publication boundary and the three
area-specific write paths:

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F2.1 | Root coordinator, Simplified Schema check | First owner's full check refuses | Exit 1; all three files replaced | Refuse before replacing any file |
| R1-F2.2 | Root coordinator, Darkmatter check | Second owner's full check refuses | Exit 1; all three files replaced | Refuse before replacing any file |
| R1-F2.3 | Root coordinator, Claudine check | Third owner's full check refuses | Exit 1; all three files replaced | Refuse before replacing any file |
| R1-F2.4 | Root coordinator control | Every check succeeds | Clean: exit 0; all three replaced | Publish validated candidates |
| R1-F2.5 | Simplified Schema example's `write` | Fresh bytes fail artifact acceptance | Clean by code invariant: `check_artifact(fresh)` precedes any write | Existing file preserved |
| R1-F2.6 | Darkmatter example's `write` | Same refusal, with current lower artifact | Clean by the same invariant | Existing file preserved |
| R1-F2.7 | Claudine example's `write` | Same refusal, with current lower artifacts | Clean by the same invariant | Existing file preserved |

The area paths' generation-failure and check/write controls also pass in their
L1 suites. The failure is confined to the root coordinator's ordering.

**Recommended repair:** validate each candidate against the freshly generated
lower candidates before publication. This can be implemented in Rust on the
generator side or through a candidate-check mode; Python does not need a new
hashing implementation. Keep post-write checks if useful, but they cannot
replace pre-write validation. Extend the existing coordinator contract tests to
assert that any owner's validation refusal preserves all three original files.
No crash-safe multi-file transaction or protection against arbitrary disk
failures is requested.

## Implementation decisions (informational)

These choices need no author intervention. The spec explicitly delegates
technical choices and allows recorded departures closest to its intent.
The two deficient outcomes above remain ordinary findings with recommended
repairs; they are not pending design decisions.

| Implementer choice | Review disposition |
| --- | --- |
| Exercise generator examples through their injected `run` functions in-process instead of spawning Cargo or adding shipped binaries | Keep. It verifies argument handling and output without requiring examples in nextest archives. |
| Add `serde_path_to_error` for located refusals | Keep. It is already in the workspace dependency graph and serves an explicit diagnostic requirement. |
| Reject host-backed identities in custom Serde implementations, including their use outside artifact generation | Keep. These are newly added serialization implementations; no existing serialization contract is lost. |
| Bypass identity-keyed preparation state during uncached generation while retaining deterministic content-keyed regex and structural-validator caches | Keep. These caches cannot substitute an old source's result for changed content. |
| Store only lowered envelope JSON in Simplified Schema's artifact | Keep. Admission consumes that form; ordered typed records are exercised elsewhere. |
| Report a wrong artifact owner as `UnusableRecord` | Keep. A separate reason variant adds no required behavior. |
| Decode sections lazily and fall back per damaged section, after whole-header source/dependency checks | Keep as a documented performance departure. Changed sources or dependencies still refuse the owning artifact; decoded sections are checked before use. Malformed-section behavior has explicit tests. |
| Pretty-print the header and put each compact payload section on one line; reject other layouts | Keep. Artifacts have one generated canonical representation, and refusal falls back safely. |
| Reparse trigger match conditions from compiled text; serialize prepared payloads only | Keep. Request-bound path conditions remain outside immutable prepared data. |
| Compare bundle names and files without comparing optional prepared descriptors | Keep. Eligibility separately checks content; the descriptor changes preparation cost. |
| Use curated downstream source-input lists instead of every lower-crate Rust file | Keep the bounded approach; repair the demonstrated missing preparation input under R1-F1. |
| Expose injected registry loading and a hidden baseline-attachment seam publicly for tests | Keep. These are contained, reversible API choices. |
| Include both envelope YAML sources directly in Darkmatter's inventory as well as its lower-artifact dependency | Keep. Darkmatter directly prepares kind definitions from them. |
| Decode Claudine's complete trigger bundle as one section | Keep. Splitting it alone would not avoid discovery's eager payload reads. |
| Perform structural root-coordinator checks before replacement and full checks afterward | Repair under R1-F2. The implementation identifies a concrete alternative; no author choice is needed. |
| Put coordinator tests in test-toolkit and use a stand-in generator | Keep. This exercises the real script without nested builds. |
| Return generation errors as strings and add two typed-error guard allowances | Keep. This follows the other generator APIs and concerns a developer command. |
| Give single-layer schemas a role-bearing layered identity distinct from a canonical identity | Keep. Separate domains are explicit; occasional duplicate compilation is acceptable. |
| Spawn the current test executable directly for cross-process identity stability | Keep. It exercises a real process boundary and pins deterministic encoding. |
| Keep immutable artifact registries and fallback prepared constants across benchmark resets; clear dynamic caches | Keep. This retains the previous distinction between immutable shipped constants and dynamic cached work, now documented and tested. |
| Remove speed-only reduced-schema fixtures from seven CLI modules and the equivalent library fixture; retain custom-schema and shadowing fixtures | Keep. The log records the fixture-by-fixture reasoning and unchanged assertions. |
| Lazily project expression declarations after profiling | Keep. It avoids work for documents without declaration reads, with public composition tests and a work counter. |
| Try plain JSON decoding first, repeating failures with path tracking | Keep. Refusal locations remain covered while successful loads avoid that cost. |
| Use paired loaded-host timings and retain the Claudine debug goal miss | Informational limitation. The spec calls timings local goals; the paired 1450 → 839 ms sample and work counters demonstrate the intended work removal. The 650 ms goal is not claimed achieved. |

Deferred work and reasons:

- Per-document lazy trigger decoding was tried and reverted because discovery
  still reads all payloads. Making the registry lazy would affect discovery,
  assembly, tracing, and effective-schema selection for an estimated 60 ms
  debug improvement. Keep this out of the required repair.
- Further effective-validator compile optimization was left outside this
  feature; the spec explicitly retains that document-dependent runtime work.
- Exhaustive downstream input declarations were replaced by the curated
  approach to avoid unnecessary downstream archive builds. R1-F1 requests
  complete coverage of actual preparation inputs, not every source file.
- Full candidate validation before root replacement was deferred because the
  coordinator lacks Python xxHash. R1-F2 removes that deferral using the
  existing Rust implementation.
- Quiet-host timing evidence was unavailable; the log reports paired samples
  under load rather than claiming a quiet measurement.
- Cross-OS local runs were left to normal CI. Their absence does not block
  readiness in this review.

None of these items passes the test for interrupting the author. There are no
human review items or blocked findings.

## Verification and input robustness

All behavior here can be verified at **Level 1**: prepared data, schema
selection, validation, source positions, cache identity, and headless developer
commands. No requirement changes keyboard encoding, terminal presentation, or
browser interaction, so Level 2 or Level 3 evidence is unnecessary.

| Requirement | Strongest relevant verification inspected or run |
| --- | --- |
| Freshness and ordered prepared-data round trips | L1 `shipped_artifacts` suites in all three crates; all three freshness tests rerun |
| Accepted artifacts remove preparation work; stale/custom inputs use ordinary preparation | L1 counters and public schema validation/discovery in artifact and prepared-trigger suites |
| Disk suppression, surviving bundle imports, changed custom bundle text | L1 public discovery and validation in `prepared_trigger_bundles`; existing kind-registry/editor tests retained |
| Lazy positions retain authored coordinates | L1 deferred-position tests, plus retained coordinate fixtures; Darkmatter's four entry-point tests rerun |
| Layer roles, override order, imports, templates, phase changes, context isolation, canonical fallback | L1 `schema_identities` and `layered_validator_identities`, rerun |
| No unnecessary declaration projection | L1 public composition and counters in `raw_declaration_projection`, rerun |
| DMLS reuses immutable data while retaining workspace/buffer invalidation | Existing L1 schema/editor tests inspected through the implementation and recorded full Darkmatter gate; no new interaction contract |
| Root regeneration preserves originals when validation refuses | Gap: R1-F2 |
| Downstream freshness runs for preparation-code changes | Gap: R1-F1 |

The new format reader is the generated JSON artifact mechanism, including the
deferred section reader. Its normal invalid-input outcome is refusal followed
by preparation from compiled text, without new warnings or side effects.
For deferred sections, header/layout/member-list failures refuse the artifact;
section decoding or record failures refuse that section. Explicit `null` is
allowed for nullable metadata; required-presence deserializers distinguish it
from omission. Unknown and duplicate members, trailing content, invalid JSON,
source/dependency mismatches, incompatible revisions, and unusable records
are covered by the shared loader and owner-specific L1 tests.

I checked the new reader code for load-bearing `serde(default)`, optional
members, silent `filter_map`, `unwrap_or_default`, and discarded parse errors.
The optional record fields use required-presence handling where omission is
invalid; the header's dependency filtering follows an explicit refusal check.
No additional realistic wrong-result case emerged from that inspection.
The root coordinator's weaker validation/publication boundary is covered in
R1-F2 rather than treated as another field-matrix finding.

Review runs through canonical recipes:

- Simplified Schema: `just test shipped_artifact` — 98 targeted tests passed;
  its recipe also ran all 1693 minimal-feature tests. `just test schema_identities`
  — 12 targeted tests passed, plus the minimal suite again.
- Darkmatter: `just test shipped_artifact` — 50 tests passed across the area;
  `just test layered_validator_identities` — 13; `just test prepared_trigger_bundles`
  — 15; `just test deferred_positions` — 4; `just test raw_declaration_projection`
  — 3. All passed.
- Claudine: `just test-library shipped_artifact` — 23 passed;
  `just test-library --features work-counters shipped_artifact` — 25 passed,
  including the instrumentation-dependent cases.
- Every completed review run had zero tests at or above 5 seconds. The
  implementation's recorded full suites and lint runs were inspected; they
  were not repeated wholesale. Its recorded 5–7.8 s Claudine tests occurred
  under load and were individually remeasured at 1–2.2 s, satisfying the
  supplied loaded-host rule.
- Planner reproductions inspected actual pull-request cells and filters.
  Coordinator failure/control reproductions ran the real script with isolated
  temporary files and its supported stand-in interface.

The new test files are declared in consolidated test targets. Counter tests
are enabled by their owners' CI feature metadata. No terminal or browser
window was opened. One attempted combined test filter was expanded by the
recipe shell and exited 127; the affected modules were subsequently run with
separate simple filters, as listed above.

No verified false behavior statement in the implementation log required an
in-place correction. The recorded departures and timing limitation are
explicit. The spec's review iteration is now 1; it is not marked completed.
