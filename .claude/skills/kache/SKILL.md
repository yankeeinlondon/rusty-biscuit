---
name: kache
description: Expert knowledge for kache (Kunobi), the content-addressed Rust/C++ build cache that wraps rustc and restores artifacts via reflink, hardlink, or copy. Use when installing or configuring kache, sizing or cleaning its store, wiring S3/MinIO/R2 or GitHub Actions caches, diagnosing misses or slow keying, evaluating kache for macOS/Linux/Windows/WSL filesystems, interpreting storage-layout warnings, or comparing it with Cargo incremental compilation, sccache, cargo-sweep, and cargo clean.
---

# kache

## Evaluation workflow

Before recommending adoption, removal, or a storage-mode override:

1. Run `kache --version`, `kache doctor`, and `kache stats --since 24h`.
2. Run a longer stats window when history exists; identical 24-hour and 7-day totals often mean the
   cache is new, not that the hit rate is steady state.
3. Record the store and target filesystems, store size and cap, representative `target/` size,
   volume free space, worktree count, remote-cache status, and whether builds are clean-build,
   cross-worktree, or edit-compile loops.
4. Judge value by compile-cost-weighted hit rate, time saved, disk consumed, and workflow fit.
   `doctor` proves wiring and integrity, not usefulness.
5. Make the recommendation for the measured host. Do not generalize an NTFS result to APFS,
   btrfs, XFS-reflink, ReFS, or even Linux ext4.

Read [platforms.md](platforms.md) for restore semantics and
[when-not-to-use.md](when-not-to-use.md) before making an adoption decision.

## What it is, in one paragraph

kache is a drop-in `RUSTC_WRAPPER` (and `cc`/`c++` wrapper) that intercepts every compiler
invocation, computes a **blake3 content-addressed key** from rustc version, source, dependencies,
flags, target triple and features, and stores the resulting artifact **once** in a local store.
Cache hits restore through a reflink where the filesystem supports it, a hardlink where that is
safe, or an independent copy otherwise. Reflinks and hardlinks let worktrees share physical
storage; copies do not. An optional daemon syncs the store to S3-compatible object storage for
sharing across machines and CI. Apache-2.0.

## Mental model — five facts that decide most questions

1. **The filesystem and OS decide the economics.** Reflink/block-clone filesystems (APFS, btrfs,
   XFS-with-reflink, ReFS Dev Drive) get zero-copy restores with independent files. Linux ext4
   normally hardlinks restored immutable artifacts. Windows NTFS defaults to independent copies
   because its shared hardlink attributes make the safe hardlink path unsuitable for general
   builds. See [platforms.md](platforms.md).
2. **It replaces incremental compilation, it doesn't complement it.** kache strips Cargo's
   incremental rustc flag while the wrapper is active, including under `KACHE_DISABLED=1`. This is
   the biggest behavioral change on adoption — and the main reason it might be wrong for you. See
   [when-not-to-use.md](when-not-to-use.md).
3. **It caches dependencies, not everything.** rlibs, rmetas, dylibs, cdylibs, and proc-macros are
   cacheable. User-facing binaries and test harnesses are skipped by default; link and unsupported
   compiler shapes still pass through. So `target/` continues to grow.
4. **The store is bounded; `target/` is not.** `local_max_size` + LRU + `gc --max-age` give the
   store a predictable ceiling. Nothing bounds `target/` except you.
5. **A huge `target/` makes kache slow.** Per-crate file operations degrade badly on enormous
   trees — measured at ~18 s/crate keying on a 957k-file `target/deps`, versus ~30–170 ms on a
   clean one. Target hygiene remains necessary *for speed*, not just disk.

## Fast decision checklist — does kache fit this host?

**Strong yes:**
- Multiple git worktrees of the same repo (the flagship case — one blob, many links)
- Reflink-capable filesystem (APFS, btrfs, XFS-reflink, ReFS; ZFS 2.2+ with
  `block_cloning` — verify)
- CI runners, or several machines building the same target triple → S3 sharing
- Heavy dependency graphs where deps dominate compile time (tokio, kube, tauri)

**Marginal:**
- Single worktree on Linux ext4 — store ingestion costs a second copy, although hits can restore
  by hardlink. It can still buy cheap re-cleaning and remote sharing.
- Windows NTFS with strong measured cache reuse — every safe restore is a copy, so budget both the
  store and restored target bytes.
- Tight disk where the store's cap would compete with `target/` for the same volume

**No:**
- Workflows that depend on incremental compilation for a fast inner loop
- Link-dominated builds (many binaries/test executables) — those aren't cached
- C/C++-only projects needing *remote* sharing (C/C++ caching is local-only)
- Windows NTFS with one worktree, no remote, low weighted hit rate, and limited free space

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
- **Measure disk context.** Compare store size, representative target size, configured cap, and
  volume free space. Reject a cap that exceeds realistic headroom even when hit rates are good.
- **Never assume the restore mode.** Reflink support depends on the filesystem *as mounted*, not
  just the OS. `doctor` reports the store filesystem in current releases, but the actual affected
  output path and platform fallback still decide whether a restore reflinks, hardlinks, or copies.
- **Do not recommend Windows hardlinks casually.** `[cache] windows_hardlink = true` trades safety
  for deduplication on NTFS. A restored output that is deleted or rewritten can fail or corrupt the
  shared store blob; keep the safe copy default unless the build's output lifecycle is proven.
- **Separate host policy from repository policy.** A tracked Cargo wrapper affects every developer
  OS. Prefer explicit CI activation and host-local opt-in when filesystems differ across the team.
- **Size `local_max_size` against the volume**, not by habit. A store that thrashes at its cap
  evicts fresh entries before they can score a hit, which is worse than a smaller working set.
- **Keep `target/` small even with kache** — for keying speed. This is the least obvious operating
  requirement and the easiest to get wrong.
- **Change codegen flags before warming the cache.** Flags are in the blake3 key, so a profile
  change invalidates entries and repopulates the store.
- **Don't recommend kache as a fix for a single unbounded `target/`.** It reduces what accumulates
  and makes cleanup cheap, but the growth of one build tree is a cargo problem.
