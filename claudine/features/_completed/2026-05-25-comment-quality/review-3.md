---
ready: false
agent: codex
model: ""
---

# Review: Comment Quality — Iteration 3

## Findings

### High: the change set still contains user-visible prompt-rendering behavior changes

The spec is explicit that the cleanup pass "does not modify behavior" and is limited to comments plus allowed `#[allow(missing_docs)]` placement ([spec.md](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/features/2026-05-25-comment-quality/spec.md:186), [spec.md](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/features/2026-05-25-comment-quality/spec.md:281)). The current implementation still includes prompt-reporting rendering changes inside the comment-quality surface:

- [system_prompt.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/prompt_reporting/system_prompt.rs:24) emits a new rendered header with a `📔` emoji, background color markup, and changed spacing.
- [formatting.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/prompt_reporting/formatting.rs:13) changes prompt/blockquote chrome constants, including a solid user-prompt border and a background-painted system-prompt border.
- [system_prompt.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/prompt_reporting/system_prompt.rs:175) now relies on the header/block quote alignment as rendered terminal geometry.

Those may be desirable prompt-reporting changes, but they are not implementation of this comment-quality spec. They change user-observable terminal output, snapshots, glyph width assumptions, and SGR styling. That violates the acceptance contract for this feature and broadens the review burden beyond a comment cleanup.

Verification level present: Level 1 only. Existing assertions/snapshots check strings after manufactured rendering, but the changed behavior depends on real terminal glyph widths and SGR rendering. If these rendering changes stay in scope, they need at least Level 2 real-terminal capture coverage for the visible header/block quote geometry. If this remains a comment-quality feature, the rendering changes should be split out or reverted from this change set.

## Coverage Notes

No Level 2 or Level 3 verification is required for the comment-quality rubric itself. Its user-facing executable behavior is `just check-comments`, which is appropriately covered by Level 1 process tests that run the script against temporary Rust fixtures.

If the prompt-rendering changes above remain, they introduce terminal UX requirements and need Level 2 coverage. No OS keyboard-input behavior is part of this feature, so Level 3 is not applicable.

## Verification

- `just check-comments claudine` passed with no findings.
- `./scripts/check-comments-tests.sh` passed all 11 Level 1 fixture cases.
- I attempted `cargo test -p claudine --lib --color=never` and `cargo test -p claudine-cli --bins --color=never`; both blocked on Cargo package/artifact locks. I terminated only the two Cargo PIDs launched from this workspace rather than leaving stuck processes running.
- I did not run `cargo doc -p claudine` / `cargo doc -p claudine-cli` because Cargo was already blocked on the same locks.

## Verdict

Not ready for production as this feature. The rubric docs, checker, and checker tests look materially aligned now, but the implementation still contains out-of-scope terminal rendering behavior changes that should be split into the prompt-reporting work or independently reviewed with appropriate Level 2 terminal coverage.

## Response (2026-05-26)

### Finding — Addressed by reverting the rendering changes

The rendering-only portion of commit `44a038fc3`
("refactor(claudine): use emoji headers and solid-block quotes in prompt
reporting") was reverted on top of the branch. The revert restores:

- `PROMPT_BORDER` to `┃ ` and `SYSTEM_PROMPT_BORDER` to `┃ ` in
  [formatting.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/prompt_reporting/formatting.rs).
- The `■` marker glyph and orange-text (no background bar) header in
  [system_prompt.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/prompt_reporting/system_prompt.rs)'s
  `render_system_prompt_header`.
- The `■` marker glyph in
  [user_prompt.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/prompt_reporting/user_prompt.rs)'s
  `render_user_prompt_header`.
- The `{NERD_FONT_REPO_GLYPH} /` (with leading space) format in
  `resolve_display_label`.
- The matching test assertions and the
  `wrap_commands__wrapper_reports_removed_sensitive_env_names.snap`
  snapshot.

The one piece of commit `44a038fc3` that is *kept* is the docstring
simplification on `resolve_display_label`, because that was an explicit
comment-quality cleanup recorded in the response to
[review-2.md](review-2.md) (drops branch-by-branch HOW narration, the
`\u{F02A2}` codepoint quote, and the literal `<blue-400>[label]…`
Prose-syntax quote). That docstring change has no behavior impact and
falls squarely inside the comment-quality acceptance contract.

After the revert this change set contains no user-visible terminal
output changes; no Level 2 terminal capture coverage is required and
the rendering refactor can be pursued in its own feature with the
appropriate verification when desired.

### Verification

- `just check-comments claudine/lib/src claudine/cli/src` exits 0 with
  no findings.
- `cargo test -p claudine --lib prompt_reporting::` passes.
- `./scripts/check-comments-tests.sh` still passes all 11 Level 1
  fixture cases.
