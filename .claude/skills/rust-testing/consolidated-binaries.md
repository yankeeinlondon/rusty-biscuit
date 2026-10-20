# Consolidated Integration-Test Binaries

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

A package whose `Cargo.toml` sets `autotests = false` builds **one test binary
per execution contract** instead of one per `tests/*.rs` file. Packages without
it still use Cargo's per-file discovery.

```text
tests/
  common/mod.rs          # shared helpers, compiled once per binary
  l1/main.rs             # `#[path = "../common/mod.rs"] mod common;`, then one `mod` per former target
  l1/context_command.rs  # a former target, now module `context_command`
  level2/main.rs         # declared with required-features = ["terminal-tests"]
```

## Rules

- **Discovery is explicit.** Every root is a `[[test]]` with a `path`. A new
  `tests/foo.rs`, an undeclared `tests/x/main.rs`, or a module no `mod` reaches
  compiles into nothing. Each package's `l1` binary has a layout gate that fails
  on all three: `claudine/cli/tests/l1/test_placement.rs`, elsewhere
  `tests/l1/test_layout.rs` calling `test_toolkit::test_layout`.
- **Add a test file** by adding `mod <name>;` to the right root, with any OS
  `cfg` on that declaration (`#[cfg(unix)] mod compose_cli;`). An OS condition
  never makes a new binary. Modules reach helpers through `crate::common`, not
  their own `mod common;`.
- **The module name is the first segment of every test path**, so tier markers
  apply to it ([tier-filters.md](tier-filters.md)). Check a name against
  `_tier_filter` before choosing it. Where a former target's name would newly
  match, or stop matching, a tier or override filter, the module takes a
  neutral alias with the marker stripped (`biscuit-tui-cli`'s `terminal_render`,
  `biscuit-terminal-cli`'s `prose_cells`).
- **The feature boundary.** Tests share a binary only when tier, the exact
  `required-features` set, harness mode, and target-wide settings all match.
  Never union features (darkmatter keeps `level3-terminal` and `level3-browser`
  apart), and `harness = false` targets do not move.
- **Target names are the tier:** `l1`, `level2`, `level3`. When one tier has two
  feature contracts, the feature-less target keeps the bare name and the other
  takes a suffix (`biscuit-file`'s `l1` and `l1-fetch`). An L1 test that needs
  feature F joins the existing target for F whatever its tier name; the tier
  filter still selects it at L1 (`biscuit-tui/cli/tests/level2/windows_captured_stdout.rs`).
  Create a target only when no target has that feature set.
- **Snapshots follow `module_path!()`.** An assertion in
  `tests/l1/layout_matrix.rs` reads `tests/l1/snapshots/l1__layout_matrix__*.snap`.
  Moving a module moves its snapshots: move them byte-for-byte and run with
  `INSTA_UPDATE=no`. Never regenerate to go green.
- **Proptest regressions live in `tests/proptest-regressions/<stem>.txt`.**
  proptest walks up to the nearest `main.rs`, now the binary root, so a seed
  file left beside its module is silently no longer replayed.
- **Do not copy the duplicated-helper shape.** `biscuit-terminal/lib/tests/l1/parity_helpers.rs`
  is compiled as its own module and again inside each parity module
  (`#[allow(clippy::duplicate_mod)] #[path = …]`) to keep legacy test
  identities. New helpers go in `common/` or beside their one user, declared once.

## Process isolation is a nextest guarantee, not a Rust one

Nextest runs **each test case in its own process**, even within one binary, so
mutable statics, the current directory, env changes, `SetStdHandle` rewiring,
`atexit` handlers, and `serial_test` state stay per-case. The exception is code
that runs before libtest picks a case (a global constructor or allocator),
which runs in every process.

**`cargo test` breaks that contract:** it runs every case of a binary in one
process on parallel threads, so a test that sets an env var or changes
directory races every former-binary sibling it now shares a process with. Run a
consolidated suite only through the nextest-backed recipes or `cargo nextest`,
and never present `cargo test` as an equivalent. `cargo test --doc` is
unaffected.
