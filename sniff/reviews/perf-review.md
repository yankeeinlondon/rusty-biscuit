# Sniff Performance Review

**Date:** April 11, 2026  
**Scope:** `sniff/lib` and `sniff/cli`  
**Status:** Comprehensive Review

## Executive Summary

The Sniff library and CLI are feature-rich but suffer from significant performance bottlenecks due to redundant filesystem traversals, sequential execution of independent tasks, and inefficient system discovery patterns. While some parallelization exists (e.g., in programs detection and top-level domain dispatch), the core filesystem and hardware detection paths are largely sequential. Furthermore, the lack of structured metrics makes it difficult to pinpoint and measure these delays in a production environment.

## 1. Top Performance Opportunities

### 1.1 Redundant Filesystem Traversals
The library currently performs multiple full-tree walks during a single `detect()` call:
- **Manifest Indexing:** `ManifestIndex::build` walks the tree to find package manifests.
- **File Inventory:** `scan_file_inventory` walks the tree to classify every file.
- **Documentation Discovery:** `detect_docs` walks the tree to find Markdown files.

**Recommendation:** Consolidate all tree-walking operations into a single, shared parallel walk using `ignore::WalkParallel`. This walk should populate a unified "System View" that includes manifests, file classifications, and documentation metadata in one pass.

### 1.2 Sequential Stage Execution
In `detect_filesystem_with_request`, the stages (Git, Repo, Inventory, Formatting, Docs) are executed one after another. 
- **Repo Detection** waits for **Git Detection**.
- **Inventory** waits for **Repo Detection**.
- **Docs** waits for everything.

**Recommendation:** Use `std::thread::scope` or `rayon` to parallelize these stages. Most stages only need the `root` path and can run independently. Dependency-injection can be used to share the results of the consolidated walk (see 1.1) once it completes.

### 1.3 Redundant PATH Scanning
Both the `os` and `programs` modules scan the system `PATH` to find executables. `os::package_manager` performs its own `is_file` and `metadata` checks for every known package manager, while `programs::ExecutableIndex` does a similar scan for hundreds of potential binaries.

**Recommendation:** Move `ExecutableIndex` to a shared internal utility or the `os` module and ensure it is built exactly once. OS package manager detection should query this index instead of performing manual filesystem probes.

### 1.4 Sequential Hardware Detection
On macOS, `detect_audio_devices` takes approximately 1.5 seconds. Currently, it runs sequentially with GPU and Storage detection.

**Recommendation:** Parallelize hardware sub-tasks. Audio, GPU, and Storage detection should run concurrently. *Note: Ensure the "audio-before-GPU" constraint on macOS is still respected if using a lock, or verify if the interference is still a concern in modern Metal versions.*

### 1.5 Subprocess Overhead
Network detection frequently shells out to `route` or `ip` to find the default gateway. Subprocess execution is significantly slower than native platform APIs.

**Recommendation:** Use platform-specific native APIs (e.g., `netlink` on Linux, `GetAdaptersAddresses` on Windows, or `sysctl`/`route` sockets on macOS) to retrieve routing information without spawning processes.

---

## 2. Tracing and Metrics Opportunities

### 2.1 Granular Instrumentation
While `#[instrument]` is used at high levels, many "leaf" functions that are called thousands of times (e.g., `classify_file`, `command_exists_in_path`) are not instrumented. This makes it impossible to see the "long tail" of small operations in a trace.

**Recommendation:** Add `trace!` or `debug!` spans to hot loops. Specifically, instrument the `ignore` walker's callback to track how much time is spent in `hyperpolyglot` vs. simple extension matching.

### 2.2 Structured Performance Metrics
The library lacks a dedicated metrics collection system. We cannot easily answer questions like "What was the p99 of Git discovery today?" or "Which package manager check is the slowest?"

**Recommendation:**
- **Duration Logging:** Explicitly log the duration of every major stage (Git, Repo, Hardware, etc.) as a field in a `tracing` event.
- **Counter Metrics:** Track the number of files scanned, files classified by content, and cache hits/misses (e.g., for WAN IP or Hardware capabilities).
- **Metric Export:** Provide a `metrics` feature that allows users to export these as a structured JSON object or via the `metrics` crate for Prometheus/Grafana integration.

### 2.3 Cost-Aware JSON Output
The `--json` output should optionally include a `performance` block that summarizes where time was spent during that specific run.

**Recommendation:** Add a `metadata` or `performance` field to `SniffResult` (when requested via a flag like `--perf`) containing a breakdown of execution time per module and sub-task.

---

## 3. Immediate Action Plan (Priority Order)

1. **Unify the Walk:** Combine `ManifestIndex` and `FileInventory` walks into a single `WalkParallel` implementation.
2. **Parallelize Hardware:** Concurrently detect Audio, GPU, and Storage.
3. **Cache PATH:** Ensure `get_path_dirs()` is called once and results are cached or shared.
4. **Metrics Layer:** Implement a simple internal timer for the 4 top-level domains and report durations in `trace` logs.
