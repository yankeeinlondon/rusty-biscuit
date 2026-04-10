# Review 3: recent-commits

## Findings

1. High: `get_recent_commits_by_hash()` no longer preserves a correct "from hash to HEAD" result set.
   Evidence:
   The implementation manually pushes the target commit into the output first at [sniff/lib/src/filesystem/git/recent_commits.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/git/recent_commits.rs#L392), then starts a second `HEAD` revwalk and hides the target OID at [sniff/lib/src/filesystem/git/recent_commits.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/git/recent_commits.rs#L444). There is no ancestry check anywhere in this path. That means:
   - if the requested hash is not an ancestor of `HEAD`, `hide(target_oid)` does not define a meaningful "since hash" boundary, so the function returns `{target_commit} + HEAD history`;
   - even when the hash is an ancestor, the returned order is `target_commit` first and newer descendants after it, which is inconsistent with the newest-first behavior used by the duration/date paths and assumed by the renderers.
   Impact:
   One of the three core query modes can return the wrong commit set and the wrong order, which makes both `describe()` and the grouped change views misleading for hash-based queries.
   Recommendation:
   Validate that the requested commit is reachable from `HEAD` (for example via `graph_descendant_of`) and drive a single ancestry walk that stops at the boundary commit instead of manually prepending it.

2. Medium: the generated Markdown hyperlinks are not valid cross-platform file URIs.
   Evidence:
   Both `describe()` and `file_grouped_changes()` build links with raw string concatenation via `format!("file://{}", abs.to_string_lossy())` at [sniff/lib/src/filesystem/git/recent_commits.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/git/recent_commits.rs#L544) and [sniff/lib/src/filesystem/git/recent_commits.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/git/recent_commits.rs#L605). On Windows that produces malformed URIs like `file://C:\repo\file.rs` instead of `file:///C:/repo/file.rs`, and on every platform it leaves spaces and reserved characters unescaped.
   Impact:
   Non-plain output does not reliably satisfy the spec's hyperlink requirement, especially on Windows, which is a supported platform for `sniff`.
   Recommendation:
   Centralize file-URL generation with `url::Url::from_file_path` or an equivalent helper and render the resulting URL string into Markdown.

3. Medium: filtered `--json` output still exposes the full unfiltered monorepo package inventory.
   Evidence:
   `CommitDescSet` serializes a top-level `packages` field at [sniff/lib/src/filesystem/git/recent_commits.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/git/recent_commits.rs#L165), and the CLI prints that struct verbatim for JSON output at [sniff/cli/src/output/recent_commits.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/src/output/recent_commits.rs#L68). `filter_by_package()` and `filter_by_package_area()` mutate per-commit files/metadata, but never rewrite or remove `self.packages` at [sniff/lib/src/filesystem/git/recent_commits.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/git/recent_commits.rs#L651) and [sniff/lib/src/filesystem/git/recent_commits.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/git/recent_commits.rs#L703). The serialized `Package` type also carries absolute filesystem paths at [sniff/lib/src/filesystem/repo/types.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/repo/types.rs#L171).
   Impact:
   `sniff repo recent-commits --json --package foo` still leaks unrelated package metadata, including absolute paths, even though the user asked for a narrowed result set.
   Recommendation:
   Treat `CommitDescSet.packages` as internal state (`#[serde(skip)]`) or rewrite it during filtering so the JSON payload reflects the scoped result instead of the original repo snapshot.

4. Medium: the tests are still too shallow for the feature's highest-risk paths.
   Evidence:
   The only helper that actually builds a multi-commit repo is unused at [sniff/lib/tests/integration.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/tests/integration.rs#L1047). Hash coverage only exercises `HEAD` and a partial `HEAD` hash at [sniff/lib/tests/integration.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/tests/integration.rs#L1148), so it never checks an older boundary commit or a non-ancestor hash. The library integration tests never create a monorepo and only assert the non-monorepo error path for package filters at [sniff/lib/tests/integration.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/tests/integration.rs#L1295). The CLI coverage is mostly smoke tests for success / JSON / plain / no-error at [sniff/cli/tests/cli.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/tests/cli.rs#L1781), with no coverage for `--package`, `--package-area`, hash/date/today/yesterday routing, or scoped JSON output.
   Impact:
   The remaining correctness issues above can regress silently because the suite does not exercise the actual boundary conditions the spec and tech design called out.
   Recommendation:
   Add end-to-end fixture repos that cover multiple commits, an older hash boundary, a non-ancestor hash, and a real monorepo layout; then assert exact commit order/sets and scoped text/JSON output.

## Open Question

- The CLI currently interprets `today` and `yesterday` on UTC day boundaries at [sniff/cli/src/output/recent_commits.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/src/output/recent_commits.rs#L34). If the intended UX is local calendar days rather than UTC calendar days, that behavior still needs an explicit decision and test coverage.

## Verification

- `cargo test -p sniff --test integration recent_commits -- --nocapture`
- `cargo test -p sniff-cli test_repo_recent_commits_ -- --nocapture`
