---
ready: false
agent: codex
model: ""
---

# Review: Comment Quality — Iteration 2

## Findings

### High: the final checker baseline is not clean

The spec defines the cleanup target as `claudine/lib/src/` and `claudine/cli/src/`, and the implementation plan records the final heuristic check as empty. In the current tree, `just check-comments claudine/lib/src claudine/cli/src` still emits three findings:

- [system_prompt.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/prompt_reporting/system_prompt.rs:32) — `long-doc-short-fn`
- [prepare.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/prepare.rs:233) — `long-doc-short-fn`
- [magic_at.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/completion/composition/magic_at.rs:20) — `long-doc-short-fn`

The first finding is also a real rubric miss, not just a noisy heuristic: the docblock still narrates branch-by-branch formatting, glyph codepoints, and Prose link/color syntax immediately above a small function. That is exactly the kind of HOW / glyph / format narration this feature is meant to clean up.

Verification level present: Level 1 process verification. The new fixture tests pass, but the user-facing `just check-comments ...` command still reports findings in the required scope. This is not ready until the target baseline is clean or the remaining findings are explicitly documented as accepted false positives outside the acceptance criteria.

### High: the feature change set still contains out-of-scope functional changes

The spec is explicit that the cleanup pass does not modify behavior: "only comments and (where allowed) `#[allow(missing_docs)]` placement." The current feature branch/change set includes substantial functional changes outside the requested cleanup scope. Examples:

- [sync.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:374) changes remote-signal advertisement semantics from "send our own-namespace state" to "send all chunks we have" and stops filtering remote advertisements by peer namespace.
- [session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:680) changes remote update ingestion to require and validate chunk metadata from the incoming Loro document instead of constructing local metadata.

Those may be valid remote-signal fixes, but they are not implementation of the comment-quality spec, and they are not comment-only. They broaden the blast radius enough that this feature cannot be marked production-ready as a comment cleanup. Split them into their own feature/review path or remove them from this change set.

Verification level present: Level 1 unit tests were added around some remote metadata rejection paths, but that does not address the comment-quality acceptance contract. No Level 2/Level 3 terminal verification is relevant here because these are not terminal UX requirements.

## Verification

- `./scripts/check-comments-tests.sh` passed: 11 Level 1 fixture cases.
- `just check-comments claudine/lib/src claudine/cli/src` exited 0 as designed, but printed the three findings listed above.
- I attempted `cargo test -p claudine --lib --color=never` and `cargo test -p claudine-cli --test wrap_commands --color=never`; both ran into Cargo package/artifact lock contention and lengthy rebuilds. I stopped the processes rather than leaving long-running commands active in this non-interactive review.

## Verdict

Not ready for production. The checker/test gap from the first review is mostly addressed, but the required cleanup baseline is still not clean, and the change set still contains non-comment functional changes outside the feature scope.

## Response (2026-05-26)

### Finding #1 — Addressed

Cleaned up the three `long-doc-short-fn` findings in `claudine/lib/src` and
`claudine/cli/src`:

- `claudine/lib/src/prompt_reporting/system_prompt.rs:32`
  (`resolve_display_label`) — rewrote the docblock to drop the numbered
  branch-by-branch HOW narration, the `\u{F02A2}` codepoint quote, and the
  literal `<blue-400>[label](file://abs)</blue-400>` Prose-syntax quote that
  would drift if the constant or Prose API changed. Kept the contract: plain
  text out, callers handle styling/OSC8, and the three conditions that
  determine label form (linked to [`NERD_FONT_REPO_GLYPH`] instead of
  quoting it).
- `claudine/lib/src/composition/prepare.rs:233`
  (`parse_selection_hints_from_frontmatter`) — trimmed the contract paragraph
  to fold the "literal vs templated" rule into the eager-resolution WHY,
  bringing the docblock under the 15-line heuristic threshold while keeping
  both the criterion-B (why eager) and criterion-A (literal-only contract)
  content.
- `claudine/cli/src/completion/composition/magic_at.rs:20` (`gather_magic`)
  — collapsed the file-tier shadow prose that duplicated the module-level
  `//!` block and dropped the spec §5.5 table (mirroring spec inline is
  exactly the drift trap the rubric flags). Kept the `dir`-argument
  narrowing contract because it is a non-obvious coupling not derivable
  from the type.

`just check-comments claudine/lib/src claudine/cli/src` now exits 0 with no
findings. `cargo test -p claudine --lib prompt_reporting::` (115 tests),
`cargo test -p claudine --lib composition::prepare` (20 tests), and
`cargo test -p claudine-cli --bins magic` (16 tests) all pass after the
edits. `./scripts/check-comments-tests.sh` still passes all 11 fixture
cases.

### Finding #2 — Rejected (out-of-scope scoping error)

The reviewer saw the full branch diff and conflated two independently
tracked features that share the `claudine` working branch. The
`remote-signal/daemon/src/sync.rs` and `remote-signal/daemon/src/session_log.rs`
changes belong to feature
[`2026-05-24-remote-signal`](../2026-05-24-remote-signal/spec.md), which is
on its own review cycle (currently at
[`review-12.md`](../2026-05-24-remote-signal/review-12.md)). They are not
part of the `2026-05-25-comment-quality` change set — they are part of a
parallel feature that happens to be developed on the same branch.

Verifying: `git log --oneline main..HEAD` shows 44 commits across both
feature directories; the remote-signal commits (`2d6c75097`, `b1e164a35`,
`7cf53770d`, etc.) have `refactor(remote-signal…)` / `fix(remote-signal…)`
subjects and were never claimed as comment-quality work in any of this
feature's commits. The comment-quality spec's "comments only" rule applies
to *this feature's* commits — which it does.

No remote-signal changes were reverted as part of this response.
