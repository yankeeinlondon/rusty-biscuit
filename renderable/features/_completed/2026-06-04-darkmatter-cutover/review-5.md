---
ready: true
agent: codex
model: ""
---

# Review 5 - Darkmatter Cutover

## Findings

### Medium - Authoritative architecture docs describe a `ComponentPolicy.style` field that does not exist

The implementation intentionally stores component colors as `StyleColor`
fields and projects them into node `Style`, with an additional render hint for
browser opacity (`darkmatter/lib/src/layout/page.rs:31-44`,
`darkmatter/lib/src/markdown/render_tree/decorate.rs:151-200`). This is the
behavior now covered by the browser opacity test.

Several authoritative documents still describe a different architecture:

- `darkmatter/lib/src/layout/mod.rs:4-10,44-56` says `ComponentPolicy` contains
  optional `style` and names `ComponentPolicy.style`.
- `renderable/docs/layout-and-style.md:425-433` says frontmatter lowers
  straight into `renderable::style::Style` via `ComponentPolicy`.
- `.claude/skills/darkmatter/SKILL.md:182-188` repeats the same claim.

This is not only terminology drift: it hides the `StyleColor` retention and
`darkmatter.style` opacity-hint path that a maintainer must preserve. Update
these locations to describe the implemented layout/color policy and its
target-specific opacity projection. The repository's documentation acceptance
criterion and comment-drift policy are not met while they contradict the code.

## Requirement Status

| Requirement | Status | Strongest verification |
|---|---:|---|
| Deprecated page-layout types and allows removed | Met | Level 1 mechanical search/build |
| Bespoke component CSS and `LayoutContext` component math removed | Met | Level 1 mechanical search |
| Component policy is the single source of layout/color truth | Met | Level 1 unit tests |
| Component color opacity survives browser rendering | Met | Browser-tier computed style |
| Terminal component layout/color and page-frame behavior | Met | Level 1 plus Level 2 |
| Browser component layout/style, including percentage width | Met | Browser-tier used geometry/computed style |
| Browser page max-width centering | Met | Browser-tier used margins |
| `style:` v1 parsing and strict warning surface | Met | Level 1 parser/CLI tests |
| Documentation updated and accurate | Not met | Manual review |

## Verification

- Mechanical searches found no removed vocabulary or helpers in active
  darkmatter implementation code.
- `git diff --check` passed.
- Focused browser and library test runs were started, but a cold/shared Cargo
  build exceeded the non-interactive command limit and was terminated before
  tests executed.
- No Level 3 coverage is required because this feature has no keyboard or
  mouse interaction requirement.
