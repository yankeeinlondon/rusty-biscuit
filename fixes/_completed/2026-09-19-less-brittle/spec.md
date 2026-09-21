---
kind: fix
name: less-brittle
date: 2026-09-19
status: draft
related:
  - 2026-09-12-single-os-compile
  - 2026-09-19-nightly-scope
  - 2026-09-12-less-mistakes-during-cicd
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-09-19
review_iterations: 6
completed: true
---

# Schedule the archive-path guard reliably without rejecting safe fallbacks

## Problem

The archive execution work described in `2026-09-12-single-os-compile`
makes hosted test jobs run binaries compiled by a separate producer job.
A compile-time `env!("CARGO_MANIFEST_DIR")` embeds the producer's checkout
location. Reading a fixture through that path can work on a consumer with the
same checkout location and fail when the checkout moves. Compile-time binary
locations have the same problem when archived executables are extracted to a
new location.

The `test-toolkit` package's
[archive-path guard](../../tools/test-toolkit/tests/archive_path_guard.rs)
scans Rust source for these forms and for literal `/home/runner/work/` paths.
It has two separate tests: detecting violations and rejecting stale file
exemptions. It currently skips build scripts and several directory names;
its scan is not a proof that every archived target is portable.

**Selection is narrower than the files checked.** The guard runs as part of
`test-toolkit`'s unit-test suite, but checks source across the repository.
Changes in unrelated packages do not reliably select that suite. The current
checkout contains five direct compile-time manifest-path lookups in Claudine
and DMLS tests and seven runtime-first fallbacks in Messenger tests. All twelve
match the guard, although their behavior differs. This mismatch allows
violations to accumulate until another change happens to select `test-toolkit`.

**Text matching does not distinguish a fallback from a direct lookup.** The
seven Messenger tests first read the runtime variable:

```rust
std::env::var_os("CARGO_MANIFEST_DIR")
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
```

This honors a nonempty remapped checkout path. It is not fully equivalent to
[`manifest_dir!`](../../biscuit-test-harness/src/bin_exe.rs), the
`biscuit-test-harness` macro used to find the calling package's fixtures:
the macro also ignores empty runtime values. The corresponding
[`bin_exe!`](../../biscuit-test-harness/src/bin_exe.rs) macro locates a test's
executable by trying the normalized `NEXTEST_BIN_EXE_<name>` variable, then
`CARGO_BIN_EXE_<name>`, then the compile-time fallback, ignoring empty values.
The guard should recommend these shared implementations and avoid accepting a
runtime lookup that has no connection to the path actually used.

## Review notes

- The proposed scheduling change is intentional: a repository scan must be
  selected by the source it checks, independently of unrelated package tests.
  However, the current registry attaches companion suites to package gates;
  adding an entry alone does not create an independent execution cell. The
  required integration decision is recorded under **Open Questions**.
- The earlier file-wide exemption is replaced below with recognition of a
  bounded fallback expression. A runtime read in one function cannot make an
  unconditional compile-time lookup in another function safe. Relocation tests
  exercise selected cases and do not close that gap for every source file.
- Changed-file scanning identifies the file requiring repair, not necessarily
  the author who introduced the violation. An old violation in a newly changed
  file still fails. This fix does not attempt line-level blame or grandfather
  individual lines.
- The existing `status: draft` is preserved as requested, but is outside the
  supplied `$schema` enumeration. The author must change it to `draft-spec`
  before this document can pass validation against that schema. Review metadata
  does not declare the implementation complete or the design finalized.

## Decisions

### Selection follows the scanned source and the guard's own inputs

The canonical planner in the `repo-deps` package,
[affected_scope.py](../../scripts/ci/affected_scope.py), must select one
Linux-hosted guard execution when a changed repository-relative path ends in
`.rs` and belongs to the guard's scan domain. Use the same inclusion policy
for selection and scanning, including Rust files outside workspace members;
a workspace-member-only trigger would leave part of the current corpus
unchecked. Removed Rust files also trigger exemption maintenance.

Explicit changes to the guard's implementation, matcher fixtures, exemptions,
recipe, selection logic, and relevant registry or execution configuration also
select the guard. This is a narrow list of owned inputs, not a rule that every
workflow, manifest, or lockfile edit selects it. Ordinary documentation and
unrelated configuration changes select no guard. Existing package selections
remain intact; selecting the guard alone must not add the other `test-toolkit`
suites, a dependent compile check, or additional operating-system test runs.

The planner applies environment availability and event policy before emitting
the execution. The guard does not force Linux into an event that excludes it.
At review time, [.github/ci/environments.json](../../.github/ci/environments.json)
selects only WSL2 for `schedule`; therefore the nightly does not execute this
Linux-only guard. `2026-09-19-nightly-scope` must not be assumed implemented.
A manual full-workspace run includes the guard on Linux. A normal push to
`main` selects it only when a trigger matches; this is not an unconditional
repository scan after documentation-only pushes.

### Scan scope is explicit and comes from the resolved plan

The planner supplies the guard's scan mode and paths through its one canonical
resolved plan. Reuse the existing `change_inventory` representation, including
`diff_available`; do not have the recipe or workflow compute another Git diff.
If new plan fields are required, update the Python validator, shared contract,
Rust readers, projections, and receipt version handling together.

| Invocation | Files checked for new violations |
| --- | --- |
| Pull request with an available change inventory | Existing eligible Rust files in that inventory |
| Selected push to `main` | Full eligible corpus |
| Manual full-workspace run | Full eligible corpus |
| Event with Linux enabled but no available diff | Full eligible corpus |
| Standalone guard recipe without plan input | Full eligible corpus |
| Local planned validation | The mode recorded by its event-specific plan |

An explicitly available empty inventory is distinct from unavailable input.
An unreadable or malformed supplied plan is an error, not an empty scan or an
implicit standalone invocation. A valid plan with `diff_available: false`
requests the full scan. Print the mode and number of files checked so an empty
changed-file scan cannot be mistaken for full-tree validation.

Paths are repository-relative, normalized, sorted, and deduplicated by the
planner. Scan the current destination of a rename; skip deleted files when
looking for new violations, while still checking exemption validity. If the
current inventory cannot distinguish a deletion from an unexpectedly missing
file, extend the planner's input contract rather than silently ignoring all
missing paths. An existing file or directory that cannot be read fails with
its path and an actionable error. Do not follow directory symlinks outside the
checkout or recurse through cycles. Preserve portable path handling on macOS,
Linux, native Windows, and WSL2.

Changed-file mode intentionally ignores violations in untouched files. A
full-tree run detects those when it is selected. Do not claim every merge
runs a full scan: existing event and proven-environment reuse rules still
apply, subject to the evidence requirements below.

### Recognize safe fallbacks at the expression, not file, level

Retain the shared macros as the preferred spelling. Recognize an inline
manifest lookup only when the compile-time value is the fallback of the same
runtime-first expression, reading `CARGO_MANIFEST_DIR`, rejecting empty values,
and returning that runtime path when present. A lookup elsewhere in the file,
a different variable, or an unused runtime read grants no exemption.

For a binary lookup, match the same binary target and the shared resolver's
ordering: nonempty `NEXTEST_BIN_EXE_<name>` with hyphens converted to
underscores, then nonempty verbatim `CARGO_BIN_EXE_<name>`, then the compiled
path. Do not accept an arbitrary `NEXTEST_BIN_EXE_` occurrence as proof that
another executable's path is safe.

Use a small, documented set of recognized expression shapes, with token-aware
handling of whitespace, multiline calls, comments, and strings. This is not a
general Rust data-flow analysis. Unsupported custom helpers receive a diagnostic
recommending the shared macro, rather than a file-wide exemption. The matcher
must distinguish executable macro invocations from examples embedded in strings
or comments; fixture source containing forbidden forms must not cause the guard
to flag its own matcher tests. Restrict self-exclusion to the actual guard
implementation, not every file with a matching basename.

The runtime-first exemption applies only to the relevant macro occurrence.
Keep the existing hosted-root literal check independent: adding a runtime
lookup must not suppress a baked `/home/runner/work/` path. Broad detection of
arbitrary absolute paths remains out of scope.

### Migrate all twelve known sites to the shared macro

Replace the direct lookups in:

- `claudine-cli`: [compose_initialize_acceptance.rs](../../claudine/cli/tests/compose_initialize_acceptance.rs).
- `claudine`: [nested_span.rs](../../claudine/lib/src/composition/lifecycle/tests/nested_span.rs).
- `dmls`: [nested_span_tests.rs](../../darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs),
  [sequence_tests.rs](../../darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs),
  and [mapping_only_corpus.rs](../../darkmatter/dmls/tests/mapping_only_corpus.rs).

Replace the seven duplicated fallback helpers in the `messenger` package's
`research_refresh`, `research_validation`, `research_corpus`,
`research_publication`, and `research_lifecycle` test files, and the
`messenger-cli` package's `research_cli` and `research_lifecycle_cli` test files.
These changes also align empty-value behavior with the shared helper.

Add `biscuit-test-harness` as a direct dev-dependency in
[messenger/lib/Cargo.toml](../../messenger/lib/Cargo.toml); its presence through
`test-toolkit` is not enough to import the macro. The other affected packages
already declare it. Update dependency documentation and the lockfile if Cargo
requires it. Preserve ownership when replacing a borrowed `Path` with the
macro's owned `PathBuf`, and remove only imports and comments made stale by
these changes. No new file exemptions are needed.

### Keep exemption maintenance independent of violation suppression

Validate every allowlist entry against raw live forbidden forms in the full
eligible tree, even during changed-file scans. Do not use the filtered list of
violations for this check: a valid runtime-fallback exemption must not make the
harness's own implementation appear stale. Reject duplicate exemption entries
and keep a reason for each. A deleted or renamed exempted file must require
updating the exemption in the same change.

Existing skipped directories remain a documented limitation, including the
broad `scripts` exclusion even though `repo-deps` has archived tests. This fix
must not describe those directories as universally unexecuted. Tightening that
existing coverage gap and repairing all affected script tests is separate work.
Do not add skipped directories or weaken the hosted-root check to make this
change pass.

### Preserve one scheduler, meaningful results, and valid reuse

The guard must appear in the resolved plan and execute through the existing
area workflow. Keep stored identities keyed by package, environment, and gate;
do not introduce an area-keyed result or a seventh top-level CI job. The owning
area's coverage audit detects a planned guard that did not run. Failure or
cancellation reaches `ci-gate` through existing job results; that gate remains
a policy-free fold. Do not run the guard in preflight or make it advisory.

The selected implementation must specify whether it builds a Rust guard through
the existing archive producer or uses an existing script runtime. A Rust guard
cannot be declared toolchain-free at build time. An archived consumer may need
no compiler, but that does not waive compilation or artifact identity checks.

Ordinary `test-toolkit` L1 evidence cannot prove a separately selected guard.
If guard evidence is reusable, its identity must include the scan mode, selected
path set, all scanned source contents, full-tree exemption-maintenance inputs,
and the guard implementation and configuration. Package-local source identity
alone is insufficient for a repository-wide scan. In particular, changed-file
pull-request evidence cannot satisfy a full-tree push scan. Initially prefer
non-reusable guard execution unless the existing evidence system can represent
these inputs correctly; never label a partial scan as a complete full-tree pass.
Respect event-level proven-environment rules without inventing an extra Linux
run. A reused event must not be presented as a newly completed full-tree scan.

Local validation must execute the planned guard through the same canonical
recipe, or clearly report that the Linux CI execution remains outstanding.
A local run on macOS does not manufacture Linux evidence. The standalone recipe
must work on all supported operating systems and default to the full tree.

## Open Questions

### How should the independently selected guard fit the package gate model?

The current [suite registry](../../scripts/ci/affected_scope.py) belongs to the
`repo-deps` package and attaches companion suites to an owner's test or lint
cell. It does not independently schedule one test-toolkit suite without that
owner's other work. Resolve this design before implementation; a new registry
entry by itself does not meet the requirement.

1. **Recommended: add explicit suite-only selection to the existing lint-cell
   path, owned by `test-toolkit`.** The plan records the guard command as the
   required work for that lint cell; when ordinary toolkit lint is also
   selected, combine both requirements in one cell. Pros: preserves package
   identity and Linux-only policy without adding a gate kind or package.
   Cons: requires deliberate changes to command projection, local execution,
   coverage reporting, and validation so a guard-only cell does not implicitly
   run Clippy, L1, or other companion suites. Recommend this because the guard
   checks source policy and its result does not depend on the host OS.
2. **Create a dedicated guard package with one selected Linux test suite.**
   Pros: clear ownership, isolated dependencies, and ordinary test reporting.
   Cons: adds a package and still needs explicit rules preventing default
   all-environment tests and unrelated compile/lint work. More structure than
   this single scan appears to need.
3. **Introduce a dedicated guard gate owned by `test-toolkit`.** Pros: explicit
   independent identity and scan evidence. Cons: expands the closed gate model
   across schemas, workflows, receipts, local execution, and reporting for one
   check. Prefer only if lint-cell integration cannot express the contract
   cleanly without misleading results.

## Acceptance

- Planner tests in [test_affected_scope.py](../../scripts/ci/test_affected_scope.py)
  cover source changes across package areas and outside members, Rust deletions
  and renames, skipped directories, guard-owned non-Rust inputs, unrelated
  documentation, and explicit full scope. Guard-only selection adds exactly
  one Linux execution and no unrelated toolkit suite or operating-system cell.
- Event tests cover pull request, push, manual dispatch, the current WSL2-only
  schedule, absent diff, explicit empty inventory, and proven-environment reuse.
  No consumer independently derives source scope or adds Linux to the nightly.
- Matcher fixtures cover a valid fallback, empty runtime values, a runtime read
  elsewhere in the same file, a different variable or binary, incorrect binary
  precedence, a second unsafe occurrence, multiline forms, comments, strings,
  and an independent hosted-root violation. Diagnostics name file, line, and
  the preferred shared macro.
- Scan fixtures prove listed/unlisted behavior, full-tree defaults, malformed
  plan failure, read failures, deleted files, rename destinations, duplicate
  paths, and directory symlink boundaries. No check silently succeeds because
  it could not read its inputs.
- Exemption tests cover live, stale, renamed, and duplicate entries in both scan
  modes. Safe fallback recognition does not erase raw matches used to validate
  existing exemptions. No new exemptions or skipped directories are introduced.
- Run the canonical full guard recipe and `just test test-toolkit` on the
  implementation checkout. Run focused Nextest-backed package recipes for the
  migrated Claudine, DMLS, and Messenger tests, enabling Messenger's `research`
  feature so its affected test modules actually execute.
- Use the existing relocation fixture approach to exercise representative
  fixture reads with a remapped runtime checkout and unavailable producer path.
  Reuse existing helper tests for empty values and binary naming where they
  already prove the behavior; do not add another full CI matrix or launch
  terminal/browser windows to test this source guard.
- Workflow and reporting contracts prove guard failure blocks merging, missing
  execution fails the owning coverage audit, and guard-only selection neither
  invokes unrelated suites nor creates false full-tree evidence. Run the
  repository's compact CI contract recipes and `actionlint` if workflows change.
- Update [.github/ci/README.md](../../.github/ci/README.md), the relevant canonical
  recipe documentation, and the CI/CD skill alongside the implementation so
  scheduling, scan limitations, and evidence rules match the shipped behavior.

## Not done

- No general Clippy ban on `env!`, custom compiler lint, or whole-program proof
  of path portability.
- No general scan for arbitrary absolute paths such as `/Users/` and
  `/Volumes/`; many are synthetic test inputs. The existing hosted-root literal
  check remains in force.
- No change to archive manifests, producer selection, consumer rejection rules,
  or environment cadence.
- No repair of unrelated specifications' status metadata. The author owns those
  lifecycle changes, including `2026-09-12-single-os-compile`.
