# Platform and filesystem variance

The OS matters less than the **filesystem**. kache's economics change entirely depending on whether
a cache hit can be restored by cloning blocks or has to duplicate them.

## The restore-mode ladder

From the docs: hits restore *"zero-copy — a reflink (copy-on-write clone) where the filesystem
supports it (APFS, btrfs, XFS-with-reflink), and a hardlink or copy otherwise."*

| Mode | Filesystems | Restore cost | Store cost | Independent copies? |
| --- | --- | --- | --- | --- |
| **Reflink** (best) | APFS, btrfs, XFS with `reflink=1`, ZFS 2.2+ with `block_cloning` (verify) | Zero-copy clone | Cloned, not copied | Yes — store GC frees store bytes |
| **Hardlink** | ext4, NTFS, most others | Zero-copy link | **A real second copy** | No — same inode |
| **Copy** | Cross-filesystem store/target, or where links are unavailable | Full copy | Full copy | Yes |

Two consequences that catch people out:

**Populating the store costs a copy on non-reflink filesystems.** Restores don't duplicate bytes,
but getting a freshly compiled artifact *into* the store does. On ext4 the cache is a genuine second
copy of every unique artifact. The Kunobi write-up is explicit that this is the worst case for
storage numbers, and that many CI runners are in it.

**Under hardlinks, store GC can't reclaim referenced blobs.** Evicting a store entry only frees
space when every link is gone — a live `target/` holding a link keeps the bytes alive. Under
reflink the copies are independent, so `gc` genuinely frees store bytes. Budget accordingly:
on hardlink hosts, `local_max_size` bounds *unique* store bytes, not total footprint.

**Never infer the mode from the OS.** "Linux" tells you nothing — ext4 and btrfs behave oppositely.
`kache doctor` reports the store's filesystem from 0.12.0 (`Cache FS  apfs (local)`) but not older
versions, and it names the filesystem rather than proving a clone succeeds. To be certain, test the
syscall directly between the store and a target directory:

```bash
cp -c  <store-file> <target-dir>/probe      # macOS: fails if clonefile isn't possible
cp --reflink=always <store-file> <target-dir>/probe   # Linux: fails if reflink isn't possible
```

Note that APFS `clonefile` **does** succeed between separate volumes in the same container
(verified on macOS 26), so a `/Volumes/x` layout is not automatically a downgrade — measure it.

## macOS

- APFS → reflink. Best case, and the platform kache is tuned for.
- The store lives under `~/Library/Caches/kache` (with `index.db` alongside the blobs).
- kache **automatically excludes its own store from Time Machine and Spotlight** — worth knowing
  before you go hunting for why backups didn't grow.
- The daemon installs as a **launchd** login agent (`~/Library/LaunchAgents/ninja.kunobi.kache.plist`).
- The docs cite APFS specifically as a reason incremental compilation is disabled: running cargo's
  incremental alongside artifact caching *"can corrupt artifacts on certain filesystems like APFS."*

## Linux

- **ext4** → hardlink mode. The most common Linux case, and the weakest one.
- **btrfs** and **XFS with `reflink=1`** → reflink. Modern `mkfs.xfs` enables reflink by default;
  confirm with `xfs_info <mount> | grep reflink`.
- **ZFS** → block cloning landed in OpenZFS 2.2; check `zpool get feature@block_cloning <pool>`.
  Pool-level `active` does **not** guarantee `FICLONE` succeeds inside a container or on every
  dataset — verify with `kache doctor` rather than assuming.
- Store defaults under `~/.cache/kache`.
- The daemon installs as a **systemd** user service.
- In containers/LXC, check that the store and `target/` are on the *same* filesystem — a bind mount
  or overlay boundary silently downgrades reflink/hardlink to copy.

## Windows

- **ReFS**, including a **Dev Drive** → **zero-copy restores via block cloning**, added in kache
  0.8.0. This makes a Dev Drive the recommended location for both the store and build trees on
  Windows.
- **NTFS** → hardlink mode. As of 0.8.0 kache *warns* on NTFS that it can't do zero-copy; a
  hardlink option is available.
- Store location follows the platform cache dir (under `%LOCALAPPDATA%`); confirm via `kache doctor`,
  which prints the resolved path.
- `kache daemon install` is documented in terms of launchd/systemd; on Windows verify the daemon is
  actually running as a service with `kache daemon status` after `init`, and fall back to
  `kache daemon start` if not.

## WSL specifically

WSL is a Linux install, not a Windows one — install the Linux build inside the distro.

The distro's root filesystem is **ext4 inside a VHDX**, so kache runs in **hardlink mode**, and the
store is a second copy of every unique artifact. Two implications:

- Size `local_max_size` against the *ext4 filesystem's* capacity, remembering that the VHDX is
  usually thin-provisioned with a virtual size far larger than the volume backing it. A store cap
  that fits `df` may not fit the host volume.
- To get reflink on WSL, attach a second virtual disk formatted btrfs or XFS-with-reflink
  (`wsl --mount <vhdx> --vhd`) and put both the store and the worktrees on it. This also isolates
  build artifacts from the distro's root filesystem, so a runaway build fills a disposable volume
  instead of killing the distro.

## Cross-platform behaviour that does *not* vary

- Incremental compilation is disabled (`CARGO_INCREMENTAL=0`) on every platform.
- Cache keys are portable — the blake3 key excludes absolute paths and machine identity, which is
  what makes S3 sharing across machines work.
- The exclusion list (binary crates, dylibs, proc-macros, link steps) is the same everywhere.
- C/C++ object caching is **local-only** on all platforms; only Rust artifacts sync to a remote.
