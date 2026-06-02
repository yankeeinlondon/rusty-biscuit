---
agent: codex
model: ""
ready: false
---

# Review: Ternary-Conditional Commands in Frontmatter Shell Expansion

## Findings

### High: branch argument interpolation can synthesize new executable actions

The implementation now anchors ternary branch boundaries to the original source, which addresses the prior boundary-shift issue. However, each pipeline branch is still stored as raw original text, interpolated later, and then tokenized as shell syntax:

- [frontmatter_shell_expansion.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:377)
- [frontmatter_shell_expansion.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:861)
- [frontmatter_shell_expansion.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:862)

That means an interpolation in an argument position can introduce `&&` / `||` and create additional pipeline actions after the executable-interpolation check has already passed. For example:

```yaml
spec: "README.md' && date && echo '"
out: "$({{has_spec}} ? basename '{{spec}}' : '')"
```

The original then-branch has a static executable, `basename`, so `validate_branch_no_executable_interpolation` accepts it. After interpolation, tokenization sees a chain that includes `date`; `date` was not statically present in either branch. If `date` is allowlisted by executable prefix, this violates the spec's invariant that every executable the directive could run is statically determinable at parse time.

Suggested fix: preserve branch shape across interpolation, not just branch boundaries. Either interpolate only into already-tokenized argument slots with shell metacharacters treated as literal argument text, or validate that the post-interpolation pipeline has the same action count, operators, and executable tokens as the original branch. Add Level 1 tests where an interpolated branch argument contains `&& date` and where quoted interpolation contains a quote that would break out of the original quoted argument.

Verification level present: Level 1 parser/execution tests cover interpolated arguments, but only benign values. Level 1 is the correct level for this non-terminal behavior; the missing cases are security-shape tests.

### Medium: valid branch pipelines with colon arguments are rejected

After splitting a ternary, the code rejects any top-level `?` or `:` in either branch:

- [frontmatter_shell_expansion.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:213)
- [frontmatter_shell_expansion.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:397)

Rejecting an additional top-level `?` is consistent with the v1 nested-ternary non-goal, but rejecting any colon is broader than the spec and broader than the existing pipeline grammar. A branch such as this is a normal command pipeline and should not be classified as a nested ternary:

```yaml
out: "$(flag ? echo http://example.com : '')"
```

The splitter has already consumed the ternary separator. A later colon in branch text can be ordinary argument content, especially in URLs, times, drive-like strings, or key/value formats. The existing tokenizer accepts `:` inside words, so this is a regression from "branches are command pipelines as today."

Suggested fix: reject nested ternaries by detecting an additional top-level `?` with a matching later top-level `:`, or by parsing branch text with clearer ternary state. Do not reject a bare colon in otherwise valid pipeline text. Add Level 1 parser and compose tests for unquoted colon arguments in then and else branches.

Verification level present: Level 1 tests cover nested ternary rejection, but not valid colon-bearing command arguments. Level 1 is sufficient here.

## Test Rigor

This feature is frontmatter parsing, expression evaluation, shell allowlist validation, and command execution. There are no terminal rendering, keypress, paste, IME, mouse, or real-terminal encoder/decoder requirements in the spec, so Level 2 and Level 3 tests are not required for production readiness.

The existing and added tests are Level 1, which is the appropriate tier, but the Level 1 suite still misses the two cases above.

## Verification

I attempted `cargo test -p darkmatter ternary --color=never`. It was still compiling dependencies after roughly 90 seconds, so I stopped it per the non-interactive session guidance. No completed test result is available from this review pass.

## Production Readiness

Not ready for production. The implementation still has a static-command-invariant gap around interpolated branch arguments, and it rejects valid pipeline branch syntax containing ordinary colon arguments.
