# Sniff Tracing Instrumentation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add structured tracing to the sniff library and CLI, wiring the existing `--verbose` flag to a tracing subscriber and instrumenting all detection entry points, high-value internal functions, and silent `.ok()` failure sites.

**Architecture:** The library crate (`sniff`) depends on `tracing` only — it emits spans and events but never configures a subscriber. The CLI crate (`sniff-cli`) depends on `tracing` + `tracing-subscriber` and initializes the subscriber in `main.rs` based on the `--verbose` count. When no subscriber is active (default operation), all tracing macros are no-ops with zero overhead.

**Tech Stack:** `tracing = "0.1"`, `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`

**Source spec:** `sniff/docs/debug-review.md`

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `sniff/lib/Cargo.toml` | Modify | Add `tracing` dependency |
| `sniff/cli/Cargo.toml` | Modify | Add `tracing` + `tracing-subscriber` dependencies |
| `sniff/cli/src/main.rs` | Modify | Add `init_tracing()` call |
| `sniff/cli/src/commands.rs` | Modify | Call `init_tracing()`, add root span |
| `sniff/lib/src/lib.rs` | Modify | `#[instrument]` on `detect_with_plan` |
| `sniff/lib/src/os/mod.rs` | Modify | `#[instrument]` on `detect_os_with_request` |
| `sniff/lib/src/hardware/mod.rs` | Modify | `#[instrument]` on `detect_hardware_with_request` |
| `sniff/lib/src/network/mod.rs` | Modify | `#[instrument]` on `detect_network_with_request`, trace WAN IP cache |
| `sniff/lib/src/filesystem/mod.rs` | Modify | `#[instrument]` on `detect_filesystem_with_request` |
| `sniff/lib/src/filesystem/git.rs` | Modify | `#[instrument]` on `discover`, `detect_git_with_request`; `warn!`/`debug!` on `.ok()` sites |
| `sniff/lib/src/filesystem/repo.rs` | Modify | `#[instrument]` on `detect_repo_structure`; `debug!` on `.ok()` read failures |
| `sniff/lib/src/filesystem/languages.rs` | Modify | `#[instrument]` on `detect_languages`; `trace!` on iteration |
| `sniff/lib/src/filesystem/docs.rs` | Modify | `#[instrument]` on `detect_docs`; `debug!` on read failures |
| `sniff/lib/src/filesystem/blast_radius.rs` | Modify | `#[instrument]` on `find_blast_radius_documents` |
| `sniff/lib/src/programs/mod.rs` | Modify | `#[instrument]` on `ProgramsInfo::detect` |
| `sniff/lib/src/programs/find_program.rs` | Modify | `#[instrument]` on `ExecutableIndex::build`, `find_programs_parallel`; `trace!` on PATH iteration |
| `sniff/lib/src/os/package_manager.rs` | Modify | `debug!` on subprocess failures |
| `sniff/lib/src/os/distro.rs` | Modify | `debug!` on file read failures |
| `sniff/lib/src/os/locale.rs` | Modify | `#[instrument]` on `detect_locale` |
| `sniff/lib/src/services/mod.rs` | Modify | `#[instrument]` on `ServiceManager::detect`, `detect_services`; `warn!` on subprocess failures |

---

### Task 1: Add `tracing` dependency to sniff library

**Files:**
- Modify: `sniff/lib/Cargo.toml`

- [ ] **Step 1: Add tracing to lib dependencies**

In `sniff/lib/Cargo.toml`, add to the `[dependencies]` section (alphabetical placement after `thiserror`):

```toml
tracing = "0.1"
```

- [ ] **Step 2: Verify it compiles**

Run: `just build -p sniff`
Expected: Compiles successfully with no errors.

- [ ] **Step 3: Commit**

```bash
git add sniff/lib/Cargo.toml
git commit -m "feat(sniff): add tracing dependency to sniff library"
```

---

### Task 2: Add `tracing` + `tracing-subscriber` to sniff CLI

**Files:**
- Modify: `sniff/cli/Cargo.toml`

- [ ] **Step 1: Add tracing deps to CLI**

In `sniff/cli/Cargo.toml`, add to the `[dependencies]` section:

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

- [ ] **Step 2: Verify it compiles**

Run: `just build -p sniff-cli`
Expected: Compiles successfully with no errors.

- [ ] **Step 3: Commit**

```bash
git add sniff/cli/Cargo.toml
git commit -m "feat(sniff-cli): add tracing and tracing-subscriber dependencies"
```

---

### Task 3: Initialize tracing subscriber in CLI and add root span

**Files:**
- Modify: `sniff/cli/src/main.rs`
- Modify: `sniff/cli/src/commands.rs`

- [ ] **Step 1: Add `init_tracing` function to `main.rs`**

Add the `init_tracing` function to `sniff/cli/src/main.rs`. Follow the darkmatter pattern — lazy init that skips if verbose=0 and RUST_LOG is unset:

```rust
mod args;
mod commands;
mod install;
mod output;

use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize tracing subscriber based on verbosity level.
///
/// Verbosity levels:
/// - 0 (default): No subscriber (zero overhead)
/// - 1 (-v): INFO for sniff crates
/// - 2 (-vv): DEBUG for sniff crates
/// - 3+ (-vvv): TRACE for sniff crates with file/line numbers
///
/// Setting `RUST_LOG` overrides all verbosity levels.
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

    let filter =
        EnvFilter::try_new(&base_filter).unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_thread_ids(false)
                .with_file(verbose >= 3)
                .with_line_number(verbose >= 3)
                .with_writer(std::io::stderr)
                .compact(),
        )
        .init();
}

#[tokio::main]
async fn main() {
    if let Err(e) = commands::run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
```

- [ ] **Step 2: Make `init_tracing` callable from commands**

The function is defined in `main.rs`. Make it `pub(crate)` so `commands.rs` can call it:

```rust
pub(crate) fn init_tracing(verbose: u8) {
```

- [ ] **Step 3: Call `init_tracing` in `commands::run()` and add root span**

In `sniff/cli/src/commands.rs`, right after `let cli = Cli::parse();` (line ~36), add:

```rust
use tracing::info_span;

// ... inside run(), after `let cli = Cli::parse();`:
crate::init_tracing(cli.verbose);

let _root = info_span!("sniff",
    command = ?cli.command.as_ref().map(|c| format!("{c:?}")),
    json = cli.json,
    plain = cli.plain,
).entered();
```

Add `use tracing::info_span;` to the top of `commands.rs`.

- [ ] **Step 4: Verify it compiles and the --verbose flag produces tracing output**

Run: `just build -p sniff-cli`
Expected: Compiles successfully.

Then test:
```bash
cargo run -p sniff-cli -- -v --json 2>/dev/null | head -1
cargo run -p sniff-cli -- -v --json 2>&1 >/dev/null | head -5
```
Expected: First command outputs JSON to stdout. Second command shows tracing output on stderr with INFO-level spans.

- [ ] **Step 5: Commit**

```bash
git add sniff/cli/src/main.rs sniff/cli/src/commands.rs
git commit -m "feat(sniff-cli): wire --verbose flag to tracing subscriber

Follows darkmatter CLI pattern: lazy init, RUST_LOG override,
verbosity-mapped filter levels, stderr output."
```

---

### Task 4: Instrument `detect_with_plan` root span

**Files:**
- Modify: `sniff/lib/src/lib.rs:236`

- [ ] **Step 1: Add `#[instrument]` to `detect_with_plan`**

Add `use tracing::instrument;` to the top of `lib.rs`, then annotate:

```rust
use tracing::instrument;

#[instrument(skip(plan), fields(
    os = plan.os.is_some(),
    hw = plan.hardware.is_some(),
    net = plan.network.is_some(),
    fs = plan.filesystem.is_some(),
))]
pub fn detect_with_plan(plan: DetectionPlan) -> Result<SniffResult> {
```

- [ ] **Step 2: Add timing spans for each domain thread**

Inside `detect_with_plan`, wrap each `s.spawn` with a tracing span so each domain's wall time is visible:

```rust
let os_handle = plan
    .os
    .as_ref()
    .map(|req| {
        s.spawn(move || {
            let _span = tracing::info_span!("detect_os").entered();
            os::detect_os_with_request(req)
        })
    });

let hw_handle = plan
    .hardware
    .as_ref()
    .map(|req| {
        s.spawn(move || {
            let _span = tracing::info_span!("detect_hardware").entered();
            hardware::detect_hardware_with_request(req)
        })
    });

let net_handle = plan
    .network
    .as_ref()
    .map(|req| {
        s.spawn(move || {
            let _span = tracing::info_span!("detect_network").entered();
            network::detect_network_with_request(req)
        })
    });

let fs_handle = plan
    .filesystem
    .as_ref()
    .map(|req| {
        s.spawn(move || {
            let _span = tracing::info_span!("detect_filesystem").entered();
            filesystem::detect_filesystem_with_request(&base, req)
        })
    });
```

- [ ] **Step 3: Verify it compiles**

Run: `just build -p sniff`
Expected: Compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add sniff/lib/src/lib.rs
git commit -m "feat(sniff): instrument detect_with_plan with tracing spans

Root span logs which domains are requested. Each domain thread
gets its own info_span for wall-time visibility."
```

---

### Task 5: Instrument the 4 domain entry points

**Files:**
- Modify: `sniff/lib/src/os/mod.rs:204`
- Modify: `sniff/lib/src/hardware/mod.rs:57`
- Modify: `sniff/lib/src/network/mod.rs:132`
- Modify: `sniff/lib/src/filesystem/mod.rs:60`

- [ ] **Step 1: Instrument `detect_os_with_request`**

In `sniff/lib/src/os/mod.rs`, add `use tracing::instrument;` to the imports, then:

```rust
#[instrument(skip(request), fields(
    pkg_managers = request.include_package_managers,
    locale = request.include_locale,
    timezone = request.include_timezone,
))]
pub fn detect_os_with_request(request: &OsRequest) -> Result<OsInfo> {
```

- [ ] **Step 2: Instrument `detect_hardware_with_request`**

In `sniff/lib/src/hardware/mod.rs`, add `use tracing::instrument;` then:

```rust
#[instrument(skip(request), fields(
    storage = request.include_storage,
    gpu = request.include_gpu,
    audio = request.include_audio,
))]
pub fn detect_hardware_with_request(request: &HardwareRequest) -> Result<HardwareInfo> {
```

- [ ] **Step 3: Instrument `detect_network_with_request`**

In `sniff/lib/src/network/mod.rs`, add `use tracing::instrument;` then:

```rust
#[instrument(skip(request), fields(
    wan_ip = request.include_wan_ip,
    force_refresh = request.force_refresh,
))]
pub fn detect_network_with_request(request: &NetworkRequest) -> Result<NetworkInfo> {
```

- [ ] **Step 4: Instrument `detect_filesystem_with_request`**

In `sniff/lib/src/filesystem/mod.rs`, add `use tracing::instrument;` then:

```rust
#[instrument(skip(request), fields(
    git = request.git.is_some(),
    repo = request.repo.is_some(),
    files = request.include_file_inventory,
    docs = request.include_docs,
))]
pub fn detect_filesystem_with_request(
    root: &Path,
    request: &FilesystemRequest,
) -> Result<FilesystemInfo> {
```

- [ ] **Step 5: Verify it compiles**

Run: `just build -p sniff`
Expected: Compiles with no errors.

- [ ] **Step 6: Commit**

```bash
git add sniff/lib/src/os/mod.rs sniff/lib/src/hardware/mod.rs sniff/lib/src/network/mod.rs sniff/lib/src/filesystem/mod.rs
git commit -m "feat(sniff): instrument 4 domain detection entry points

Adds #[instrument] to detect_os_with_request, detect_hardware_with_request,
detect_network_with_request, and detect_filesystem_with_request."
```

---

### Task 6: Instrument WAN IP cache

**Files:**
- Modify: `sniff/lib/src/network/mod.rs:297`

- [ ] **Step 1: Add tracing to the `detect_wan_ip` function**

In the `#[cfg(feature = "network")] fn detect_wan_ip` function at line 297, add cache observability:

```rust
#[cfg(feature = "network")]
fn detect_wan_ip(force_refresh: bool) -> Option<String> {
    // Check cache first (unless refresh is forced)
    if !force_refresh {
        if let Ok(guard) = WAN_IP_CACHE.lock() {
            if let Some(entry) = guard.as_ref() {
                if entry.fetched_at.elapsed() < WAN_IP_TTL {
                    tracing::debug!("WAN IP served from cache");
                    return entry.value.clone();
                }
                tracing::debug!("WAN IP cache expired");
            }
        }
    }

    // Fetch fresh value
    let value = WanIpDetector::new().detect();

    match &value {
        Some(ip) => tracing::info!(ip = %ip, "WAN IP resolved"),
        None => tracing::warn!("WAN IP detection failed"),
    }

    // Update cache
    if let Ok(mut guard) = WAN_IP_CACHE.lock() {
        *guard = Some(WanIpCacheEntry {
            value: value.clone(),
            fetched_at: std::time::Instant::now(),
        });
    }

    value
}
```

- [ ] **Step 2: Add `warn!` to `detect_local_interfaces` permission error**

In the `detect_local_interfaces` function, add a `warn!` when permission is denied. Locate the `PermissionDenied` arm and add:

```rust
Err(PermissionOrError::PermissionDenied) => {
    tracing::warn!("network interface detection: permission denied");
    Ok(NetworkInfo {
        // ... existing fields ...
    })
}
```

Note: this is in `detect_network_with_request`, not `detect_local_interfaces` directly. Find the match arm that returns `PermissionDenied` and add the `warn!` there.

- [ ] **Step 3: Verify it compiles**

Run: `just build -p sniff`
Expected: Compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add sniff/lib/src/network/mod.rs
git commit -m "feat(sniff): instrument WAN IP cache and network permission errors

Cache hits/misses/expirations now visible at debug level.
WAN IP resolution logged at info. Permission denied at warn."
```

---

### Task 7: Instrument filesystem/git functions

**Files:**
- Modify: `sniff/lib/src/filesystem/git.rs`

- [ ] **Step 1: Add tracing imports**

At the top of `git.rs`, add:

```rust
use tracing::{debug, instrument, trace, warn};
```

- [ ] **Step 2: Instrument `GitRepo::discover`**

```rust
#[instrument(skip_all, fields(path = %path.display()))]
pub fn discover(path: &Path) -> Result<Option<Self>> {
    let repo = match Repository::discover(path) {
        Ok(r) => r,
        Err(e) => {
            debug!(error = %e, "not a git repository");
            return Ok(None);
        }
    };
    // ... rest unchanged
```

- [ ] **Step 3: Instrument `detect_git_with_request`**

```rust
#[instrument(skip_all, fields(path = %path.display()))]
pub fn detect_git_with_request(path: &Path, request: &GitRequest) -> Result<Option<GitInfo>> {
```

- [ ] **Step 4: Add `warn!` to `.ok()` sites that discard subprocess/git errors**

Find the `.ok()` calls in `git.rs` that discard subprocess or git2 errors and add tracing. The 44 `.ok()` calls in this file fall into categories:

**Subprocess calls** — add `warn!`:
Look for `Command::new(...)...output().ok()` patterns and replace with:
```rust
.output()
.map_err(|e| { warn!(error = %e, "git subprocess failed"); e })
.ok()
```

**git2 API calls** (`.head().ok()`, `.find_commit().ok()`, etc.) — add `debug!`:
```rust
.head()
.map_err(|e| { debug!(error = %e, "could not read HEAD"); e })
.ok()
```

**Parse/conversion calls** (`.to_str()`, `.parse()`, iterator `.ok()` filters) — leave as-is.

Focus on the highest-value sites:
- `detect_worktrees()` subprocess call
- `detect_dirty_files()` or equivalent diff/status calls
- Remote URL resolution failures
- Commit parsing failures

- [ ] **Step 5: Verify it compiles**

Run: `just build -p sniff`
Expected: Compiles with no errors.

- [ ] **Step 6: Commit**

```bash
git add sniff/lib/src/filesystem/git.rs
git commit -m "feat(sniff): instrument git detection with tracing

Adds #[instrument] to discover and detect_git_with_request.
Surfaces silent git2 errors at debug level and subprocess
failures at warn level."
```

---

### Task 8: Instrument filesystem/repo.rs

**Files:**
- Modify: `sniff/lib/src/filesystem/repo.rs`

- [ ] **Step 1: Add tracing imports**

```rust
use tracing::{debug, instrument, trace};
```

- [ ] **Step 2: Instrument `detect_repo_structure`**

```rust
#[instrument(skip_all, fields(root = %root.display()))]
pub fn detect_repo_structure(root: &Path) -> Result<Option<RepoInfo>> {
```

- [ ] **Step 3: Add `debug!` to config file read failures**

This file has ~30 `.ok()` calls, mostly `read_to_string(...).ok()`. These are config file reads (Cargo.toml, package.json, pyproject.toml, go.mod, etc.) where the file may legitimately not exist. Add `debug!` to the ones in detection paths (not version-extraction helpers):

```rust
// Pattern for detection-path reads:
let content = std::fs::read_to_string(lock_path)
    .map_err(|e| { debug!(path = %lock_path.display(), error = %e, "could not read file"); e })
    .ok()?;
```

Apply to: `lock_path` reads (line ~330), `toml_path` (line ~722), `package_json_path` (line ~855), `pyproject_path` (line ~939), `requirements_path` (line ~999), `go_mod_path` (line ~1032), `lerna_json_path` (line ~1410).

Leave the version-extraction helpers (lines ~1473–1537) as-is since they are parse utilities, not detection paths.

- [ ] **Step 4: Verify it compiles**

Run: `just build -p sniff`
Expected: Compiles with no errors.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/filesystem/repo.rs
git commit -m "feat(sniff): instrument repo detection with tracing

Adds #[instrument] to detect_repo_structure. Surfaces config file
read failures at debug level for workspace/package detection."
```

---

### Task 9: Instrument filesystem/languages.rs and filesystem/docs.rs

**Files:**
- Modify: `sniff/lib/src/filesystem/languages.rs`
- Modify: `sniff/lib/src/filesystem/docs.rs`

- [ ] **Step 1: Instrument `detect_languages`**

In `languages.rs`, add imports and instrument:

```rust
use tracing::{debug, instrument, trace};

#[instrument(skip_all, fields(root = %root.display()))]
pub fn detect_languages(root: &Path) -> Result<LanguageBreakdown> {
```

Add a `trace!` inside the file walk loop for per-file classification:

```rust
trace!(path = %entry.path().display(), "classifying file");
```

Add a `debug!` summary after the walk:

```rust
debug!(file_count = count, language_count = languages.len(), "language detection complete");
```

- [ ] **Step 2: Instrument `detect_docs`**

In `docs.rs`, add imports and instrument:

```rust
use tracing::{debug, instrument};

#[instrument(skip_all, fields(root = %root.display()))]
pub fn detect_docs(root: &Path) -> Option<Vec<MarkdownMeta>> {
```

Add `debug!` to the `read_to_string` failure at line ~185:

```rust
let content = fs::read_to_string(path)
    .map_err(|e| { debug!(path = %path.display(), error = %e, "could not read doc file"); e })
    .ok()?;
```

- [ ] **Step 3: Verify it compiles**

Run: `just build -p sniff`
Expected: Compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add sniff/lib/src/filesystem/languages.rs sniff/lib/src/filesystem/docs.rs
git commit -m "feat(sniff): instrument language and doc detection with tracing

Languages: trace-level per-file, debug summary.
Docs: debug on read failures."
```

---

### Task 10: Instrument filesystem/blast_radius.rs

**Files:**
- Modify: `sniff/lib/src/filesystem/blast_radius.rs`

- [ ] **Step 1: Instrument `find_blast_radius_documents`**

```rust
use tracing::{debug, instrument};

#[instrument(skip_all)]
pub fn find_blast_radius_documents(/* existing params */) -> /* existing return */ {
```

Add `debug!` to any `.ok()` calls that discard path analysis errors.

- [ ] **Step 2: Verify and commit**

Run: `just build -p sniff`

```bash
git add sniff/lib/src/filesystem/blast_radius.rs
git commit -m "feat(sniff): instrument blast radius detection with tracing"
```

---

### Task 11: Instrument programs detection

**Files:**
- Modify: `sniff/lib/src/programs/mod.rs`
- Modify: `sniff/lib/src/programs/find_program.rs`

- [ ] **Step 1: Instrument `ProgramsInfo::detect`**

In `programs/mod.rs`:

```rust
use tracing::{info_span, instrument};

#[instrument(skip_all)]
pub fn detect() -> Self {
    use std::sync::Arc;

    let _build_span = info_span!("build_executable_index").entered();
    let index = Arc::new(ExecutableIndex::build());
    drop(_build_span);

    // ... rest of rayon::join pairs unchanged
```

- [ ] **Step 2: Instrument `ExecutableIndex::build` with trace-level PATH iteration**

In `programs/find_program.rs`:

```rust
use tracing::{debug, instrument, trace};

#[instrument(skip_all)]
pub fn build() -> Self {
    let mut path_executables = HashMap::new();

    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            trace!(dir = %dir.display(), "scanning PATH directory");
            // ... existing logic
        }
    }

    // After PATH scan:
    debug!(path_count = path_executables.len(), "PATH scan complete");

    // ... macOS bundle scanning
```

- [ ] **Step 3: Instrument `find_programs_parallel`**

```rust
#[instrument(skip_all, fields(program_count = programs.len()))]
pub fn find_programs_parallel(programs: &[&str]) -> HashMap<String, Option<PathBuf>> {
```

- [ ] **Step 4: Verify it compiles**

Run: `just build -p sniff`
Expected: Compiles with no errors.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/mod.rs sniff/lib/src/programs/find_program.rs
git commit -m "feat(sniff): instrument program detection with tracing

Adds spans to ProgramsInfo::detect, ExecutableIndex::build, and
find_programs_parallel. Trace-level PATH directory scanning."
```

---

### Task 12: Instrument OS subsystem functions

**Files:**
- Modify: `sniff/lib/src/os/distro.rs`
- Modify: `sniff/lib/src/os/locale.rs`
- Modify: `sniff/lib/src/os/package_manager.rs`

- [ ] **Step 1: Instrument `detect_locale`**

In `os/locale.rs`:

```rust
use tracing::instrument;

#[instrument(skip_all)]
pub fn detect_locale() -> LocaleInfo {
```

- [ ] **Step 2: Add `debug!` to distro file read failures**

In `os/distro.rs`, add:

```rust
use tracing::debug;
```

At the 3 `read_to_string(...).ok()` sites (lines ~289, ~355, ~423):

```rust
let content = fs::read_to_string(path)
    .map_err(|e| { debug!(path = %path.display(), error = %e, "could not read distro file"); e })
    .ok()?;
```

- [ ] **Step 3: Add `debug!` to package manager subprocess failures**

In `os/package_manager.rs`, add `use tracing::debug;` and find `.ok()` calls on subprocess `Command` invocations. Add:

```rust
.map_err(|e| { debug!(error = %e, "package manager detection subprocess failed"); e })
```

- [ ] **Step 4: Verify it compiles**

Run: `just build -p sniff`
Expected: Compiles with no errors.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/os/distro.rs sniff/lib/src/os/locale.rs sniff/lib/src/os/package_manager.rs
git commit -m "feat(sniff): instrument OS subsystem detection with tracing

Adds #[instrument] to detect_locale. Surfaces distro file read
failures and package manager subprocess failures at debug level."
```

---

### Task 13: Instrument services detection

**Files:**
- Modify: `sniff/lib/src/services/mod.rs`

- [ ] **Step 1: Instrument service detection functions**

```rust
use tracing::{debug, instrument, warn};

#[instrument(skip_all)]
pub fn detect() -> Self {
    // ... existing ServiceManager::detect impl
}

#[instrument(skip_all)]
pub fn detect_services() -> ServicesInfo {
    // ... existing impl
}
```

- [ ] **Step 2: Add `warn!` to subprocess failures in service detection**

The 8 `.ok()` calls in this file include subprocess calls and file reads. For `read_to_string` at line ~440:

```rust
fs::read_to_string(path)
    .map_err(|e| { debug!(path = %path.display(), error = %e, "could not read service file"); e })
    .ok()
    .map(|s| s.trim().to_string())
```

For any `Command::new(...).output().ok()` patterns, add:

```rust
.map_err(|e| { warn!(error = %e, "service detection subprocess failed"); e })
.ok()
```

- [ ] **Step 3: Verify it compiles**

Run: `just build -p sniff`
Expected: Compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add sniff/lib/src/services/mod.rs
git commit -m "feat(sniff): instrument service detection with tracing

Adds #[instrument] to ServiceManager::detect and detect_services.
Surfaces subprocess failures at warn and file reads at debug."
```

---

### Task 14: Run full test suite and verify tracing output

**Files:** None (verification only)

- [ ] **Step 1: Run all sniff tests**

Run: `just test -p sniff -p sniff-cli`
Expected: All tests pass (tracing with no subscriber is a no-op).

- [ ] **Step 2: Manually test tracing output at each verbosity level**

```bash
# No tracing (default)
cargo run -p sniff-cli -- --json 2>/dev/null | python3 -m json.tool > /dev/null && echo "OK: clean JSON"

# -v: INFO spans visible
cargo run -p sniff-cli -- -v --json 2>&1 >/dev/null | grep -c "INFO" && echo "OK: INFO traces"

# -vv: DEBUG events visible
cargo run -p sniff-cli -- -vv --json 2>&1 >/dev/null | grep -c "DEBUG" && echo "OK: DEBUG traces"

# -vvv: TRACE events with file/line
cargo run -p sniff-cli -- -vvv --json 2>&1 >/dev/null | grep -c "TRACE" && echo "OK: TRACE traces"

# RUST_LOG override
RUST_LOG=sniff=trace cargo run -p sniff-cli -- --json 2>&1 >/dev/null | grep -c "TRACE" && echo "OK: RUST_LOG override"
```

- [ ] **Step 3: Verify JSON output is clean (no tracing on stdout)**

```bash
cargo run -p sniff-cli -- -vvv --json 2>/dev/null | python3 -m json.tool > /dev/null
```
Expected: Valid JSON. All tracing goes to stderr, never stdout.

- [ ] **Step 4: Commit (if any fixes needed)**

If verification revealed issues, fix and commit with a descriptive message.

---

### Task 15: Run lints

**Files:** None (verification only)

- [ ] **Step 1: Run clippy on both packages**

Run: `just lint -p sniff -p sniff-cli`
Expected: No new warnings. Fix any that appear.

- [ ] **Step 2: Commit lint fixes if any**

```bash
git add -A
git commit -m "fix(sniff): address clippy warnings from tracing instrumentation"
```
