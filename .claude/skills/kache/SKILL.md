---
name: kache
description: Expert knowledge for kache (Kunobi) — the content-addressed Rust/C++ build cache that wraps rustc, stores artifacts once by blake3 key, and restores them via reflink or hardlink across worktrees, machines, and CI. Use when installing/configuring kache, sizing its store, wiring S3/MinIO/R2 remote caches or GitHub Actions, diagnosing cache misses or slow keying, deciding whether kache fits a host, or comparing it to sccache and cargo-sweep.
tools: [Read, Write, Edit, Grep, Glob, Bash, WebFetch]
---

# kache

## How to use this skill

Activate for anything involving kache: installation, configuration, store sizing, remote/S3 sync,
CI wiring, cache-miss diagnosis, or the decision of whether to adopt it on a given machine. Also
activate when someone is fighting duplicated `target/` directories across git worktrees, or
comparing build-cache options (sccache, cargo-sweep, `cargo clean`).

## What it is, in one paragraph

kache is a drop-in `RUSTC_WRAPPER` (and `cc`/`c++` wrapper) that intercepts every compiler
invocation, computes a **blake3 content-addressed key** from rustc version, source, dependencies,
flags, target triple and features, and stores the resulting artifact **once** in a local store.
Cache hits are restored **zero-copy** — a reflink where the filesystem supports it, a hardlink or
copy otherwise — so N worktrees share one physical copy. An optional daemon syncs the store to
S3-compatible object storage for sharing across machines and CI. Apache-2.0.

## Mental model — five facts that decide most questions

1. **The filesystem decides the economics.** Reflink filesystems (APFS, btrfs, XFS-with-reflink)
   get true zero-copy. Everywhere else kache falls back to hardlinks, and *populating* the store
   costs a real second copy. See [platforms.md](platforms.md).
2. **It replaces incremental compilation, it doesn't complement it.** kache sets
   `CARGO_INCREMENTAL=0` while active. This is the single biggest behavioural change on adoption —
   and the main reason it might be wrong for you. See [when-not-to-use.md](when-not-to-use.md).
3. **It caches dependencies, not everything.** rlibs and rmetas, yes. Binary crates, dynamic
   libraries, proc-macros, link steps: **not** by default (`KACHE_CACHE_EXECUTABLES=1` overrides).
   So `target/` still grows — just far more slowly.
4. **The store is bounded; `target/` is not.** `local_max_size` + LRU + `gc --max-age` give the
   store a predictable ceiling. Nothing bounds `target/` except you.
5. **A huge `target/` makes kache slow.** Per-crate file operations degrade badly on enormous
   trees — measured at ~18 s/crate keying on a 957k-file `target/deps`, versus ~30–170 ms on a
   clean one. Target hygiene remains necessary *for speed*, not just disk.

## Fast decision checklist — does kache fit this host?

**Strong yes:**
- Multiple git worktrees of the same repo (the flagship case — one blob, many links)
- Reflink-capable filesystem (APFS, btrfs, XFS-reflink; ZFS 2.2+ with `block_cloning` — verify)
- CI runners, or several machines building the same target triple → S3 sharing
- Heavy dependency graphs where deps dominate compile time (tokio, kube, tauri)

**Marginal:**
- Single worktree on a non-reflink filesystem (ext4, NTFS) — the store is a second copy, and
  there's nothing to dedup against. Still buys cheap re-cleaning and CI sharing.
- Tight disk where the store's cap would compete with `target/` for the same volume

**No:**
- Workflows that depend on incremental compilation for a fast inner loop
- Link-dominated builds (many binaries/test executables) — those aren't cached
- C/C++-only projects needing *remote* sharing (C/C++ caching is local-only)

Details and the reasoning in [when-not-to-use.md](when-not-to-use.md).

## Quick reference

| Task | Command |
| --- | --- |
| Set up (cargo wrapper + daemon service) | `kache init` / `kache init -y` / `kache init --check` |
| Health check — **always start here** | `kache doctor` |
| Cache stats | `kache stats --since 24h` |
| Live dashboard | `kache monitor` |
| List entries | `kache list --sort size` (or `name`, `hits`, `age`) |
| Why did this crate not hit? | `kache why-miss <crate>` |
| Bounded cleanup (LRU) | `kache gc` / `kache gc --max-age 7d` |
| Remove `target/` dirs under cwd | `kache clean --dry-run` then `kache clean` |
| Wipe cache | `kache purge` / `kache purge --crate-name <c>` |
| Remote sync | `kache sync` / `--pull` / `--push` / `--dry-run` |
| Record manifest for prefetch warming | `kache save-manifest --namespace <ns>` |
| Build report | `kache report --format markdown --since 7d` |
| Daemon | `kache daemon start\|stop\|restart\|install\|uninstall\|log\|run` (no `status` — use `doctor`) |
| Edit config | `kache config` |

## Key references

- [Installation, per OS](installation.md) — mise, brew, apt, AUR, winget, scoop, choco, cargo
- [Platform & filesystem variance](platforms.md) — reflink vs hardlink, per-OS paths and daemons
- [Configuration best practices](configuration.md) — store sizing, gc policy, keying speed
- [Remote object storage](remote-cache.md) — S3/MinIO/Ceph/R2, warm vs sync, CI
- [When not to use kache](when-not-to-use.md) — honest limits and the incremental tradeoff
- [Online resources](resources.md) — where to look when this skill is not enough

## Operating rules I follow

- **Run `kache doctor` *and* `kache stats` before diagnosing anything.** `doctor` catches wiring
  faults (daemon unreachable, stale locks, wrapper not wired, store paths). `stats` is what reveals
  whether kache is actually *earning its keep* — a green doctor with a 13% hit rate is a failing
  cache. Judge by hit rate, weighted-by-compile-cost, and time saved, not by health checks.
- **Never assume the restore mode.** Reflink support depends on the filesystem *as mounted*, not
  the OS, and `doctor` does not report it. Test it directly: on macOS `cp -c src dst` fails if
  cloning isn't possible; on Linux use `cp --reflink=always`.
- **Size `local_max_size` against the volume**, not by habit. A store that thrashes at its cap
  evicts fresh entries before they can score a hit, which is worse than a smaller working set.
- **Keep `target/` small even with kache** — for keying speed. This is the least obvious operating
  requirement and the easiest to get wrong.
- **Change codegen flags before warming the cache.** Flags are in the blake3 key, so a profile
  change invalidates entries and repopulates the store.
- **Don't recommend kache as a fix for a single unbounded `target/`.** It reduces what accumulates
  and makes cleanup cheap, but the growth of one build tree is a cargo problem.
