# Tests in CI: Archives and Repository Reads

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

## Your tests run from an archive, not from this checkout

Every hosted L1, L2, browser, and WSL2 cell executes binaries a **different
job** compiled. One producer per planned build key makes an immutable nextest
archive; each consumer verifies it and runs the canonical tier recipe in
archive mode. Consumers have no Cargo, rustc, or linker and never compile a
replacement for something missing; they refuse. The CI contract is in
`.github/ci/README.md` and the `rust-devops` skill.

What that requires of a test:

- **Never resolve a path at compile time.** `env!("CARGO_BIN_EXE_<name>")` and
  `env!("CARGO_MANIFEST_DIR")` name the producer's directories. Use
  `biscuit_test_harness::bin_exe!("<name>")`, which prefers nextest's run-time
  `NEXTEST_BIN_EXE_*`, and `biscuit_test_harness::manifest_dir!()`, which prefers
  the `--workspace-remap`-rewritten run-time variable
  (`biscuit-test-harness/src/bin_exe.rs`). `tools/test-toolkit/tests/archive_path_guard.rs`
  fails on a new compile-time site, with a small allowlist for targets never
  archive-executed.
- **Declare anything the archive would not carry.** Test binaries, non-test
  `bin` targets, build-script output, and linked paths are archived; a `dylib`
  and an `example` are not. Those are `[package.metadata.ci] archive-includes`,
  which also builds a declared example before archiving; a hand-written
  `archive.include` does not, and `on-missing = "ignore"` turns the gap into a
  test that panics on a missing file.
  A compile-time *tool* another package's tests spawn is a `sidecars` entry.
- **Provision runtime facilities, not compile-time ones.** tmux, Chrome, Node,
  and CLI stubs are the consumer's job; anything that had to be built is the
  producer's.
- **Reproduce locally** with `just cross-check <pkg> --os <os>`: it ships the
  archive to the destination, hides the producer's target directory, and
  extracts to a different path, so a compile-time path assumption fails there.

A cell that could not run is `MISSING — blocked by build <key>`: an
infrastructure failure, never a test result and never baseline-eligible.

## A test that reads a repository file is scheduled by it

A change to a doc, schema, or fixture selects no package, so the planner finds
the L1 tests that read it from their source and runs exactly those, on Linux
in CI or on the pushing host, whose exact-tree run satisfies the cell. It
recognizes a read only in forms it can resolve statically; spell yours in one
of them, or the file's next edit will not run your test:

- **Embed it:** `include_str!("../../docs/x.md")`, which also makes a missing
  file a compile error.
- **Join it onto a root in the same expression:**
  `manifest_dir!().join("tests/fixtures/x.json")`,
  `repo_root().join("darkmatter/docs/x.md")`, or a name the same file binds to
  one (`let root = repo_root();`, `fn docs() -> PathBuf { manifest_dir!().join("docs") }`).
  `.parent()` steps are followed; a root-anchored directory covers every file
  under it.
- **Write the full repository-relative path** as a literal (a table of
  documents walked later), in a file that reads through a root somewhere.

Not reads, and why:

- a join onto one top-level directory (`repo_root().join("docs")`) is too
  coarse; name the file;
- a root call followed by a comma (`stage(&repo_root(), "docs/x.md")`) leaves
  the file unanchored; bind `let root = repo_root();` first;
- paths from `format!`, run-time values, or a helper in another file are
  invisible; a literal joined onto a tempdir is a fixture, not a read.

Scope of a read:

- The tier comes from the test's path, not its binary's name, so an L1 test in
  a `level2` binary counts.
- A read inside a helper schedules every L1 test in the helper's binary, and a
  shared `tests/common` module is a helper in every binary that includes it;
  spell the path in the one binary that needs it (`fn repo_inputs` in
  `tools/test-toolkit/tests/kache_*_contracts.rs`).
- A read in a target whose `required-features` the package's CI `features`
  omit schedules nothing.
- A Markdown document's `::file` targets need no spelling: a test that reads
  the document is scheduled when a transcluded file changes.

**Prompt folders are never test inputs.** A change under `prompts/`,
`darkmatter/prompts/`, or `claudine/prompts/` schedules nothing. Test a prompt
itself in the package's opt-in `prompts` binary (`required-features =
["prompt-tests"]`, run by `just test-prompts`); a behavior test that needs a
prompt as input reads a frozen copy under `tests/fixtures/frozen_prompts/`.

**Another package's source** (a script your tests execute) is scanned only if
your package lists it in `[package.metadata.ci.tests] source-inputs`, and the
tests must still spell the path in one of the forms above.

Rationale and evidence rules: [`docs/cicd/test-inputs.md`](../../../docs/cicd/test-inputs.md).
