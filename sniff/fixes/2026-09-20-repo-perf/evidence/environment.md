# Performance environment fingerprint

Captured once on 2026-09-25 for the Phase 1 baseline. Later phases must re-check
every row, and must write down any change, before comparing timings against this
record.

| Field | Value |
|---|---|
| Host | Ken's development Mac, Apple M4 Max |
| OS | macOS 27.2 (build 26B5091g), `aarch64-apple-darwin` |
| Cores | 16 logical (`hw.perflevel0` 12 performance + `hw.perflevel1` 4 efficiency) |
| Memory | 128 GiB |
| Available parallelism | 16 |
| Worker policy in force | `ignore` 0.4.25 default `available_parallelism().min(12)` = **12** walker threads (only for the parallel side; the baseline walk is serial, `WalkBuilder::build()`) |
| Corpus filesystem | `/Volumes/coding`: internal SSD, **case-sensitive APFS** |
| Corpus root spelling | `/Volumes/coding/wt/rusty-biscuit/fix-sniff`, a linked Git worktree (`.git` is a file) of `/Volumes/coding/personal/rusty-biscuit` |
| Toolchain | `rustc 1.98.1 (48a229cea 2026-09-01)`, LLVM 22.1.8; `cargo 1.98.1 (797e8a9bc 2026-08-05)` |
| Build wrapper | `rustc-wrapper = "kache"` (`~/.cargo/config.toml`). This affects compile caching only, never timed regions |
| Baseline commit | `1634e6e55b7bc1d42989e058e87eb7ac59d0e2f6` (`HEAD` of `fix/sniff`) plus the uncommitted Phase 1 harness |
| After commit | not yet; recorded in Phase 4 |
| Uncommitted state at measurement | Phase 1 harness in `sniff/lib/src/filesystem/repo/{nested.rs,mod.rs}`, `sniff/lib/benches/cases/repo.rs`, `sniff/lib/examples/work_counts.rs`; fix-directory docs; unrelated: `prompts/_implement/implement-plan.md` (modified), `sniff/.ai/plans/2026-02-16.plan-for-repo-remotes-completion.md` (deleted), `sniff/fixes/2026-09-25-recent-commits/` (untracked). These unrelated changes were not made by this implementation |
| `Cargo.lock` | unchanged versus `HEAD`; sha256 `296ac0a008bd0c54409972580f569a8b13bda15927c39bd69a139fc268ec5b53`; `ignore` locked at 0.4.25 |
| Feature set (timing) | `sniff` with `bench-internals` only (no `network`/`remote`) |
| Feature set (counters) | `sniff` default features (`work_counts` example) |
| Build profiles | release = Cargo `bench` profile (inherits `release`); debug = `cargo bench --profile dev` (`debug = "line-tables-only"`, dependencies `debug = 0`) |
| Ignore configuration | builder `hidden(false)`, `git_ignore(true)`, `git_global(true)`, `git_exclude(true)`, `should_skip_directory_name` prune; no global `core.excludesFile` configured; the shared `.git/info/exclude` has 60 lines |
| Cache treatment | **warm** filesystem cache. Each case gets a 3 s Criterion warm-up, and the corpus was walked many times immediately before. No cold-cache claim is made |
| Test-runner concurrency | nextest `test-threads = -2` (`.config/nextest.toml`), i.e. 14 on this host. It is relevant to Phase 4's concurrent-detection probe; no tests ran during timed regions |
| Host load | busy shared host. The 1-minute load average fell from 9.8 to 3.7 across the campaign; `kache` and window-server activity came from other sessions. Each bracket's start/end load is in `baseline/raw/*.log` |
| Measurement serialization | the four bench invocations ran back to back in one shell. This implementation ran no builds or tests during them. Other host activity could not be controlled and is recorded above |
