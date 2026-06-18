---
agent: codex
model: ""
ready: true
---

# Review: File Links Directive, Iteration 5

## Findings

### Low: the public error table omits `TargetNotDirectory`

Iteration 5 adds a distinct `TargetNotDirectory` error for `--dir` paths that
resolve to regular files, but the directive reference still lists only
`TargetNotFound`.

References:

- [file-links.md](../../docs/inline/file-links.md#errors)
- [types.rs](../../lib/src/markdown/compose/file_links/types.rs)

Suggested fix: add `TargetNotDirectory` to the Errors table and describe it as
an existing `--dir` target that is not a directory. This is documentation drift
only; strict and permissive behavior are both implemented and covered at
Level 1.

## Verification Matrix

| Requirement | Strongest verification | Assessment |
|---|---:|---|
| Glob/`--dir` parsing, depth, extension filtering, self-exclusion | Level 1 | Appropriate |
| Repository/CWD boundary and symlink handling | Level 1 | Appropriate |
| Empty and invalid target strictness behavior | Level 1 | Appropriate |
| Concurrent transclusion execution and single discovery walk | Structural/Level 1 | Appropriate |
| Visible hierarchy, document glyphs, and repository/folder icons | Level 2 | Appropriate |
| Dimmed root prefix, highlighted target, dotfile italics, gitignore dimming | Level 2 | Appropriate |
| OSC8 destination for every rendered file | Level 2 | Appropriate |

No Level 3 verification is required because the feature has no keyboard,
mouse, paste, IME, or terminal-input encoding behavior.

## Verification

Static review confirms that the two iteration 4 findings are resolved:

- `--dir` rejects regular files with a line-aware `TargetNotDirectory` error,
  with strict and permissive compose tests.
- The Level 2 test separately verifies the visible dimmed `/docs/` prefix and
  uses the same complete file list for visible-name and OSC8 destination
  assertions.

`git diff --check` passed.

Focused Rust tests and the Level 2 recipe could not run because this host has
no installed Rust toolchain (`rustup toolchain list` reports
`no installed toolchains`). Both attempted focused test commands failed before
compilation:

```text
cargo test --color=never -p darkmatter --lib file_links
cargo test --color=never -p biscuit-terminal --lib gitignore_matcher
```

## Production Readiness

Ready for production. All user-observable requirements have verification at
the appropriate level, and no functional blocker was found. The omitted error
table row should be corrected as routine documentation maintenance.
