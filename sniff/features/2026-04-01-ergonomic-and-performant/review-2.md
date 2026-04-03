# Sniff Review Follow-Up

Most of the high-value recommendations from the first review have been implemented well. The request-based surface is materially better, several expensive paths are now gated correctly, and the repo/filesystem rescanning issues are much improved.

What remains is a smaller set of gaps where the library still either leaves performance on the table or does not yet give callers the last bit of control they now reasonably expect from the new API.

## 1. Parallelize top-level `DetectionPlan` execution

### Current state

`detect_with_plan()` still runs `os`, `hardware`, `network`, and `filesystem` strictly sequentially in `sniff/lib/src/lib.rs:236`.

### Why this still matters

The new request types make the per-domain work much more controllable, but a caller asking for multiple independent domains still pays end-to-end latency for the sum of all selected sections. That undercuts one of the main benefits of the refactor.

This is especially noticeable for plans like:

- `OsRequest::summary() + HardwareRequest::summary() + NetworkRequest::interfaces_only()`
- full top-level detection with both hardware and filesystem enabled
- any caller that wants both network WAN lookup and filesystem analysis

### Recommendation

Make `detect_with_plan()` orchestrate independent domains concurrently.

Concrete shape:

- run `os`, `hardware`, and `network` in parallel
- run `filesystem` concurrently with them as well, since it only depends on `base_dir`
- use `rayon::join`, scoped threads, or a small internal task helper that preserves `Result` propagation cleanly

This should improve wall-clock latency without changing the API at all.

## 2. Split “file changes” from “full dirty-file diff payloads” in git detection

### Current state

`GitRequest` now correctly lets callers skip file changes entirely, but `include_file_changes = true` still takes the heavy path in `sniff/lib/src/filesystem/git.rs:652-667`.

That heavy path calls `get_repo_status_with_changes()` in `sniff/lib/src/filesystem/git.rs:1263`, which still does all of the following together:

- status counting
- per-file line stats via `get_file_diff_stats()`
- construction of `RepoStatus.dirty`
- unified diff generation through `build_dirty_files()`

### Why this still matters

There is still no middle tier for callers that want:

- changed file paths
- status
- line counts

but do not want:

- full `RepoStatus.dirty[*].diff` payloads

That means `GitRequest::full()` is still more expensive than it needs to be for many callers, especially library consumers that only need compact machine-readable change summaries.

### Recommendation

Split the current `include_file_changes` flag into at least two levels, for example:

- `include_file_change_stats`
- `include_file_diffs`

or an enum such as:

- `StatusCountsOnly`
- `FileStats`
- `FullDiffs`

Then refactor `get_repo_status_with_changes()` so diff text generation is only done for the `FullDiffs` tier.

This is the biggest remaining “don’t make callers pay for what they didn’t ask for” issue in the git path.

## 3. Unbundle timezone detection from NTP probing

### Current state

`OsRequest` now has a single `include_time` flag in `sniff/lib/src/request.rs:111-152`, and `detect_os_with_request()` uses that to call `detect_timezone()` in `sniff/lib/src/os/mod.rs:231-232`.

But `detect_timezone()` in `sniff/lib/src/os/time.rs:356-404` still always calls `detect_ntp_status()` at line 393.

### Why this still matters

Timezone/offset/DST are cheap and local. NTP status is materially more expensive and more failure-prone, especially on Linux where command execution and timeout behavior are involved.

So the current API still forces callers to buy both together:

- “I need timezone” also implies “probe NTP”
- there is still no cheap “time metadata only” mode

### Recommendation

Split this into two request knobs, either:

- `include_timezone`
- `include_ntp_status`

or a small time sub-request type.

Then keep `TimeInfo.ntp_status` optional when NTP is not requested.

That would align the OS API with the same selective-cost philosophy now used in network, hardware, and filesystem.

## 4. Program detection still repeats PATH and macOS bundle scans across categories

### Current state

Category-level concurrency is now in place in `sniff/lib/src/programs/mod.rs:174-207`, which is good. But each category still independently calls `find_programs_with_source_parallel()`:

- `sniff/lib/src/programs/editors.rs:95`
- `sniff/lib/src/programs/utilities.rs:138`
- `sniff/lib/src/programs/pkg_mngrs.rs`
- `sniff/lib/src/programs/tts_clients.rs`
- `sniff/lib/src/programs/terminal_apps.rs`
- `sniff/lib/src/programs/headless_audio.rs`
- `sniff/lib/src/programs/ai_cli.rs`

And `find_programs_with_source_parallel()` in `sniff/lib/src/programs/find_program.rs:111-118` just parallelizes repeated calls to `find_program_with_source()`, which itself runs `which(...)` and macOS bundle fallback per candidate.

### Why this still matters

The new category-level parallelism improves latency, but it also increases duplicate work:

- repeated PATH traversal for aliases across categories
- repeated macOS `.app` bundle probing across categories
- more filesystem contention on machines with many installed applications

### Recommendation

Add a shared executable index for “detect all programs” flows:

- build the PATH lookup map once
- build the macOS app bundle index once
- resolve all category candidates against that shared index

Keep the existing category constructors for convenience, but have `ProgramsInfo::detect()` route through the shared index so full-library callers get the cheaper path.

This is a lower-priority optimization than the git and top-level orchestration gaps, but it is still a worthwhile next step.

## 5. The library README still lags behind the implemented API

### Current state

`sniff/lib/README.md` still documents the old library surface in several places:

- uses `sniff_lib` instead of the actual crate name `sniff` (`sniff/lib/Cargo.toml:2`; README examples start at `sniff/lib/README.md:35`)
- shows a stale `SniffConfig` field `include_cpu_usage` in `sniff/lib/README.md:152-161`
- demonstrates `os.arch` in examples even though `OsInfo` has no `arch` field (`sniff/lib/README.md:45`, `sniff/lib/README.md:195`; actual `OsInfo` is in `sniff/lib/src/os/mod.rs:128-155`)
- does not position `DetectionPlan` and the request types as the primary ergonomic API for selective detection

### Why this still matters

The code now offers a much better caller experience than the docs advertise. Right now, a new library consumer is still likely to discover the legacy coarse API first and miss the new selective request model entirely.

### Recommendation

Refresh the README so it reflects the current ergonomic story:

- use `sniff` consistently in examples
- keep `SniffConfig` as the compatibility/simple builder
- promote `DetectionPlan` + `request::*` as the primary API for callers who care about cost control
- add one “fast summary” example and one “filesystem/git selective” example

This is not just doc polish. It is necessary for callers to actually benefit from the refactor.

## Suggested priority order

1. Parallelize `detect_with_plan()`
2. Split git file stats from full diff payload generation
3. Unbundle timezone from NTP
4. Refresh README around `DetectionPlan`
5. Add shared executable indexing for all-program detection

## Bottom line

The major architectural shift has landed successfully. The remaining work is mostly about finishing the selective-cost model so it is consistent everywhere:

- independent domains should run concurrently
- “summary vs. expensive detail” should be split one level further in git and OS time
- the docs should now steer callers toward the new fine-grained API
