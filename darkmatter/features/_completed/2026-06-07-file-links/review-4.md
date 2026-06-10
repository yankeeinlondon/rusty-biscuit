---
agent: codex
model: ""
ready: false
---

# Review: File Links Directive, Iteration 4

## Findings

### High: the Level 2 test still does not verify the complete root and link contract

The revised real-terminal test now associates SGR state with the target,
dotfile, and gitignored filenames, which resolves most of the previous review's
false-positive risk. Two user-visible requirements remain below their required
verification level:

- The fixture expects a dimmed `/docs/` root prefix, but the test never asserts
  that `/docs/` is visible or that the prefix itself carries dim styling. The
  only Level 2 dim assertions now target `ignored.md` and `buried.md`.
- The test says every rendered file has its own correct OSC8 destination, but
  checks only four of the ten rendered files. Regressions that omit links from
  `.txt`, `.pdf`, `.xlsx`, `.docx`, dotfiles, or `beta.md` would leave it green.

References:

- [level2_render_tree_terminal.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/tests/level2_render_tree_terminal.rs:1582)
- [level2_render_tree_terminal.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/tests/level2_render_tree_terminal.rs:1651)
- [level2_render_tree_terminal.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/tests/level2_render_tree_terminal.rs:1685)

Suggested fix: assert that the visible root line contains `/docs/topics`, then
inspect the active SGR state at `/docs/` separately from `topics`. Build the
expected relative-file list once and use it for both the visible-name checks
and OSC8 destination checks so every rendered file is verified.

Verification level present: Level 1 for root-prefix rendering; Level 2 for four
representative file links. Required level: Level 2 for the rendered root style
and for every file's hyperlink contract.

### Medium: `--dir` accepts a regular file and silently treats it as an empty directory

Directory discovery checks only `target.exists()`. If the target is a regular
file, `collect_files()` calls `read_dir()`, silently drops the resulting error,
and returns an empty match:

- [discovery.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/file_links/discovery.rs:185)
- [discovery.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/file_links/discovery.rs:269)

For `::file-links --dir report.pdf`, strict mode therefore renders
`No matching files`, while permissive mode removes the directive with an empty
warning. Both outcomes misdiagnose invalid syntax: `--dir` requires a
directory.

Suggested fix: require `target.is_dir()` before discovery and return a
line-aware `TargetNotDirectory` error, or broaden the existing target error to
distinguish missing and non-directory paths. Add Level 1 compose tests for both
strict and permissive handling.

## Verification Matrix

| Requirement | Strongest effective verification | Assessment |
|---|---:|---|
| Glob/`--dir`, depth, extension filtering, boundaries, self-exclusion | Level 1 | Appropriate, except non-directory `--dir` input |
| In-bound and escaping symlinks | Level 1 | Appropriate |
| Concurrent transclusion resolution and single discovery walk | Structural/Level 1 | Appropriate |
| Visible hierarchy and extension glyphs | Level 2 | Appropriate |
| Repository icon, target highlight, dotfile italics, gitignored dimming | Level 2 | Appropriate |
| Dimmed boundary-relative root prefix | Level 1 | Gap |
| OSC8 link for every rendered file with correct destination | Partial Level 2 | Gap |

## Verification

Focused compilation and tests could not start because `rustup toolchain list`
reports `no installed toolchains`. The following commands failed before
compilation:

```text
cargo test --color=never -p biscuit-terminal --lib gitignore_matcher
cargo test --color=never -p darkmatter --lib file_links
cargo test --color=never -p biscuit-terminal --test filesystem_parity
cargo check --color=never -p darkmatter -p biscuit-terminal
```

`git diff --check` passed.

## Production Readiness

Not ready for production. The implementation is substantially improved, but
the required dimmed root prefix and complete per-file hyperlink behavior still
lack effective Level 2 verification, which is a production-readiness blocker
under the review's stated test-rigor policy.
