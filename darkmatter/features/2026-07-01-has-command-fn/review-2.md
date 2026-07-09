---
ready: false
agent: codex/default
created: 2026-07-01T22:38:08
implemented: true
---

# Review 2 - `has_command(cmd)`

## Findings

### High - Existing relative executable paths return `true`, despite the spec requiring `false`

The handler delegates directly to `which::which(raw)` in `darkmatter/lib/src/markdown/compose/expression/functions.rs:1163`. That does not implement the spec's relative-path contract. The `which` v7 API used by `darkmatter` explicitly resolves relative paths against the process CWD: "If given a relative path, returns an absolute path to the file if it exists and is executable." The same crate exposes `which_global`, documented as ignoring CWD and not resolving relative paths.

Impact: `has_command("./mytool")` and `has_command("bin/mytool")` can return `true` whenever an executable exists at that relative path under the process CWD. The spec and public docs both say relative paths are intentionally not resolved and always return `false` (`darkmatter/docs/topics/darkmatter-expressions.md:459-465`). This is observable expression behavior, not just a documentation mismatch.

Required fix: change the implementation to use `which::which_global(raw)` or otherwise reject path-containing relative inputs before probing. Keep bare command names working through PATH and keep absolute executable paths working.

Verification level: current strongest coverage is Level 1, but it is incomplete. `has_command_relative_path_is_not_resolved` only checks missing relative paths (`./mytool`, `bin/mytool`) from the current CWD (`darkmatter/lib/src/markdown/compose/expression/functions.rs:2955-2965`), so it cannot catch this bug. Add a Level 1 test that creates an executable in a temp CWD, serializes/restores CWD like the nearby `file_exists` CWD tests, and asserts both `./mytool` and `bin/mytool` return `false`.

## Coverage Notes

The implementation now includes the handler, `FS_FUNCTIONS` registration, descriptor catalog row, generated docs row, prose docs, and focused Level 1 tests for the rest of the specified behavior. For this feature, Level 1 is the appropriate verification level because the behavior is expression evaluation and filesystem/PATH probing, not terminal rendering or OS keyboard input.

I ran:

```text
cargo nextest run --color=never -p darkmatter -E 'test(/has_command/) + test(/descriptor_signature_set_equals_dispatchable_signature_set/) + test(/narrative_doc_function_table_matches_catalog/)'
```

Result: 13 passed. That passing result does not close the finding above because the failing relative-existing-executable scenario is not present in the suite.

## Open Questions

None.

## Production Readiness

Not ready for production. The feature is mostly implemented, but one documented Definition-of-Done behavior is wrong and the Level 1 tests miss the failing case.
