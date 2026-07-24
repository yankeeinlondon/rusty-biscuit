# Development Environment Initialization

Run `just init` from the repository root after cloning Rusty Biscuit. The
recipe prepares the host to build and test the monorepo without changing Cargo
settings for unrelated repositories.

## What `just init` Does

Initialization runs these stages in order:

1. Ensures Rust, Cargo, and the platform C build tools are available.
2. Installs the repository-pinned kache compiler cache.
3. Ensures GitNexus and its native Tree-sitter dependency are usable.
4. Builds and installs `sniff`, then reports whether the runtime is native,
   WSL 1, or WSL 2.
5. Runs `kache doctor` to verify that Cargo resolves the repository-local
   compiler wrapper.
6. Builds and installs the core Rusty Biscuit developer CLIs.

The recipe is idempotent. Running it again repairs missing tools and updates
kache when the repository pin changes.

## Build Caching

The tracked `.cargo/config.toml` configures:

```toml
[build]
rustc-wrapper = "kache"
```

Cargo discovers this file from the repository root and its package
subdirectories. Direct Cargo commands and package-area `just` recipes therefore
use the same cache. The initialization recipe does not call `kache init`
because that command edits the user-wide Cargo configuration and would affect
unrelated checkouts.

`just init` installs the kache version declared by `KACHE_VERSION` in the root
`justfile`. It temporarily clears `RUSTC_WRAPPER` while installing kache so an
absent or older wrapper cannot intercept its own installation.

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

| Runtime | Local cache | Login service |
|---|---|---|
| Native Linux | Enabled | Optional systemd user service |
| macOS | Enabled | Optional launchd service |
| Native Windows | Enabled | Not installed by `just init` |
| WSL 2 | Enabled | Optional when systemd is enabled |
| WSL 1 | Best effort; `kache doctor` must pass | Do not install automatically |

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

It also ensures GitNexus is installed globally through npm. GitNexus requires
Node.js 22 or newer and a writable global npm prefix, or `sudo` access for that
prefix.

## Troubleshooting

If initialization stops, fix the reported prerequisite and run `just init`
again. Useful focused checks are:

```sh
rustc --version
cargo --version
kache doctor
sniff runtime
gitnexus status
```

If Cargo reports that `kache` cannot be found, run `just _ensure-kache` and
then `just cache-status`. If a WSL 2 host should support a daemon but
`systemctl --user` is unavailable, enable systemd for that WSL distribution or
use local-only caching.
