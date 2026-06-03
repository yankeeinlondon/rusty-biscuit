---
ready: false
agent: codex
model: ""
---

# Review 4

## Findings

### High: detailed section alignment can misclassify remove/add edits as rename-plus-edit

The detailed diff fallback pairs any remaining new section with the first unused old section at the same heading level, regardless of whether they occupy a corresponding relative position. The spec narrows that tie-break to sections at the same relative position at the same level, with leftovers becoming added/removed after positional pairing.

Relevant code:

- `darkmatter/lib/src/markdown/hash/explain.rs:560` says the fallback is "corresponding position at the same level"
- `darkmatter/lib/src/markdown/hash/explain.rs:567` actually uses the first unused old section with the same level:

```rust
if let Some(j) = (0..old.len()).find(|&j| !old_used[j] && old[j].level == level) {
```

This can produce the wrong user-facing report. For example:

```markdown
# A

a

# B

b
```

changed to:

```markdown
# B

b

# C

c
```

Pass 1 anchors `B`. Pass 3 then pairs new `C` with old `A` solely because both are H1, reporting `"C" section: heading renamed and content has changed` instead of reporting `C` added and `A` removed. That violates the detailed alignment rules in `spec.md:263-268` and makes `md hash --diff` / `md hash --save` explanations misleading for common section deletion plus addition edits.

Add a regression around this exact shape, then constrain pass 3 to true corresponding positions among the remaining same-level candidates. If there is no corresponding candidate after already-anchored siblings are accounted for, leave the sections unmatched so they surface as added/removed.

Verification level: Level 1 is appropriate here because this is pure library/CLI diff classification, not terminal rendering or input encoding. Current Level 1 tests cover renames, rename-plus-edit, reorder, added, and removed sections separately, but not this mixed anchored-sibling remove/add case.

## Production Readiness

Not ready for production. The core hashing, save, ignore-policy, and CLI surfaces are broadly implemented, and no Level 2/Level 3 terminal coverage is required for this feature. However, detailed diff output is a user-facing requirement, and the remaining alignment bug can produce materially incorrect explanations.

## Verification Notes

I attempted focused test runs:

- `cargo test -p darkmatter hash:: --color=never`
- `cargo test -p darkmatter-cli test_hash --test cli --color=never`

Both were still compiling dependencies when this review was written; no failing test output was observed before the review file was saved.
