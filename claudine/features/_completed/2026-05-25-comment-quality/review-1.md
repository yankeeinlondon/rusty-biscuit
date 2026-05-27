---
ready: false
agent: codex
model: ""
---

# Review: Comment Quality

## Findings

### High: The cleanup changed prompt-reporting behavior

The spec requires the cleanup commits to make no behavior changes: "only comments and (where allowed) `#[allow(missing_docs)]` placement" ([spec.md](spec.md#L186)). The implementation changes user-visible prompt reporting output:

- [formatting.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/prompt_reporting/formatting.rs:13) changes both prompt blockquote borders from `█ ` / orange background-fill cells to `┃ `.
- [system_prompt.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/prompt_reporting/system_prompt.rs:24) changes the system prompt header to add a leading newline, switch from the previous icon/background styling to an orange foreground `■`, and change spacing around the action text.
- [system_prompt.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/prompt_reporting/system_prompt.rs:56) changes Nerd Font hyperlink labels from `{glyph}/path` to `{glyph} /path`.
- [user_prompt.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/prompt_reporting/user_prompt.rs:15) changes the user prompt header from the speaking-head icon to a green foreground `■` and adds a leading newline.

The tests were updated to assert the new glyphs and prefixes, so they no longer protect the no-behavior-change acceptance criterion. Verification level present: Level 1 unit tests only, and they encode the changed behavior. Because this is terminal-rendered user-facing output, any intentional rendering change would need its own spec plus at least Level 2 terminal capture coverage for glyphs, widths, and styling. For this feature, the fix is to revert these output changes and keep the edits comment-only.

### High: `check-comments.sh` misses common Rust function shapes

The heuristic is supposed to flag long docblocks attached to short functions and redundant accessor docs ([spec.md](spec.md#L209)). The current parser only starts watching a function when the `fn` line itself contains an opening brace with positive brace depth ([scripts/check-comments.sh](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/scripts/check-comments.sh:167), [scripts/check-comments.sh](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/scripts/check-comments.sh:179)). It then clears the pending docblock even when the signature continues on following lines ([scripts/check-comments.sh](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/scripts/check-comments.sh:191)).

That means it misses multi-line signatures, which are common in this codebase:

```rust
/// doc
/// ... 16 lines total ...
pub fn accessor(
    &self,
) -> bool {
    self.enabled
}
```

It also misses `long-doc-short-fn` for one-line functions where `{` and `}` balance on the signature line, because `count_braces` returns zero and the script never watches the body. Verification level present: manual spot checks only. A Level 1 fixture test should cover single-line functions, multi-line signatures, multi-line bodies, and accepted false-positive/false-negative behavior before this is treated as ready.

### High: The heuristic script has no automated regression coverage

The new script is the only executable functionality in the feature, but there are no tests for the four promised categories (`long-doc-short-fn`, `arguments-block`, `heavy-example`, `redundant-accessor`) or for warn-only exit behavior. `rg` finds only the script, docs, and plan/spec references; there is no fixture test harness for `scripts/check-comments.sh`.

User-observable requirement: `just check-comments` should produce parseable findings for the specified suspicious patterns and exit 0. Strongest verification present: ad hoc manual runs against the current tree, which only prove the current tree is clean. Required minimum: Level 1 process tests that run the script against temporary Rust fixtures and assert stdout plus exit status.

### Medium: `docs/comment-quality.md` does not fully match the required shape

The acceptance criteria require `docs/comment-quality.md` to include "one expanded before/after pair per anti-pattern and per positive criterion" ([spec.md](spec.md#L272)). The anti-pattern sections have before/after examples, but the positive criteria sections are mostly "Example" plus rationale rather than before/after pairs, especially Criteria A-E ([docs/comment-quality.md](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/docs/comment-quality.md:300)). The content is useful, but it does not satisfy the documented acceptance criterion as written.

## Verification

- `./scripts/check-comments.sh claudine/lib/src claudine/cli/src` passed with no findings.
- `just check-comments` passed with no findings.
- `cargo test -p claudine --color=never` and `cargo test -p claudine-cli --color=never` were attempted, but both spent about 60 seconds blocked/rebuilding after Cargo file-lock waits. I stopped the runs to avoid hanging the non-interactive review, so the spec's test-pass acceptance criterion is not verified here.

## Verdict

Not ready for production. The feature violates its no-behavior-change contract and the new heuristic needs fixture-level regression tests before it can be trusted as review tooling.
