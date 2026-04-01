# Sniff Review Follow-Up 4

At this point the library is in good shape. The major issues from the earlier reviews appear to be addressed, and what remains is mostly a short list of lower-priority refinements.

## 1. Reuse repo-wide inventory for top-level filesystem file/language summaries

### Current state

Full repo detection already builds a repo-wide inventory in [`sniff/lib/src/filesystem/repo.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/repo.rs#L563):

- [`sniff/lib/src/filesystem/repo.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/repo.rs#L566)

But [`detect_filesystem_with_request()`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/mod.rs#L60) still performs a separate top-level inventory scan for `files` / `languages`:

- [`sniff/lib/src/filesystem/mod.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/mod.rs#L83)
- [`sniff/lib/src/filesystem/mod.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/mod.rs#L101)

### Why this still matters

For callers requesting both:

- `RepoRequest::full()`
- `include_file_inventory = true`

the same tree can still be walked twice:

1. once to enrich packages
2. again to compute top-level file/language summaries

That is a real remaining cost on large repositories.

### Recommendation

Push the shared inventory one level higher so filesystem detection can reuse it for both:

- package boundary/package language enrichment
- top-level `files` and `languages`

The cleanest shape is probably to have repo detection optionally surface the inventory or an already-computed summary when full mode is requested.

## 2. WAN IP lookup still blocks local network detection and is cached forever

### Current state

`detect_network_with_request()` still performs WAN lookup before local interface enumeration in [`sniff/lib/src/network/mod.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/network/mod.rs#L122):

- WAN lookup first at [`sniff/lib/src/network/mod.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/network/mod.rs#L123)
- interface enumeration only afterward at [`sniff/lib/src/network/mod.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/network/mod.rs#L129)

The WAN result is also stored in a process-lifetime `OnceLock`:

- [`sniff/lib/src/network/mod.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/network/mod.rs#L18)
- [`sniff/lib/src/network/mod.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/network/mod.rs#L268)

### Why this still matters

Two remaining costs/ergonomics issues:

- Callers that request WAN IP still wait for that HTTP path before getting local interface results, even though those two tasks are independent.
- Long-lived callers cannot refresh a stale WAN IP without restarting the process.

### Recommendation

Two reasonable follow-ups:

- run WAN lookup concurrently with local interface enumeration
- replace the permanent `OnceLock` cache with either a short TTL cache or an explicit refresh knob on `NetworkRequest`

This is lower priority than the earlier refactors, but it would finish the “fast local, optional remote” story more cleanly.

## 3. `ExecutableIndex` is closer to `which`, but still hardcodes Windows executable semantics

### Current state

The shared executable index is much better now, but Windows behavior is still approximated in [`sniff/lib/src/programs/find_program.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/programs/find_program.rs#L162):

- Windows treats any regular file as executable in [`sniff/lib/src/programs/find_program.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/programs/find_program.rs#L178)
- extension normalization is hardcoded to `[".exe", ".cmd", ".bat", ".com", ".ps1"]` in [`sniff/lib/src/programs/find_program.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/programs/find_program.rs#L190)

The codebase already has a more PATHEXT-aware pattern in services detection:

- [`sniff/lib/src/services/mod.rs`](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/services/mod.rs#L469)

### Why this still matters

This is now a narrower edge case, but it still means the index is not a full behavioral match for shell resolution on Windows:

- custom `PATHEXT` values are ignored
- some regular files may be indexed even though they are not actually invokable by name

### Recommendation

If Windows fidelity matters for program detection, reuse the same PATHEXT-driven logic already used in the services module instead of hardcoding the extension list here.

This is a smaller cross-platform correctness cleanup, not a major architectural problem.

## Priority

1. Reuse the repo-wide inventory for top-level filesystem summaries
2. Make WAN lookup concurrent and refreshable
3. Tighten Windows `ExecutableIndex` semantics around `PATHEXT`
