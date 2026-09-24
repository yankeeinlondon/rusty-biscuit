# Installing kache

Verified against the Kunobi installation docs, July 2026. Prerequisite: **Rust 1.95 or later**.
kache ships as a self-contained binary with no runtime dependencies.

## Preferred: `cargo binstall`, on every OS

`cargo binstall` fetches a prebuilt binary rather than compiling from source, and it accepts an
exact version — so it is the one path that works identically on macOS, Linux, and Windows *and*
can honour a repository's pin:

```bash
cargo binstall --no-confirm --version <pinned> kache
```

Prefer it over the per-OS package managers below, which are fallbacks: each resolves its own
version, so a team using several of them drifts apart.

In **rusty-biscuit**, do not run this by hand. `just init` decides from a filesystem
qualification probe (`scripts/kache-host.sh qualify`), never from the OS name. On a qualifying
host it installs or upgrades to the latest release, then writes placement, owns the daemon, and
activates last. On a non-qualifying host it never installs kache. The explicit recipe installs or
upgrades on any host and stops there; `true` is binary-only (no config writes):

```bash
just install-kache        # binary + default store-cap seed on a config-less host
just install-kache true   # binary only
```

**Version floor: 0.23.0** (`.github/kache-min-version`). It is the measured line: the store
precedence stack (`local_store` + `ignore_env`) and the daemon's socket-follows-store behavior were
verified on 0.23.1. The floor is a check, never a pin — installs always target the latest release.
Neither recipe touches the daemon; only `just init` does. See `docs/initialization.md` and
`docs/kache-strategy.md`.

## macOS: the hardened runtime strips `DYLD_*` — re-sign ad hoc

The prebuilt release (`cargo binstall`, the 0.19.0 build too) is signed with the **hardened
runtime** (`codesign -dvv` → `flags=0x10000(runtime)`). dyld removes every `DYLD_*` variable from
a hardened process's environment at launch, so kache cannot pass them on to rustc. Rust's bundled
`rust-lld` loads `libLLVM.dylib` through `DYLD_FALLBACK_LIBRARY_PATH`, which rustup exports.
The symptom is a wrapped link, typically wasm, dying with:

```text
dyld: Library not loaded: @rpath/libLLVM.dylib
error: failed to invoke LLD: signal: 6 (SIGABRT)
```

Remedy: re-sign the installed binary ad hoc, which drops the hardened flag:

```bash
codesign --force -s - "$(command -v kache)"
codesign -dvv "$(command -v kache)"   # expect flags=0x2(adhoc)
```

A source install (`cargo install kache`) is ad hoc-signed by construction and is the fallback.
Every upgrade replaces the binary with a hardened one again, so the re-sign must follow every
install. In rusty-biscuit, `install-kache` does it and gates on
`scripts/kache-host.sh probe-passthrough` (a stub `rustc` run through `RUSTC_WRAPPER=kache`
that must see a `DYLD_*` variable arrive). The probe runs first, so an already re-signed binary is
not rewritten; replacing the binary makes the running daemon restart itself. Do not work around
the failure with toolchain `libLLVM.dylib` symlinks or recipe-level `DYLD_*` exports.

Two things hide or fake this symptom when you diagnose it (measured 2026-09-23):

- **`llvm-tools-preview` masks it.** `just init` runs `rustup component add llvm-tools-preview`,
  which installs a real `libLLVM.dylib` in `<toolchain>/lib/rustlib/<host>/lib/`. That is where
  `rust-lld`'s `@rpath` looks, so on that toolchain the link succeeds even through a hardened
  kache. To prove the passthrough, rename that file aside for the test, restore it afterwards, and
  compare against a pristine binstall release as the control.
- **Cargo replays old warnings.** If the dyld error appears as a *warning* (`stripping debug info
  with rust-objcopy failed`) with the same `dyld[<pid>]` on every build, cargo is re-emitting a
  cached diagnostic for an up-to-date unit, compiled before the fix. It is not a live failure, and
  it persists under `RUSTC_WRAPPER=""` until that unit rebuilds.

## macOS

```bash
# mise (recommended by the docs)
mise use -g github:kunobi-ninja/kache@latest

# Homebrew — stable
brew install kunobi-ninja/kunobi/kache
# Homebrew — RC/beta channel
brew install kunobi-ninja/kunobi/kache-unstable

# prebuilt binary via cargo-binstall
cargo binstall kache

# from source
cargo install kache
```

## Linux

```bash
# APT (Debian/Ubuntu)
sudo mkdir -p /etc/apt/keyrings
curl -fsSL https://r2.kunobi.com/kache/apt/gpg.key | sudo gpg --dearmor -o /etc/apt/keyrings/kache.gpg
echo "deb [signed-by=/etc/apt/keyrings/kache.gpg] https://r2.kunobi.com/kache/apt stable main" \
  | sudo tee /etc/apt/sources.list.d/kache.list
sudo apt update && sudo apt install kache

# Arch (AUR)
paru -S kache-bin      # or: yay -S kache-bin

# from source
cargo install kache
```

`mise` and `cargo binstall` also work on Linux and are the simplest route inside containers or WSL
where you don't want to add an apt source.

## Windows

```powershell
winget install kunobi-ninja.kache            # stable
winget install kunobi-ninja.kache.Unstable   # RC/beta

scoop bucket add kunobi https://github.com/kunobi-ninja/scoop-kunobi
scoop install kunobi/kache

choco install kache

cargo install kache
```

Inside **WSL**, install the *Linux* build in the distro — the Windows binary won't wrap the Linux
rustc. A Windows-side install is only needed if you build Windows targets natively on that host.

## Post-install

```bash
kache --version
kache init          # configures the cargo wrapper, installs + starts the daemon
kache doctor        # verify
```

`kache init` options:

| Flag | Effect |
| --- | --- |
| `-y`, `--yes` | Accept defaults, non-interactive (use in provisioning scripts) |
| `--no-service` | Configure the wrapper but don't install the daemon as a login service |
| `--check` | Print what would change without modifying anything |

`init` is **idempotent** — re-run it any time to repair a broken configuration.

## What `init` actually wires up

The cargo integration is a single line, which you can also write by hand:

```toml
# ~/.cargo/config.toml
[build]
rustc-wrapper = "kache"
```

Or per-shell / per-CI-step: `export RUSTC_WRAPPER=kache` (PowerShell:
`$env:RUSTC_WRAPPER = "kache"`).

Prefer the env var when you want kache active for one build or one agent only; prefer
`~/.cargo/config.toml` for a machine-wide default. Note that `~/.cargo/config.toml` is often on a
shared or synced home directory — check before assuming the setting is host-local.

### In this repository

`just init` is the only activator. On a qualifying host it writes `[build] rustc-wrapper = "kache"`
into `$CARGO_HOME/config.toml` as its last step, after install, store placement, and the daemon
all succeed. The repo tracks no wrapper, and CI never installs or uses kache. Do not run
`kache init` or export `RUSTC_WRAPPER=kache` in shell profiles. Either bypasses the probe and the
ordered sequence. Rules that are easy to trip over:

- **Windows NTFS dev hosts do not qualify.** NTFS restores by copy, so the store becomes a real
  second copy of every cached artifact. A ReFS Dev Drive that holds the store *and* the checkout
  qualifies when the probe passes.
- **Opting one command out:** `RUSTC_WRAPPER=""` wins over the Cargo config file. The standing
  `cross-check` clones build this way.
- **Never** answer kache's storage-layout advisory with `windows_hardlink = true` (Cargo rewrites
  object outputs, which that setting forbids) or `storage_layout_advice = false` (silences the
  signal, not the cause).

`just kache-status` re-runs the probe init used. It reports the store `kache doctor` resolves,
env passthrough, the checkout and worktree-base devices, and the exact undo. It exits non-zero
on drift while kache is active. Decision table and evidence: `docs/kache-strategy.md`.

## Verifying it's actually working

```bash
kache doctor                 # wrapper wired? daemon reachable? store OK?
kache stats --since 1h       # hits/misses after a build
kache why-miss <crate>       # if a crate you expected to hit didn't
```

A build that "seems the same speed" with no entries in `kache list` usually means the wrapper isn't
wired — `RUSTC_WRAPPER` unset in that shell, or a different cargo config in play.

## CI

For GitHub Actions use the official action rather than installing by hand:

```yaml
- uses: kunobi-ninja/kache-action@v1
```

It installs kache, wires `RUSTC_WRAPPER`, and persists the store between runs — GitHub Actions cache
by default, or S3 when configured. See [remote-cache.md](remote-cache.md).

## Uninstalling / backing out

```bash
kache daemon stop && kache daemon uninstall
kache purge                 # drop the store contents
```

Then remove `rustc-wrapper` from `~/.cargo/config.toml` (or unset `RUSTC_WRAPPER`) and delete the
store directory — the one `kache doctor` reports (run it before uninstalling). Removing the wrapper
re-enables cargo's incremental compilation on the next build.

`KACHE_DISABLED=1` is not a full rollback: current kache still strips Cargo's incremental flags
while acting as the wrapper. Remove or override the wrapper when comparing normal Cargo incremental
builds with kache.
