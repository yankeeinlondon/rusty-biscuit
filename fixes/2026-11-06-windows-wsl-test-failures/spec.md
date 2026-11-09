---
kind: fix
name: windows-wsl-test-failures
date: 2026-11-06
status: planned
depends-on:
  - 2026-11-06-ubuntu-archive-disk
next_spec: 2026-11-06-legible-ci-names
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
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-08
review_iterations: 0
implemented: false
---

# Clear the Windows and WSL2 test failures

## Why

The Nov 6 nightly had 141 failures in about 19,900 Windows tests, plus one
compile error. The initial diagnosis is provisional: the Git-layout failures may expose a
product gap. A failing test run obscures other defects and prevents confidence
in Windows binaries. WSL2's state is unknown until
`2026-11-06-ubuntu-archive-disk` lands.

## Known failures

| Tests | Package | Apparent cause | Verdict so far |
|---|---|---|---|
| compile | `biscuit-visualized` | E0284 in `embed_render_tests.rs:450`; Windows only | Test code |
| 24 | `tree-hugger-cli` `index_corruption` | Expects `RUNNER~1`, output has `runneradmin` | Test bug |
| 96 | `tree-hugger-cli` `ownership` | Symlink targets spelled with `/` do not resolve | Probably test bug |
| 6 | `worktree` remove lifecycle | Compares two spellings of one path | Test bug |
| 2 | `claudine-cli` | Short vs long path, reversed | Test bug |
| 4 | `darkmatter` `kind_semantic_checks` | Expects `/`; path appears to use `\` | Probably test bug |
| 1 | `model_id` `ui` | `trybuild` offline, no `proc-macro2` | Test environment |
| 3 | `worktree`, `git-wt-cli` | Runner Git at `ucrt64\bin\git.exe`, not recognized | **Real, small product gap** |
| 1 | `messenger` | Timing wait expires; intermittent | Probably flaky |
| 4 | `darkmatter-cli` | Intermittent; cause unknown | **Unexplained** |

These are the original investigation's hypotheses, not confirmed diagnoses.
No Windows reproduction evidence is recorded here. The numeric rows total 141;
the compile-error row is separate. Before implementation, attach the nightly's
run URL, commit, failed-test identities, and relevant log excerpts to the
implementation log so the counts and diagnoses can be checked. Source line
numbers above identify the original report and may have moved.

Some fixtures already contain native-path joins, so inspect the current source
before applying the original diagnosis. A previous fix is evidence to verify,
not a reason to repeat it.

## Outcome

- Every known failure has a recorded cause, correction, and passing targeted
  reproduction on the affected environment. A compile failure is cleared by
  building and executing the affected test binary, not by a library-only check.
- Native Windows and WSL2 each have three consecutive green runs on `main`,
  following the existing OS evidence standard. Record the selected packages,
  tests, and evidence reused for each run; an empty plan or a test that silently
  returned early does not prove a repaired behavior executed.
- Required tests that cannot execute because of an existing capability gap are
  explicitly recorded as unmet, with provisioning identified as the required
  change. An accepted gap is distinct from a passing test. This fix must not
  broaden gaps or remove required coverage to claim success.
- Path assertions verify the documented surface: filesystem identity, native
  path text, and portable presentation each retain their own meaning.
- The Windows Git-layout failures are either corrected with regression coverage
  or disproved with recorded evidence explaining the actual cause.

## Dependency and scope

`2026-11-06-ubuntu-archive-disk` must deliver usable Ubuntu-built archives before
WSL2 failures can be inventoried. Windows investigation can proceed independently.
Missing archives and failed provisioning are not test failures; keep their repair
with that prerequisite. Once archives execute, record the WSL2 failures by
package and test identity and address them here under the same rules as Windows.

This work covers the rows above and failures exposed by restored WSL2 execution.
It does not redesign CI scheduling, change the merge gate, or introduce a new
benchmark program. Any product defect found gets a product fix; preserve the
assertion's behavioral guarantee rather than weakening it to get a pass.

## Design decisions

### Compare paths according to the contract being tested

Use the existing path APIs before adding a shared helper. For two existing paths
that must resolve to the same location, use simplified canonical paths and fail
if resolution fails. Do not fall back to comparing unnormalized strings. Paths
that deliberately name missing or deleted entries cannot be canonicalized;
construct their expected spelling from the appropriate existing parent instead.

For portable output, construct the expected value with the existing portable
path conversion. For native-path values, preserve native separators; on Windows,
a fixture built with `join("a/b")` may need its components collected into a native
path. For a launch-directory spelling contract, model the spelling the child
reports. Do not canonicalize whole diagnostics, lowercase paths, replace every
backslash, or reduce assertions to a basename or suffix. Those approaches can
hide incorrect output or confuse distinct files on Unix.

> Reader's note: the original requirement for one canonical comparison helper
> has been narrowed. Canonicalization resolves filesystem aliases, but it cannot
> prove that an API emitted the promised path text. Share a test-only assertion
> helper only where multiple callers truly need the same comparison contract;
> do not add a universal normalizer or change public output to satisfy a test.

For symlink fixtures, preserve relative targets and traversal semantics. Build
Windows targets with native components without resolving `..` away. Keep the
checks that reject files outside the owning repository; a portable fixture must
still exercise that protection.

The authoritative path rules are in the
[Windows path guide](../../../.claude/skills/os/windows-paths.md), and native
schema file values are documented in
[Darkmatter's schema validation guide](../../../darkmatter/docs/inline/schema-validation.md).

### Preserve Windows Git discovery and launch behavior

In the `worktree` package,
[`launcher_layout`](../../../worktree/lib/src/git/program.rs) recognizes Git for
Windows launchers and selects their underlying executable and environment.
Determine whether the failing runner supplied a launcher or a direct
`ucrt64/bin/git.exe`; these are different inputs. Extend recognized layouts only
when reproduction shows it is needed, with the matching environment values and
executable selection verified. Preserve existing layouts, ambiguous-layout
rejection, and fallback behavior for unknown executables. Include a real Git
operation on native Windows so a synthetic directory layout alone cannot claim
the launcher works. Update the module documentation if its supported layouts change.

### Repair test prerequisites and timing without hiding failures

For `model_id`, reproduce the offline nested Cargo build in an extracted archive.
Ensure its locked dependencies are available before offline execution, and keep
the compile-pass and compile-fail assertions active. Do not enable network access
inside tests as a repair. Its current early return when Cargo is unavailable is
not passing evidence: use the existing capability policy to distinguish an
unsupported environment from an executed compiler test. WSL2 intentionally has no
Cargo toolchain; do not provision one there to bypass the archive contract.

For `messenger` and `darkmatter-cli`, record the exact failing assertion, child
exit status, and captured output before choosing a correction. Classify fixture
isolation, rendering, process cleanup, and actual product behavior separately.
Use bounded condition-based synchronization for asynchronous behavior; do not
add retries, fixed sleeps, or larger timeouts as a substitute for diagnosis.
Keep ordinary tests within the repository's five-second L1 budget.

## Investigation and validation

1. Reconcile the original nightly inventory with current source and any existing
   fixes. Record a disposition for every row, including failures that no longer
   reproduce and the evidence still needed to clear them.
2. Reproduce unexplained Windows failures on the configured `$BUILD_WIN` host
   through the canonical cross-check tooling. Use PowerShell for native Windows
   processes. A passing run without the hosted runner's 8.3 short-name temporary
   directory does not clear a short-name failure: reproduce that spelling where
   available, otherwise retain the missing hosted evidence explicitly.
3. After the prerequisite archive repair, reproduce WSL2 through `$BUILD_WSL`
   using the Ubuntu-built nextest archive with builder paths unavailable. A
   normal Linux checkout run cannot establish that archive relocation works.
   Use runtime fixture and binary lookup and preserve the toolchain-free guest.
4. Add or repair focused regression assertions for each confirmed cause. Run
   the affected package area's canonical `just test` and `just lint` checks;
   use `just test-l2` only for tests requiring a real terminal, with no window
   gaining focus. Select packages by the code changed, not the entire workspace.
5. Before any later push, review `just ci-local --plan` and reuse qualifying
   evidence per package, environment, and test tier. Let the existing scheduled
   runs provide consecutive-run confirmation; do not trigger full-workspace
   runs merely to diagnose these failures. Native Windows evidence cannot be
   substituted with WSL2 evidence.

This is failure reproduction, not a performance spike. No new performance study
is required. If diagnosis identifies a performance cause, first use a quick
sample on the affected host; broader measurement needs an explicit design decision.

The implementation log must retain run links, commits, test filters, outcomes,
and any remaining capability gaps. Update current package documentation when
public behavior changes and the OS skill's relevant topic page when a new
platform-specific fact is established. Leave this spec as the decision snapshot.

## Open Questions

None at review time. Failure causes remain investigation work, not unresolved
design choices. If reproduction reveals alternatives with materially different
public behavior or CI policy, record the question here with two or three options,
their advantages and drawbacks, and a recommendation before implementing it.
