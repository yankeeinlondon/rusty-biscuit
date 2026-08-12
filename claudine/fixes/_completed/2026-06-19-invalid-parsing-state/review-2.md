---
ready: false
agent: codex/default
created: 2026-06-20T11:16:46
implemented: true
---

# Review 2

Not ready for production. The interpolation fix and the main shell leak guard are in the right place, but the shell path still violates the spec's trimmed-whole-value contract for valid padded values.

## Findings

### High: padded whole-value `$()` values are rejected instead of expanded

Requirement: a frontmatter string scalar whose **trimmed** content is exactly a `$(...)` shell expansion, including supported suffixes, must parse and expand when frontmatter shell expansion is enabled. The spec and docs both define the boundary with trimmed content, not byte-0 `$(`.

Implementation: `scan_frontmatter` sends the raw string to `parse_shell_value` at `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:995`, and `parse_shell_value` immediately returns `Ok(None)` unless the untrimmed value starts with `$(` at `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:274`. A value such as `"  $(echo ok)  "` therefore produces no directive. The no-candidate path then runs `validate_no_whole_value_shell_leak` at `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:1031`; that guard trims, recognizes a clean whole-value directive at `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:1637`, and returns the generic "survived shell expansion" error instead of executing the command. The new test classification even names padded-valid forms as `CleanDirective` at `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:2226`, but there is no execution test proving they expand.

Impact: valid frontmatter that satisfies the spec's whole-value shape after trimming fails composition:

```yaml
value: "  $(echo ok)  "
```

This should resolve to `ok` when shell expansion is enabled. Instead, it is treated as a post-expansion leak because the executor never saw it.

Verification level: Level 1 is appropriate for this non-terminal compose semantics, but the present Level 1 coverage encodes the rejection path for padded malformed values and only classifies padded valid values in a helper. It does not cover the required successful expansion behavior for trimmed whole-value `$()` scalars.

Fix direction: make the shell scan and parser apply the same trimming boundary used by the spec and leak guard, while preserving the existing mixed-string behavior for real prefixes/suffixes. Add an execution-level Darkmatter test for `"  $(echo ok)  "` expanding successfully, plus a suffix variant such as `"  $(echo ok)::no-cache  "` if suffix trimming is intended to be supported.

## Verification Matrix

| Requirement | Strongest verification present | Status |
| --- | --- | --- |
| Malformed whole-value `{{ ... }}` is fatal even with `fail_fast = false` | Level 1 Darkmatter unit test and Level 1 Claudine CLI dry-run regression | OK |
| Typed whole-value `{{ ... }}` results are preserved | Level 1 Darkmatter unit tests for scalar and aggregate JSON values | OK |
| Mixed malformed interpolation remains lenient | Level 1 Darkmatter unit test | OK |
| Whole-value `$()` parse/execution failures are fatal when shell expansion is enabled | Level 1 Darkmatter unit tests, including padded malformed/no-command guard cases | Mostly OK |
| Whole-value `$()` values whose trimmed content is valid expand when shell expansion is enabled | Missing for padded valid values | Gap |
| Raw expansion syntax does not appear in successful effective frontmatter | Level 1 leak-guard tests and Level 1 Claudine dry-run regression for the original interpolation bug | OK for covered shapes |
| Documentation states the strict whole-value contract | Docs updated in Claudine and Darkmatter | OK |

## Notes

No Level 2 or Level 3 testing is required for this feature as specified. The observable behavior is compose preparation, diagnostics, and effective-frontmatter content, not terminal rendering or keyboard input encoding.
