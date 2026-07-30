# kache + sweep config (agreed 2026-07-29)

Short reference for the decisions. Full investigation and evidence: `kache-sessions.md`.

## Repo integration (2026-07-29)

kache is the compiler cache for every build in this repo, on every OS:

- **Wiring:** tracked `.cargo/config.toml` → `[build] rustc-wrapper = "kache"`. Chosen over
  `kache init` (user-wide `~/.cargo/config.toml`) because it is repo-scoped, applies to CI, and
  cannot leak into unrelated checkouts.
- **Version authority:** `.github/kache-version` = **0.12.0**, consumed by the root justfile and
  CI.
- **Store cap:** `just init` seeds `local_max_size = "100GiB"` when a host has no kache config
  (never overwrites). Config path: `~/.config/kache/config.toml` on macOS/Linux,
  `%APPDATA%\kache\config.toml` on Windows.
- **Sweep:** version-controlled at `scripts/sweep.sh` (three passes + census below), run via
  `just sweep [roots...]`. Schedule per host: launchd (Mac, done — see below), Task Scheduler on
  Windows, cron/systemd timer on Linux.
- **CI:** the tracked config applies to every workflow, so each leg either installs kache or
  neutralizes the wrapper. The `enable-kache` composite action installs kache on Linux/macOS
  (via `kache-action@v1`, GitHub-cache-backed) and clears `RUSTC_WRAPPER` on Windows
  (kache-action@v1 rejects win32-x64); every other workflow sets `RUSTC_WRAPPER: ""` at
  workflow level. Guarded by `ci_workflow_contracts.rs`.
- **Health:** judge with `kache stats`, **not** `kache doctor`.

## Future ambitions — kache in Windows CI

Windows legs currently build wrapper-free (Option A). The candidate upgrades, in order of
likely value:

**Option B — manual install + store persistence.** Install `kache.exe` in `enable-kache` on
Windows via `cargo binstall` (kache ships win32 prebuilt binaries; `kache-action@v1` is the only
piece that rejects Windows), then persist the store across runs with `actions/cache` on
`%LOCALAPPDATA%\kache`. Watch-points:

- GitHub's cache quota is 10 GB/repo — the Windows store needs a small cap (~5–8 GiB), which
  risks LRU churn against a 71 GB full-build working set.
- NTFS is hardlink mode: the store is a real second copy, and save/restore of a multi-GB store
  adds minutes per leg — measure against the compile time it saves.
- If `kache-action` ever ships win32 support, this collapses to deleting the Windows exclusion.

**Option C — S3 remote.** `kache sync --pull`/`--push` (or the daemon) against an S3-compatible
bucket for Windows legs. No 10 GB quota, and the same remote could warm Windows *dev* machines —
kache's designed multi-machine path. Costs: bucket + credentials in CI secrets, and the daemon
is the least-proven part of kache on Windows.

Trigger for revisiting: once hit rates on the Linux/macOS legs (`kache report`) prove the
steady-state win, prototype B on one Windows leg and compare wall time against Option A.

## kache (Mac)

| | |
| --- | --- |
| Version | **0.12.0** — never run 0.7.x, it was silently write-only |
| Wiring | `~/.cargo/config.toml` → `[build] rustc-wrapper = "kache"` |
| Store | `~/Library/Caches/kache`, `local_max_size = "100GiB"` |
| Config | `~/.config/kache/config.toml` |
| Daemon | launchd agent `ninja.kunobi.kache` |
| Remote | none configured (local-only; daemon therefore optional) |
| Health | judge with `kache stats`, **not** `kache doctor` |

## Sweep script

**What:** `~/.local/bin/rusty-biscuit-sweep.sh` — prunes Cargo `target/` dirs, which cargo never
garbage-collects. Logs to `~/Library/Logs/rusty-biscuit-sweep.log`.

**Frequency:** `com.ken.rusty-biscuit-sweep` launchd agent, **Sun + Wed 04:00**.
(launchd can't do "every 3 days at a fixed time"; twice weekly keeps the 04:00 window.)

**Roots:**
- `~/.claudine/worktrees/rusty-biscuit`
- `/Volumes/coding/personal/rusty-biscuit`

**Strategy — three escalating passes, most-targeted first:**

```bash
cargo sweep -r --installed      "$root"   # drop artifacts from uninstalled toolchains
cargo sweep -r --time 14        "$root"   # drop artifacts untouched >14 days
cargo sweep -r --maxsize 120GB  "$root"   # BACKSTOP: cap a target/, oldest-first
```

Plus a `[census]` line logging the 10 largest `target/` dirs before sweeping, and
`tmutil thinlocalsnapshots` afterwards so freed blocks actually return to the volume.

Passes 1–2 do the real work (50–100+ GiB per run historically). Pass 3 only fires on runaways.

## Starting maxsize: **120GB**

- `cargo-sweep`'s size unit is **decimal** and defaults to MB unsuffixed → 120GB = **111.8 GiB**.
- Reference: a clean full workspace build+test (`just test`, 72 packages) = **71 G**, already with
  `debug = "line-tables-only"`. So ~41 GiB of headroom above a legitimate pre-release run.
- Sized from the sweep log, not a guess: single-target reclaims have hit 107 GiB, and live targets
  were observed at 222 G (`darkmatter`) and 135 G (`claudine`).
- **Do not size a cap from a partial build.** My first estimate of 20GB came from `sniff` (one
  package area only) and would have fought every full build.

**Revisit when:** the census shows normal working worktrees regularly sitting above ~90 G → raise
to 150–200GB. If nothing ever approaches 120GB, it can come down.

## Other settings, deliberately unchanged

- `[profile.dev] debug = "line-tables-only"` — already committed workspace-wide (`43056c8bc`).
  Nothing to do.
- `local_max_size` stays at 100GiB pending real post-purge usage data.
- `[profile.dev.package."*"] debug = 0` — available if disk pressure returns; costs `file:line` in
  dependency backtrace frames. Not needed at 1.0 Ti free.
- `cargo-sweep` **stays** alongside kache. Not redundant: kache's per-crate keying degrades ~100×
  on a huge tree (~18 s/crate on a 957k-file `target/deps` vs ~30–170 ms clean).

## Why aggressive sweeping is now safe

Measured on identical commits, same workspace:

| | Cold store | Warm store |
| --- | --- | --- |
| Wall time | 35.1 min | **18.3 min** |
| Hit rate | 0% | **99.6%** (99.9% by compile cost) |

Swept artifacts come back as link-restores, not recompiles.

## Other hosts

- **build-linux** (ZFS, `block_cloning active`, reflink-capable) — **install next**; it's at 6.2 GB
  free with a 79 G target. Confirm `doctor` reports reflink inside the LXC.
- **build-win** (ext4 in WSL2) — deferred. Hardlink mode means the store is a real second copy, and
  there's one worktree so nothing to dedup against. If adopted: `local_max_size` ~30GiB against its
  196 G filesystem, or give it a btrfs/XFS-reflink second VHDX.
- Install **0.12.0+** on both, not a package-repo default. Verify with the two-directory test:
  build a small crate in dir A, copy to B, `rm B/target`, rebuild — B should hit every entry and
  the store should not grow.

## Files and backups

```
scripts/sweep.sh                                              (version-controlled; `just sweep`)
~/.local/bin/rusty-biscuit-sweep.sh                      (+ .bak)
~/Library/LaunchAgents/com.ken.rusty-biscuit-sweep.plist  (+ .bak)
~/.config/kache/config.toml
```

The Mac launchd wrapper and plist predate `scripts/sweep.sh` and remain unversioned — only the
`.bak` copies beside them. The sweep logic itself now lives in the repo; the host-specific pieces
are only the scheduler entry and the roots list.
