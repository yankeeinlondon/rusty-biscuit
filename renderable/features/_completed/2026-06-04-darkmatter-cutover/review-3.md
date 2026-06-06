---
ready: false
agent: codex
model: ""
---

# Review 3 - Darkmatter Cutover

## Findings

### High - The browser percentage test does not verify the used component width

The new browser-tier test reaches real Chrome, but it queries the computed
`max-width` declaration rather than the table's used width
(`darkmatter/lib/tests/browser_render.rs:622-649`). Chrome normally reports the
authored `50%` for `getComputedStyle(table).maxWidth`, so the fallback branch
only proves that the percentage survived serialization. It does not prove that
the table is constrained to half of its containing block, which was the
user-visible behavior required by review 2.

The fixture is also an intrinsically narrow two-column table (`TABLE_MD`), so
its rendered width can remain below the cap even when `max-width` works. A
broken or ignored cap could therefore produce the same visible result.

Exercise the constraint with content or an explicit width that would otherwise
exceed 50%, then compare the table's used pixel width with the used pixel width
of its actual containing block. The current harness exposes only computed-style
queries, so this can be done by asserting `width` after forcing the table wider,
or by adding a small geometry-query API that reads `getBoundingClientRect()`.

Verification level: browser tier is present, but the assertion does not verify
the observable behavior. Under the requested rigor policy, this remains a
high-severity production-readiness gap.

## Requirement Status

| Requirement | Status | Strongest verification |
|---|---:|---|
| Deprecated page-layout types and allows removed | Met | Level 1 mechanical search/build |
| Bespoke component CSS and `LayoutContext` component math removed | Met | Level 1 mechanical search |
| Component policy is the single source of layout/color truth | Met | Level 1 unit tests |
| Component color opacity survives browser rendering | Met | Browser-tier computed style |
| Terminal component layout/color behavior | Met | Level 2 real-terminal capture |
| Browser fixed-length component layout/style | Met | Browser-tier computed style |
| Browser percentage component layout | Verification gap | Browser tier checks declaration, not used geometry |
| Terminal percentage page-frame layout | Met | Level 2 real-terminal capture |
| Slim renderable-typed page frame and pronounced mode flip | Met | Level 1 plus Level 2/browser tests |
| Documentation updated | Met | Manual review |

## Verification

- Mechanical acceptance searches found no removed vocabulary or helpers outside
  historical feature documents and explanatory comments.
- `git diff --cached --check` passed before the review-2 fixes were committed.
- Focused Cargo test attempts could not complete within the non-interactive
  command limit because another process held the shared build-directory lock;
  compilation began after the lock cleared but exceeded the command window.
- No Level 3 coverage is required because this feature has no keyboard or mouse
  interaction requirement.
