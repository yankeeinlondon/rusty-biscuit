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

In **rusty-biscuit**, do not run this by hand — use the recipe, which reads the single version
authority at `.github/kache-version`:

```bash
just install-kache
```

That installs and stops. Activation is a separate, deliberate step
(`RUSTC_WRAPPER=kache` for one shell, or `kache init` host-wide), because whether kache pays off
depends on the store's filesystem. See `docs/initialization.md` and `docs/kache-strategy.md`.

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

Avoid committing `rustc-wrapper = "kache"` to a cross-platform repository unless installation,
filesystem behavior, bypass semantics, and CI setup are intentional for every supported host.
Explicit CI activation plus host-local developer opt-in is usually the safer mixed-OS policy.

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

`KACHE_DISABLED=1` is not a full rollback: current kache still strips Cargo's incremental flags
while acting as the wrapper. Remove or override the wrapper when comparing normal Cargo incremental
builds with kache.
