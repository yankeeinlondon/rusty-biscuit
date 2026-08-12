---
ready: false
agent: "codex/default"
created: "2026-06-29T12:22:02"
implemented: true
---

# Review 10

## Findings

### High - `claudine errors` wraps stable codes so the human introspection surface is not handleable by copy/paste

The feature's handleability contract makes `claudine errors` the discoverability surface for every stable `err.code` and its detail fields. The JSON path preserves the code strings, but the default human report currently wraps the longest composition code inside the `Code` cell:

```text
│ `composition.invalid_file_         │ correctable │ author │ error ...
│ reference`                         │             │        │       ...
```

That means the default report does not contain the public code `composition.invalid_file_reference` as a contiguous string even at `COLUMNS=200`. This is not just a brittle assertion: the command is teaching users which exact versioned codes to match in lifecycle `when:` clauses, and the longest representative code cannot be copied or searched directly from the default output.

The cause is the pinned code column in `claudine/cli/src/commands/errors.rs`:72-95. It computes the first-column width from the raw code (`function_first_column_width("Code", CODES.iter().map(|spec| spec.code))`) but renders the cell as backticked Prose (`inline_code_text(&format!("`{}`", spec.code), ...)`). The table then wraps `composition.invalid_file_reference` despite that exact code being present in the registry at `claudine/lib/src/diagnostics/registry.rs`:165-180.

Verification level present: Level 1 CLI spawn. `cargo nextest run --color=never -p claudine-cli --test errors_command` fails in `errors_default_exits_zero_and_lists_representative_codes` (`claudine/cli/tests/errors_command.rs`:28-70) because stdout does not contain `composition.invalid_file_reference`. The JSON tests in the same target pass, so the registry data is intact and the bug is isolated to default terminal rendering.

Required verification level: Level 1 is sufficient for the copyable plain-output contract. Keep the existing default-output test green by making code cells non-wrapping or by sizing the column for the rendered/backticked code width. Level 2 is not required for this specific blocker because the failing behavior is already visible in the non-PTY command output and does not depend on terminal encoder/decoder behavior.

## Notes

The previous review's transport blocker appears addressed: `MarkdownLoad` and `SequenceExternalLoad` now carry typed source sub-enums, and the focused library tests verify the source chain is recoverable. The lifecycle alias, `err.severity`, and docs-scan issues from review 8 also appear addressed.

Checks run:

- `env -u CDPATH scripts/check-error-transport.sh` passed.
- `env -u CDPATH scripts/check-lifecycle-doc-facets.sh` passed.
- `cargo nextest run --color=never -p claudine --lib lifecycle_context diagnostics::facets diagnostics::registry composition::resolve composition::sequence` passed, 99/99.
- `cargo nextest run --color=never -p claudine-cli --test errors_command` failed, 3/4 passed, 1 failed as described above.

I did not run the full Level 2 terminal suites in this pass. Existing Level 2 invalid-file-reference capture coverage remains present in both `darkmatter/cli/tests/level2_errors.rs` and `claudine/cli/tests/level2_invalid_file_reference_capture.rs`, and this new blocker is already caught at Level 1.

Production ready: **no**.
