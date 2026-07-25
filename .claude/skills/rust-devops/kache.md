---
prompt: |-
    The [kache](https://kunobi.ninja/docs/kache) library is a drop-in RUSTC_WRAPPER that caches Rust compilation artifacts. Cache keys are blake3 hashes of normalized rustc invocations; cache hits restore via hardlinks, and identical blobs are stored once and shared. Optional S3 sync (AWS, Ceph, MinIO, R2) shares the cache across machines.

    Your job is to do a deep-dive on this package and provide answer to the following questions:

    - functional footprint
    - when to use? when not to use? 
    - OS considerations
    - configuration
        - with `mise`
        - without `mise`
    - benefits of a remote cache versus just a local cache
    - how to setup a kache daemon process
last_updated: 2026-06-02
---
## Functional Footprint

kache is a `RUSTC_WRAPPER`-compatible binary that intercepts every `rustc` invocation from cargo and manages a content-addressed build cache. Its components are:

**Wrapper** — When `RUSTC_WRAPPER=kache` is set, cargo calls kache instead of rustc directly. For each invocation the wrapper parses arguments, computes a blake3 cache key, checks the local store, and on a miss delegates to the real rustc before recording the outputs.

**Local store** — A SQLite index (`index.db`) plus content-addressed blobs under `~/.cache/kache/store/`. Cache hits are restored via hardlinks into `target/` — no bytes copied, the inode is shared. Identical blobs from different crates are stored once and linked many times.

**Daemon** — A background process handling async S3 uploads, remote cache checks, and prefetch. Communicates with the wrapper over a Unix socket (`daemon.sock`) with sub-millisecond RPC. Optional for local-only caching; required for remote features.

**Monitor** — A TUI dashboard (`kache` or `kache monitor`) showing per-crate hit/miss status, store utilization, and transfer progress.

**CLI** — Subcommands for init, doctor, config, sync, list, stats, why-miss, and daemon lifecycle management.

### Cache key composition

The blake3 cache key is deterministic and portable across machines. It hashes: a key version scalar, the full `rustc --version --verbose` output, target triple, crate identity (name, types, edition), sorted codegen options, sorted feature and cfg flags, blake3 hashes of all source files discovered via a `dep-info` pre-pass (including `env!()`/`option_env!()` dependencies), extern dependency rlib/rmeta hashes, normalized `RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS`, sorted `CARGO_CFG_*` variables, and linker identity for binary outputs.

Paths in flags are normalized by replacing the current working directory with `.`, so machines with different checkout paths produce identical keys for the same source + toolchain.

### What kache does not cache by default

User-facing binaries and test harnesses are skipped by default. Dynamic libraries and proc-macros remain cached. Enable `cache_executables = true` or `KACHE_CACHE_EXECUTABLES=1` to opt in to executable caching. Incremental compilation is automatically disabled when kache is active.

### Requirements

- Rust 1.95+ (uses `let ... && let` chains and other recent stabilizations)
- Self-contained binary with no runtime dependencies

## When to Use

- **Frequent clean builds.** Monorepos with many crates where `cargo clean` or branch switching invalidates `target/`. kache restores via hardlinks in seconds.
- **CI pipelines.** Ephemeral runners that start from scratch each time. With S3 or GitHub Actions cache, artifacts from a previous run or a teammate's build are already available.
- **macOS worktrees.** kache disables incremental compilation, which avoids APFS-related corruption in git worktrees.
- **Team shared caches.** Multiple developers or CI runners compiling the same dependency graph share artifacts through S3.
- **Large dependency graphs.** Projects with hundreds of crates benefit most — the time saved on unchanged dependencies compounds.

## When Not to Use

- **Single-crate projects with fast compile times.** The overhead of cache key computation (including the `dep-info` pre-pass) may exceed the compilation itself.
- **Projects relying heavily on incremental compilation.** kache disables incremental by design; its artifact-level caching replaces it, but if you depend on incremental's fine-grained re-use for edit-compile cycles, kache's crate-level granularity may feel slower during active development.
- **Builds with heavy `env!()` usage.** Environment variables baked into the binary become part of the cache key, causing unnecessary misses if values change across runs.
- **When `sccache` or `cargo-cache` already meets your needs** and you have no reason to switch.

## OS Considerations

| Platform                       | Status        | Notes                                                                                                                                                      |
|--------------------------------|---------------|------------------------------------------------------------------------------------------------------------------------------------------------------------|
| macOS (Intel + Apple Silicon)  | Supported     | Pre-built binaries. Store excluded from Time Machine and Spotlight. Incremental disabled to avoid APFS corruption in worktrees. launchd service available. |
| Linux (x86_64 + aarch64, musl) | Supported     | Statically linked musl binaries. systemd user service available.                                                                                           |
| Windows (x64 + ARM64)          | Supported     | Native MSVC binaries are available; GitHub Actions support both architectures.                                                                              |

Cache directory locations follow platform conventions: `~/Library/Caches/kache` on macOS, `~/.cache/kache` on Linux.

## Configuration

### With `mise`

```bash
# install globally (recommended)
mise use -g github:kunobi-ninja/kache@latest

# or pin a specific version
mise use -g github:kunobi-ninja/kache@0.8.0
```

To pin per-project so every contributor gets the same version, add to `mise.toml`:

```toml
[tools]
"github:kunobi-ninja/kache" = "latest"
```

Then run `mise install` in the project root.

After the binary is available, run `kache init` to wire it into cargo, install the daemon as a login service, and start it:

```bash
kache init          # interactive setup
kache init -y       # accept all defaults
kache init --check  # dry-run, no changes
kache doctor        # verify everything is wired correctly
```

### Without `mise`

**Install from pre-built binary:**

```bash
cargo binstall --git https://github.com/kunobi-ninja/kache kache
```

**Or build from source:**

```bash
cargo install --git https://github.com/kunobi-ninja/kache kache
```

**Wire into cargo** — either set the environment variable:

```bash
export RUSTC_WRAPPER=kache
```

Or persist in `~/.cargo/config.toml`:

```toml
[build]
rustc-wrapper = "kache"
```

The config file approach applies to every project. The env var is better for CI or per-project testing.

### Config file

Priority: `KACHE_CONFIG` env var → nearest `.kache.toml` walking up from cwd → `~/.config/kache/config.toml` (respects `XDG_CONFIG_HOME`).

Edit interactively with `kache config` (TUI editor), or write TOML directly.

**Local-only config** (no file needed — defaults work out of the box):

```toml
[cache]
local_max_size = "50GiB"
clean_incremental = true
```

**Remote S3 config:**

```toml
[cache.remote]
type = "s3"
bucket = "my-build-cache"
endpoint = "https://s3.example.com"   # omit for AWS S3
profile = "my-aws-profile"            # omit for default chain
region = "us-east-1"
prefix = "artifacts"
```

**Exclude patterns** for sources that should compile normally but never use kache:

```toml
[cache]
exclude = [
  "crates/problematic-rust-crate/**",
  "vendor/problematic-clib/**",
  "$CARGO_HOME/registry/src/**/some-crate-*/**",
]
```

### Key environment variables

| Variable                                      | Default      | Purpose                                      |
|-----------------------------------------------|--------------|----------------------------------------------|
| `KACHE_DISABLED`                              | `0`          | Disable caching (pass-through to rustc)      |
| `KACHE_CACHE_EXECUTABLES`                     | `false`      | Cache bin/dylib/cdylib/proc-macro outputs    |
| `KACHE_S3_BUCKET`                             | —            | S3 bucket name                               |
| `KACHE_S3_ENDPOINT`                           | —            | S3 endpoint (required for Ceph, MinIO, R2)   |
| `KACHE_S3_ACCESS_KEY` / `KACHE_S3_SECRET_KEY` | —            | Explicit S3 credentials                      |
| `KACHE_COMPRESSION_LEVEL`                     | `3`          | Zstd level (1–22)                            |
| `KACHE_LOG`                                   | `kache=warn` | Log level (`trace`, `debug`, `info`, `warn`) |
| `KACHE_PROGRESS`                              | `auto`       | CI progress lines: `always`, `never`, `auto` |

### Disabling temporarily

```bash
KACHE_DISABLED=1 cargo build
```

Even when disabled, kache strips incremental flags to prevent the macOS APFS corruption issue.

## Remote Cache vs. Local Cache

### Local cache alone

Local caching requires zero configuration beyond `RUSTC_WRAPPER=kache`. It works without the daemon, without S3, and without a network connection. After a cold build populates the store, subsequent `cargo clean && cargo build` restores everything via hardlinks in seconds. The cache survives across projects — any project using the same toolchain and dependencies reuses artifacts.

Limitations: the cache is machine-local. A new CI runner, a fresh laptop, or a teammate's machine all start with an empty store.

### Remote cache (S3)

Adding an S3-compatible remote (AWS S3, Cloudflare R2, Ceph, MinIO) shares artifacts across machines:

- **CI runners** pull pre-built dependencies from the remote before the build starts (`kache sync --pull`), turning cold builds into warm ones.
- **Developer machines** automatically upload new artifacts via the daemon after compilation, making them available to the rest of the team.
- **Prefetch** — when the daemon is running, it uses `cargo metadata` to predict which crates will be needed and downloads them from S3 before cargo asks, converting remote hits into local hits.
- **Deduplication** — artifacts are stored once per unique cache key in S3. Multiple machines producing identical outputs share the same blob.

The remote is additive: local caching always works, and the daemon degrades gracefully if S3 is unreachable. The typical CI pattern is `kache sync --pull` before the build and `kache sync --push` after (with `if: always()` to preserve partial results).

For GitHub Actions specifically, the `kunobi-ninja/kache-action@v1` action uses GitHub's built-in cache as the backend — no S3 bucket required. Note that `kache-action@v1` runs on **Linux and macOS only**: it rejects Windows runners with `Unsupported platform: win32-x64`, even though the standalone kache binary supports Windows. A Windows CI leg must therefore build without kache.

### In this repo (rusty-biscuit)

kache is **opt-in** here — there is no committed global `rustc-wrapper` (no `.cargo/config.toml`), so a plain `cargo build` never invokes kache. CI enables it deliberately per leg. There is a single version authority, `.github/kache-version` (e.g. `0.8.0`), consumed by both the root `justfile` (`KACHE_VERSION`) and the verifying `.github/actions/enable-kache` composite action. Because `kache-action@v1` rejects `win32-x64`, CI runs kache on Linux and macOS only; Windows legs build without it.

### Credential resolution

1. `KACHE_S3_ACCESS_KEY` + `KACHE_S3_SECRET_KEY` (explicit env vars)
2. AWS profile from `cache.remote.profile` or `KACHE_S3_PROFILE`
3. Standard AWS chain (`AWS_ACCESS_KEY_ID`, `~/.aws/credentials`, IAM roles)

Bucket permissions needed: `s3:GetObject`, `s3:PutObject`, `s3:ListBucket`. Read-only CI runners need only `GetObject` + `ListBucket`.

## Setting Up a kache Daemon Process

### Quick start

```bash
kache daemon start     # start in background, blocks until socket is ready
kache daemon           # show status (running/stopped, PID, version, uptime)
kache daemon log       # tail the daemon log (follow mode)
kache daemon stop      # graceful shutdown
```

For debugging, run in the foreground:

```bash
kache daemon run       # foreground, logs to stderr
```

### Install as a persistent system service

**macOS (launchd):**

```bash
kache daemon install    # creates ~/Library/LaunchAgents/ninja.kunobi.kache.plist
kache daemon uninstall  # remove it
```

The daemon starts on login and restarts if it crashes.

**Linux (systemd):**

```bash
kache daemon install    # creates a systemd user unit and enables it
kache daemon uninstall  # remove it

systemctl --user status kache   # check status
```

### Automatic restart on binary update

kache tracks a build epoch (the mtime of the kache binary). When a newer binary makes an RPC call to an older daemon, the daemon schedules a graceful shutdown and the wrapper restarts it. No manual intervention needed after updates.

### When the daemon is offline

If the daemon is down or unreachable, the wrapper continues without it:

- Remote checks are skipped (wrapper compiles on local miss)
- Upload jobs are dropped (artifacts not pushed until daemon returns)
- Prefetch is skipped for the current build session

Local caching is unaffected. The monitor shows `daemon: offline` in this state.

### Idle timeout

By default the daemon auto-shuts down after 600 seconds of inactivity (`KACHE_DAEMON_IDLE_TIMEOUT`). Set to `0` to disable auto-shutdown.
