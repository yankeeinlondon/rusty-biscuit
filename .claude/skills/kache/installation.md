# Installing kache

Verified against the Kunobi installation docs, July 2026. Prerequisite: **Rust 1.95 or later**.
kache ships as a self-contained binary with no runtime dependencies.

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

Or per-shell / per-CI-step: `export RUSTC_WRAPPER=kache`.

Prefer the env var when you want kache active for one build or one agent only; prefer
`~/.cargo/config.toml` for a machine-wide default. Note that `~/.cargo/config.toml` is often on a
shared or synced home directory — check before assuming the setting is host-local.

### In this repository

Activation is host policy; the repo tracks no wrapper and CI does not use kache. Two rules that
are easy to trip over, because a machine-wide `kache init` is invisible from inside a clone:

- **Windows dev hosts: leave it off.** NTFS restores by copy, so the store becomes a real second
  copy of every cached artifact. Opt in only after a ReFS Dev Drive holding the store *and*
  `target/` is measured.
- **Never** answer kache's storage-layout advisory with `windows_hardlink = true` (Cargo rewrites
  object outputs, which that setting forbids) or `storage_layout_advice = false` (silences the
  signal, not the cause).

`just kache-status` reports what is active on the current host, whether the volume clones blocks,
and the exact undo. Decision table and evidence: `docs/kache-strategy.md`.

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
store directory — see [platforms.md](platforms.md) for its location on each OS. Removing the wrapper
re-enables cargo's incremental compilation on the next build.
