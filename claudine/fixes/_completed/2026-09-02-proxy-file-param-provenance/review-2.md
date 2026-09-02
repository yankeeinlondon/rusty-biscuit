---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-02T11:02:07+01:00
spec: 2026-09-02-proxy-file-param-provenance/spec.md
implemented: true
implemented_by: codex/default
log: claudine/fixes/2026-09-02-proxy-file-param-provenance/log.md
description: A **fix** review of `2026-09-02-proxy-file-param-provenance/spec.md`
fix: 2026-09-02-proxy-file-param-provenance/review-2.md
previous: 2026-09-02-proxy-file-param-provenance/review-1.md
next: 2026-09-02-proxy-file-param-provenance/review-3.md
---

# Review 2: Proxy File Parameter Provenance

## Verdict

The fix is **not ready for production**. Review 1's candidate-ordering defect
is repaired at the `FileReference` boundary, the focused root-union, re-entry,
and process suites pass, and the package lint gates are green. However, the new
diagnostic carrier collapses distinct caller records that materialize to the
same path, root-union selection validates the whole instance using one
property's caller origin, and sequence task records can resurrect a task param
that a higher-precedence runtime or reserved-overlay value replaced. Required
Level 1 assertions for complete proxy diagnostic evidence and all shipped
derived paths also remain absent.

## Findings

### 1. High: semantic-path indexing collapses distinct caller provenance

`CallerProjection::provenance` is a `HashMap<String,
CallerFileProvenance>` keyed only by the materialized candidate string.
`resolve_caller_file_overrides` inserts every scalar or array projection under
`candidate.to_string_lossy()` (`schema_validation.rs:552-555`), and
`file_reference_error` later recovers provenance using only the semantic path
passed to the builtin (`expression/functions/mod.rs:1455-1466`).

Two caller properties can legitimately resolve to the same file, as can two
array elements with different raw spellings such as `spec.md` and
`./spec.md`. The later insertion overwrites the earlier record. A failure while
reading the first value can consequently report the other property's raw
reference and property name. Because caller records are iterated by property
name, changing an unrelated property name can also change which diagnostic
survives. The same lossy map is included in request identity
(`context/options.rs:2058-2076`), so the cache fingerprint represents the
collapsed carrier rather than every materialized projection.

This violates D1, D6, D7 and acceptance criteria 11-13. It also means the
implementation does not actually retain provenance per winning property.

**Required change:** retain provenance at the value/property occurrence that
produced the expression argument, or use an identity that cannot alias distinct
properties and array elements. Add Level 1 cases for two properties sharing a
candidate and for duplicate array identities with different raw spellings;
assert the correct raw value, property, origin, candidate, and request identity
for each failure.

### 2. High: root-union applicability applies one caller origin to unrelated fields

Root-arm selection is performed separately for each caller record and passes
that record's origin into the validator
(`schema_validation.rs:520-525, 580-585`). `root_schema_arm_applies` then uses
the same origin and base directory to validate the entire effective instance
(`schema_validation.rs:738-750`).

An arm can contain document-owned eager file properties or other caller file
properties with different origins. Those values must not be resolved using the
origin of whichever property happens to be classified. A document-owned eager
file can therefore make an otherwise applicable arm fail—or a wrong arm
pass—based on a file present in the caller's launch directory rather than the
target document directory. With two caller properties from different origins,
the same effective instance can select different root arms for each property.

The new union tests use string discriminators and only the caller file under
classification, so they do not exercise mixed ownership or multiple caller
origins. This violates D1, D2, D5 and acceptance criteria 7, 9, and 11.

**Required change:** select one applicable root schema path for the effective
instance using provenance-correct validation for every field, then classify
all caller properties against that path. Add Level 1 cases with a
document-authored eager file sibling and with two caller file properties whose
origins differ.

### 3. High: sequence task provenance can override higher-precedence layers

`WrapperPromptRunner::run` creates caller records for every evaluated task
param and then overlays CLI caller records (`task_run.rs:405-411`). It does not
remove a task record when the effective `set_overrides` value came from a
runtime mutation or the reserved overlay. Those layers intentionally outrank
task params and user setters (`sequence/task/mod.rs:737-754`).

Darkmatter then reconstructs its classification instance from every retained
record (`schema_validation.rs:483-490`) and installs the projected record value
into frontmatter. If a task param named `spec` is replaced by a runtime
mutation before the prompt task runs, the stale task record is materialized and
silently restores the lower-precedence value. The mixed-origin process test
uses disjoint keys and cannot detect this regression.

This changes established sequence precedence, contrary to D1, D4 and the
explicit non-goal, and violates acceptance criteria 4, 7, 10, and 11.

**Required change:** build provenance records from the winning layered values,
preserving each winning layer's origin, rather than from all raw task params.
Add Level 1 process cases in which a task file param is shadowed independently
by a CLI setter, runtime mutation, and reserved overlay.

### 4. Medium: structured candidate detail asserts `missing` without evidence

`file_reference_detail` correctly leaves the top-level `failure` field null
because the read-side classifier cannot distinguish a clean miss from a
permission failure (`error/render/mod.rs:226-229`), but it unconditionally
serializes the retained candidate with `"disposition": "missing"`
(`error/render/mod.rs:265-269`). That makes the nested field claim the same
classification the surrounding comment says is unavailable.

**Required change:** carry the resolver's actual candidate disposition, or
leave it unknown/null until the resolver provides it. Add a Level 1 structured
detail test for a non-missing read failure so machine diagnostics never report
an invented miss.

### 5. High: required Level 1 evidence remains incomplete

The direct/proxy failure process matrix checks a headline, the raw spelling,
and the target filename (`compose_caller_file_provenance.rs:498-525`). It does
not assert equal diagnostic codes, caller bases/origins, selected candidates,
or structured detail across the two routes. The Darkmatter provenance test
asserts those fields only for one direct lazy-read failure. The shipped workflow
test asserts `iteration` and `log`, but not the derived `review` path or the
optional `design` path (`compose_caller_file_provenance.rs:93-123`).

These are Level 1 semantic/process requirements; Levels 2 and 3 are not needed.
Under the review's stated rigor rules, the missing route-level evidence is a
gap rather than production-ready coverage. This leaves acceptance criteria 3,
12, and 15 unverified.

**Required change:** expose/capture structured diagnostics in the fake-provider
process matrix and compare code plus raw/origin/candidate fields for direct and
proxied malformed, eager-missing, and lazy-read-missing cases. Extend the exact
shipped fixture to assert `review`, `log`, and both present/absent `design`
derivation beside the specification.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
|---|---|---|
| Shipped `implement.md` routes an area-relative `spec` to the lazy target and reaches the provider | Level 1 fake-provider process test | Appropriate and present. |
| Router, direct target, and proxied target read the same specification | Level 1 fake-provider process tests | Appropriate for the single-property success case. |
| Target derives `review`, `log`, and optional `design` beside `spec` | Level 1 assertion for `log`; indirect use of `review`; no present-design assertion | **Gap:** complete path derivation is not asserted (Finding 5). |
| Lazy local files use `FileReference`-owned candidate ordering | Level 1 collision matrix for implicit, magic, package, and explicit-relative references | Appropriate and present; Review 1 Finding 1 is resolved. |
| Scalar, array, property-union, and root-union schemas select exactly one applicable file arm | Level 1 scalar/array/property/root-union tests | **Broken for mixed origins:** root applicability uses one caller origin for the whole arm (Finding 2). |
| Caller origin survives proxy, retry, resume, loop, inline-compose, and sequence/task routes | Level 1 fake-provider process tests for every named route | Appropriate for the covered non-conflicting values. |
| Task values and CLI caller values retain distinct origins without changing precedence | Level 1 mixed-origin process test with disjoint keys | **Broken/gap:** runtime and overlay shadowing are neither preserved nor tested (Finding 3). |
| Missing/malformed direct and proxy failures retain equal typed identity and provenance evidence | Level 1 process headline/raw checks; direct Darkmatter field assertions | **Gap:** proxy code/origin/candidate equality is not asserted, and aliased paths can report wrong provenance (Findings 1 and 5). |
| Equal raw values from different origins have distinct compose/cache identities while approval identity is unchanged | Level 1 compose fingerprint and approval-cache tests | Appropriate for distinct paths; aliased semantic projections are collapsed (Finding 1). |
| Absent/null/default/document-owned values keep their ownership | Level 1 focused Darkmatter tests | Partial; root-union validation can apply caller context to document-owned fields (Finding 2). |
| Native semantic and portable presentation paths preserve identity on macOS, Linux, and Windows | Level 1 host tests plus an enabled `#[cfg(windows)]` test | Appropriate; macOS was exercised here, while the Windows row is intended for Windows CI. |

Levels 2 and 3 are not applicable. This fix changes filesystem semantics,
schema selection, process routing, and structured errors; it does not claim
terminal rendering or physical-key behavior.

## Verification Performed

- Focused Darkmatter Level 1 schema suite: **29 passed; 6,299 filtered**.
- Focused Claudine caller-provenance process suite: **10 passed**.
- `darkmatter/just lint`: **passed** for `darkmatter`, `darkmatter-cli`, and
  `dmls`.
- `claudine/just lint`: **passed** for the diagnostic guards and all five
  package-area crates.
- `biscuit-file/just lint`: **passed** for the library and CLI.
- `git diff --check`: **passed**.
- GitNexus change detection: **high-risk**, with six affected Darkmatter
  compose/classification execution flows.

The implementation log also records successful full package and root Level 1
and CI gates. Those green gates establish that covered behavior is stable on
this macOS host; they do not cover the contradictory provenance, selection,
and precedence cases above.

## Closure Criteria

Resolve Findings 1-4, add the Level 1 route assertions in Finding 5, and rerun
the Biscuit File, Darkmatter, and Claudine package-area test/lint gates plus the
specified root gates. Production readiness requires provenance to remain
unambiguous for equal semantic paths, one provenance-correct root-arm decision
per effective instance, and sequence provenance that follows—rather than
overrides—the established layer winner.
