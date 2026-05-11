# Implementation Plan: More Context Variables

## Overview

Expand Darkmatter's compose-time `ctx` namespace from a small date/time snapshot into a rich runtime context assembled from the local clock, environment, repository/monorepo inspection, document discovery, OS, and hardware -- all powered by the `sniff` library.

**Source documents:**

- [spec.md](./spec.md)
- [tech-design.md](./tech-design.md)

**Key invariants:**

- Context is captured once per compose run and reused across the entire transclusion graph
- Context base directory is the process CWD, not the input file location
- Runtime `ctx` values win over user-defined `ctx` values on collision
- Invalid (non-object) user `ctx` is a hard error unless `--allow-ctx-override` is passed

---

## Phase 1: Add `sniff` dependency and new context module skeleton

**Goal:** Wire up the sniff dependency and create the new `context/` module structure without changing any existing behavior.

### Task 1.1: Add `sniff` path dependency

**File:** `darkmatter/lib/Cargo.toml`

Add under `[dependencies]`:

```toml
sniff = { path = "../../sniff/lib" }
```

**Verification:** `cargo check -p darkmatter` succeeds.

### Task 1.2: Create context module skeleton

Create the following files:

| File | Responsibility |
|------|----------------|
| `darkmatter/lib/src/markdown/compose/context/mod.rs` | Module root; re-exports `capture`, `format`, `merge`, `diagnostics` |
| `darkmatter/lib/src/markdown/compose/context/capture.rs` | Raw runtime fact capture from chrono, std::env, sniff |
| `darkmatter/lib/src/markdown/compose/context/format.rs` | Normalize raw facts into stable JSON values; create human-friendly string/list variants |
| `darkmatter/lib/src/markdown/compose/context/merge.rs` | Merge user-defined `ctx` with runtime `ctx`; produce diagnostics |
| `darkmatter/lib/src/markdown/compose/context/diagnostics.rs` | `ContextMergeDiagnostic` enum and related types |

Register the module in `darkmatter/lib/src/markdown/compose/mod.rs`:

```rust
mod context;
```

**Verification:** `cargo check -p darkmatter` succeeds. No behavior changes.

---

## Phase 2: Map-backed `ComposeContext`

**Goal:** Refactor `ComposeContext` to use a `serde_json::Map<String, Value>` backing store while preserving the existing public fields for compatibility.

### Task 2.1: Add `values` field to `ComposeContext`

**File:** `darkmatter/lib/src/markdown/compose/types.rs`

Add a private backing store to the existing struct:

```rust
pub struct ComposeContext {
    // --- Existing public fields (kept for backward compatibility) ---
    pub now: String,
    pub utc: String,
    pub today: String,
    pub yesterday: String,
    pub tomorrow: String,
    pub dow: String,
    pub dow_abbr: String,
    pub year: String,
    pub month: String,
    pub month_name: String,
    pub month_name_abbr: String,
    pub env: HashMap<String, String>,

    // --- New internal backing store ---
    values: serde_json::Map<String, serde_json::Value>,
}
```

### Task 2.2: Add new methods to `ComposeContext`

**File:** `darkmatter/lib/src/markdown/compose/types.rs`

Add:

- `capture_for_dir(base_dir: &Path) -> Self` -- main capture entrypoint using CWD
- `get(key: &str) -> Option<&Value>` -- lookup from the `values` map
- `as_object(&self) -> Value` -- returns `Value::Object(self.values.clone())`
- `keys(&self) -> impl Iterator<Item = &str>` -- iterates exposed key names

Update existing `ComposeContext::capture()` to delegate to `capture_for_dir(std::env::current_dir())`.

Update `ComposeContext::fixed_for_testing()` to also populate the `values` map with legacy fields.

### Task 2.3: Populate `values` with existing fields during capture

**File:** `darkmatter/lib/src/markdown/compose/context/capture.rs`

During `capture_for_dir`, after computing the chrono-based fields, insert them all into `values`:

- `now`, `utc` (aliased as `now_utc`), `today`, `yesterday`, `tomorrow`
- `dow` (aliased as `day`), `dow_abbr` (aliased as `day_abbr`)
- `year`, `month`, `month_name`, `month_name_abbr`

Also populate the new date/time variables from the spec that don't exist yet:

- `today_utc`, `yesterday_utc`, `tomorrow_utc`
- `start_of_week_sun`, `start_of_week_mon`, `end_of_week_sun`, `end_of_week_mon` (plus `_utc` variants)
- `time`, `time_military`, `timezone`, `timezone_offset`
- `day_utc`, `day_abbr_utc`, `year_utc`
- `day_of_month`, `day_of_month_suffixed`
- `season`
- `timestamp`, `timestamp_ms`

### Task 2.4: Compatibility aliases

Ensure these aliases exist in `values` for backward compatibility:

| Legacy name | Canonical name | Both present in values? |
|-------------|---------------|------------------------|
| `utc` | `now_utc` | Yes |
| `dow` | `day` | Yes |
| `dow_abbr` | `day_abbr` | Yes |

### Task 2.5: Unit tests for map-backed context

**File:** `darkmatter/lib/src/markdown/compose/context/capture.rs` (tests module)

Tests:

- `capture_for_dir` produces all legacy fields in `values`
- `get("today")` returns the same value as the `today` public field
- `get("day")` and `get("dow")` return the same value
- `get("now_utc")` and `get("utc")` return the same value
- `as_object()` returns a complete JSON object
- `keys()` includes all expected keys
- New date/time fields (`today_utc`, `start_of_week_sun`, `time`, `timezone`, `timestamp`, `season`, etc.) are populated
- `day_of_month_suffixed` correctly formats ordinal suffixes (1st, 2nd, 3rd, 4th, 11th, 12th, 13th, 21st, etc.)

**Verification:** `just test` in `darkmatter/` passes. All existing tests still pass.

---

## Phase 3: Sniff-powered context capture

**Goal:** Use `sniff` to capture repository, language, document, OS, and hardware information and populate them into the `values` map.

### Task 3.1: Repository and monorepo context

**File:** `darkmatter/lib/src/markdown/compose/context/capture.rs`

Use:

- `sniff::filesystem::git::detect_git(base_dir, false, 10)` -- for git info
- `sniff::filesystem::repo::detect_repo(repo_root_or_base_dir)` -- for repo/monorepo info

Populate these `values` keys:

| Key | Type | Source |
|-----|------|--------|
| `repo` | `String \| null` | `GitInfo.repo` |
| `repo_root` | `String \| null` | `GitInfo.repo_root` (as string) |
| `is_monorepo` | `bool` | `RepoInfo.is_monorepo` |
| `package_root` | `String \| null` | `Package.path` for CWD's package |
| `package_area_root` | `String \| null` | Package area root dir for CWD |
| `packages` | `[String] \| null` | `RepoInfo.packages[*].name` |
| `package_areas` | `[String] \| null` | Unique `RepoInfo.packages[*].package_area` values |
| `current_package` | `String \| null` | `RepoInfo.package_for_dir(cwd).name` |
| `current_package_area` | `String \| null` | `RepoInfo.package_area_for_dir(cwd)` |

**Null rules** (from spec):

- `repo` / `repo_root`: null if not in a git repo
- `is_monorepo`: false if not in a repo
- `package_root`, `package_area_root`, `packages`, `package_areas`, `current_package`, `current_package_area`: null if not in a monorepo

### Task 3.2: Dirty/staged/untracked file context

**File:** `darkmatter/lib/src/markdown/compose/context/capture.rs`

Use `GitInfo.file_changes` (already captured by `detect_git`) to derive changed path sets. Alternatively use `sniff::filesystem::blast_radius::collect_changed_paths` with different `ChangeScope` values:

- `ChangeScope::Dirty` for dirty files
- `ChangeScope::Staged` for staged files
- Untracked = filter `FileStatus::Untracked` from file_changes

Then populate:

| Key | Type | Description |
|-----|------|-------------|
| `dirty_files` | `String` | Comma-separated repo-relative paths |
| `dirty_files_list` | `String` | Markdown bullet list |
| `dirty_source_code_files` | `String` | Comma-separated, source code only |
| `dirty_source_code_files_list` | `String` | Markdown bullet list |
| `staged_files` | `String` | Comma-separated |
| `staged_files_list` | `String` | Markdown bullet list |
| `untracked_files` | `String` | Comma-separated |
| `untracked_files_list` | `String` | Markdown bullet list |

**Formatting rules:**

- Paths are repo-relative, sorted lexicographically
- Comma-separated: `", "` join
- List: `"- item\n"` per entry
- Empty collections: empty string for both variants

### Task 3.3: Package/package-area dirty/staged context

**File:** `darkmatter/lib/src/markdown/compose/context/capture.rs`

Derive by intersecting changed paths with package roots:

| Key | Type |
|-----|------|
| `dirty_packages` | `String` (csv) |
| `dirty_packages_list` | `String` (md list) |
| `dirty_package_areas` | `String` (csv) |
| `dirty_package_areas_list` | `String` (md list) |
| `staged_packages` | `String` (csv) |
| `staged_packages_list` | `String` (md list) |
| `staged_package_areas` | `String` (csv) |
| `staged_package_areas_list` | `String` (md list) |
| `current_package_has_staged_files` | `bool` |
| `current_package_area_has_staged_files` | `bool` |
| `current_package_has_dirty_files` | `bool` |
| `current_package_area_has_dirty_files` | `bool` |

Boolean flags default to `false` if not in a monorepo or CWD not in a package/area.

### Task 3.4: Programming language and package manager context

**File:** `darkmatter/lib/src/markdown/compose/context/capture.rs`

Use `RepoInfo.packages[*].primary_language`, `languages`, and `package_managers`.

| Key | Type | Rules |
|-----|------|-------|
| `programming_languages_in_repo` | `String \| null` | Comma-separated unique languages across all packages; null if not in repo |
| `programming_language` | `String \| null` | See rules below; null if not in repo |
| `package_manager` | `String \| null` | See rules below; null if not in repo |

**`programming_language` rules:**

- Not in repo: null
- In monorepo + in a package: that package's `primary_language`
- In monorepo + in a package area: comma-separated unique primary languages across packages in that area (single if all same)
- Not in monorepo: repo's primary language (most common across packages, or first)

**`package_manager` rules:**

- Not in repo: null
- In monorepo + in package: that package's `package_managers[0]`
- In monorepo + in package area: single answer if all packages agree, else null
- In monorepo + not in package/area: single answer if all packages agree, else null
- Not in monorepo: detected package manager

### Task 3.5: Document context

**File:** `darkmatter/lib/src/markdown/compose/context/capture.rs`

Use `sniff::filesystem::docs::detect_docs(repo_root_or_base_dir)`.

| Key | Type | Description |
|-----|------|-------------|
| `docs_readme` | `String` | Comma-separated README paths, scope-filtered |
| `docs_blast_radius` | `String` | Comma-separated docs with `blast_radius` frontmatter, scope-filtered |
| `docs_drift` | `String` | Comma-separated docs at risk of drift |

**Scope filtering:**

- In package: filter to that package
- In package area: filter to packages in that area
- Otherwise: repo-wide

**`docs_drift` algorithm:**

1. Collect dirty source-code files for active scope
2. Load markdown docs with blast-radius metadata
3. Return docs whose `blast_radius` intersects the dirty source set

### Task 3.6: Skill context

**File:** `darkmatter/lib/src/markdown/compose/context/capture.rs`

This is not covered by `sniff` -- implement locally in darkmatter.

| Key | Type | Description |
|-----|------|-------------|
| `docs_skill` | `String \| null` | Repo-relative path to best matching SKILL.md |

**Discovery:**

- Scan `{repo_root}/.claude/skills/*/SKILL.md`
- Scan `{repo_root}/.agents/skills/*/SKILL.md`

**Matching:**

- In monorepo: prefer skill whose directory name matches the package area or package name
- Not in monorepo: look for skill matching the repo name
- Return repo-relative path to the best match, or null

### Task 3.7: OS and hardware context

**File:** `darkmatter/lib/src/markdown/compose/context/capture.rs`

Use:

- `sniff::os::detect_os()` -> `OsInfo`
- `sniff::hardware::detect_hardware()` -> `HardwareInfo`

| Key | Type | Source |
|-----|------|--------|
| `os` | `String \| null` | Normalize `OsType` to `"Windows"`, `"macOS"`, `"Linux"`, or null |
| `os_distro` | `String` | `OsInfo.distribution` on Linux, empty string on macOS/Windows |
| `os_package_manager` | `String \| null` | `OsInfo.system_package_managers` primary |
| `os_version` | `String` | `OsInfo.version` |
| `memory_total` | `Number` | `MemoryInfo.total_bytes` |
| `memory_used` | `Number` | Percentage: `used_bytes * 100 / total_bytes` |
| `memory_avail` | `Number` | `MemoryInfo.available_bytes` |
| `cpu_cores` | `Number` | `CpuInfo.logical_cores` |
| `cpu_arch` | `String` | `CpuInfo.arch` |
| `gpu` | `String \| null` | GPU device name(s), comma-separated; null if none |

### Task 3.8: Formatting helpers module

**File:** `darkmatter/lib/src/markdown/compose/context/format.rs`

Shared formatting functions used by capture:

- `fn format_csv(items: &[impl AsRef<str>]) -> String` -- joins with `", "`
- `fn format_md_list(items: &[impl AsRef<str>]) -> String` -- joins with `"- item\n"`
- `fn to_repo_relative(path: &Path, repo_root: &Path) -> String`
- `fn ordinal_suffix(n: u32) -> &'static str` -- returns "st", "nd", "rd", "th"
- `fn determine_season(month: u32, day: u32) -> &'static str` -- returns "Spring", "Summer", "Fall", "Winter" based on meteorological seasons

### Task 3.9: Intermediate `ContextCapture` struct

**File:** `darkmatter/lib/src/markdown/compose/context/capture.rs`

To avoid repeated sniff calls, build a single intermediate struct that holds all raw sniff results:

```rust
struct ContextCapture {
    base_dir: PathBuf,
    git_info: Option<GitInfo>,
    repo_info: Option<RepoInfo>,
    docs: Option<Vec<MarkdownMeta>>,
    os_info: Option<OsInfo>,
    hardware_info: Option<HardwareInfo>,
    current_package: Option<Package>,
    current_package_area: Option<String>,
    dirty_paths: Vec<PathBuf>,
    staged_paths: Vec<PathBuf>,
    untracked_paths: Vec<PathBuf>,
}
```

All derived variables in tasks 3.1-3.7 are computed from this single struct.

If any sniff call fails (e.g., not in a git repo, or `current_dir()` fails), record a `PartialRuntimeCapture` diagnostic and set the affected fields to null.

### Task 3.10: Unit tests for sniff-powered capture

**File:** `darkmatter/lib/src/markdown/compose/context/capture.rs` (tests module)

Tests:

- Non-repo directory: repo/monorepo fields are null, OS/hardware fields still populated
- Non-monorepo repo: `is_monorepo` is false, package fields are null
- Formatting helpers: csv, md_list, ordinal suffixes, season determination
- `docs_skill` returns null when no skills directory exists

### Task 3.11: Integration tests for sniff-powered capture

**File:** `darkmatter/lib/tests/compose_context_integration.rs` (or similar)

Use `tempfile` + `git2` to create synthetic repos:

- Plain git repo: verify `repo`, `repo_root`, `is_monorepo=false`
- Monorepo with packages: verify `packages`, `current_package`, `dirty_packages`
- Dirty/staged/untracked file detection
- Package manager and language detection

**Verification:** `just test` in `darkmatter/` passes.

---

## Phase 4: Materialize `ctx` into effective state with merge semantics

**Goal:** Refactor `EffectiveState` so runtime `ctx` is materialized as a real namespace in `data`, replacing the special-case accessor. Add merge diagnostics.

### Task 4.1: Define `ContextMergeDiagnostic`

**File:** `darkmatter/lib/src/markdown/compose/context/diagnostics.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextMergeDiagnostic {
    /// User ctx was an object; merged successfully
    UserCtxMerged { had_key_collisions: bool },
    /// User ctx was not an object; replaced with runtime ctx
    InvalidUserCtxReplaced,
    /// A sniff capture area partially failed
    PartialRuntimeCapture { area: &'static str, detail: String },
}
```

### Task 4.2: Add merge logic

**File:** `darkmatter/lib/src/markdown/compose/context/merge.rs`

```rust
pub struct CtxMergeResult {
    pub merged_ctx: serde_json::Value,
    pub diagnostics: Vec<ContextMergeDiagnostic>,
}

pub fn merge_ctx(
    user_ctx: Option<&serde_json::Value>,
    runtime_ctx: serde_json::Value,
    allow_override: bool,
) -> Result<CtxMergeResult, CtxMergeError>
```

**Merge policy:**

1. No user `ctx`: insert runtime `ctx` directly. No diagnostics.
2. User `ctx` is an object: deep-merge `user_ctx` into `runtime_ctx` (runtime wins on collision). Emit `UserCtxMerged { had_key_collisions }`.
3. User `ctx` is not an object:
   - If `allow_override` is false: return `Err(CtxMergeError::InvalidUserCtx)`
   - If `allow_override` is true: use runtime `ctx`, emit `InvalidUserCtxReplaced`

### Task 4.3: Refactor `EffectiveState` / `EffectiveStateBuilder`

**File:** `darkmatter/lib/src/markdown/compose/state.rs`

Changes to `EffectiveStateBuilder::build()`:

1. After merging frontmatter/external state as today, inspect `data["ctx"]`.
2. Call `merge_ctx(data.get("ctx"), context.as_object(), allow_ctx_override)`.
3. Store the merged result back into `data["ctx"]`.
4. Store diagnostics on `EffectiveState`.

Add to `EffectiveStateBuilder`:

- `with_allow_ctx_override(bool)` method
- Internal field: `allow_ctx_override: bool` (default false)

Add to `EffectiveState`:

- `ctx_diagnostics: Vec<ContextMergeDiagnostic>` field
- `pub fn ctx_diagnostics(&self) -> &[ContextMergeDiagnostic]` accessor

### Task 4.4: Refactor `get_context_value` to use `values` map

**File:** `darkmatter/lib/src/markdown/compose/state.rs`

Now that `ctx` is materialized in `data["ctx"]`, the `get()` method's `ctx.*` branch should first try `data["ctx"][key]`. If found, return it. If not found, fall back to the existing field-by-field match (for safety during transition).

Eventually the field-by-field match can be removed, but keeping it as a fallback ensures nothing breaks.

The `env.*` special case remains for now (env is not stored in `values`).

### Task 4.5: Propagate `allow_ctx_override` through the pipeline

**Files:**

- `darkmatter/lib/src/markdown/compose/types.rs` -- add `allow_ctx_override: bool` to `ComposeOptions`, with builder method `with_allow_ctx_override(bool)`
- `darkmatter/lib/src/markdown/compose/mod.rs` -- pass `options.allow_ctx_override` to the `EffectiveStateBuilder`

### Task 4.6: Convert diagnostics to `ComposeWarning`s

**File:** `darkmatter/lib/src/markdown/compose/mod.rs`

After building `EffectiveState`, iterate `state.ctx_diagnostics()` and convert each to a `ComposeWarning`:

| Diagnostic | Warning message |
|-----------|-----------------|
| `UserCtxMerged { had_key_collisions: false }` | Use the "all keys were preserved" message from the spec |
| `UserCtxMerged { had_key_collisions: true }` | Use the "document keys were overwritten" message from the spec |
| `InvalidUserCtxReplaced` | Document `ctx` was not an object; replaced with runtime context |
| `PartialRuntimeCapture { area, detail }` | Partial runtime capture for `{area}`: `{detail}` |

The relative/absolute filepath should be included per the spec format.

### Task 4.7: Unit tests for merge and state materialization

**File:** `darkmatter/lib/src/markdown/compose/context/merge.rs` (tests module) and `state.rs` (tests module)

Tests:

- No user ctx: runtime ctx inserted, no diagnostics
- User ctx object with no collisions: merged, `had_key_collisions: false`
- User ctx object with collisions: merged, runtime wins, `had_key_collisions: true`
- User ctx is a string: error by default
- User ctx is a string + allow_override: warning, runtime ctx used
- User ctx is an array: same error/warning behavior
- User ctx is null: treated as "no user ctx"
- `EffectiveState.get("ctx.today")` works via data lookup after materialization
- `EffectiveState.get("ctx.repo")` works for new sniff-derived fields
- `EffectiveState.get("ctx.day")` and `get("ctx.dow")` return same value (alias)

**Verification:** `just test` in `darkmatter/` passes. All existing interpolation tests still pass.

---

## Phase 5: CLI integration

**Goal:** Add `--allow-ctx-override` flag, propagate it, and print compose warnings to stderr.

### Task 5.1: Add `--allow-ctx-override` CLI flag

**File:** `darkmatter/cli/src/args.rs`

Add to the `Compose` variant:

```rust
/// Allow non-object ctx frontmatter (downgrade error to warning)
#[arg(long)]
allow_ctx_override: bool,
```

### Task 5.2: Propagate flag to `ComposeOptions`

**File:** `darkmatter/cli/src/commands.rs`

In the compose command handler, after building options:

```rust
options = options.with_allow_ctx_override(allow_ctx_override);
```

### Task 5.3: Print compose warnings to stderr

**File:** `darkmatter/cli/src/commands.rs`

After `md.compose_with(options)` returns `(composed, report)`:

- Iterate `report.warnings`
- Print each to stderr using `biscuit-terminal`'s `Status` formatting with warning state
- This fixes the existing gap where warnings were silently discarded (`_report`)

### Task 5.4: CLI integration tests

Tests (can be integration tests or manual verification):

- `md compose doc.md` with document defining `ctx` as object: prints merge warning
- `md compose doc.md` with document defining `ctx: "hello"`: returns error
- `md compose doc.md --allow-ctx-override` with `ctx: "hello"`: prints warning, succeeds
- Compose warnings (from any source) are now printed to stderr

**Verification:** `just test` in `darkmatter/` passes.

---

## Phase 6: Cache hashing update

**Goal:** Update `context_hash()` to hash the full normalized context object instead of the hard-coded subset.

### Task 6.1: Update `context_hash`

**File:** `darkmatter/lib/src/markdown/compose/cache/hashing.rs`

Replace the current implementation that only hashes `today`, `yesterday`, `tomorrow`, and env vars with:

```rust
pub(crate) fn context_hash(ctx: &ComposeContext) -> u64 {
    // Hash the full normalized values map (excludes env, which is hashed separately below)
    let mut values_without_env = ctx.values.clone();
    // Remove volatile fields that change every second
    values_without_env.remove("now");
    values_without_env.remove("now_utc");
    values_without_env.remove("utc");
    values_without_env.remove("time");
    values_without_env.remove("time_military");
    values_without_env.remove("timestamp");
    values_without_env.remove("timestamp_ms");
    // Remove memory_used (volatile percentage)
    values_without_env.remove("memory_used");
    values_without_env.remove("memory_avail");

    let canonical = canonical_json_sorted(&Value::Object(values_without_env));
    let mut parts = vec![canonical];

    // Sort env vars for determinism (kept separate)
    let mut env_pairs: Vec<_> = ctx.env.iter().collect();
    env_pairs.sort_by_key(|(k, _)| *k);
    for (k, v) in env_pairs {
        parts.push(format!("env.{}={}", k, v));
    }

    xx_hash(&parts.join("\0"))
}
```

**Key decisions:**

- Volatile per-second fields (`now`, `utc`, `time`, `timestamp`, `timestamp_ms`) excluded from hash
- Volatile system state fields (`memory_used`, `memory_avail`) excluded from hash
- All other output-affecting values (dates, repo, packages, OS, hardware) included
- `env` remains hashed separately for clarity

### Task 6.2: Unit tests for updated hashing

**File:** `darkmatter/lib/src/markdown/compose/cache/hashing.rs` (tests module)

Tests:

- Context hash changes when `today` changes (existing behavior preserved)
- Context hash changes when `repo` changes
- Context hash changes when `os` changes
- Context hash does NOT change when only `now` or `timestamp` changes
- Context hash is deterministic for the same inputs

**Verification:** `just test` in `darkmatter/` passes.

---

## Phase 7: Documentation updates

**Goal:** Update all affected documentation in the same change.

### Task 7.1: Update context-variables.md

**File:** `darkmatter/docs/topics/context-variables.md`

The existing content is comprehensive but needs updates:

- Add formal type annotations for each variable
- Mark compatibility aliases
- Add the new `gpu` variable (from hardware section)
- Ensure all null/empty-string rules are documented

### Task 7.2: Update interpolation docs

**File:** `darkmatter/docs/inline/interpolation.md`

- Document that `ctx.*` now resolves from a materialized namespace in state
- Document the merge behavior when documents define `ctx`
- Document `--allow-ctx-override`

### Task 7.3: Update CLI compose docs

**File:** `darkmatter/docs/cli/compose.md`

- Document `--allow-ctx-override` flag
- Document that compose warnings are now printed to stderr

### Task 7.4: Update dependencies doc

**File:** `docs/dependencies.md`

- Record the new `sniff` dependency from `darkmatter/lib`

### Task 7.5: Update lib README if needed

**File:** `darkmatter/lib/README.md`

- Note the new `context/` module under compose
- Note the `sniff` dependency

---

## Phase 8: Regression and end-to-end testing

**Goal:** Ensure nothing breaks and the full feature works end-to-end.

### Task 8.1: Regression tests for existing interpolation

Verify these existing patterns still work unchanged:

- `{{ ctx.today }}` resolves to a date string
- `{{ ctx.year }}` resolves to a year string
- `{{ ctx.dow }}` resolves to a day-of-week name
- `{{ env.HOME }}` resolves to the HOME env var
- `{{ ctx.month_name }}` resolves to a month name

### Task 8.2: Alias regression tests

Add tests for:

- `{{ ctx.day }}` returns same as `{{ ctx.dow }}`
- `{{ ctx.day_abbr }}` returns same as `{{ ctx.dow_abbr }}`
- `{{ ctx.now_utc }}` returns same as `{{ ctx.utc }}`

### Task 8.3: End-to-end compose tests

Test full compose pipeline with documents that use new variables:

- Document using `{{ ctx.repo }}` in a git repo
- Document using `{{ ctx.os }}` and `{{ ctx.cpu_arch }}`
- Document using `{{ ctx.is_monorepo }}` in a ternary expression
- Parent-child transclusion reuses the same context (dates match)

### Task 8.4: Integration test with synthetic monorepo

Use `tempfile` + `git2` to create a monorepo with:

- Two packages in different package areas
- Some dirty and staged files
- A README in one package

Verify:

- `ctx.current_package` matches when CWD is in a package
- `ctx.dirty_packages` lists the correct packages
- `ctx.docs_readme` is scope-filtered correctly
- `ctx.programming_language` resolves correctly for the package

---

## Execution Order and Dependencies

```
Phase 1 (skeleton + dependency)
    |
Phase 2 (map-backed ComposeContext)
    |
Phase 3 (sniff capture) ─── can start as soon as Phase 2 values map is in place
    |
Phase 4 (state materialization + merge) ─── depends on Phase 2 + 3
    |
Phase 5 (CLI) ─── depends on Phase 4
    |
Phase 6 (cache hashing) ─── depends on Phase 2 (values map), can parallel with 4-5
    |
Phase 7 (docs) ─── depends on all code phases being stable
    |
Phase 8 (regression + e2e) ─── depends on all phases
```

**Parallelization opportunities:**

- Phase 6 (cache hashing) can be done in parallel with Phase 4-5 since it only depends on the `values` map from Phase 2
- Task 3.7 (OS/hardware) is independent of tasks 3.1-3.6 (repo/docs) and can be developed in parallel
- Documentation (Phase 7) can start drafts alongside Phase 4-5

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| `sniff` calls slow down compose | Capture once in `ContextCapture`; reuse across all derived fields. Use non-deep git detection. |
| `current_dir()` fails | Record `PartialRuntimeCapture` diagnostic; set sniff-derived fields to null. Compose still succeeds. |
| Breaking existing `ctx.*` interpolation | Keep legacy public fields on `ComposeContext`. Fallback in `get_context_value`. Regression tests. |
| Cache invalidation too aggressive | Exclude volatile fields (`now`, `timestamp`, `memory_used`) from hash. |
| `sniff` API changes | Pin to path dependency. Types are local to the monorepo and under our control. |

---

## Open Questions (from tech design, to resolve during implementation)

1. **`docs_skill`** - The matching heuristic (directory name matches package/area/repo name) should be confirmed. Current plan implements the proposed behavior.
2. **Memory values** - Using raw numbers (bytes for total/avail, percentage for used). If human-readable strings needed later, add `_human` variants.
3. **`programming_language`** - Comma-separated string in mixed contexts. A parallel `programming_languages` array may be added later.
4. **Warning message formatting** - The spec uses HTML-like markup tags (`<blue-500>`, `<inverse>`, etc.). These are `biscuit-terminal` `Prose` markup and should be rendered via `Status` in the CLI layer, not in the library.
