---
area: claudine
status: draft-spec
created: 2026-10-20
origin: 2026-10-20-handle-indented-frontmatter
packages:
    - darkmatter
---

# Handle-indented-frontmatter follow-ups

Items still open after the follow-up pass on review 1 of
`2026-10-20-handle-indented-frontmatter`. The review marked that fix
production ready with no findings. See
[review-1.md](../2026-10-20-handle-indented-frontmatter/review-1.md) for
the full context.

## O1: Existing formatting guide incorrectly excludes tab recovery

- **Defect:** the `frontmatter` rule section of Darkmatter's formatting guide
  says frontmatter with tab indentation is kept exactly as written and gets no
  repair. Cleanup actually recovers tab indentation. The `md clean` reference,
  the cleanup tests in `darkmatter/lib/tests/l1/clean_frontmatter_repair.rs`,
  and the DMLS formatting tests in
  `darkmatter/dmls/tests/l1/formatting_transaction.rs` all show the
  recovery happening. They also show the new `yaml.root-indentation` repair
  applying after tab recovery.
- **Where:** `darkmatter/docs/topics/markdown-formatting.md`, in the
  `### frontmatter` section. It is the paragraph beginning "Frontmatter that
  strict YAML rejects but Darkmatter itself reads".
- **Why not fixed now:** this was a review observation, not a finding. The
  statement predates this fix, and the fix did not change it. The follow-up
  pass does not repair observations.
- **What fixing it looks like:** take tab indentation out of that paragraph's
  "kept exactly as written" examples, so the bare `{{ … }}` value is the only
  one left. Then add a sentence saying tab-indented frontmatter is recovered.
  That sentence should link the repair list in `darkmatter/docs/cli/clean.md`.
  This is a documentation-only change, so no tests are needed.
