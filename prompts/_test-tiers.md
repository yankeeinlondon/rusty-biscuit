## Test Tiers and Placement

Every test you add or rename must be compiled by a declared target **and** selected by a tier that
actually runs. A test that misses either one still passes: it simply never runs. Check both before
you call the work done.

- **Tier comes from the test's name.** Any `::`-separated segment of the test path that starts with
  `level2_`, `level3_`, `browser_`, `real_`, or `slow_` takes the test out of L1. That includes
  *module* names: a module called `real_shells` strands every test inside it. Use a marker only when
  the test needs that tier's resource, and never as ordinary English ("real", "browser").
- **A marker needs a live recipe.** A `level2_`, `level3_`, `browser_`, or `real_` test runs only if
  its area's `test-l2` / `test-l3` / `test-browser` / `test-real` recipe is real rather than a
  "not applicable" stub. Check with `just check-tier-coverage <area>`; CI refuses a stranded test
  (`completion-test-stranded`).
- **Consolidated packages declare every test.** Where a package's `Cargo.toml` sets
  `autotests = false`, a new file under `tests/` is never compiled until it sits in a declared
  binary's directory (`tests/l1/`, `tests/level2/`, …) and that binary's `main.rs` declares it with
  `mod`. Share helpers through `use crate::common;`, and write `#[path]` and `include_str!` paths
  relative to the file's new location. The package's layout gate test fails on anything left out.
- **Feature-gated tests need a feature CI enables.** A test behind `#[cfg(feature = "…")]` or a
  target's `required-features` compiles only where that feature is on; confirm it is in the
  package's `[package.metadata.ci.tests] features`.
- **A test that reads a repository file must say so in a form CI can see.** Use `include_str!`, or
  join a literal onto `repo_root()` / `manifest_dir!()` / `CARGO_MANIFEST_DIR`, so a change to that
  file runs the test (see `docs/cicd/test-inputs.md`).

When reviewing, treat a new test that no declared target compiles, or that no running tier selects,
as a missing test.
