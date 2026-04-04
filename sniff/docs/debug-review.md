# Sniff Debug & Tracing Review

Review date: 2026-04-03
Scope: `sniff/lib` and `sniff/cli` packages

## Executive Summary

The sniff library and CLI have **zero** tracing instrumentation across ~50,000 lines of code. Neither package depends on the `tracing` crate. Every other major package in this monorepo (claudine, research, darkmatter, playa, unchained-ai) has adopted `tracing` with the established conventions documented below. This makes sniff an outlier and leaves operators with no observability into detection performance, silent failures, or cache behavior.

## Current State

| Metric | Value |
|--------|-------|
| `tracing` dependency | **None** |
| `tracing-subscriber` dependency | **None** |
| `trace!` / `debug!` / `info!` / `warn!` / `error!` macros | **0** |
| `#[instrument]` attributes | **0** |
| `.ok()` calls silently discarding errors | **304** across 34 files |
| `eprintln!` in CLI (non-doc) | **4** (main, commands, install, just output) |
| `println!` for user output in CLI | ~47 |
| `--verbose` flag | Defined (`-v`, stackable) but unused for tracing |

The `--verbose` flag exists in `args.rs:180` and is threaded into a few output renderers, but it controls output detail level only -- it is never connected to a tracing subscriber.

## Monorepo Convention (Established Pattern)

Other packages follow this pattern consistently:

**Library crate:** `tracing = "0.1"` as a dependency. Public functions annotated with `#[instrument]`. Internal functions use `debug!`/`trace!` for operational visibility.

**CLI crate:** `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`. Subscriber initialized in `main()` with verbosity-driven filter levels. Output goes to **stderr** so stdout remains clean for data.

**Verbosity mapping** (from research/cli, darkmatter/cli):

| Level | Filter |
|-------|--------|
| default | `warn` |
| `-v` | `warn,sniff=info` |
| `-vv` | `info,sniff=debug` |
| `-vvv` | `debug,sniff=trace` + file/line numbers |
| `RUST_LOG` set | Overrides all above |

---

## Recommendations

### 1. Add `tracing` to `sniff/lib/Cargo.toml`

```toml
tracing = "0.1"
```

No features needed. The library should only **emit** events, never configure subscribers.

### 2. Add `tracing` + `tracing-subscriber` to `sniff/cli/Cargo.toml`

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### 3. Initialize subscriber in CLI `main.rs`

Wire the existing `--verbose` flag to a tracing subscriber. Follow the darkmatter pattern (lazy init -- skip if verbose=0 and RUST_LOG is unset):

```rust
fn init_tracing(verbose: u8) {
    let explicit_rust_log = std::env::var("RUST_LOG").ok();
    if verbose == 0 && explicit_rust_log.is_none() {
        return;
    }

    let base_filter = explicit_rust_log.unwrap_or_else(|| match verbose {
        1 => "warn,sniff=info,sniff_cli=info".into(),
        2 => "info,sniff=debug,sniff_cli=debug".into(),
        _ => "debug,sniff=trace,sniff_cli=trace".into(),
    });

    let filter = tracing_subscriber::EnvFilter::try_new(&base_filter)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_writer(std::io::stderr)
        .compact()
        .with_file(verbose >= 3)
        .with_line_number(verbose >= 3)
        .init();
}
```

This preserves zero overhead in normal operation (no subscriber = events are no-ops).

### 4. Instrument the library entry points

Add `#[instrument]` to the top-level detection orchestrator and each domain detection function. These form the **span tree** that makes `--verbose` useful:

| Function | File | Priority |
|----------|------|----------|
| `detect_with_plan()` | `lib.rs:236` | **High** -- root span, shows total wall time |
| `detect_os_with_request()` | `os/mod.rs` | **High** -- includes pkg manager detection |
| `detect_hardware_with_request()` | `hardware/mod.rs` | Medium |
| `detect_network_with_request()` | `network/mod.rs` | **High** -- includes WAN IP fetch |
| `detect_filesystem_with_request()` | `filesystem/mod.rs` | **High** -- most complex domain |

Use `skip` to avoid logging large request structs:

```rust
#[instrument(skip(plan), fields(
    os = plan.os.is_some(),
    hw = plan.hardware.is_some(),
    net = plan.network.is_some(),
    fs = plan.filesystem.is_some(),
))]
pub fn detect_with_plan(plan: DetectionPlan) -> Result<SniffResult> {
```

### 5. Instrument high-value internal functions

These are the functions where users will most want debug visibility:

#### Filesystem / Git (highest complexity, most silent failures)

| Function | File | Why |
|----------|------|-----|
| `GitRepo::discover()` | `filesystem/git.rs` | git2 errors are silently converted |
| `detect_git_info()` | `filesystem/git.rs` | Orchestrates all git detection |
| `get_commits()` | `filesystem/git.rs` | Slow on large repos, count matters |
| `detect_dirty_files()` | `filesystem/git.rs` | Frequent silent `.ok()` usage |
| `detect_worktrees()` | `filesystem/git.rs` | Calls `git worktree list` subprocess |
| `detect_repo_structure()` | `filesystem/repo.rs` | 54 `.ok()` calls -- most in the lib |
| `detect_languages()` | `filesystem/languages.rs` | Walks entire repo tree |
| `detect_docs()` | `filesystem/docs.rs` | 11 `.ok()` calls |
| `find_blast_radius_documents()` | `filesystem/blast_radius.rs` | Complex path analysis |

#### Programs

| Function | File | Why |
|----------|------|-----|
| `ExecutableIndex::build()` | `programs/find_program.rs` | Scans all PATH dirs + bundles |
| `find_programs_parallel()` | `programs/find_program.rs` | Rayon parallel scan |
| `detect_programs()` | `programs/mod.rs` | Aggregates all 8 categories |

#### OS

| Function | File | Why |
|----------|------|-----|
| `detect_package_managers()` | `os/package_manager.rs` | 2500+ lines, platform-specific |
| `detect_distro()` | `os/distro.rs` | Reads /etc files, can fail silently |
| `detect_locale()` | `os/locale.rs` | Reads 5+ env vars, fallback chain |

#### Network

| Function | File | Why |
|----------|------|-----|
| `detect_wan_ip()` | `network/mod.rs` | HTTP call with 5min cache, silent failure |
| `detect_interfaces()` | `network/mod.rs` | Permission errors silently swallowed |

#### Services

| Function | File | Why |
|----------|------|-----|
| `ServiceManager::detect()` | `services/mod.rs` | Init system detection |
| `services_detailed()` | `services/mod.rs` | Parses subprocess output |

### 6. Replace silent `.ok()` with `warn!` or `debug!` at key sites

The 304 `.ok()` calls represent the biggest observability gap. Not all need tracing -- many are legitimate (e.g., parse attempts, optional fields). Focus on these categories:

**Must trace (use `warn!`):** Subprocess failures, file read failures in detection paths, permission errors, network errors.

```rust
// Before (silent)
let output = Command::new("git").args(["worktree", "list"]).output().ok()?;

// After (observable)
let output = Command::new("git")
    .args(["worktree", "list"])
    .output()
    .map_err(|e| { warn!(error = %e, "git worktree list failed"); e })
    .ok()?;
```

**Should trace (use `debug!`):** Config file reads, optional feature detection, fallback paths.

```rust
// Before
let content = fs::read_to_string(path).ok()?;

// After
let content = fs::read_to_string(path)
    .map_err(|e| { debug!(path = %path.display(), error = %e, "could not read file"); e })
    .ok()?;
```

**Leave as-is:** Parse conversions (`.parse::<u64>().ok()?`), iterator filters (`.filter_map(|e| e.ok())`), and env var reads for optional values.

### 7. Add timing spans for performance-critical operations

These operations dominate wall time and users need visibility:

```rust
// In detect_with_plan, wrap each thread spawn:
let os_handle = plan.os.as_ref().map(|req| {
    s.spawn(move || {
        let _span = tracing::info_span!("detect_os").entered();
        os::detect_os_with_request(req)
    })
});
```

This enables output like:
```
 INFO sniff: detect_os: 45ms
 INFO sniff: detect_hardware: 120ms
 INFO sniff: detect_network: 1.2s
 INFO sniff: detect_filesystem: 890ms
```

### 8. Add trace-level events for expensive iterations

For functions that iterate over large collections, add `trace!` at the item level so `RUST_LOG=sniff=trace` reveals bottlenecks:

```rust
// In ExecutableIndex::build()
for dir in std::env::split_paths(&path_var) {
    trace!(dir = %dir.display(), "scanning PATH directory");
    // ...
}

// In detect_languages()
for entry in walker {
    trace!(path = %entry.path().display(), "classifying file");
    // ...
}
```

### 9. Instrument the CLI command dispatch

Add a root span in `commands::run()` that captures the subcommand being executed:

```rust
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // ... completions handling ...
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    let _root = tracing::info_span!("sniff",
        command = ?cli.command.as_ref().map(|c| format!("{c:?}")),
        json = cli.json,
        plain = cli.plain,
    ).entered();

    // ... rest of dispatch ...
}
```

### 10. Instrument the WAN IP cache

The network module has a static mutex cache with a 5-minute TTL. This is invisible today:

```rust
debug!(cached = cache_entry.is_some(), expired = is_expired, "WAN IP cache check");
// on fetch:
info!(ip = %ip, "WAN IP resolved");
// on cache hit:
debug!("WAN IP served from cache");
```

---

## Priority Order

| Phase | Scope | Effort | Impact |
|-------|-------|--------|--------|
| **P0** | Add crate deps, init subscriber, wire `--verbose` | Small | Enables all subsequent work |
| **P1** | `#[instrument]` on 5 domain entry points + `detect_with_plan` | Small | Shows detection timing breakdown |
| **P2** | `warn!` on subprocess/file/network `.ok()` failures (~20 sites) | Medium | Surfaces silent failures |
| **P3** | `debug!` on config reads, fallback paths (~30 sites) | Medium | Debugging detection logic |
| **P4** | `trace!` on iteration-heavy paths (PATH scan, repo walk, language detection) | Medium | Performance profiling |
| **P5** | Remaining `.ok()` audit across 34 files | Large | Completeness |

## Files Not Needing Tracing

These files are pure data models / enums / serialization with no I/O and don't need instrumentation:

- `error.rs` -- error type definitions only
- `request.rs` -- builder types, no side effects
- `programs/types.rs`, `programs/enums.rs`, `programs/schema.rs` -- data definitions
- `network/interface.rs` -- data structures
- `filesystem/file_types/model.rs` -- data structures
- `hardware/gpu.rs` (struct definitions), `hardware/cpu.rs` (struct definitions)

## Anti-Patterns to Avoid

1. **Don't instrument private helpers that are called in tight loops** -- the span overhead adds up. Use `trace!` events instead of `#[instrument]` for these.
2. **Don't log full data structures** -- use `skip` on `#[instrument]` for `SniffResult`, `GitRepo`, `Vec<Package>`, etc. Log counts instead (`packages = result.len()`).
3. **Don't add `info!` to hot paths** -- `detect_languages()` walks thousands of files. Item-level events should be `trace!`, summary should be `debug!` or `info!`.
4. **Don't initialize the subscriber when verbose=0** -- follow darkmatter's lazy pattern to preserve zero overhead for normal users.
5. **Don't use `tracing::error!` for expected failures** -- missing optional files, unavailable hardware features, or absent package managers are not errors. Use `debug!` or `warn!`.
