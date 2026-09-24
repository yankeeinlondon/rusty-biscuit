# Consolidated Integration-Test Binaries

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

Fourteen packages no longer build one executable per `tests/*.rs` file. Each
builds **one test binary per execution contract**: `claudine-cli`,
`darkmatter`, `darkmatter-cli`, and `biscuit-terminal`
(`2026-09-21-consolidated-test-binaries`), then `tree-hugger`, `claudine`,
`sniff`, `biscuit-file`, `schematic-gen`, `biscuit-terminal-cli`,
`claudine-gen`, `dmls`, `sniff-cli`, and `biscuit-tui-cli`
(`2026-09-22-consolidated-test-binaries-wave-2`). A package's `Cargo.toml`
says which shape it has: `autotests = false` means consolidated. The rest
still use Cargo's per-file discovery.

```text
tests/
  common/mod.rs          # shared helpers, compiled once per binary
  l1/main.rs             # crate root: `#[path = "../common/mod.rs"] mod common;`, then one `mod` per former target
  l1/context_command.rs  # a former target, now module `context_command`
  level2/main.rs         # declared with required-features = ["terminal-tests"]
```

- **Discovery is explicit.** The package sets `autotests = false` and lists
  every root as a `[[test]]` with a `path` (Cargo does not discover
  `tests/l1/main.rs` on its own). A new `tests/foo.rs`, an undeclared
  `tests/x/main.rs`, or a module file that no `mod` reaches compiles into
  nothing, so its tests never run. A layout gate inside each package's `l1`
  binary fails on all three: claudine's `test_placement.rs`, and elsewhere
  `test_layout.rs`, which calls `test_toolkit::test_layout`.
- **Adding a test file** means adding `mod <name>;` to the right root, with
  any OS `cfg` on that declaration (`#[cfg(unix)] mod compose_cli;`). An OS
  condition never makes a new binary. Modules reach helpers through
  `crate::common`, not their own `mod common;`.
- **The module name is the old target name** and is the first segment of every
  test path: `claudine-cli::l1 context_command::<test>`. Tier markers still apply to
  the name, so a module named `level2_*` would put all its tests in Level 2.
  Check the name against `_tier_filter` before choosing it. Where the old name
  would newly match, or stop matching, a tier or override filter, the module
  takes a neutral alias with the marker stripped: `biscuit-tui-cli`'s
  `real_terminal_render` is `terminal_render` (its `level2_*` tests would
  otherwise also match the stub `real` tier), and `biscuit-terminal-cli`'s
  `level2_prose_cells` is `prose_cells` (43 of its tests are unmarked L1).
- **The feature boundary.** Tests share a binary only when tier, the exact
  `required-features` set, harness mode, and target-wide settings all match.
  A consolidation never unions features. Darkmatter keeps
  `level3-terminal` and `level3-browser` separate for this reason, and
  `harness = false` targets (benches) do not move.
- **Target names** are the tier: `l1`, `level2`, `level3`. When one tier has
  two feature contracts, the feature-less target keeps the bare name and the
  other takes a suffix (`biscuit-file`'s `l1` and `l1-fetch`). An L1-tier test
  that needs feature F joins the package's existing target for F, whatever its
  tier name, and the tier filter still selects it at L1: `biscuit-tui-cli`'s
  `windows_captured_stdout` lives in `level2`. A new target is made only when
  no target has that feature set.
- **Snapshots follow `module_path!()`.** An insta assertion in
  `tests/l1/layout_matrix.rs` reads `tests/l1/snapshots/l1__layout_matrix__*.snap`.
  Moving a module therefore moves its snapshots. Move them byte-for-byte and
  run with `INSTA_UPDATE=no`. Never regenerate to go green.
- **Proptest regressions move to `tests/proptest-regressions/<stem>.txt`.**
  proptest walks up from the source file to the nearest `main.rs`, which is now
  the binary root, so a seed file left beside its module is silently no longer
  replayed. Move it byte-for-byte.
- **Legacy shape, not a pattern.** biscuit-terminal's
  `tests/l1/parity_helpers.rs` is compiled once as its own module and again
  privately inside 18 parity modules (`#[allow(clippy::duplicate_mod)]
  #[path = "parity_helpers.rs"]`). Each former binary ran its own copy of
  its unit tests, and the move kept those test identities. New helpers go in
  `common/` or beside their one user and are declared once.

### Process isolation is a nextest guarantee, not a Rust one

Nextest runs **each test case in its own process**, even when cases share a
binary. So under the canonical recipes a consolidated binary shares nothing
between cases that per-file binaries did not share: mutable statics, the
current directory, environment changes, `SetStdHandle` rewiring, `atexit`
handlers, and `serial_test` state all stay per-case. One exception: code that
runs before libtest picks a case (a global constructor or allocator) runs in
every one of those processes.

**`cargo test` breaks that contract.** Its harness runs every case of a binary
in one process on parallel threads. Consolidation puts up to a hundred formerly
separate crates into one such process. A test that sets an env var or changes
directory then races every sibling it now shares a binary with. Run a migrated
suite only through the Nextest-backed recipes (or `cargo nextest` directly).
Never present `cargo test` in docs or recipes as an equivalent way to run one.
`cargo test --doc` is unaffected: doctests are not integration-test binaries.
