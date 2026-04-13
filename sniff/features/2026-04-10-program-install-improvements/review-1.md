# Implementation Review: Program Install Improvements

- **Date:** 2026-04-10
- **Reviewer:** Gemini CLI
- **Status:** Needs Fixes

## Summary

The core of the program install improvements—specifically the transition to a reasoned `InstallPlan` and the new `HostCapabilities` model—has been implemented with strong architectural alignment to the tech design. However, there are critical functional bugs in the execution path for RemoteBash methods, performance regressions in host detection, and missing cache validation logic.

## Critical Bugs

### 1. RemoteBash Execution is Broken
The tech design explicitly states that RemoteBash should be selectable in the plan and executable after explicit user confirmation.
- **Problem:** `sniff/lib/src/programs/installer.rs::build_install_command` still contains a hard-coded error for `InstallationMethod::RemoteBash`, stating it is "NOT SUPPORTED for security reasons".
- **Impact:** Even when a user confirms the remote-bash installation in the CLI, the library call to `execute()` will fail.
- **Dry-run Impact:** `InstallPlan::execute` with `dry_run: true` also fails for RemoteBash because it calls `build_install_command` before checking the dry-run flag.

## Functional Gaps

### 1. Missing Cache Hostname Validation
The tech design requires that a "host mismatch invalidates the cache."
- **Problem:** `sniff/lib/src/programs/host_capability.rs::load_host_capabilities_from` loads the `HostCapabilityCacheFile` but never compares the `hostname` field in the envelope with the current system's hostname.
- **Impact:** A cache file moved between machines (e.g., in a synced home directory) will be blindly trusted, leading to incorrect plans.

### 2. Inconsistent RemoteBash Availability Logic
- **Problem:** `sniff/lib/src/programs/installer.rs::method_available` explicitly returns `false` for RemoteBash.
- **Workaround:** `types.rs` and `install_plan.rs` both have to manually add `|| (method.is_remote_bash() && host.has_bash)` to their availability checks.
- **Recommendation:** `method_available` should be updated to accept `HostCapabilities` or at least a `has_bash` boolean so the logic is centralized.

## Performance Issues

### 1. Sequential Verification Probes
The tech design explicitly requires: "run probes in parallel for installed managers."
- **Problem:** `sniff/lib/src/programs/host_capability.rs::detect_verified_lang_pkg_mgrs` runs probes for Npm, Pnpm, Yarn, Bun, and Cargo sequentially.
- **Impact:** Since each probe has a 2-second timeout, a host with multiple managers installed but slow response times (or missing global packages) can take up to 10 seconds to build a plan on a cache miss. Parallelizing these with `rayon` (which is already a dependency) would reduce the worst-case latency to 2 seconds.

## Ergonomics & Code Quality

- **NPM Writability Check:** The implementation of `detect_npm_global_prefix_writable` uses a marker file approach, which is robust.
- **CLI Rendering:** The use of `biscuit-terminal` and `Prose` in `install_plan_cmd.rs` is excellent and provides high-quality, plan-aware feedback to the user.
- **Bucket Logic:** The bucket-based priority system in `install_plan.rs` is clean and matches the design Phase 2 perfectly.

## Test Coverage

- **Strengths:** Excellent unit test coverage for `HostCapabilities` (default PM mapping, sudo probes) and `InstallPlan` (bucket selection rules).
- **Gaps:** 
    - No integration tests for the `--via` override logic in the CLI.
    - No tests for cache invalidation on hostname mismatch (since the logic is missing).
    - No tests for the RemoteBash execution failure (though it is currently a bug).

## Recommended Actions

1. **Fix RemoteBash Execution:** Update `build_install_command` to return a valid shell command (e.g., `curl -sSfL ... | bash`) for RemoteBash variants.
2. **Parallelize Probes:** Use `rayon` in `host_capability.rs` to run verification probes in parallel.
3. **Add Hostname Validation:** Update `load_host_capabilities_from` to invalidate the cache if the hostname changed.
4. **Centralize Availability:** Move the `has_bash` check into `method_available` to keep the library logic DRY.
