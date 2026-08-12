---
ready: false
agent: codex/default
created: 2026-06-25T09:51:29
implemented: true
---

# Review 2 - `inline-compose` Document Hashing

## Verdict

Not ready for production.

The library-level hash stamping path now compiles and the focused library tests
I ran pass. However, the production CLI path can still leave a stale stored
`hash:` after markdown cleanup rewrites the body. This is the same user-facing
contract gap called out in review 1, and the new Level 1 cleanup test does not
verify the final hash after cleanup.

## Findings

### High - CLI cleanup still invalidates the just-stamped `hash:`

**Requirement:** every successful `inline-compose` closure writes a `hash:`
frontmatter property describing the final on-disk document, and `md hash --diff`
round-trips with no false positives.

**Evidence:** `apply_inline_closure` stamps and atomically writes the hash before
returning
([closure.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/closure.rs:121)).
The actual CLI closure path then calls `cleanup_inline_output`
([inline.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/inline.rs:176)).
That cleanup function applies `darkmatter::markdown::cleanup::cleanup_content`
to the body and writes the file again when the body changes
([inline_cleanup.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/composition/inline_cleanup.rs:25)).

The new regression test exercises this dirty-body path and confirms cleanup
rewrites `# Generated Title\nParagraph...` into `# Generated Title\n\nParagraph...`
([inline.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/inline.rs:237)),
but it never recomputes the stored hash or runs `md hash --diff` afterward. The
stored hash is still computed for the pre-cleanup body, while the final file can
contain the post-cleanup body.

**Verification level:** current coverage is Level 1. That is the right level for
this file-mutation requirement, but the strongest Level 1 test for the dirty
cleanup path checks body cleanup only, not the final `hash:` contract. The CLI
`md hash --diff` integration test uses already-clean provider output, so it does
not exercise the path that mutates the body after stamping.

**Suggested fix:** make cleanup part of the text that is hashed and atomically
written, or re-stamp after cleanup whenever cleanup changes the body. Add a
Level 1 CLI/closure-path test with dirty provider output that asserts the final
file passes `md hash --diff` or `Markdown::compare_hash`.

### Medium - Cross-platform CLI hash verification is still Unix-only

**Requirement:** all packages in this monorepo must compile and work on macOS,
Windows, and Linux. The feature applies to direct `claudine inline-compose` runs,
not only the pure library closure.

**Evidence:** the end-to-end CLI hash test is gated with `#[cfg(unix)]`
([inline_compose_hash.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/inline_compose_hash.rs:45))
and uses a shell-script fake `goose` provider. There is no Windows-compatible
Level 1 CLI test that verifies `inline-compose` stamps a hash which `md hash
--diff` accepts.

**Verification level:** Level 1 is appropriate for this requirement because no
real terminal, keyboard encoder, or rendered glyph behavior is involved.
Coverage exists for Unix only; Windows relies on library tests and does not
exercise the CLI provider path.

**Suggested fix:** use a cross-platform fake provider executable or shared test
helper so the same CLI hash assertion runs on Windows.

## Coverage Notes

The spec's library-level requirements are well represented by Level 1 tests:
Simple hash stamping, self-reference stability, malformed-hash rejection before
write, unchanged-body rejection via the Simple body segment, non-Simple
normalization, frontmatter-change signaling, and deterministic output are all
covered in `composition::closure`.

No Level 2 or Level 3 tests are required for the core hashing contract because
the feature mutates files and does not specify terminal rendering, terminal input
encoding, keyboard behavior, mouse behavior, or paste/IME behavior.

## Verification Attempted

- `cargo nextest run -p claudine apply_closure_hash_is_self_referentially_stable apply_closure_downgrades_structured_hash_to_simple --color=never`
  - Result: passed, 2 tests run.
- `cargo nextest run -p claudine-cli try_inline_closure_writes_cleaned_body_to_disk --color=never`
  - Result: passed, 1 test run.

These runs confirm compile health for the focused library and CLI cleanup tests,
but they do not close the final-file hash drift gap above.
