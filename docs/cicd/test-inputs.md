# Changed files that tests read

This page explains how CI in this repository decides what to run when a change
touches a file that is not source code — a Markdown document, a YAML schema, a
JSON fixture — and when a run on a developer's machine may stand in for a CI
run. It assumes you know CI and Rust, not this repository.

## Background: CI runs what the change touches

This is a Rust workspace of several dozen packages, and a full build and test
of all of them takes hours. So CI does not run everything on every change. A
planner (`scripts/ci/affected_scope.py`) reads the diff and schedules work per
package, per operating system, per test tier. Each unit of work is a **cell**,
identified by `{package, environment, gate}` — for example
`darkmatter / ubuntu-latest / L1`, where L1 is the fast, in-process test tier.
The planner writes its decision as one JSON document, the **plan**, which every
later CI job reads instead of deciding anything again.

The planner selects packages by *source*: a changed `.rs` file (or another
code suffix) selects the package that owns it. Everything else — docs,
manifests, lockfiles, fixtures — selects nothing. That rule is what keeps CI
affordable: a typo fix in a README does not rebuild and retest a package.

## The problem this solves

Some tests read files by path at run time. One darkmatter test, for example,
opens three documents and asserts that they still describe a rule the same way
the code implements it. When those documents were edited, moved, and one of
them deleted — in three documentation-only commits — the planner scheduled
nothing each time. Neither CI nor the local pre-push check ran the test, and it
went red on `main`, where it was found only by a manual full run.

This was not rare. Replaying `main`'s history since 2026-08-01, 32 of 62
merges changed a non-source file that some test of an unselected package
reads. Those tests simply did not run.

## How the tests that read a file are found

The planner derives the mapping from source code; nobody maintains it. The
index (`scripts/ci/test_inputs.py`) walks each Cargo target the way the
compiler does — from its root file through every `mod` declaration — so it
knows each file's module path and whether it is test code. It then looks for
string literals that name repository paths, in exactly these forms:

- **Embedded:** `include_str!("…")` / `include_bytes!("…")`.
- **Joined onto a root:** `repo_root().join("docs/x.md")`,
  `manifest_dir!().join("tests/fixtures/x.json")`,
  `env!("CARGO_MANIFEST_DIR")`, or a name the same file binds to one of those
  (`let root = repo_root();`). `.parent()` steps are followed. A root-anchored
  directory counts as a read of every file under it.
- **A full repository path** written as a literal (for example in a table of
  documents a test walks), in a file that reads through a root somewhere.

Everything else is ignored on purpose. Tests write thousands of fixture files
into temporary directories using names like `.claude/skills/x/SKILL.md` or
`README.md`; counting those would schedule tests for every README edit. A path
built at run time (`format!`) is invisible to the index.

Because the walk is exact, each reference resolves to a precise nextest test
identity: `binary_id(darkmatter::l1) & test(=schema_phase_validation::public_docs_…)`
for a literal inside a test function, or the whole binary,
`binary_id(darkmatter::l1)`, for one in a helper. The index does not know
which tests call a helper: a `pub` item, or a private one wrapped by a `pub`
one, can be called from any module of its binary. A module-scoped unit would
miss those callers, so a helper names its binary. The cost is a wider cell,
which runs every L1 test in that binary; the alternative was an L1 test that a
change to the file it reads would not run.

Only a reference an L1 test can reach gets a unit. Tier is read from the
test's path (`level2_`, `level3_`, `browser_`, `real_`, and `slow_` markers on
any segment), never from the binary's name: `biscuit-tui-cli::level2` holds
L1 tests. A helper gets no unit only when its binary holds no L1 test at all.
A target whose `required-features` the package's CI `features` do not
enable gets no unit either, because the L1 cell never compiles it.

## What gets scheduled

The index runs only for changed, deleted, or renamed-away files that are not
source, plus any source file a package declares as a `source-inputs` entry
(next section). For each reference it finds:

- **In shipped code** (an embed outside tests): the file is compiled into the
  product, so it is treated as that package's source and selected normally.
  This is rare — once in 1,174 commits of history.
- **In test code:** the package gets **one** L1 cell on `ubuntu-latest` whose
  `test_filter` names exactly the tests that read the file. No lint, no compile
  check, no other operating system, no dependent packages. If the plan already
  runs that package's whole L1 suite, nothing is added. If the package is also
  an unchanged dependent of a changed one, it is not compiled a second time in
  that package's dependents check: its own test build already compiles it
  against the change.

### Another package's source: `source-inputs`

Source is left out of the index on purpose. A source change already selects
its owning package, and many tests name other packages' source only to scan
it: guard tests that walk whole package directories. Indexing every source
path would attach those tests to most source edits. (Measured on 2026-09-24:
1,875 tracked source paths are named by some other package's test, almost all
through such directory scans.)

Some of those couplings are contracts, though. `tools/test-toolkit`'s kache
suites execute `scripts/kache-host.sh` and `scripts/kache-config-merge.py`,
which belong to `repo-deps`; a change to either script alone selected only
`repo-deps`, and the tests that prove the scripts' behavior did not run. The
reading package names such files in its own manifest:

```toml
[package.metadata.ci.tests]
source-inputs = ["scripts/kache-config-merge.py", "scripts/kache-host.sh"]
```

A change to a declared path is scanned like a changed test input, and only the
declaring package's references count. The declaration says *that* the
coupling exists; which tests read the file is still derived from source, so
each test must spell the path in one of the forms above. The planner refuses an
entry that is not source, does not exist, or is the declarer's own source,
and `RealWorkspaceTestInputTests` in `scripts/ci/test_affected_scope.py` fails
when a declared path is spelled by none of the declarer's L1 tests.

The test job intersects the filter with the tier's own selection
(`BISCUIT_TEST_NARROW` in `just/devops.just`), so it can only narrow, never
widen. It runs with `--no-tests=fail`: a filter that no longer matches anything
fails instead of passing having run nothing. The completion check
(`scripts/ci/completion.py`) accepts exactly that one extra narrowing and
nothing else.

## When a local run stands in for CI

Developers push through a pre-push hook that plans the same way CI does, runs
the cells for their own machine, and publishes the results as a **receipt** (a
Git note). CI then skips any cell a qualifying receipt already proves.

The general rule is that a receipt only counts for its own operating system: a
macOS run never stands in for Linux. **Narrowed test-input cells are the one
exception.** A narrowed cell run on any machine satisfies the planned
`ubuntu-latest` cell, if and only if:

1. the receipt's cell carries the **identical** `test_filter`, and
2. the receipt was recorded on the head's **exact** file tree.

The second condition matters because an ordinary receipt may be reused on a
later commit when the package's compiled inputs are unchanged. A document is
not a compiled input, so that equivalence would vouch for a run against the
old wording. Only the exact tree counts.

The receipt stays honest about where it ran: its evidence names the host's
notes ref (for example `refs/notes/ci-local/macos-latest`), and the report says
"narrowed, on macos-latest". A narrowed receipt never stands in for any
machine's whole test suite. If no qualifying receipt exists — the push skipped
the hook, or the tree differs — CI runs the cell on Linux as usual.

### Why this is safe without classifying tests

Whether a *test* behaves identically on every OS cannot be decided reliably
from its code, and a hand-maintained "OS-independent" marker would drift. So
the exception is justified by the *change*, which the planner knows exactly: a
narrowed cell exists only when none of the package's source changed. The test
code is therefore the same code that last passed on every OS; the only new
input is the content of the file it reads, and whether that content satisfies
the test does not depend on the OS.

The remaining risk is new content that drives an OS-specific branch
differently — say, a fixture that gains a Windows-style path. That risk is the
same one the repository already accepts for Windows and WSL2, which are tested
after merge: the nightly run executes every package's full L1 on Linux,
Windows, and WSL2, so such a case surfaces within a day.

A declared `source-inputs` file weakens the argument: a script can branch on
the operating system itself, so a narrowed run on macOS does not show what the
changed script does on Linux. The same exception applies anyway, and the same
nightly run is the backstop. The script's owning package is also selected by
the change, on every environment the event schedules.

## Cost

Measured on the same history, the rule adds about 49 narrowed cells per 62
merges (roughly seven a week). Each one's cost is dominated by compiling the
package's tests: about 7–10 runner-minutes for the CI-tooling package that
accounts for most of them, and 10–12 for the largest library. That is 50–80
runner-minutes a week before local receipts are counted; every push that goes
through the hook satisfies its cells locally and costs CI nothing. Planning
takes 1.3–1.7 s longer when a non-source file changed.

For comparison, treating Markdown as source would have cost 25–30
runner-minutes per documentation edit, README typos included.

## For test authors

If your test reads a repository file, write the path in one of the forms
above, or the file's next edit will not run your test. `include_str!` is
worth considering where it fits: it also turns a missing file into a compile
error. Details for agents working in this repository are in the `rust-testing`
skill.

## Limits

- Python and TypeScript test suites are not indexed.
- A test reading another package's *source* file is selected only when the
  reading package declares that file in `source-inputs`.
- A path named inside a shared helper module (`tests/common/…`) schedules
  every L1 test in every binary that includes the module. Spell the path in
  the one binary that needs it, as the kache suites' `repo_inputs()` do.
- A narrowed cell still compiles the package's whole test archive; only the
  test run is narrowed.

## Where the pieces live

| Piece | File |
|---|---|
| The index | `scripts/ci/test_inputs.py` |
| Selection | `select_test_inputs`, `test_input_only_cells`, `cell_evidence` in `scripts/ci/affected_scope.py` |
| Declared source inputs | `source-inputs` in `[package.metadata.ci.tests]`; `validate_source_inputs`, `test_input_references` in `scripts/ci/affected_scope.py` |
| Rename sources | `scripts/ci/diff_scope.py` (`--renamed-from`) |
| Narrowing in the test run | `_tier_filter` and `_test` in `just/devops.just` |
| Completion check | `without_narrowing` in `scripts/ci/completion.py` |
| Receipts and verification | `record_cells`, `_resolve_narrowed` in `scripts/ci/local_evidence.py` |
| Field contracts | `.github/ci/schemas/README.md` (`test_filter`) |
| Design record | `fixes/2026-09-22-test-input-blind-spot/spec.md` |
