# Development Environment Initialization

Run `just init` from the repository root after cloning Rusty Biscuit. The
recipe prepares the host to build and test the monorepo without changing Cargo
settings for unrelated repositories.

## What `just init` Does

Initialization runs these stages in order:

1. Ensures Rust, Cargo, platform C build tools, native libraries, and the
   repository's portable shell-utility baseline are available.
2. Ensures the CI/CD command-line tools applicable to the host are available.
3. Ensures Cargo artifact maintenance and GitNexus are usable.
4. Builds and installs `sniff`, reports whether the runtime is native, WSL 1,
   or WSL 2, and installs the core Rusty Biscuit developer CLIs.

The recipe is idempotent. Running it again repairs missing tools.

It also owns the kache compiler cache end to end on hosts whose filesystem
earns it — decided by a probe, not the OS. See
[Build Caching](#build-caching); on a non-qualifying host init leaves kache
off and says why.

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
the only `bash` is the WSL launcher stub, imports persisted PATH entries added
by WinGet into the current session, and then delegates to `just init`.

Windows-specific behavior of the recipe itself:

- **Rust** is installed by downloading `rustup-init.exe` for the host's MSVC
  triple (`x86_64`, `aarch64`, or `i686`) — not via `sh.rustup.rs`, which
  misdetects under Cygwin/MSYS and picks the GNU triple.
- **The linker** is `link.exe` from the Visual Studio C++ workload (detected
  with `vswhere`), not `cc`. If missing, the recipe installs *Visual Studio
  2022 Build Tools* with the C++ workload via winget (or the
  `vs_BuildTools.exe` bootstrapper when winget is unavailable). This is a
  multi-GB install and prompts for administrator approval.
- **jq** (used to read package native declarations from `cargo metadata`)
  is installed via winget when absent.
- **Node.js 22+** (for GitNexus) is not auto-installed; the recipe prints the
  `winget install OpenJS.NodeJS.LTS` remediation and stops.

WSL remains a supported Linux environment: run `just init` directly inside the
WSL terminal, not through `scripts\init.ps1`.


## Build Caching

kache is a **per-host** compiler cache, and `just init` owns it end to end on
hosts whose filesystem earns it. The decision is made by a **filesystem
probe**, never by the OS name: APFS, btrfs, XFS-with-reflink, and ReFS restore
cache hits by cloning blocks; ext4 and NTFS fall back to hardlink or copy, so
the store becomes a genuine second copy of every artifact. This repository
still tracks no Cargo wrapper configuration — a qualifying host's activation
is written into `$CARGO_HOME/config.toml` by init itself, never into the repo,
and Cargo works normally without it. `docs/kache-strategy.md` records the
measured evidence and the rulings.

### What init does on a qualifying host

The `_ensure-kache` step runs one ordered sequence, activation last:

1. **Qualification probe** (`scripts/kache-host.sh qualify`) — device ids plus
   clone probes between the candidate store location and the checkout; it
   needs no kache installed to answer. `just kache-status` runs the same
   clone check through the same script, but from the store `kache doctor`
   resolves at the time, into the checkout and the configured worktree base;
   a store that has moved or cannot clone fails status with the reason, and
   so does a worktree-base setting `wt` itself refuses (a missing `WT` path,
   one naming a Git repository, a malformed `~/.worktree.json`, or a missing
   `base_dir`). init names such a setting in its report but, as with any
   base, does not let it change the verdict.
2. **Install or upgrade to the latest release** (`just install-kache`) — the
   floor in `.github/kache-min-version` (**0.23.0**) is a check, never a pin.
   On macOS the binary is re-signed ad hoc and gated on an env-passthrough
   probe: the `cargo binstall` release ships with the hardened runtime, which
   strips every `DYLD_*` variable at launch and breaks `rust-lld`'s
   `libLLVM.dylib` resolution — the wasm link failure this flow exists to
   prevent.
3. **Store placement** — `kache doctor` resolves the store kache uses today;
   when it sits off the serving device, the placement cascade picks one that
   does (a `kache/` directory at the checkout volume's mount point, the user
   cache dir, or a user-owned directory on that volume — never a root-owned
   mount point). That last step is `kache/` at the mount point when you may
   create it there, else `kache/` in the highest ancestor of the checkout below
   the mount point that you own, can write, and that is not inside a Git
   working tree (`/data/src/kache` for a checkout under a root-owned `/data`).
   On Windows the cascade is `%LOCALAPPDATA%\kache` when it is on the ReFS
   checkout volume, else `kache\` at that drive's root, else the same ancestor
   rule, so a Dev Drive beside an NTFS system drive still gets a store on the
   Dev Drive.
4. **User config write** — `[cache] local_store = "<store>"` plus
   `ignore_env = true` into `~/.config/kache/config.toml`
   (`%APPDATA%\kache\config.toml` on Windows) via a targeted merge that
   preserves every other key and comment, validates before replacing, and
   keeps a dated backup (`scripts/kache-config-merge.py`). The config file is
   the single source of truth: interactive shells, the daemon, editors, and
   scheduled jobs all resolve the same store. It is written even when the
   value equals kache's default — the pin is deliberate. That holds only
   while every kache process loads this file: a non-empty `KACHE_CONFIG`
   selects another one, and `ignore_env = true` does not stop it. Right after
   the write init checks that its own environment exports no such override
   and that `kache doctor` resolves the store just pinned; an override is a
   pre-activation failure whose WARNING names `KACHE_CONFIG=<path>`, before
   any daemon work.
5. **Daemon lifecycle** — installed when absent, ensured running, restarted
   when the running daemon's version mismatches the installed binary or the
   config write changed the file; strictly after the config write, so the
   daemon's first read of the store is the ratified one. init then waits
   (bounded) for the daemon to report the installed binary's version, not
   just to be running: a restart the service manager reports as done while
   the old daemon still answers is a pre-activation failure, with a WARNING
   naming both versions and kache left off. The running daemon must also
   report (`daemon_config_path` in `kache daemon --json`) that it loaded the
   user config; a service environment that selects another file is a
   pre-activation failure naming that file.
6. **Activation, last** — `[build] rustc-wrapper = "kache"` in
   `$CARGO_HOME/config.toml`, or in the legacy `$CARGO_HOME/config` when that
   file exists: Cargo then reads it and ignores `config.toml` (with only a
   warning), so init writes the file Cargo reads. A tracked `.cargo/config.toml` wrapper remains
   forbidden. init then re-decides the wrapper Cargo would actually use for
   the checkout. When something outranks the host entry — an inherited
   `RUSTC_WRAPPER` (even set but empty), `CARGO_BUILD_RUSTC_WRAPPER`, or a
   repository or parent-directory `.cargo/config.toml` — the report header
   says `activation INCOMPLETE`, the activation line says the entry was
   written but is not in effect, and a `kache NOT ACTIVE` WARNING names each
   overriding source with its undo. The same happens when the winning wrapper
   is named kache but is not the binary init verified: it must resolve, by
   Cargo's rules, to an existing executable that is the `kache` on `PATH`
   (the one installed, re-signed, and version- and passthrough-checked), so a
   missing path — which fails every Cargo build — or another executable
   called kache is `activation INCOMPLETE` too. The host entry is kept, since
   it takes effect once the override is removed, and init still completes its
   other steps.

**Failure contract.** Any failure before activation prints loud WARNING
lines, leaves kache off (an activation an earlier run wrote is neutralized —
`rustc-wrapper = ""`, which Cargo treats as no-wrapper), and `just init` still
completes its other setup steps. "Off" is re-checked against the wrapper Cargo
would actually use: when kache stays active through something init cannot
edit — an inherited `RUSTC_WRAPPER`/`CARGO_BUILD_RUSTC_WRAPPER`, an unwritable
host Cargo config, another Cargo config — init prints `kache STILL
ACTIVE — manual action required` with each source and its undo instead, and
still continues. A broken kache must never sit under a wrapped
build. init closes with one report block: the verdict and the fact that
decided it (device ids, probe result), worktree-base coverage, version
against the floor, the passthrough result, the store path and whether it
moved, an abandoned old store's size (deleting it stays with you), daemon
state, and activation state. Its header reads `qualified and set up` only
when Cargo will actually run kache for this checkout.

### Non-qualifying hosts

A non-qualifying verdict ends the sequence at the probe: init names the
reason and **never installs kache** there. If kache is already installed:

- at or above the floor — reported only;
- below the floor — an interactive init asks before upgrading (the confirmed
  upgrade is **binary-only**: `just install-kache true`, no daemon work
  follows); a non-interactive init stops with an error instead of upgrading
  or skipping silently.

If kache is already the wrapper Cargo would use on a non-qualifying host, init
reports that as drift and prints the undo, leaving the change to you. Init and
`kache-status` decide that the same way (`scripts/kache-host.sh wrapper`), by
Cargo's precedence: a set `RUSTC_WRAPPER` wins even when empty, then
`CARGO_BUILD_RUSTC_WRAPPER`, then `.cargo/config[.toml]` in the checkout and
every directory above it (nearer first), then `$CARGO_HOME` — one file per
directory, the legacy extensionless `config` whenever it exists, since Cargo
then ignores the `config.toml` beside it; an empty
`rustc-wrapper = ""` — what a failed init leaves behind — means no wrapper.
A kache-named winner is then resolved to the executable Cargo would start (a
bare name on `PATH`; a path relative to the working directory for an
environment value, or to the directory holding the config's `.cargo/`), and
`kache-status` reports `active BROKEN` and fails with drift unless that is the
`kache` on `PATH`.

### By hand

```sh
just install-kache        # install or upgrade to the latest release
just install-kache true   # binary-only (positional boolean): no config writes
just kache-status         # where this host stands; exits non-zero on drift while active
```

While kache is active, `kache-status` fails with one named problem per drifted
fact: a below-floor binary; a daemon whose service is not installed, that is
not running, or that runs another version than the installed binary; a user
config (`scripts/kache-host.sh config-path`, the file init writes) whose
`[cache] local_store` is missing or names another store than `kache doctor`
resolves, or whose `ignore_env` is missing or false; a `KACHE_CONFIG` in the
shell, or a daemon that loaded another config file (or names none), each of
which bypasses that pin; and the device, clone, and passthrough checks above. With kache not in use it prints the same facts and
exits 0.

`install-kache` installs the latest release using `cargo binstall` (fetching
a prebuilt binary rather than compiling from source), clears `RUSTC_WRAPPER`
during the install so an absent or older wrapper cannot intercept its own
installation, and on hosts with no kache configuration seeds a default store
cap of `local_max_size = "100GiB"` (full mode only; an existing config is
never overwritten, and a read-only config directory is reported rather than
fatal). On a qualifying host, activation belongs to `just init` — by hand,
`just kache-status` rules whether the filesystem earns it.

The standing `cross-check` clones must stay unwrapped; use the empty-string
form (`RUSTC_WRAPPER=""`) for that, not merely unsetting the variable — code
that merely unsets now inherits the config-file wrapper on qualifying hosts.

If kache prints storage-layout advice on a non-clone volume, do **not** take
its first two suggestions: `windows_hardlink = true` is unsafe because Cargo
rewrites object outputs, and `storage_layout_advice = false` silences the
signal instead of the cause.

Activation disables Cargo's incremental compilation, which is the largest
behavioral change on adoption.

The store is bounded; `target/` is not. Run `just sweep` to prune stale
build artifacts (`cargo sweep`: uninstalled toolchains, then anything untouched
for 14 days, then a 120 GB per-root cap only when the target filesystem has
less than 100 GiB free). With a warm kache store,
swept artifacts return as link-restores rather than recompiles, and a lean
`target/` keeps kache's per-crate keying fast. Schedule it per host — launchd
on macOS, `just install-windows-sweep` on Windows, and cron or a systemd timer
on Linux. The Windows task uses an 80 GB cap and can be inspected with
`just windows-sweep-status`. The decisions and sizing evidence live in
`docs/kache-strategy.md`.

Judge the cache with `kache stats` (hit rate, time saved), not `kache doctor`
— a green doctor with a low hit rate is a failing cache.

On qualifying hosts init installs and manages the daemon (it is part of step
5 above). `just cache-daemon-install` remains for hosts that want the login
service without init's full flow — remote caching is the reason to run a
daemon at all:

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

The probe decides — nothing below is a conclusion to skip it with. The column
records what the probe concludes for each runtime's typical filesystem; a
login service is installed by init only where the probe qualifies.

| Runtime | Typical restore mode | Probe verdict | Login service |
|---|---|---|---|
| macOS (APFS) | reflink | qualifies when the store and checkout share a device | installed and managed by `just init` |
| Linux (btrfs, XFS-reflink) | reflink | qualifies | installed and managed by `just init` |
| Linux (ext4) | hardlink | does not qualify — store is a second copy | by hand only |
| WSL 2 | hardlink (ext4 in a VHDX) | does not qualify (the canonical negative case) | by hand only |
| Native Windows (ReFS / Dev Drive) | block clone | qualifies by filesystem type (the qualification probe and its store candidate ran on a ReFS Dev Drive 2026-09-23; a full `just init` has not) | by hand only |
| Native Windows (NTFS) | copy | does not qualify | not installed |
| WSL 1 | best effort | not recommended | do not install |

Under WSL, keep the repository and kache store in the Linux filesystem, such
as `~/coding` and `~/.cache/kache`. Builds under `/mnt/c`, `/mnt/d`, or another
Windows-mounted path pay cross-filesystem overhead and may lose cheap
reflink/hardlink restores.

Local hits and misses keep working without the daemon; remote checks, uploads,
and prefetching require a running daemon or an explicit `kache sync`.

## Installed Developer Tools

After the prerequisites are ready, initialization installs these monorepo
CLIs:

- `sniff`
- Biscuit Hash (`bh`)
- Biscuit Terminal CLI tools
- Darkmatter (`md`)
- Playa
- Biscuit Speaks (`so-you-say`)
- Claudine

Initialization also guarantees the shell utilities used by shared recipes,
including `find`, `du`, `awk`, `jq`, `fd`, `rg`, and `fzf`, and installs
`eza` plus `cargo-sweep` for repository and target-directory maintenance.

## Installed CI/CD Tools

The CI/CD stage ensures tools invoked directly by repository workflows and
their local reproduction recipes:

- Python 3.10+, GitHub CLI, Node.js 22+, npm, and pnpm
- `cargo-nextest`, `cargo-llvm-cov`, and `release-plz`
- `cargo-fuzz` plus the nightly Rust toolchain on supported Unix hosts
- `cross` and Bencher on Linux, where their owning workflows run

Third-party GitHub Actions remain responsible for binaries used only inside
the action implementation. A successful `just init` never treats a missing
applicable command as a successful bootstrap.

It also ensures GitNexus is installed globally through npm, installing its
native build toolchain first (`node-gyp`, `node-addon-api`, and `tree-sitter`
with npm's `--allow-scripts` approval so the native binding actually builds).
GitNexus requires Node.js 22 or newer and a writable global npm prefix, or
`sudo` access for that prefix.

## Troubleshooting

If initialization stops, fix the reported prerequisite and run `just init`
again. Useful focused checks are:

```sh
just _ensure-host-tools
just _ensure-ci-tools
rustc --version
cargo --version
sniff runtime
gitnexus status
just kache-status   # compiler-cache verdict for this host
```

If Cargo reports that `kache` cannot be found, the wrapper is active but the
binary is not on PATH — clear it for this shell with `export RUSTC_WRAPPER=""`
(the empty value overrides the config file), or neutralize/remove the
`rustc-wrapper` line in `$CARGO_HOME/config.toml` (the legacy
`$CARGO_HOME/config` instead, when it exists), then re-run `just init` to
repair the setup. If a WSL 2 host
should support a daemon but
`systemctl --user` is unavailable, enable systemd for that WSL distribution or
use local-only caching.
