# Technical Design: More Context Variables

## Summary

This feature expands Darkmatter's compose-time `ctx` namespace from a small date/time snapshot into a larger runtime context assembled from:

- existing local clock and environment capture
- repository and monorepo inspection via `sniff`
- markdown document discovery via `sniff`
- operating system and hardware inspection via `sniff`

The new context remains deterministic for a single compose run by being captured once and reused across the entire transclusion graph.

The design also formalizes how user-defined `ctx` frontmatter is merged with Darkmatter's runtime `ctx`, adds the `--allow-ctx-override` CLI escape hatch, and ensures cache keys incorporate any new output-affecting context values.

## Goals

- Add `sniff` as a `darkmatter` library dependency and use it to enrich compose-time context.
- Capture context once per compose run and reuse it across all recursively composed documents.
- Preserve the existing `ctx.*` interpolation model.
- Merge user-defined `ctx` objects with runtime `ctx` objects, with runtime values winning on collisions.
- Fail by default when a document defines `ctx` as a non-object, with CLI opt-in to downgrade that failure to a warning.
- Surface context-merge warnings in the CLI instead of silently discarding them.
- Keep the implementation maintainable as the variable catalog grows.

## Non-Goals

- Changing interpolation grammar or expression semantics.
- Adding remote or network-backed context capture.
- Reworking render-time behavior in `biscuit-terminal`.
- Perfecting every possible `sniff`-derived data point in the first pass if the source library does not expose it cleanly.

## Current State

Today Darkmatter has:

- `ComposeContext` in [darkmatter/lib/src/markdown/compose/types.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/types.rs) with a small fixed set of date/time fields plus `env`.
- `EffectiveState` in [darkmatter/lib/src/markdown/compose/state.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/state.rs) that special-cases `ctx.*` and `env.*`.
- cache hashing in [darkmatter/lib/src/markdown/compose/cache/hashing.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/cache/hashing.rs) that only hashes `today`, `yesterday`, `tomorrow`, and sorted env vars.
- CLI compose wiring in [darkmatter/cli/src/commands.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/src/commands.rs) and [darkmatter/cli/src/args.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/src/args.rs).
- a documented future-state variable catalog in [darkmatter/docs/topics/context-variables.md](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/docs/topics/context-variables.md).

Current limitations:

- `ctx` is hard-coded field-by-field and does not scale.
- a document-level `ctx` object is not merged into runtime context today.
- invalid `ctx` overrides are not validated.
- compose warnings are not printed by the CLI.
- cache hashing does not know about any future repo/OS/hardware fields.

## Proposed Architecture

### 1. Add `sniff` to `darkmatter/lib`

Add a path dependency in [darkmatter/lib/Cargo.toml](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/Cargo.toml):

```toml
sniff = { path = "../../sniff/lib" }
```

No network feature is required for this feature.

### 2. Split context capture into a dedicated builder

Introduce a new internal module:

- `darkmatter/lib/src/markdown/compose/context/mod.rs`

Supporting files:

- `capture.rs`
- `format.rs`
- `merge.rs`
- `diagnostics.rs`

Responsibilities:

- capture raw runtime facts from `chrono`, `std::env`, `std::env::current_dir`, and `sniff`
- normalize them into a stable JSON object for `ctx`
- create human-friendly string/list variants
- generate merge diagnostics when document state already defines `ctx`

This keeps `types.rs` from becoming the dumping ground for formatting logic.

### 3. Keep `ComposeContext`, but make it map-backed

`ComposeContext` should remain the public runtime-context type, but stop being the authoritative storage format for every individual key.

Recommended shape:

```rust
pub struct ComposeContext {
    // Existing public fields kept for compatibility
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

    // New internal backing store
    values: serde_json::Map<String, serde_json::Value>,
}
```

Add methods:

- `ComposeContext::capture()` delegates to `capture_for_dir(current_dir)`
- `ComposeContext::capture_for_dir(base_dir: &Path) -> Self`
- `ComposeContext::get(key: &str) -> Option<Value>`
- `ComposeContext::as_object() -> Value`
- `ComposeContext::keys() -> impl Iterator<Item = &str>`

Rationale:

- preserves the current public API surface
- avoids a giant `match` statement in `get_context_value()`
- allows new context variables to be added by inserting into a map
- keeps cache hashing straightforward by hashing normalized values

### 4. Capture based on process CWD, not input file location

The spec explicitly says some values depend on the current working directory. The context base directory must therefore be:

- the shell CWD when `md compose` starts
- not the parent directory of the input document
- not recomputed for transcluded children

Library behavior:

- `ComposeOptions::new()` captures context using `std::env::current_dir()`
- if `current_dir()` fails, capture still succeeds with `sniff`-derived fields set to `null` and a warning recorded

This preserves existing "capture once at options creation" behavior while extending the data gathered.

### 5. Materialize `ctx` into effective state

Refactor `EffectiveState` so `ctx` becomes a real namespace in `data`, instead of only being resolved via a special-case accessor.

New merge flow:

1. Merge frontmatter, `--state`, and `--set` exactly as today.
2. Build runtime `ctx_runtime` from `ComposeContext::as_object()`.
3. Inspect any existing root-level `ctx` value in the merged state.
4. Apply merge policy:
   - if no user `ctx`: insert runtime `ctx`
   - if user `ctx` is an object: deep-merge `user_ctx` with `ctx_runtime`, with runtime values winning
   - if user `ctx` is not an object:
     - error by default
     - if `allow_ctx_override` is enabled, warn and replace it with runtime `ctx`
5. Store the merged object back under `data["ctx"]`

After this change:

- `ctx.foo` is resolved through the normal nested JSON lookup path
- only `env.*` remains special-cased

### 6. Add explicit context merge diagnostics

Add a small diagnostic model for state-building outcomes:

```rust
pub enum ContextMergeDiagnostic {
    UserCtxMerged { had_key_collisions: bool },
    InvalidUserCtxReplaced,
    PartialRuntimeCapture { area: &'static str, detail: String },
}
```

`EffectiveState` should carry these diagnostics so compose can convert them into `ComposeWarning`s and the CLI can print them.

This avoids encoding behavior only in free-form strings.

### 7. Add `allow_ctx_override` to compose options and CLI

Add to `ComposeOptions`:

```rust
pub allow_ctx_override: bool
```

Builder:

- `with_allow_ctx_override(bool)`

CLI flag in [darkmatter/cli/src/args.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/src/args.rs):

- `--allow-ctx-override`

CLI wiring in [darkmatter/cli/src/commands.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/src/commands.rs):

- propagate the flag into `ComposeOptions`

Behavior:

- default: invalid document `ctx` is a hard compose error
- with flag: emit warning and continue with runtime `ctx`

## Context Variable Model

### Value types

Use these conventions consistently:

- date/time literals: JSON strings
- booleans: JSON booleans
- numeric counters or timestamps: JSON numbers
- collections such as `packages` or `docs_readme`: JSON arrays of strings
- absent values: JSON null
- `*_list` helpers: markdown list strings
- `*_csv` is not introduced unless explicitly needed; use the names already in the spec such as `dirty_files`

### Compatibility aliases

The documented catalog introduces names that differ from current runtime names. Keep aliases for compatibility:

- `ctx.utc` remains as an alias of `ctx.now_utc`
- `ctx.dow` remains as an alias of `ctx.day`
- `ctx.dow_abbr` remains as an alias of `ctx.day_abbr`

Canonical docs should move toward:

- `now_utc`
- `day`
- `day_abbr`

### Formatting policy

List-like string helpers should use repo-relative or cwd-relative display paths, sorted lexicographically, joined with:

- `", "` for comma-separated variants
- `"- item"` markdown bullets for `_list` variants

Empty-list behavior:

- structured list fields: `[]`
- string/list helper fields: empty string

Null behavior:

- values that are conceptually "unknown/not applicable" remain `null`
- example: `repo_root`, `package_root`, `package_manager`

## Sniff Integration

### Repository and monorepo context

Use:

- `sniff::filesystem::git::detect_git(base_dir, false, 10)`
- `sniff::filesystem::repo::detect_repo(repo_root_or_base_dir)`
- `sniff::filesystem::blast_radius::collect_changed_paths(...)`

Derived variables:

- `repo`
- `repo_root`
- `is_monorepo`
- `package_root`
- `package_area_root`
- `packages`
- `package_areas`
- `current_package`
- `current_package_area`
- `dirty_files`
- `dirty_files_list`
- `dirty_source_code_files`
- `dirty_source_code_files_list`
- `staged_files`
- `staged_files_list`
- `untracked_files`
- `untracked_files_list`
- `dirty_packages`
- `dirty_packages_list`
- `dirty_package_areas`
- `dirty_package_areas_list`
- `staged_packages`
- `staged_packages_list`
- `staged_package_areas`
- `staged_package_areas_list`
- `current_package_has_staged_files`
- `current_package_area_has_staged_files`
- `current_package_has_dirty_files`
- `current_package_area_has_dirty_files`

Implementation notes:

- package names come from `RepoInfo.packages[*].name`
- package area names come from `RepoInfo.packages[*].package_area`
- directory roots come from `RepoInfo.root`, `Package.path`, and package-area grouping
- dirty/staged/untracked paths come from git status plus `collect_changed_paths`
- package and package-area "dirty" membership is derived by intersecting changed paths with package roots

### Programming language and package manager context

Use `RepoInfo.packages[*].primary_language`, `languages`, and `package_managers`.

Derived variables:

- `programming_languages_in_repo`
- `programming_language`
- `package_manager`

Rules:

- preserve the rules already written in [darkmatter/docs/topics/context-variables.md](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/docs/topics/context-variables.md)
- when multiple package managers apply in scope and there is no single answer, return `null`
- when multiple languages apply in scope, return a comma-separated string of unique language names for `programming_language`

### Document context

Use:

- `sniff::filesystem::docs::detect_docs(base_dir_or_repo_root)`
- `sniff::filesystem::blast_radius` for drift detection

Derived variables:

- `docs_readme`
- `docs_blast_radius`
- `docs_drift`

Scope rules:

- if in a package: filter docs to that package
- else if in a package area: filter docs to packages in that area
- else return repo-wide results

`docs_drift` algorithm:

1. collect dirty source-code files for the active scope
2. load markdown docs with blast-radius metadata
3. return docs whose `blast_radius` intersects the changed source set

### Skill context

`sniff` does not currently provide skill inventory. Implement this locally in Darkmatter.

Discovery:

- scan `{repo_root}/.claude/skills/*/SKILL.md`
- scan `{repo_root}/.agents/skills/*/SKILL.md`

Derived variable:

- `docs_skill`

Proposed behavior:

- when in a monorepo package, prefer a skill whose directory name matches the package area or package
- when in a non-monorepo repo, look for a skill matching the repo name
- return a repo-relative path string to the best matching `SKILL.md`
- return `null` if nothing matches

This is the one major part of the spec not naturally covered by `sniff`.

### OS and hardware context

Use:

- `sniff::os::detect_os()`
- `sniff::hardware::detect_hardware()`

Derived variables from the current spec/docs:

- `os`
- `os_distro`
- `os_package_manager`
- `os_version`
- `memory_total`
- `memory_used`
- `memory_avail`
- `cpu_cores`
- `cpu_arch`

Formatting:

- `os` should normalize to `Windows`, `macOS`, `Linux`, or `null`
- `os_distro` should be an empty string on macOS/Windows and distro name on Linux
- memory values should be strings with human-readable binary units to make direct interpolation readable
- `memory_used` should be a numeric percentage string or number; prefer number if interpolation truthiness and comparison matter

Recommendation:

- use numbers for raw values where possible
- if human-readable strings are needed later, add parallel `_human` variables instead of losing structured comparisons

## Compose Pipeline Integration

### Root compose

No stage ordering changes are needed. The work happens before interpolation/page-block evaluation by enriching `EffectiveState`.

Updated flow:

1. `ComposeOptions::new()` captures `ComposeContext`
2. `run_compose_pipeline_internal()` builds `EffectiveState`
3. `EffectiveStateBuilder` merges runtime `ctx` into root state
4. interpolation/page-blocks/transclusion conditions consume the enriched state
5. child transclusions reuse `options.context().clone()`

### Child transclusions

Do not recapture context inside recursive compose calls.

This is already aligned with the current code path in [darkmatter/lib/src/markdown/compose/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs), which passes the same context forward.

## Diagnostics and CLI Output

### Library behavior

When a document defines `ctx`:

- object with no collisions:
  - add warning with code like `ctx_merged`
- object with collisions:
  - add warning with code like `ctx_merged_overwritten`
- non-object and override disabled:
  - return compose error
- non-object and override enabled:
  - add warning with code like `ctx_override_replaced`

### CLI behavior

The CLI currently discards compose warnings. This feature should change that.

Implementation:

- after `compose_with(options)` returns `(composed, report)`, print `report.warnings` to stderr
- use `biscuit-terminal` `Status` formatting for warning/error presentation
- emit the specific user-facing messages described in [darkmatter/docs/topics/context-variables.md](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/docs/topics/context-variables.md)

This change is useful beyond this feature because it fixes an already documented gap in compose output.

## Cache Hashing

`context_hash()` must be updated to hash the normalized context object, not the current hard-coded subset.

Recommended strategy:

1. convert `ComposeContext.values` to canonical sorted JSON
2. hash that canonical JSON
3. keep `env` out of `values` if it is already hashed separately, or include a normalized `env` object once and remove duplicate handling

Preferred approach:

- include all output-affecting `ctx` values in `values`
- keep `env` hashed separately only if it remains excluded from `values`

Important:

- do not hash unstable diagnostics or transient failure messages
- only hash the values exposed to interpolation

## Performance

`sniff` calls are more expensive than the current clock snapshot, but the cost is acceptable because:

- capture happens once per compose run
- results are reused across the entire transclusion graph
- most `sniff` filesystem queries are local and non-networked

Mitigations:

- use non-deep git detection
- avoid repeated document scans for multiple derived fields by building one intermediate `ContextCapture`
- compute dirty/staged/untracked path sets once, then derive package/package-area/document projections from those sets

## Testing Plan

### Unit tests

Add tests in:

- `darkmatter/lib/src/markdown/compose/context/*`
- `darkmatter/lib/src/markdown/compose/state.rs`
- `darkmatter/lib/src/markdown/compose/cache/hashing.rs`

Cases:

- capture produces existing legacy aliases
- object `ctx` merges without collisions
- object `ctx` merges with collisions and runtime wins
- scalar/list/string `ctx` errors by default
- scalar/list/string `ctx` warns and continues when override is enabled
- null/non-repo/non-monorepo fallbacks
- list helpers format deterministically
- context hash changes when any exposed ctx value changes

### Integration tests

Use temp git repos and small synthetic monorepos to verify:

- `repo_root`, package, and package-area detection
- dirty/staged/untracked projections
- document-scoped `docs_readme` and `docs_drift`
- reuse of one captured context across parent/child transclusions
- CLI `--allow-ctx-override`

### Regression tests

Keep existing interpolation tests passing for:

- `ctx.today`
- `ctx.year`
- `ctx.dow`
- `env.HOME`

Add alias tests for:

- `ctx.day`
- `ctx.day_abbr`
- `ctx.now_utc`

## Documentation Updates

Update in the same change:

- [darkmatter/docs/topics/context-variables.md](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/docs/topics/context-variables.md)
- [darkmatter/docs/inline/interpolation.md](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/docs/inline/interpolation.md)
- [darkmatter/docs/cli/compose.md](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/docs/cli/compose.md)
- [darkmatter/lib/README.md](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/README.md) if public API details change
- [docs/dependencies.md](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/docs/dependencies.md) to record the new `sniff` dependency

## Rollout Plan

### Phase 1

- add `sniff` dependency
- add map-backed `ComposeContext`
- implement capture for date/time, repo/monorepo, docs, OS, hardware
- merge runtime `ctx` into effective state
- add `--allow-ctx-override`
- print compose warnings in CLI

### Phase 2

- refine ambiguous fields such as `docs_skill`
- add more human-readable helper aliases if needed
- consider exposing `env` as a normal object namespace in state as a later cleanup

## Open Questions

1. `docs_skill` is underspecified in the current spec. The design above proposes returning the best matching `SKILL.md` path, but this should be explicitly confirmed.
2. `memory_total`, `memory_avail`, and `memory_used` need a final type decision. Numbers are better for expressions; human-readable strings are better for prose.
3. `programming_language` is documented as a comma-separated string in mixed contexts. If richer downstream use is desired, a parallel structured variable may be worth adding later.
4. The context docs mention status output with HTML-like formatting. The final warning renderer should confirm whether those exact strings should live in the library or only in the CLI formatter.

## Recommended Decisions

- Use a map-backed `ComposeContext` while keeping legacy public fields for compatibility.
- Materialize merged runtime `ctx` into `EffectiveState.data`.
- Add structured diagnostics for `ctx` merge outcomes.
- Capture context from process CWD exactly once per compose run.
- Hash the full normalized exposed context set for cache correctness.
- Fix CLI warning suppression as part of this feature instead of treating it as follow-up work.
