# Release-specific behavior

Read this file only when installed-version behavior affects diagnosis or configuration. These are
observed compatibility notes, not a replacement for the upstream changelog. Keep durable guidance
in `SKILL.md` or the topical references and keep version-bound facts here.

## 0.19.0

- `stats --last-build` can span multiple Cargo commands. Events without explicit session IDs are
  grouped per build root using a five-minute idle boundary, so an aggregate may disagree sharply
  with `monitor` for one command. Use `--root`, record the exact time window, and treat the live
  build counters as the command-level evidence. Version 0.23.1 still documents this session model.
- The default event log was 10 MiB and retained only 1,000 lines after rotation. Under concurrent
  builds this host retained roughly 30 minutes, so identical `--since 24h` and `--since 7d` totals
  did **not** prove that the cache was merely new. Check the oldest retained event before inferring
  the history window from the requested duration.
- macOS executable and test-output caching was enabled by default on the measured installation.
  Large test executables and dSYM bundles consequently dominated the store. Check the effective
  `cache_executables` value rather than relying on a cross-version default.
- Size GC skipped an entire entry when a last-reference blob was shared with a target. Private
  blobs belonging to that entry could therefore remain, and `local_max_size` was not a hard
  namespace bound. `kache gc --json` distinguished `entries_unreclaimable` from SQLite lock
  failures; `gc_evict_shared = true` enforced eviction of reusable entries but could not free
  blocks still retained by targets.
- `kache list --sort hits | head` could panic when `head` closed the pipe. Use the command's pager,
  `--no-pager` with full output redirection, or structured `--json` output instead of a truncating
  pipe when automating this release.

## 0.23.1

- `stats --help` explicitly says that `--last-build` reports the latest activity session, that a
  session may span Cargo commands, and that events without IDs use a five-minute idle gap per root.
  This confirms that the session-versus-command distinction observed in 0.19.0 remains relevant.
- `list` provides `--no-pager` and all principal diagnostic commands provide `--json`; prefer these
  interfaces in automation rather than parsing interactive output.
