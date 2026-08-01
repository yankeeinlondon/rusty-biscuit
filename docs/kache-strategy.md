# kache + sweep config (agreed 2026-07-29)

Short reference for the decisions. Full investigation and evidence: `kache-sessions.md`.

## Repo integration (revised 2026-07-30)

kache is an **optional, per-host** cache. The repository tracks no Cargo wrapper and CI does not
use it. Rationale and measurements: `fixes/2026-07-30-ci-cd-stabilization/plan.md`.

- **Wiring: none tracked.** The earlier design put `[build] rustc-wrapper = "kache"` in a tracked
  `.cargo/config.toml`. That imposed one answer on every contributor's filesystem, hard-failed
  Cargo for anyone without kache installed, and forced five CI legs plus every other workflow to
  *neutralize* the wrapper they had just been given. Activation is now a host decision:
  `RUSTC_WRAPPER=kache` per shell, or `kache init` host-wide with informed consent.
- **Install:** `just install-kache` — `cargo binstall` at the pinned version, on every OS.
  Explicitly *not* a dependency of `just init`; installing and activating are separate decisions.
- **Version authority:** `.github/kache-version` = **0.12.0**, consumed by the root justfile.
- **Store cap:** `just install-kache` seeds `local_max_size = "100GiB"` when a host has no kache
  config (never overwrites). Config path: `~/.config/kache/config.toml` on macOS/Linux,
  `%APPDATA%\kache\config.toml` on Windows.
- **Sweep:** version-controlled at `scripts/sweep.sh` (three passes + census below), run via
  `just sweep [roots...]`. Schedule per host: launchd (Mac, done — see below), cron (build-linux,
  done — see below), Task Scheduler on Windows. Unaffected by the above — target hygiene matters
  with or without a cache.
- **CI: kache removed.** `Swatinem/rust-cache@v2` remains on every native leg. The measured legs
  returned 0–6% hit rates (0.4–2.3% weighted by compile cost, ~2–15s saved) because
  `kache-action@v1` fell back to the GitHub Actions cache, whose entries are immutable and
  branch-scoped, so a store shared by all same-platform area jobs could never accumulate.
  Revisit only with an S3/R2 backend and a measured comparison against a no-kache control.
- **Health:** judge with `kache stats`, **not** `kache doctor`.
- **Probe before activating:** never infer the restore mode from the OS. `kache doctor` reports the
  store filesystem from 0.12.0; confirm cloning with `cp -c` (macOS) or `cp --reflink=always`
  (Linux) between the store and a target directory.

## Per-host activation decision

Activation is a **host** decision and the repository cannot see it — `kache init`
writes `$CARGO_HOME/config.toml` and affects every Rust repo on that machine. Run
`just kache-status` to see what is actually active here and whether this
filesystem earns it.

The restore mode is a property of the **filesystem**, not the OS, so these are
defaults to start from and not conclusions to skip the probe with:

| Target | Decision | Rationale |
|---|---|---|
| **macOS dev** | Opt in after probe | Measured 99.6% warm on the current APFS layout. Other store/target layouts must still prove clone support. |
| **Linux dev** | Opt in after probe | ext4 is hardlink mode: store ingestion is a second copy and live `target/` links limit reclamation. btrfs / XFS-reflink are stronger candidates. |
| **Windows dev** | **Off by default** | NTFS restores by copy. Opt in only after a ReFS Dev Drive — holding the store *and* `target/` — is measured. |
| **WSL2 dev** | Qualified like Linux | A normal distro root is commonly ext4 in a VHDX; measure storage and restore behavior. |
| **WSL2 CI guest** | No | It executes a prebuilt nextest archive and compiles nothing inside the guest. |
| **CI** | Off for now | Keep `Swatinem/rust-cache@v2`. Revisit only with an S3/R2 backend. |

### Two remedies kache suggests that this repo rejects

On a non-clone volume kache prints storage-layout advice offering three fixes.
Only the third is ours:

- `[cache] windows_hardlink = true` — kache itself conditions this on the build
  never deleting or rewriting an object output. Cargo does both, routinely.
- `[cache] storage_layout_advice = false` — silences the signal rather than the
  cause, and the signal is what tells you the volume is one we decided against.
- A ReFS Dev Drive holding store and `target/` together — the supported way to
  make Windows worth activating.

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

**Strategy — four escalating passes, most-targeted first:**

```bash
rm -rf <target>/*/incremental             # over SWEEP_INCREMENTAL_MAX_GIB (default 15)
cargo sweep -r --installed      "$root"   # drop artifacts from uninstalled toolchains
cargo sweep -r --time 14        "$root"   # drop artifacts untouched >14 days
cargo sweep -r --maxsize 120GB  "$root"   # BACKSTOP: cap a target/, oldest-first
```

Plus a `[census]` line logging the 10 largest `target/` dirs before sweeping, and
`tmutil thinlocalsnapshots` afterwards so freed blocks actually return to the volume.

Passes 2–3 do the real work (50–100+ GiB per run historically). Pass 4 only fires on runaways.

**Why incremental needs its own size-triggered pass (added 2026-08-01).** Cargo never
garbage-collects incremental state, and the `--time` pass structurally *cannot* reach it: every
build re-touches the incremental directory of each crate it compiles, so those artifacts are
perpetually fresh no matter how stale the work behind them is. On build-linux this grew to **53 GiB
— 40% of a 134 GiB target** — while a `--time 14` scan found *zero* files older than 14 days across
the whole tree. Size is the only trigger that works, and `cargo-sweep` does not cover this at all.
Removing the directory costs one non-incremental rebuild of the workspace crates; `deps/` is
untouched.

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
- `[profile.dev.package."*"] debug = 0` — costs `file:line` in dependency backtrace frames. Still
  unset, but disk pressure **has** returned on build-linux (2026-08-01, 93% full), so this is now
  the largest remaining lever. Measured there by stripping a 1.1 G test binary and comparing
  allocated blocks: debug info is **58% of logical bytes but 67% of on-disk bytes**, because DWARF
  compresses *worse* (1.97x) than code (2.79x) — ZFS helps less here than elsewhere. With
  third-party crates at **82% of `deps/`** (141.9 G of 173 G logical), this is worth ~45 G on disk.
  Already at `line-tables-only`; this would drop dependency debug info only.
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

- **build-linux** (ZFS, `block_cloning active`, reflink-capable) — kache **installed at the pinned
  0.12.0 but deliberately not activated**: the host's `~/.cargo/config.toml` records that activating
  it disables incremental compilation at ~670 ms per edit-rebuild, and the host chose the faster
  edit loop. Sweep is scheduled here by **cron, daily 04:00** (`crontab -l`), logging to
  `~/.local/state/rusty-biscuit-sweep.log`. Two host constraints to know:
  - `~/.config` is a **read-only CIFS mount** (`//192.168.100.97/config`, `uid=0,gid=0`), so
    `just install-kache` fails at its config-seeding step and `~/.config/systemd/user` is
    unavailable — hence cron rather than a systemd user timer. The kache store is consequently
    uncapped, which is currently harmless only because kache is inactive here.
  - The volume is **160 G**, so the 120GB `--maxsize` backstop is not a guard rail on this host:
    120GB of target plus the rest of the system is ~84% full before pass 4 even fires.
- **build-win** (ext4 in WSL2) — deferred. Hardlink mode means the store is a real second copy, and
  there's one worktree so nothing to dedup against. If adopted: `local_max_size` ~30GiB against its
  196 G filesystem, or give it a btrfs/XFS-reflink second VHDX.
- Install **0.12.0+** on both, not a package-repo default. Verify with the two-directory test:
  build a small crate in dir A, copy to B, `rm B/target`, rebuild — B should hit every entry and
  the store should not grow.

## Files and backups

```
scripts/sweep.sh                                              (version-controlled; `just sweep`)
~/.local/bin/rusty-biscuit-sweep.sh                      (+ .bak)   [Mac]
~/Library/LaunchAgents/com.ken.rusty-biscuit-sweep.plist  (+ .bak)  [Mac]
~/.config/kache/config.toml                                         [Mac; unwritable on build-linux]
crontab -l                                                          [build-linux schedule]
~/.local/state/rusty-biscuit-sweep.log                              [build-linux log]
```

The Mac launchd wrapper and plist predate `scripts/sweep.sh` and remain unversioned — only the
`.bak` copies beside them. The sweep logic itself now lives in the repo; the host-specific pieces
are only the scheduler entry and the roots list.
