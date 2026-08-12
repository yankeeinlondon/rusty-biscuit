---
ready: true
agent: codex/default
created: 2026-07-01T23:04:56
---

# Review 4 - `has_command(cmd)`

## Findings

No blocking findings.

The prior review-3 blocker is resolved: the unrelated `darkmatter/justfile`
L2 runner change is no longer present in the tracked diff. The current diff is
scoped to the expression implementation, descriptor/docs, tests, and this
feature metadata.

The prior review-2 behavior bug is also resolved. `has_command_fn` now rejects
path-shaped relative inputs before calling `which`, so an executable under the
process CWD no longer makes `has_command("./tool")` or
`has_command("bin/tool")` return `true`
(`darkmatter/lib/src/markdown/compose/expression/functions.rs:1169`).

## Verification Level Review

Level 1 is the appropriate verification level for this feature. The observable
behavior is in-process expression evaluation plus host filesystem/PATH probing;
there is no terminal rendering, terminal input encoder behavior, hotkey
handling, paste/IME, mouse, or scroll requirement that would require Level 2 or
Level 3 coverage.

Level 1 coverage is present for the specified behavior:

- present command on `PATH` and missing command
- `null`, non-string, and empty string returning `false`
- absolute missing path, Unix absolute executable path, Unix non-executable
  path, and directory probes
- tilde and relative-path gaps
- regression coverage for an existing executable under process CWD
- arity errors remaining errors
- canonical `has_command` and alias `hascommand` dispatch
- descriptor/dispatch parity and generated documentation parity

The catalog and generated docs now expose `has_command(cmd)` in the Filesystem
category, and the prose documents the no-execution guarantee, no remote URL
argument, `PATHEXT`/Unix executable-bit behavior, symlink/directory behavior,
and the deliberate tilde/relative-path gaps.

## Validation Run

Focused feature validation passed:

```text
cargo nextest run --color=never -p darkmatter -E 'test(/has_command/) + test(/descriptor_signature_set_equals_dispatchable_signature_set/) + test(/narrative_doc_function_table_matches_catalog/)'
```

Result: 14 passed, 5033 skipped.

I also attempted the package-area suite with `cd darkmatter && just test`.
That run failed before completion in pre-existing layout/page tests unrelated
to this feature, including `code_panel_inverts_against_terminal_not_option_in_transparent_default`
and several color-mode inversion tests at `darkmatter/lib/src/layout/page.rs`.
The failures occur outside the `has_command` implementation surface and do not
change this feature review, but they remain release context for the package.

## Production Readiness

Ready for production for the `has_command(cmd)` feature. Each user-observable
requirement has appropriate Level 1 verification, and no Level 2 or Level 3
coverage is required for this expression function.
