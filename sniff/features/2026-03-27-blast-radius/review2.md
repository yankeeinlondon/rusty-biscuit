# Blast Radius Follow-up Review

Validation performed:

- Re-reviewed the updated implementation in `sniff/lib` and `sniff/cli`
- Ran `just test` from [/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff), which passed

## Remaining Issues

### 1. JSON output still does not match the tech design’s documented shape

Severity: Medium

Files:

- [commands.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/src/commands.rs#L170)
- [commands.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/src/commands.rs#L948)

The text-mode behavior is now in much better shape, but JSON output still diverges from the design:

- `scope` and `kind` are serialized directly from Rust enums, so values are emitted as `Dirty`, `Staged`, `SourceCode`, etc. rather than the documented lowercase forms like `dirty` and `source_code`
- `sniff blast-radius --json` returns `documents` as an array of objects, while the design documented an array of repo-relative document paths

Current tests only assert that the keys exist, not that the values and structure match the intended contract.

Recommendation:

- Serialize explicit string values for `scope` and `kind`
- Either align `documents` to the documented path-list shape, or update the tech design/docs to reflect the richer object payload
- Add exact JSON assertions in CLI tests, not just presence checks

### 2. `sniff docs --blast-radius` is still missing direct coverage

Severity: Medium

Files:

- [output/mod.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/src/output/mod.rs#L214)
- [cli.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/tests/cli.rs)

The filter implementation exists, but I still do not see:

- a unit test proving `DocsFilter { blast_radius: true, .. }` filters correctly
- a CLI integration test for `sniff docs --blast-radius`

This leaves one of the explicit spec additions effectively unverified end-to-end.

Recommendation:

- Add a `filter_docs()` unit test with mixed `has_blast_radius` values
- Add a CLI integration test that creates two docs, only one with `blast_radius`, and asserts that `sniff docs --blast-radius --plain` returns only that one

### 3. Package and package-area scoping fixes are implemented, but still not proven by tests

Severity: Medium

Files:

- [blast_radius.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/blast_radius.rs#L145)
- [cli.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/tests/cli.rs)

The new logic for:

- `--package`
- `--package-area`
- nested area prefix matching
- `UnknownPackage` / `UnknownPackageArea`

looks reasonable, but I still do not see direct temp-repo or CLI coverage for those code paths.

That matters here because package/package-area scoping was one of the bug-prone areas from the first review, and the current implementation now includes non-trivial matching and validation branches.

Recommendation:

- Add temp-repo unit tests for `collect_changed_paths()` covering:
    - exact package match
    - exact package-area match
    - nested package-area prefix match
    - unknown package error
    - unknown package-area error
- Add at least one CLI integration test for `sniff blast-radius --package-area ...` or `sniff repo staged-files --package ...`

### 4. `find_blast_radius_documents()` still does unnecessary work when there are no changed source files

Severity: Low

File:

- [blast_radius.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/blast_radius.rs#L296)

This was previously called out as a performance recommendation and it is still unimplemented. When `changed_set` is empty, the function still walks the repository and parses every Markdown document before returning an empty result.

Recommendation:

- Short-circuit immediately when `changed_set.is_empty()`

## Summary

The main functional issues from the first review are fixed. What remains is mostly contract and confidence work:

- JSON output should be made consistent with the documented shape
- the `docs --blast-radius` path still needs direct tests
- package/package-area scoping needs proof, not just implementation
