# Review 2: recent-commits

## Findings

1. High: `get_recent_commits_by_date()` implements an exact one-day window, not the "since date" behavior in the spec/design.
   Evidence:
   `spec.md:113-116` says the CLI date is "the ISO Date they want to start at".
   `tech-design.md:62, 249, 471` uses the label `since {date}` and explicitly calls for tests that verify commits "on/after the date".
   `sniff/lib/src/filesystem/git/recent_commits.rs:238-247` sets `since` to midnight of the requested date and `until` to midnight of the following day, so everything after that day is excluded.
   Impact:
   `sniff repo recent-commits 2025-12-04` silently drops commits from `2025-12-05+`, even though both the spec and tech design describe the date as the start boundary.
   Recommendation:
   Make the date path use `[date 00:00 UTC, now)` rather than `[date 00:00 UTC, next_date 00:00 UTC)`, and add an integration test with commits on multiple days to lock the behavior down.

2. High: `get_recent_commits_by_hash()` does not start from the requested commit; it falls back to a timestamp cutoff.
   Evidence:
   `spec.md:112-114` describes the hash input as "the git hash they want to start at".
   `tech-design.md:472` calls for tests verifying "`get_recent_commits_by_hash` returns commits from the hash to HEAD".
   `sniff/lib/src/filesystem/git/recent_commits.rs:267-280` resolves the hash, reads its commit time, and then calls the same time-based walker used for durations/dates.
   Impact:
   The function cannot guarantee "from hash to HEAD" semantics. It will include whatever HEAD-reachable commits happen to be newer than the resolved commit timestamp, which is not the same contract on merge-heavy histories, rebased histories, or same-timestamp commits.
   Recommendation:
   Walk by ancestry boundary instead of timestamp: resolve the target OID, push `HEAD`, and stop once that commit is reached or use a revwalk range/hide strategy that expresses the commit boundary directly.

3. Medium: package scoping does not honor the designed error semantics.
   Evidence:
   `tech-design.md:424` says `--package` on a non-monorepo should return `SniffError::NotAMonorepo`.
   `sniff/lib/src/filesystem/git/recent_commits.rs:513-516` and `sniff/lib/src/filesystem/git/recent_commits.rs:561-564` simply return early when `self.packages` is `None`, so `--package` / `--package-area` are silently ignored on non-monorepos.
   `sniff/lib/src/filesystem/git/recent_commits.rs:522-525` and `sniff/lib/src/filesystem/git/recent_commits.rs:575-577` clear all commits for unknown names instead of surfacing `UnknownPackage` / `UnknownPackageArea`.
   `sniff/cli/src/output/recent_commits.rs:56-64` then converts that cleared set into generic no-results handling.
   Impact:
   Users get misleading output: invalid scope names look like "no commits matched", and scoping a single-package repo does nothing instead of producing the documented error.
   Recommendation:
   Move scope validation into the CLI/library query path before mutation, reuse the existing `UnknownPackage`, `UnknownPackageArea`, and `NotAMonorepo` errors, and only filter once the scope is known-valid.

4. Medium: the library API promised by the spec/design is incomplete; the terminal-rendering methods were never implemented.
   Evidence:
   `spec.md:55-57`, `spec.md:74-76`, and `spec.md:92-94` require `describe_for_terminal`, `source_code_changes_for_terminal`, and `documentation_changes_for_terminal`.
   `tech-design.md:67-78`, `tech-design.md:250-257`, and `tech-design.md:395-397` repeat that those methods live on `CommitDescSet`.
   `sniff/lib/src/filesystem/git/recent_commits.rs:389-490` only defines the Markdown-producing methods. Terminal rendering is instead reimplemented in `sniff/cli/src/output/recent_commits.rs:72-90`.
   Impact:
   The CLI works, but the library contract described in the feature docs is missing. Any non-CLI consumer has to duplicate the same rendering path or depend on CLI-specific code.
   Recommendation:
   Add the `*_for_terminal` methods on `CommitDescSet`, export them as part of the public API, and make the CLI call those methods rather than owning the darkmatter rendering itself.

5. Medium: the default no-results behavior still does not satisfy the spec.
   Evidence:
   `spec.md:116` says "If NO results are found for the period then an error code and the string `none found` is returned."
   `sniff/cli/src/output/recent_commits.rs:63-64` and `sniff/cli/src/output/recent_commits.rs:78-79` delegate to the generic no-results helper.
   `sniff/cli/src/commands.rs:1010-1042` exits with no output unless `--on-error` is supplied.
   Impact:
   The default UX is silent failure instead of the explicit `"none found"` message required by the feature spec.
   Recommendation:
   Either specialize recent-commits no-results handling to emit `"none found"` by default, or update the spec/design if silent exit is the intended final behavior.

6. Medium: the tests miss the broken semantics above and contain vacuous assertions.
   Evidence:
   `tech-design.md:468-495` asked for date/hash boundary tests plus CLI coverage for date, hash, package, and package-area.
   `sniff/lib/tests/integration.rs:1118-1144` only checks "today returns something" and "future date returns nothing"; it never verifies that a past date includes later commits.
   `sniff/lib/tests/integration.rs:1148-1182` only exercises HEAD and a partial HEAD hash; it never verifies that the result stops at an older commit boundary.
   `sniff/lib/tests/integration.rs:1295-1325` uses `iter().all(...)` after filtering to a nonexistent package/area, which still passes when `self.commits` has been cleared.
   `sniff/cli/tests/cli.rs:1780-1994` has no coverage for date mode, hash mode, `--package`, `--package-area`, or the default `"none found"` behavior.
   Impact:
   The current suite passes while the main contract mismatches remain in place, so it is not protecting the feature.
   Recommendation:
   Add fixture repos with known commit ordering and timestamps, assert exact commit sets for date/hash queries, add CLI tests for date/hash/package/package-area, and replace the vacuous nonexistent-scope assertions with explicit error/no-results expectations.

## Non-blocking follow-up

- `collect_commits_in_range()` breaks on the first `commit_time < since` without explicitly calling `revwalk.set_sorting(...)` (`sniff/lib/src/filesystem/git/recent_commits.rs:305-325`). Making the sort order explicit would make the cutoff logic less fragile and easier to reason about.

## Verification

- Ran `cargo test -p sniff recent_commits -- --nocapture`
- Ran `cargo test -p sniff-cli recent_commits -- --nocapture`
