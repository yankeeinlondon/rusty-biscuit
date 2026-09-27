---
$schema: feature-review.yaml
ready: false
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/default
created: 2026-09-25T14:16:21-07:00
spec: 2026-09-24-ux-improvements/spec.md
implemented: true
next: 2026-09-24-ux-improvements/review-8.md
implemented_by: claude/opus
log: worktree/fixes/2026-09-24-ux-improvements/implementation-log.md
description: "A **fix** review of `2026-09-24-ux-improvements/spec.md`"
fix: 2026-09-24-ux-improvements/review-7.md
previous: 2026-09-24-ux-improvements/review-6.md
findings:
    - title: Lossy path encoding lets changed symlinks pass the removal handoff
      priority: high
---

# Review 7: Worktree UX improvements

## Verdict

**Not ready for production.** The previous finding's ordinary nested-repository case is fixed and its regression tests pass. A remaining defect in the same safety check lets a changed symlink pass approval verification and be deleted. The reproduction succeeded both at the worktree root and inside an untracked nested repository.

This review changes only review documents and specification metadata. All destructive reproductions used disposable repositories, with no network remote.

## Previous findings

| Review 6 unblocked finding | Assessment |
| --- | --- |
| Changes inside an untracked nested repository escape the handoff check and are deleted | Implemented for the reported case. The fingerprint now recursively reads dirty directories, including their nested Git metadata, sorts entries, reads symlink targets without following them, and propagates inspection errors. Library tests separately verify edits and new children. CLI tests verify refusal and preservation after changes, successful removal when unchanged, and refusal when a child becomes unreadable. All passed in this review. The encoding defect below remains. |

In the `worktree` package, [Inventory::fingerprint](../../lib/src/remove/inventory.rs:99), which records the contents approved for deletion, and [collect_inventory](../../lib/src/remove/inventory.rs:146), which gathers dirty paths, now document the nested-repository exception correctly. The prior misleading claim that Git enumerates every untracked file was corrected. In `worktree-cli`, the [nested-repository regression tests](../../cli/tests/remove.rs:790) exercise the actual two-invocation removal flow.

Review 6 had no blocked findings or pending human decisions, so nothing needed to be unblocked. Including nested Git metadata and refusing unreadable contents follow the existing safety requirement; neither needs another design approval. The separately scheduled ignored-file and performance changes are not assumed to be part of this implementation.

## Unblocked Findings

### High: Lossy path encoding lets changed symlinks pass the removal handoff

In the `worktree` package, [entry_digest](../../lib/src/remove/inventory.rs:191), which records a file's contents or a symlink's target, converts the target with `to_string_lossy()`. Different invalid UTF-8 bytes become the same replacement character. Consequently, distinct symlink targets produce identical fingerprint input. The hash itself works correctly; information has already been discarded before hashing.

**Reproduced on this macOS checkout using the current debug binary:**

1. Create a disposable repository on `main` with one commit and a linked worktree `feat-x` on branch `feat/x` at that commit.
2. Create an untracked symlink `link` whose target bytes are `b"target-\xff"`. Python's byte-path form of `os.symlink` can create it; a symlink target need not exist.
3. From `feat-x`, run `WT_SHELL_WRAPPER=1 wt remove feat-x --force-worktree` with stdin redirected. It exits 0 and supplies a landing directory and handoff token.
4. Replace `link` with a symlink whose target bytes are `b"target-\xfe"`.
5. From the supplied landing directory, run `wt remove --handoff <token>`.

**Observed:** exit 0, `Removed worktree`, and `Deleted branch`. Both the worktree and `feat/x` branch are gone. Repeating this with `link` inside a newly initialized, untracked `nested/` repository produces the same result. Neither symlink target was followed; the deleted work is the changed symlink itself.

**Expected:** exit 3 and preservation of the worktree, registration, branch, and changed symlink. The initial force flag approves the initial contents; it does not waive the specification's check for changes between invocations.

The same information-loss pattern also affects filenames:

- In the `worktree` package, [list_directory](../../lib/src/remove/inventory.rs:209), which fingerprints children of dirty directories, converts child names lossily. Two distinct names that collapse to the same text can conceal a rename when their contents match.
- In the `worktree` package, [git_from_raw](../../lib/src/git.rs:135), which supplies NUL-separated Git status and index output, converts the entire output lossily before inventory parsing. On a filesystem accepting non-UTF-8 filenames, the resulting path can point at a nonexistent replacement-character name. Its content is then recorded as `absent`, so editing the real file need not change the fingerprint. This filename case is a code-derived consequence, not a claimed Linux reproduction: the local macOS filesystem rejected creation of the invalid-byte filename. The symlink cases above were executed successfully.

**Required change:** preserve exact filename and symlink-target information through inventory collection and hashing, using an unambiguous encoding, or refuse before mutation when a path cannot be represented faithfully. Do not use display-oriented lossy conversion as deletion evidence. Keep symlink traversal disabled. Address the Git-output boundary as well as the new directory walk so the same defect does not remain for ordinary dirty files. The two invocations run on the same machine; byte-faithful identity matters more than producing identical hashes across different operating systems.

**Verification needed: Level 1.** Add library and CLI regressions for changing a symlink between the two byte targets above, both directly and inside an untracked nested repository. Assert exit 3 and preservation of the changed target, worktree registration, and local branch. On an appropriate Unix filesystem, cover an edited non-UTF-8 filename and a nested child renamed between two names that would decode to the same replacement text. If unsupported names are deliberately refused, assert that refusal happens before any removal. Include an unchanged-content success case. Keep these tests in declared targets selected by the live Level 1 recipe; no terminal or physical-keyboard test is needed for this defect.

## Blocked Findings

None. This finding concerns the existing promise to reject changes after approval and requires no new user decision.

## Requirement verification

Level 1 verifies logic and subprocess behavior. Level 2 verifies rendering and shell flows inside a real terminal. This specification introduces no physical-key encoding, modifier, hotkey, paste, IME, or mouse requirement needing Level 3. Existing prompt-answer injection verifies the specified removal flow without claiming to test a terminal's keyboard encoder.

| User-observable requirement | Strongest relevant verification present | Assessment |
| --- | --- | --- |
| Completion names, `base`, detached worktrees, ambiguous names, base-removal refusal | Level 1 resolver and CLI tests | Passed. |
| Safety tiers, PR source/head identity, independent force flags, dirty/ignored consent, branch retention, exit codes | Level 1 policy, repository, and CLI tests | Passed. |
| Remote destination approval, live heads, multiple push addresses, rewrite/name collisions, deletion lease | Level 1 local-remote and CLI tests | Passed. |
| Handoff content/index checks, changed branch state, token expiry/replay, unchanged nested-directory removal | Level 1 library and CLI tests; Level 2 failure scenes | Existing tests passed; changed byte-valued symlink targets are missing and fail in reproduction. |
| Report before questions, one blank line, answers default to preserving work | Level 2 tmux captures plus Level 1 policy tests | Passed. |
| Dirty-tree glyphs/colors and count replacing more than ten paths | Level 2 styled captures plus Level 1 rendering tests | Passed. |
| Bash, zsh, fish move-first flow, fork-parent/base landing, subdirectories, failed movement | Level 2 tmux shell scenes plus Level 1 wrapper tests | Passed. |
| PowerShell moves both directory states; a held Windows directory refuses before removal | Windows Level 2 console-screen tests and Level 1 wrapper tests | Appropriate tests exist; Windows execution not repeated here. |
| Local `--from`, existing/invalid/detached cases, fork records, explicit wrapper detection | Level 1 repository and CLI tests | Passed. |
| Caption/target choice, parent rows, deleted parents, merge vocabulary, PR placement and cache age | Level 1 data tests/snapshots and Level 2 displayed table | Passed. |
| Dot/badge colors, connector colors, legend, current-row emphasis on light/dark backgrounds | Level 2 styled-cell assertions plus Level 1 styles | Passed. |
| Linked PR badges when supported; number-only fallback otherwise | Level 1 link/fallback assertions and Level 2 displayed badges | Existing coverage retained. |
| Graph facts, lanes/tags, origin/fork relationships, trimming and scale arithmetic | Level 1 worktree/component tests | Worktree tests passed; component suites not rerun. |
| Actual graph pixels, narrow width, half-height limit, omitted-lane notice, table surviving image output | Level 2 private-Kitty screenshot and transmission assertions | Appropriate tests exist. Not rerun; earlier review records describe a local screenshot-capture limitation and earlier passing implementation runs. No fresh pixel-rendering result is claimed. |
| Warm/cold gathering, full-command time, unavailable/slow PR request, fresh-cache request suppression | Level 1 serial performance and request-count tests | All 17 performance tests passed. |
| Four-provider PR normalization and renderer pie-color compatibility | Level 1 provider fixtures and renderer tests | Earlier coverage retained; dependency suites not rerun. |

No additional requirement was identified whose strongest existing test is at the wrong verification level. The finding is an uncovered Level 1 state case. Cross-OS execution evidence and human sign-off are not readiness blockers for this review.

The new CLI regressions are compiled by the automatically discovered `remove` integration target; the library regressions compile in the library unit-test target. Their names select Level 1. Terminal targets declare `terminal-tests`, enabled by the live Level 2 recipe and CI feature metadata. The tier audit reports zero stranded tests.

## Validation and metadata

- `worktree/just test`: **317 passed**, 17 excluded by the area filter.
- `worktree/just test-perf`: **17 passed**, using the serial recipe.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_dirty level2_list level2_move level2_remove`: **9 passed**; backend proof reports nine tmux executions, zero skips, zero panics.
- `worktree/just lint`: **passed** for both packages.
- `just check-tier-coverage worktree`: **zero stranded tests**.
- Two disposable-repository reproductions confirmed removal after a symlink target changed between approval and handoff, at the root and inside a nested repository.

Kitty screenshot tests, cross-OS tests, and dependency suites were not rerun. No source or test files were changed, no formatting command was run, and no repository commit was created.

The requested previous-review path under `prompts/_reviews` does not exist. The existing review beside the spec was updated instead: `implemented: true` records the prior implementation, and `next` points to this review. The specification now has `review_iterations: 7`; it is not marked completed and remains in its current lifecycle directory.
