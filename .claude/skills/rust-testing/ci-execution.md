# Tests in CI: Archives and Repository Reads

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

## Your Tests Run From an Archive, Not From This Checkout

Every hosted L1, L2, browser, and WSL2 cell executes binaries a **different
job** compiled. One native owner per planned build key produces an immutable
Nextest archive; each consumer verifies its checksums, digest, inventory, and
runtime ABI, then runs the canonical tier recipe in archive mode. No consumer
has Cargo, rustc, Clippy, or a linker, and none will compile a replacement for
anything it is missing — it refuses (`.github/ci/README.md`,
[nextest.md](nextest.md), and `rust-devops`'s `ci-cd.md` for the CI contract).

What that requires of a test:

- **Never resolve a path at compile time.** `env!("CARGO_BIN_EXE_<name>")` and
  `env!("CARGO_MANIFEST_DIR")` name the *producer's* directories. Use
  `biscuit_test_harness::bin_exe!("<name>")` for a binary, which prefers
  nextest's run-time `NEXTEST_BIN_EXE_*`, and
  `biscuit_test_harness::manifest_dir!()` for a repository fixture, which
  prefers the `--workspace-remap`-rewritten run-time variable. This is enforced:
  `tools/test-toolkit/tests/archive_path_guard.rs` scans the repository and
  fails on a new site, with a small allow-list for the targets that are never
  archive-executed. The WSL2 guest no longer recreates the producer's checkout
  path, so a baked path now fails there rather than being worked around.
- **Declare anything the archive would not carry.** Test binaries, non-test
  `bin` targets, build-script output, and linked paths are archived; a `dylib`
  and an `example` are not. Those are `[package.metadata.ci] archive-includes`.
  A compile-time *tool* another package's tests spawn is a `sidecars` entry.
  Nothing is repaired by a consumer-side Cargo command.
- **Provision runtime facilities, not compile-time ones.** tmux, Chrome, Node,
  and CLI stubs are the consumer's job; anything that had to be *built* is the
  producer's.
- **Run it the same way locally.** `just cross-check <pkg> --os <os>` transfers
  the immutable archive and manifest, verifies on the destination, hides the
  producer's target directory, and extracts to a different path — which is what
  makes a compile-time path assumption fail there rather than only in CI.

A failing cell says which build it ran (planned key, realized digest,
producer) and what each stage cost. A cell that could not run at all is
`MISSING — blocked by build <key>`: its archive never arrived, and that is an
infrastructure failure, never a test result and never baseline-eligible.

## A Test That Reads a Repository File Is Scheduled By It

A change to a Markdown doc, YAML schema, or fixture selects no package, so the
planner finds the tests that read it from their source
and runs exactly those — on Linux in CI, or on the pushing host, whose
exact-tree run satisfies the CI cell. It
recognizes a read only in forms it can resolve without running anything, so
spell yours in one of them or the file's next edit will not run your test:

- **Embed it** — `include_str!("../../docs/x.md")` also makes a missing file a
  compile error rather than a runtime one.
- **Join it onto a root in the same expression** —
  `manifest_dir!().join("tests/fixtures/x.json")`,
  `repo_root().join("darkmatter/docs/x.md")`, or a name the same file binds to
  one (`let root = repo_root();`, `fn docs() -> PathBuf { manifest_dir!().join("docs") }`).
  `.parent()` steps are followed. A root-anchored directory counts for every
  file under it.
- **Or write the full repository-relative path** as a literal (a table of
  documents walked later), in a file that reads through a root somewhere.

Paths assembled from `format!`, a value computed at run time, or a helper
defined in another file are invisible, and a literal joined onto a tempdir is
correctly treated as a fixture, not a read. Only an L1 test is scheduled.
Its tier comes from its path, not its binary's name, so an L1 test in a
`level2` binary counts. A read inside a helper rather than a test function
schedules every L1 test in the helper's binary, because any of them may call
it. A shared `tests/common` module is a helper in every binary that includes
it, so spell the path in the one binary that needs it (the kache suites'
`repo_inputs()`). A read in a target whose `required-features` the
package's CI `features` leave off schedules nothing.

Another package's **source** (a script your tests execute) is not scanned
unless your package lists it in `[package.metadata.ci.tests] source-inputs`;
the tests must still spell the path in one of the forms above. The why and the evidence rules are
in [`docs/cicd/test-inputs.md`](../../../docs/cicd/test-inputs.md).
