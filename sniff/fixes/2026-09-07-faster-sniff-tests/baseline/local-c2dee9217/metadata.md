# Local baseline metadata

- Originating branch: `fix/cli-slow-tests`
- Revision: `c2dee9217f3e6be14d7a6adfeb2c90cd2cd31966`
- Measured source state: clean detached worktree at the revision above
- Preserved source/build directory: `target/sniff-phase1-baseline-c2dee9217/`
  in the originating worktree (Git-ignored, retained locally)
- Active-worktree dirty files at capture time: recorded exactly in
  [`../../enumeration/captures.json`](../../enumeration/captures.json)
- Toolchain: `rustc 1.97.1 (8bab26f4f 2026-07-14)`, LLVM 22.1.6,
  `cargo-nextest 0.9.136 (1d5bf1ec9 2026-05-16)`
- Host: `arm64-darwin`, macOS 27.0 build 26A5425a, Darwin 27.0.0
- Counter contract hash: `3bd6bdad83b7d5a9140c3cfec6d4363ec6309111`
- Production-caching boundary: `before`

This revision predates `2026-07-22-inefficient-calling`. The aggregate
hardware and OS detector entry points in `hardware/mod.rs` and `os/mod.rs` do
not contain `OnceLock`, `OnceCell`, or `Lazy` memoization. The existing
`PATH_DIRS_CACHE` in `os/package_manager.rs` is a narrower executable-search
cache and is not the coordinated detector memoization change.

The fresh audit enumeration was captured from the active worktree at the same
revision before Phase 1 documentation edits. Its dirty-file list is retained
because it describes that capture. Timed Rust baselines use the clean detached
worktree so unrelated in-progress changes cannot alter the measured source.

