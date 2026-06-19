---
ready: true
agent: codex
model: ""
---

# Review 6 - Darkmatter Cutover

## Findings

No findings.

The Review 5 documentation drift is resolved: the architecture docs now
accurately describe `ComponentPolicy` as layout plus `StyleColor` values, with
color projected onto node `Style` and browser opacity retained through the
`darkmatter.style` hint.

The Review 4 browser centering fix emits automatic side margins for the
default-margin, max-width-constrained page frame. Its browser-tier test verifies
equal, non-zero used side margins in Chromium; the focused Level 1 tests also
cover centered default margins and preservation of explicitly authored margins.

## Requirement Status

| Requirement | Status | Strongest verification |
|---|---:|---|
| Deprecated page-layout types and related allows removed | Met | Level 1 mechanical search/build |
| Bespoke component CSS and `LayoutContext` component math removed | Met | Level 1 mechanical search |
| Component policy carries renderable layout and color truth end to end | Met | Level 1 unit tests |
| Component color opacity survives browser rendering | Met | Browser-tier computed style |
| Terminal component layout/color behavior | Met | Level 2 real-terminal capture |
| Browser component layout/style, including percentage width | Met | Browser-tier used geometry/computed style |
| Terminal page frame, percentage sizing, and pronounced mode | Met | Level 1 plus Level 2 |
| Browser page max-width centering | Met | Browser-tier used margins |
| `style:` v1 parsing and strict warning surface | Met | Level 1 parser/CLI tests |
| Documentation updated and accurate | Met | Manual review |

No Level 3 coverage is required because this feature has no keyboard, mouse,
paste, or other OS-input behavior.

## Verification

- `cargo test -p darkmatter browser_render_with_max_width --lib --color=never`:
  passed.
- `cargo test -p darkmatter browser_render_authored_side_margins_suppress_centering --lib --color=never`:
  passed.
- `git diff --check`: passed.
- Mechanical searches found no removed vocabulary or helpers in active
  darkmatter implementation code and no stale `ComponentPolicy.style`
  description in the corrected authoritative docs.
- The focused browser-tier test and `cargo doc` were started, but cold/shared
  Cargo work exceeded the non-interactive command limit and was terminated.
  Their prior recorded runs passed; this review does not claim a fresh pass.
