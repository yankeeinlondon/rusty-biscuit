# Sniff Client Optimization Suggestions

Generated from client audit following the ergonomics & performance refactoring (Phase 5 of the implementation plan).

---

## Sniff CLI

### Current State

- Uses `SniffConfig` with `OutputFilter`-based skip flag chains (`commands.rs:403-424`)
- Calls `ProgramsInfo::detect()` for all 8 categories even when only 1 is needed (`commands.rs:72`)
- Calls `detect_services()` separately (already efficient, only when `Commands::Services` is requested)
- Dependency enrichment is already conditional on `--latest-versions` flag (`commands.rs:524-526`)
- `deep(true)` only set when `--refresh-remotes` is enabled (`commands.rs:389`)

### Suggested Changes

1. **HIGH: Replace OutputFilter skip chains with DetectionPlan** (`commands.rs:403-424`)
   - Create a helper `fn output_filter_to_plan(filter: &OutputFilter, ...) -> DetectionPlan`
   - Benefits: per-subsection control instead of coarse domain skips
   - Example: `sniff cpu` currently fetches full hardware (storage, GPU, audio) but only uses CPU. With `HardwareRequest::summary()`, it skips ~1.5s of audio detection on macOS.

2. **HIGH: Granular hardware subsection requests** (`commands.rs:416-421`)
   - `OutputFilter::Cpu` should use `HardwareRequest { include_storage: false, include_gpu: false, include_audio: false }`
   - `OutputFilter::Gpu` should use `HardwareRequest { include_storage: false, include_gpu: true, include_audio: false }`
   - `OutputFilter::Storage` should use `HardwareRequest { include_storage: true, include_gpu: false, include_audio: false }`

3. **HIGH: Granular filesystem subsection requests** (`commands.rs:423-425`)
   - `OutputFilter::Docs` should use `FilesystemRequest::new().without_file_inventory().without_formatting()` and potentially skip git/repo
   - `OutputFilter::Language` should use `FilesystemRequest::new().without_docs().without_formatting()`
   - `OutputFilter::Repo` should skip file inventory: `FilesystemRequest::new().without_file_inventory()`

4. **MEDIUM: Category-specific program detection** (`commands.rs:72`)
   - When `OutputFilter::Editors`, only construct `InstalledEditors::new()` instead of all 8 categories
   - Could add a `ProgramsInfo::detect_category(category)` or just call the specific `Installed*::new()` directly
   - Saves ~7/8 of program detection time for single-category queries

5. **LOW: Migrate SniffConfig to detect_with_plan** (`commands.rs:372-447`)
   - Replace `SniffConfig::new()` + skip chains with `DetectionPlan::new()` + targeted requests
   - No behavior change needed; cleaner expression of intent

---

## Darkmatter Library

### Current State

- Demand-driven context capture: scans markdown for `ctx.*` references, only captures needed groups (`capture.rs:546-561`)
- Uses module-level functions: `GitRepo::discover()`, `detect_repo_structure()`, `detect_hardware_summary()`, `detect_gpus()` (`capture.rs:277-398`)
- Already uses `detect_docs_with_packages()` for optimized doc detection (`capture.rs:398`)
- Parallel execution via `std::thread::scope` for independent captures (`capture.rs:302-446`)

### Suggested Changes

1. **MEDIUM: Skip locale/time in OS detection** (`capture.rs:333`)
   - Currently calls `os::detect_os()` which includes locale and timezone/NTP
   - Darkmatter only uses `os_type`, `distribution`, `version`, and `package_managers`
   - Use `os::detect_os_with_request(&OsRequest::full().include_locale(false).include_time(false))`
   - Saves up to 10s on Linux (NTP timedatectl timeout)

2. **LOW: GPU-only hardware request**
   - Currently calls `detect_gpus()` as a separate module function (`capture.rs:354`)
   - This is already optimal since `detect_gpus()` is cheap and standalone
   - No change needed - this is a good example of Tier 3 (module-level) API usage

3. **NO CHANGE: detect_hardware_summary()**
   - Already maps cleanly to `HardwareRequest::summary()` (`capture.rs:343`)
   - No migration needed unless darkmatter wants to consolidate calls into a single `detect_with_plan()`

4. **NO CHANGE: detect_repo_structure()**
   - Already equivalent to `RepoRequest::structure()` (`capture.rs:323`)
   - Demand-driven architecture means darkmatter benefits more from Tier 3 module-level calls than from `detect_with_plan()`

---

## Claudine Library

### Current State

- Two detection modes: standard and fast (`environment.rs:295-339`)
- Standard: `deep(false), commit_count(1), skip_network()` - for session startup
- Fast: `deep(false), commit_count(0), skip_os, skip_hardware, skip_network` - for session hooks
- Direct module calls in `system_prompt/context.rs:32-38`: `detect_git()` + `detect_repo()` separately
- AI client detection via `InstalledAiClients::new()` (already parallelized) (`config/mod.rs:78-80`)

### Suggested Changes

1. **MEDIUM: Migrate standard mode to DetectionPlan** (`environment.rs:295-312`)
   ```rust
   let plan = DetectionPlan::new()
       .os(OsRequest::full())
       .hardware(HardwareRequest::full())
       .without_network()
       .filesystem(FilesystemRequest::new()
           .git(GitRequest::full().commit_count(1)));
   let result = detect_with_plan(plan)?;
   ```
   - Clearer intent, enables future granular optimization
   - Could further optimize with `HardwareRequest::summary()` since claudine only uses arch, cores, and memory bytes

2. **HIGH: Optimize fast mode with targeted filesystem request** (`environment.rs:320-339`)
   ```rust
   let plan = DetectionPlan::new()
       .without_os()
       .without_hardware()
       .without_network()
       .filesystem(FilesystemRequest::new()
           .git(GitRequest::summary())
           .repo(RepoRequest::structure())
           .without_file_inventory()
           .without_docs()
           .without_formatting());
   ```
   - Currently does full filesystem detection but only uses git + repo
   - Skipping file inventory, docs, and formatting saves 100-500ms per hook invocation
   - Hook latency is user-facing, so even small savings matter

3. **MEDIUM: Consolidate system_prompt/context.rs calls** (`context.rs:32-38`)
   - Currently calls `detect_git(cwd, false, 1)` and `detect_repo(&root)` separately
   - Could use single `detect_with_plan()` call with appropriate `FilesystemRequest`
   - Benefit: unified control, explicit about what's needed

4. **LOW: Hardware summary for standard mode** (`environment.rs:295-312`)
   - Claudine only extracts: arch, cpu brand, cores, memory_bytes, memory_available_bytes
   - `HardwareRequest::summary()` provides exactly this (CPU + memory, no storage/GPU/audio)
   - Saves ~1.5s on macOS (audio device enumeration)

---

## Playa

### Current State

- Uses `HeadlessAudio` enum for metadata (binary name, display name, website)
- Uses `InstalledHeadlessAudio::new()` for runtime detection of available audio players (`player.rs:468`)

### Suggested Changes

- **No changes needed.** Playa's usage is already optimal:
  - Metadata access is pure lookup (no detection overhead)
  - Runtime detection is correctly scoped to a single program category
  - Does not benefit from `DetectionPlan` since it only needs program metadata

---

## Unchained-AI

### Current State

- Uses `InstalledAiClients::new()` in `agent_status.rs:124` to detect installed AI CLI platforms
- Checks for Claude Code and Codex CLI installation

### Suggested Changes

- **No changes needed.** Detection is appropriate and already parallelized via rayon.

---

## Research

### Current State

- Uses `LanguagePackageManager` enum for data modeling (no runtime detection)
- Used in: `topic.rs:5`, `db/rows.rs:16`, `migration_v2.rs:38`, `db/inventory.rs:509` (test)

### Suggested Changes

1. **LOW: Remove stale import** (`content_policy.rs:2`)
   - `use sniff::package::LanguagePackageManager;` is imported but never used in this file
   - Remove to clean up

---

## Priority Summary

| Priority | Client | Change | Estimated Savings |
|----------|--------|--------|-------------------|
| **HIGH** | sniff-cli | Granular hardware subsection requests for single-topic queries | ~1.5s on macOS |
| **HIGH** | claudine | Optimize fast mode filesystem request (skip inventory/docs/formatting) | 100-500ms per hook |
| **HIGH** | sniff-cli | Granular filesystem subsection requests | 50-500ms |
| **MEDIUM** | sniff-cli | Category-specific program detection | ~200-700ms |
| **MEDIUM** | darkmatter | Skip locale/time in OS detection | up to 10s on Linux |
| **MEDIUM** | claudine | Migrate standard mode to DetectionPlan | Cleaner code + future optimization |
| **MEDIUM** | claudine | Consolidate system_prompt direct calls | Cleaner code |
| **LOW** | sniff-cli | Migrate SniffConfig to detect_with_plan | Code clarity |
| **LOW** | claudine | Use HardwareRequest::summary() for standard mode | ~1.5s on macOS |
| **LOW** | research | Remove stale import | Code cleanup |
