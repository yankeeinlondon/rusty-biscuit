# Development Environment Initialization

Run `just init` from the repository root after cloning Rusty Biscuit. The
recipe prepares the host to build and test the monorepo without changing Cargo
settings for unrelated repositories.

## What `just init` Does

Initialization runs these stages in order:

1. Ensures Rust, Cargo, and the platform C build tools are available.
2. Ensures GitNexus and its native Tree-sitter dependency are usable.
3. Builds and installs `sniff`, then reports whether the runtime is native,
   WSL 1, or WSL 2.
4. Builds and installs the core Rusty Biscuit developer CLIs.

The recipe is idempotent. Running it again repairs missing tools.

It does **not** install or activate a compiler cache. See
[Build Caching](#build-caching) — that is an opt-in host decision, because
whether it pays off depends on the filesystem rather than the OS.

## Native Windows

Every recipe in this monorepo runs through bash, and `just` additionally needs
`cygpath` on PATH to translate recipe shebang lines. Without them, `just init`
fails before any recipe can run — with an opaque "could not find `cygpath`
executable" error — so the shell-environment check lives in a PowerShell
preflight. On native Windows, run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\init.ps1
```

It verifies `bash`/`cygpath` (Cygwin's `C:\cygwin64\bin`, or Git for Windows'
`bin` + `usr\bin`), prints exact remediation when they are missing, warns when
the only `bash` is the WSL launcher stub, and then delegates to `just init`.

Windows-specific behavior of the recipe itself:

- **Rust** is installed by downloading `rustup-init.exe` for the host's MSVC
  triple (`x86_64`, `aarch64`, or `i686`) — not via `sh.rustup.rs`, which
  misdetects under Cygwin/MSYS and picks the GNU triple.
- **The linker** is `link.exe` from the Visual Studio C++ workload (detected
  with `vswhere`), not `cc`. If missing, the recipe installs *Visual Studio
  2022 Build Tools* with the C++ workload via winget (or the
  `vs_BuildTools.exe` bootstrapper when winget is unavailable). This is a
  multi-GB install and prompts for administrator approval.
- **jq** (used to read `.github/ci/areas.json`) is installed via winget when
  absent; when no area declares Windows native packages the check is skipped.
- **Node.js 22+** (for GitNexus) is not auto-installed; the recipe prints the
  `winget install OpenJS.NodeJS.LTS` remediation and stops.

WSL remains a supported Linux environment: run `just init` directly inside the
WSL terminal, not through `scripts\init.ps1`.


## Build Caching

kache is an **optional, per-host** compiler cache. This repository tracks no
Cargo wrapper configuration: nothing in a fresh clone activates it, and Cargo
works normally without it.

That is deliberate. kache's economics are decided by the **filesystem**, not the
operating system. APFS, btrfs, XFS-with-reflink, and ReFS restore cache hits by
cloning blocks; ext4 and NTFS fall back to hardlink or copy, so the store
becomes a genuine second copy of every artifact. A tracked wrapper would impose
one answer on every contributor's machine — and would hard-fail Cargo for anyone
who has not installed kache. `docs/kache-strategy.md` records the measured
evidence.

### Installing

```sh
just install-kache
```

This installs the exact version declared by `.github/kache-version` (surfaced as
`KACHE_VERSION` in the root `justfile`) using `cargo binstall`, which fetches a
prebuilt binary rather than compiling from source. It is the supported install
path on every OS; per-OS package managers are fallbacks. On hosts with no kache
configuration it seeds a default store cap of `local_max_size = "100GiB"` — an
existing config is never overwritten. It clears `RUSTC_WRAPPER` during the
install so an absent or older wrapper cannot intercept its own installation.

Installing does not activate anything.

### Probing before activating

Never infer the restore mode from the OS. Confirm the store and `target/` can
actually clone blocks:

```sh
kache doctor                                   # reports the store filesystem (0.12.0+)
cp -c  <store-file> <target-dir>/probe                 # macOS: fails if clonefile can't
cp --reflink=always <store-file> <target-dir>/probe    # Linux: fails if reflink can't
```

### Activating

Two supported scopes:

```sh
export RUSTC_WRAPPER=kache   # this shell only — narrowest, trivially undone
kache init                   # host-wide: writes $CARGO_HOME/config.toml
```

`kache init` affects **every** Rust repository on the host, so choose it only
with that in mind. To undo: `unset RUSTC_WRAPPER`, or remove the wrapper line
from Cargo home. Do not hand-create an ignored `.cargo/config.toml` in this
repository — hidden local policy is difficult to diagnose later.

Activating disables Cargo's incremental compilation, which is the largest
behavioral change on adoption.

The store is bounded; `target/` is not. Run `just sweep` to prune stale
build artifacts (`cargo sweep`: uninstalled toolchains, then anything untouched
for 14 days, then a 120 GB per-root backstop cap). With a warm kache store,
swept artifacts return as link-restores rather than recompiles, and a lean
`target/` keeps kache's per-crate keying fast. Schedule it per host — launchd
on macOS, Task Scheduler on Windows, cron or a systemd timer on Linux. The
decisions and sizing evidence live in `docs/kache-strategy.md`.

Judge the cache with `kache stats` (hit rate, time saved), not `kache doctor`
— a green doctor with a low hit rate is a failing cache.

Local caching does not require the kache daemon. Install the optional login
service only when using remote caching:

```sh
just cache-daemon-install
```

Inspect the active runtime, cache configuration, statistics, and daemon state
with:

```sh
just cache-status
```

To bypass cache lookup for one command while retaining kache's compiler
wrapper, set `KACHE_DISABLED=1`:

```sh
KACHE_DISABLED=1 cargo build
```

## Platform Behavior

Nothing below is enabled by `just init`. The column records how strong a
candidate each runtime is once you have probed it.

| Runtime | Typical restore mode | Recommendation | Login service |
|---|---|---|---|
| macOS (APFS) | reflink | Strong candidate | Optional launchd service |
| Linux (btrfs, XFS-reflink) | reflink | Strong candidate | Optional systemd user service |
| Linux (ext4) | hardlink | Qualified — store is a second copy | Optional systemd user service |
| WSL 2 | hardlink (ext4 in a VHDX) | Qualified; measure first | Optional when systemd is enabled |
| Native Windows (ReFS / Dev Drive) | block clone | Candidate; measure first | Not installed |
| Native Windows (NTFS) | copy | Off by default | Not installed |
| WSL 1 | best effort | Not recommended | Do not install |

Under WSL, keep the repository and kache store in the Linux filesystem, such
as `~/coding` and `~/.cache/kache`. Builds under `/mnt/c`, `/mnt/d`, or another
Windows-mounted path pay cross-filesystem overhead and may lose cheap
reflink/hardlink restores.

The daemon is intentionally opt-in on every platform. Local hits and misses
continue to work without it; remote checks, uploads, and prefetching require a
running daemon or an explicit `kache sync`.

## Installed Developer Tools

After the prerequisites are ready, initialization installs these monorepo
CLIs:

- `sniff`
- Biscuit Terminal CLI tools
- Darkmatter (`md`)
- Playa
- Biscuit Speaks (`so-you-say`)

It also ensures GitNexus is installed globally through npm, installing its
native build toolchain first (`node-gyp`, `node-addon-api`, and `tree-sitter`
with npm's `--allow-scripts` approval so the native binding actually builds).
GitNexus requires Node.js 22 or newer and a writable global npm prefix, or
`sudo` access for that prefix.

## Troubleshooting

If initialization stops, fix the reported prerequisite and run `just init`
again. Useful focused checks are:

```sh
rustc --version
cargo --version
sniff runtime
gitnexus status
kache doctor   # only if you opted into the compiler cache
```

If Cargo reports that `kache` cannot be found, either activate it after
`just install-kache`, or clear the wrapper (`unset RUSTC_WRAPPER`, or remove it
from Cargo home) — a wrapper is only needed if you opted in. If a WSL 2 host
should support a daemon but
`systemctl --user` is unavailable, enable systemd for that WSL distribution or
use local-only caching.
