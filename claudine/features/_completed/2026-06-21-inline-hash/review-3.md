---
ready: true
agent: codex/default
created: 2026-06-25T11:39:32
---

# Review 3 - `inline-compose` Document Hashing

## Verdict

Ready for production.

The review-2 blocking issue is fixed: markdown cleanup now runs inside the inline
closure before unchanged-body detection, hash stamping, and the single atomic
write. The final on-disk body is therefore the body that is hashed, and the CLI
integration test now uses intentionally dirty provider output and verifies
`md hash --diff` exits 0.

## Findings

No blocking findings.

## Requirement Verification

- **Successful inline closure stamps `hash:` as Darkmatter Simple** - Level 1
  library coverage in
  [closure.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/closure.rs:985)
  verifies the two 16-hex segment shorthand.
- **Stored hash describes the final on-disk document** - Level 1 library
  coverage in
  [closure.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/closure.rs:911)
  verifies dirty generated Markdown is cleaned before hashing and that the
  stored hash matches the written document. Level 1 CLI coverage in
  [inline_compose_hash.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/inline_compose_hash.rs:75)
  exercises a real `claudine inline-compose` run and then runs `md hash --diff`.
- **Body-unchanged detection uses the same Simple body segment source of truth** -
  Level 1 library coverage remains in
  [closure.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/closure.rs:489).
- **Malformed existing `hash:` fails before write** - Level 1 library coverage in
  [closure.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/closure.rs:1115)
  checks the typed `InlineHashMalformed` path and unchanged file contents.
- **Existing non-Simple hashes are normalized to Simple** - Level 1 library
  coverage in
  [closure.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/closure.rs:1061)
  verifies structured input is rewritten as Simple and still compares cleanly.
- **Frontmatter-change signal is informational and excludes managed keys** -
  Level 1 library coverage in
  [closure.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/closure.rs:1149)
  and
  [closure.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/closure.rs:1183)
  verifies new-key and reverted-modification behavior.
- **Determinism and idempotent hash-save behavior** - Level 1 library coverage in
  [closure.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/closure.rs:1222)
  and
  [closure.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/closure.rs:1274)
  covers the stable-save and fixed-input paths.

Level 2 and Level 3 coverage are not required for this feature. The requirements
are file mutation and hash comparison behavior; they do not depend on terminal
emulator rendering, terminal input encoding, OS keyboard injection, mouse input,
paste, IME, or scrolling.

## Non-Blocking Notes

The CLI integration test resolves `md` via `CARGO_BIN_EXE_md` when available and
falls back to `target/debug/md`. The test passed in this worktree, but a cleaner
long-term shape would avoid relying on a prebuilt sibling binary by asserting the
same comparison through Darkmatter's library API or by giving the test an
explicit build step for `md`. This is test-maintenance risk, not a feature
correctness blocker.

## Verification Attempted

```text
GIT_TERMINAL_PROMPT=0 cargo nextest run -p claudine -p claudine-cli -E 'test(/apply_closure_/) + test(=inline_compose_writes_hash_that_passes_md_diff) + test(/try_inline_closure_/)' --no-tests=fail --color=never
```

Result: passed, 17 tests run, 17 passed, 4727 skipped.
