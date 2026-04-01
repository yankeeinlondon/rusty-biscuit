# Sniff Review Follow-Up 3

The `review-2` items were mostly closed well. The top-level plan now runs domains concurrently, git now separates file stats from full diff payloads, OS time splits timezone from NTP probing, and the shared program index has been introduced.

At this point I only see a short list of remaining gaps.

## 1. Preserve `which` semantics in `ExecutableIndex`

### Current state

The new shared program index in [`sniff/lib/src/programs/find_program.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/programs/find_program.rs#L88) builds its PATH map by iterating raw directory entries and indexing `entry.file_name()` directly:

- it does not verify that a PATH entry is actually executable
- it stores the literal filename rather than a normalized command name

Lookups then use exact string matching in [`sniff/lib/src/programs/find_program.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/programs/find_program.rs#L122).

### Why this still matters

This no longer matches the semantics of the old `which(...)`-based path:

- On Windows, a caller asks for `git`, but the PATH entry is usually `git.exe`. The index stores `git.exe`, while lookups ask for `git`, so the shared-index path will miss programs that `which("git")` would find.
- On Unix-like systems, the index can treat any matching filename in a PATH directory as installed, even if that file is not executable.

That means `ProgramsInfo::detect()` can now be less correct than the slower per-program fallback it replaced.

### Recommendation

Make `ExecutableIndex::build()` preserve `which` semantics:

- only index executable files
- normalize Windows entries so `git.exe` can satisfy a lookup for `git`
- ideally add a parity test layer that compares `ExecutableIndex::find_with_source()` against `find_program_with_source()` for representative PATH cases

This is the only remaining issue I’d classify as a correctness risk rather than just a performance/documentation gap.

## 2. `ProgramsInfo::refresh()` still falls back to the old expensive path

### Current state

`ProgramsInfo::detect()` now does the right thing by building one shared `ExecutableIndex` and fanning that out across all categories in parallel, but [`sniff/lib/src/programs/mod.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/programs/mod.rs#L220) still refreshes each category independently.

Those category refreshes still call `Self::new()` rather than the indexed path, for example:

- [`sniff/lib/src/programs/utilities.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/programs/utilities.rs#L275)
- [`sniff/lib/src/programs/ai_cli.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/programs/ai_cli.rs#L105)

### Why this still matters

The optimized “build once, query many” flow is now only used for initial detection. A caller that refreshes program state still pays the older repeated-scan cost.

### Recommendation

Make `ProgramsInfo::refresh()` reuse the optimized path, for example by delegating to `*self = Self::detect();`.

That keeps the public API unchanged and makes refresh consistent with the newer cost model.

## 3. Add behavior-level regression tests for the new selective-cost knobs

### Current state

The new request surfaces are in place, but the test coverage is still weighted toward:

- builder/default shape tests in [`sniff/lib/src/request.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/request.rs)
- one top-level summary-mode integration test in [`sniff/lib/tests/integration.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/tests/integration.rs#L655)

I did not find end-to-end tests that verify the new selective behavior itself.

### Why this still matters

The recent refactors changed cost behavior, not just type shapes. Those are exactly the kinds of changes that tend to regress silently later if only builder-level tests exist.

The highest-value missing cases are:

- `GitRequest::full()` populates `file_changes` but leaves unified diff payloads empty
- `GitRequest::deep()` still includes unified diffs
- `OsRequest` with timezone enabled but NTP disabled returns time data without probing NTP
- shared-index program detection preserves the same results as the old `which` path for common commands

### Recommendation

Add behavior-level tests for these selective modes, especially around git diff payload gating and executable-index parity.

## Priority

1. Fix `ExecutableIndex` semantics
2. Make `ProgramsInfo::refresh()` reuse the shared index
3. Add regression tests for the new selective behaviors
